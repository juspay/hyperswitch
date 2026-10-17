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

/// `POST /telephone_order/{provider}/{merchant_id}` — the endpoint every telephone order provider posts to.
///
/// The body belongs to the adapter the path names, so it arrives raw and is deserialized in
/// [`charge_with`]. Same shape as incoming webhooks.
#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::TelephoneOrderCharge, provider))]
pub async fn telephone_order_charge(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(String, common_utils::id_type::MerchantId)>,
) -> HttpResponse {
    let (provider, merchant_id) = path.into_inner();
    // Before parsing, so an unknown provider is labelled too.
    tracing::Span::current().record("provider", provider.as_str());

    let Ok(provider) = TelephoneOrderProvider::from_str(&provider) else {
        return errors::ApiErrorResponse::InvalidRequestUrl.error_response();
    };

    match provider {
        TelephoneOrderProvider::Twilio => {
            charge_with::<TwilioGenericPay>(state, req, body, merchant_id).await
        }
    }
}

/// The route for one adapter, written once: a new provider is a `match` arm above and a core
/// module, with no route code of its own.
#[cfg(feature = "v1")]
async fn charge_with<A>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    merchant_id: common_utils::id_type::MerchantId,
) -> HttpResponse
where
    // `Send` is what `async_trait` needs to box `A::charge`'s future.
    A: TelephoneOrderAdapter + Send,
{
    let flow = Flow::TelephoneOrderCharge;

    // By hand rather than through `web::Json<A::Request>`, which cannot be chosen from a path
    // parameter. Still before `server_wrap`: the typed struct, with every sensitive field a
    // `Secret`, is what keeps the PAN out of the `ApiEvent` log.
    let payload = match serde_json::from_slice::<A::Request>(&body) {
        Ok(payload) => payload,
        Err(error) => {
            logger::warn!("Generic pay request body could not be deserialized: {error:?}");
            // Non-2xx is right here: the request never became a charge, so there is no payment
            // outcome to put in a body.
            return errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "request body".to_string(),
                expected_format: "a telephone order charge request for this provider".to_string(),
            }
            .error_response();
        }
    };

    // The lock keys on `payment_id`, which lives in the body, so it is read here rather than left
    // to `server_wrap` — the same ordering `payments_create` uses. `NotApplicable` when the IVR
    // sent none; see `TelephoneOrderAdapter::locking_action`.
    let locking_action = A::locking_action(&payload, flow.clone());

    // A local because `server_wrap` borrows it.
    let authenticator = A::authenticator(merchant_id);

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
