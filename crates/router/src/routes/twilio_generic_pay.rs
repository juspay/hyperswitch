use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, twilio_generic_pay as twilio_generic_pay_core},
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::TwilioGenericPayCharge))]
pub async fn twilio_generic_pay_charge(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::twilio_generic_pay::TwilioGenericPayRequest>,
) -> impl Responder {
    let flow = Flow::TwilioGenericPayCharge;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, req_state| {
            twilio_generic_pay_core::twilio_generic_pay_charge(state, req_state, auth, payload)
        },
        &auth::TwilioPayAuth,
        // No lock. The payments lock keys on `payment_id`, and there is no `payment_id` at this
        // point — the core generates one while mapping the Twilio request. Dedupe on Twilio's
        // `transaction_id` is out of scope for now; when it lands, that is the key to lock on.
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
