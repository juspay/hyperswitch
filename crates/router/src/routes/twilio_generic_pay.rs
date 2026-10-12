use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, generic_pay::GenericPayAdapter, twilio_generic_pay::TwilioGenericPay},
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
            TwilioGenericPay::charge(state, req_state, auth, payload)
        },
        &auth::TwilioPayAuth,
        // No lock — the payments lock keys on `payment_id`, which is inside the body. Retry safety
        // comes from the DB instead. With `parameters.payment_id` set, a retry collides on
        // `(payment_id, merchant_id)`.
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
