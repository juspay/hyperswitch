//! Analysis for usage of Subscription in Payment flows
//!
//! Functions that are used to perform the api level configuration and retrieval
//! of various types under Subscriptions.

use std::str::FromStr;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use api_models::subscription as subscription_types;
use error_stack::report;
use hyperswitch_domain_models::errors;
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use crate::{
    core::{api_locking, subscription},
    headers::X_PROFILE_ID,
    routes::AppState,
    services::{api as oss_api, authentication as auth, authorization::permissions::Permission},
    types::domain,
};

fn extract_profile_id(req: &HttpRequest) -> Result<common_utils::id_type::ProfileId, HttpResponse> {
    let header_value = req.headers().get(X_PROFILE_ID).ok_or_else(|| {
        HttpResponse::BadRequest().json(
            errors::api_error_response::ApiErrorResponse::MissingRequiredField {
                field_name: X_PROFILE_ID,
            },
        )
    })?;

    let profile_str = header_value.to_str().unwrap_or_default();

    if profile_str.is_empty() {
        return Err(HttpResponse::BadRequest().json(
            errors::api_error_response::ApiErrorResponse::MissingRequiredField {
                field_name: X_PROFILE_ID,
            },
        ));
    }

    common_utils::id_type::ProfileId::from_str(profile_str).map_err(|_| {
        HttpResponse::BadRequest().json(
            errors::api_error_response::ApiErrorResponse::InvalidDataValue {
                field_name: X_PROFILE_ID,
            },
        )
    })
}

#[instrument(skip_all)]
pub async fn create_subscription(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<subscription_types::CreateSubscriptionRequest>,
) -> impl Responder {
    let flow = Flow::CreateSubscription;
    let profile_id = match extract_profile_id(&req) {
        Ok(id) => id,
        Err(response) => return response,
    };

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        move |state, auth: auth::AuthenticationData, payload, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::create_subscription(
                state,
                merchant_context,
                profile_id.clone(),
                payload.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileSubscriptionWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn confirm_subscription(
    state: web::Data<AppState>,
    req: HttpRequest,
    subscription_id: web::Path<common_utils::id_type::SubscriptionId>,
    json_payload: web::Json<subscription_types::ConfirmSubscriptionRequest>,
) -> impl Responder {
    let flow = Flow::ConfirmSubscription;
    let subscription_id = subscription_id.into_inner();
    let payload = json_payload.into_inner();
    let profile_id = match extract_profile_id(&req) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let api_auth = auth::ApiKeyAuth::default();

    let (auth_type, _) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok(auth) => auth,
            Err(err) => return oss_api::log_and_return_error_response(error_stack::report!(err)),
        };

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::confirm_subscription(
                state,
                merchant_context,
                profile_id.clone(),
                payload.clone(),
                subscription_id.clone(),
            )
        },
        auth::auth_type(
            &*auth_type,
            &auth::JWTAuth {
                permission: Permission::ProfileSubscriptionWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn get_subscription_plans(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<subscription_types::GetPlansQuery>,
) -> impl Responder {
    let flow = Flow::GetPlansForSubscription;
    let api_auth = auth::ApiKeyAuth::default();
    let payload = query.into_inner();

    let profile_id = match extract_profile_id(&req) {
        Ok(profile_id) => profile_id,
        Err(response) => return response,
    };

    let (auth_type, _) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok(auth) => auth,
            Err(err) => return oss_api::log_and_return_error_response(error_stack::report!(err)),
        };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, query, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::get_subscription_plans(state, merchant_context, profile_id.clone(), query)
        },
        auth::auth_type(
            &*auth_type,
            &auth::JWTAuth {
                permission: Permission::ProfileSubscriptionRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Add support for get subscription by id
#[instrument(skip_all)]
pub async fn get_subscription(
    state: web::Data<AppState>,
    req: HttpRequest,
    subscription_id: web::Path<common_utils::id_type::SubscriptionId>,
) -> impl Responder {
    let flow = Flow::GetSubscription;
    let subscription_id = subscription_id.into_inner();
    let profile_id = match extract_profile_id(&req) {
        Ok(id) => id,
        Err(response) => return response,
    };

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::get_subscription(
                state,
                merchant_context,
                profile_id.clone(),
                subscription_id.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileSubscriptionRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn create_and_confirm_subscription(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<subscription_types::CreateAndConfirmSubscriptionRequest>,
) -> impl Responder {
    let flow = Flow::CreateAndConfirmSubscription;
    let profile_id = match extract_profile_id(&req) {
        Ok(id) => id,
        Err(response) => return response,
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::create_and_confirm_subscription(
                state,
                merchant_context,
                profile_id.clone(),
                payload.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileSubscriptionWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// add support for get subscription estimate
#[instrument(skip_all)]
pub async fn get_estimate(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<subscription_types::EstimateSubscriptionQuery>,
) -> impl Responder {
    let flow = Flow::GetSubscriptionEstimate;
    let profile_id = match extract_profile_id(&req) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let api_auth = auth::ApiKeyAuth {
        is_connected_allowed: false,
        is_platform_allowed: false,
    };
    let (auth_type, _auth_flow) = match auth::get_auth_type_and_flow(req.headers(), api_auth) {
        Ok(auth) => auth,
        Err(err) => return oss_api::log_and_return_error_response(report!(err)),
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        query.into_inner(),
        |state, auth: auth::AuthenticationData, query, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::get_estimate(state, merchant_context, profile_id.clone(), query)
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn update_subscription(
    state: web::Data<AppState>,
    req: HttpRequest,
    subscription_id: web::Path<common_utils::id_type::SubscriptionId>,
    json_payload: web::Json<subscription_types::UpdateSubscriptionRequest>,
) -> impl Responder {
    let flow = Flow::UpdateSubscription;
    let subscription_id = subscription_id.into_inner();
    let profile_id = match extract_profile_id(&req) {
        Ok(id) => id,
        Err(response) => return response,
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::update_subscription(
                state,
                merchant_context,
                profile_id.clone(),
                subscription_id.clone(),
                payload.clone(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
