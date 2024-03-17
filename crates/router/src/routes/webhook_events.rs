use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, webhooks::webhook_events},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventDeliveryAttemptList))]
pub async fn list_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::WebhookEventDeliveryAttemptList;
    let (merchant_id, initial_event_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        (&merchant_id, &initial_event_id),
        |state, _, (merchant_id, initial_event_id)| {
            webhook_events::list_delivery_attempts(state, merchant_id, initial_event_id)
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::WebhookEventRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
