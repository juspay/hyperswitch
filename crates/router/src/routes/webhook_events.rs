use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, webhooks::webhook_events},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::webhook_events::{
        EventListConstraints, EventListRequestInternal, WebhookDeliveryAttemptListRequestInternal,
    },
};

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventInitialDeliveryAttemptList))]
pub async fn list_initial_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<EventListConstraints>,
) -> impl Responder {
    let flow = Flow::WebhookEventInitialDeliveryAttemptList;
    let merchant_id_or_profile_id = path.into_inner();
    let constraints = query.into_inner();

    let request_internal = EventListRequestInternal {
        merchant_id_or_profile_id: merchant_id_or_profile_id.clone(),
        constraints,
    };

    api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal| {
            webhook_events::list_initial_delivery_attempts(
                state,
                request_internal.merchant_id_or_profile_id,
                request_internal.constraints,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantOrProfileFromRoute {
                merchant_id_or_profile_id,
                required_permission: Permission::WebhookEventRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventDeliveryAttemptList))]
pub async fn list_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::WebhookEventDeliveryAttemptList;
    let (merchant_id_or_profile_id, initial_attempt_id) = path.into_inner();

    let request_internal = WebhookDeliveryAttemptListRequestInternal {
        merchant_id_or_profile_id: merchant_id_or_profile_id.clone(),
        initial_attempt_id,
    };

    api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal| {
            webhook_events::list_delivery_attempts(
                state,
                request_internal.merchant_id_or_profile_id,
                request_internal.initial_attempt_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantOrProfileFromRoute {
                merchant_id_or_profile_id,
                required_permission: Permission::WebhookEventRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
