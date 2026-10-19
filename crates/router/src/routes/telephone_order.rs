use std::str::FromStr;

use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use router_env::{instrument, logger, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        errors,
        telephone_order::{
            twilio::TwilioGenericPay, TelephoneOrderAdapter, TelephoneOrderProvider,
        },
    },
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::TelephoneOrderCharge))]
pub async fn telephone_order_charge(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(String, common_utils::id_type::MerchantId)>,
) -> HttpResponse {
    let (provider, merchant_id) = path.into_inner();

    let Ok(provider) = TelephoneOrderProvider::from_str(&provider) else {
        return errors::ApiErrorResponse::InvalidRequestUrl.error_response();
    };

    match provider {
        TelephoneOrderProvider::Twilio => {
            charge_with::<TwilioGenericPay>(state, req, body, merchant_id).await
        }
    }
}

/// The route for one adapter: a new provider is a `match` arm above and a core module.
#[cfg(feature = "v1")]
async fn charge_with<A>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    merchant_id: common_utils::id_type::MerchantId,
) -> HttpResponse
where
    A: TelephoneOrderAdapter + Send,
{
    let flow = Flow::TelephoneOrderCharge;

    let payload = match serde_json::from_slice::<A::Request>(&body) {
        Ok(payload) => payload,
        Err(error) => {
            logger::warn!("Generic pay request body could not be deserialized: {error:?}");
            return errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "request body".to_string(),
                expected_format: "a telephone order charge request for this provider".to_string(),
            }
            .error_response();
        }
    };

    // The lock keys on `payment_id`, which lives in the body, so it is read here rather than left
    // to `server_wrap` — the same ordering `payments_create` uses.
    let locking_action = A::locking_action(&payload, flow.clone());

    // A local because `server_wrap` borrows it.
    let authenticator = A::authenticator(merchant_id, &payload);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, req_state| {
            A::charge(state, req_state, auth, payload)
        },
        &*authenticator,
        locking_action,
    ))
    .await
}
