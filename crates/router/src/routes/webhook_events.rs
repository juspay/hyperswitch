use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, webhooks::webhook_events},
    routes::AppState,
    services::{
        api,
        authentication::{self as auth, UserFromToken},
        authorization::permissions::Permission,
    },
    types::api::webhook_events::{
        EventListConstraints, EventListRequestInternal, WebhookDeliveryAttemptListRequestInternal,
        WebhookDeliveryRetryRequestInternal,
    },
};

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventInitialDeliveryAttemptList))]
pub async fn list_initial_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
    json_payload: web::Json<EventListConstraints>,
) -> impl Responder {
    let flow = Flow::WebhookEventInitialDeliveryAttemptList;
    let merchant_id = path.into_inner();
    let constraints = json_payload.into_inner();

    let request_internal = EventListRequestInternal {
        merchant_id: merchant_id.clone(),
        constraints,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal, _| {
            webhook_events::list_initial_delivery_attempts(
                state,
                request_internal.merchant_id,
                request_internal.constraints,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantWebhookEventRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventInitialDeliveryAttemptList))]
pub async fn list_initial_webhook_delivery_attempts_with_jwtauth(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<EventListConstraints>,
) -> impl Responder {
    let flow = Flow::WebhookEventInitialDeliveryAttemptList;
    let constraints = json_payload.into_inner();

    let request_internal = EventListRequestInternal {
        merchant_id: common_utils::id_type::MerchantId::default(),
        constraints,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, auth: UserFromToken, mut request_internal, _| {
            let merchant_id = auth.merchant_id;
            let profile_id = auth.profile_id;

            request_internal.merchant_id = merchant_id;
            request_internal.constraints.profile_id = Some(profile_id);

            webhook_events::list_initial_delivery_attempts(
                state,
                request_internal.merchant_id,
                request_internal.constraints,
            )
        },
        &auth::JWTAuth {
            permission: Permission::ProfileWebhookEventRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventDeliveryAttemptList))]
pub async fn list_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::WebhookEventDeliveryAttemptList;
    let (merchant_id, initial_attempt_id) = path.into_inner();

    let request_internal = WebhookDeliveryAttemptListRequestInternal {
        merchant_id: merchant_id.clone(),
        initial_attempt_id,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal, _| {
            webhook_events::list_delivery_attempts(
                state,
                request_internal.merchant_id,
                request_internal.initial_attempt_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantWebhookEventRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventDeliveryRetry))]
#[cfg(feature = "v1")]
pub async fn retry_webhook_delivery_attempt(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::WebhookEventDeliveryRetry;
    let (merchant_id, event_id) = path.into_inner();

    let request_internal = WebhookDeliveryRetryRequestInternal {
        merchant_id: merchant_id.clone(),
        event_id,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal, _| {
            webhook_events::retry_delivery_attempt(
                state,
                request_internal.merchant_id,
                request_internal.event_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantWebhookEventWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
