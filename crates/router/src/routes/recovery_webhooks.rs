use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        webhooks::{self, types},
    },
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::IncomingWebhookReceive))]
pub async fn recovery_receive_incoming_webhook<W: types::OutgoingWebhookType>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
) -> impl Responder {
    let flow = Flow::RecoveryIncomingWebhookReceive;
    let (merchant_id, profile_id, connector_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        (),
        |state, auth, _, req_state| {
            webhooks::incoming_webhooks_wrapper::<W>(
                &flow,
                state.to_owned(),
                req_state,
                &req,
                auth.merchant_account,
                auth.profile,
                auth.key_store,
                &connector_id,
                body.clone(),
                false,
            )
        },
        &auth::MerchantIdAndProfileIdAuth {
            merchant_id,
            profile_id,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
