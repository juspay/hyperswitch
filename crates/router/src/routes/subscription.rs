//! Analysis for usage of Subscription in Payment flows
//!
//! Functions that are used to perform the api level configuration and retrieval
//! of various types under Subscriptions.

use actix_web::{web, HttpRequest, Responder};
use api_models::subscription as subscription_types;
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use crate::{
    core::{api_locking, subscription},
    routes::AppState,
    services::{api as oss_api, authentication as auth, authorization::permissions::Permission},
    types::domain,
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn create_subscription(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<subscription_types::CreateSubscriptionRequest>,
) -> impl Responder {
    let flow = Flow::CreateSubscription;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::create_subscription(state, merchant_context, payload.clone())
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

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn get_subscription_plans(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let client_secret = path.into_inner();
    let flow = Flow::GetPlansForSubscription;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        client_secret,
        |state, auth: auth::AuthenticationData, client_secret, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            subscription::get_subscription_plans(
                state,
                merchant_context,
                auth.profile_id,
                client_secret,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
