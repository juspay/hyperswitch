use std::str::FromStr;

use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, logger, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        errors,
        telephone_order::{
            self, twilio::TwilioGenericPay, TelephoneOrderAdapter, TelephoneOrderProvider,
        },
    },
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::TelephoneOrderCharge, payment_id))]
pub async fn telephone_order_charge(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> HttpResponse {
    let (merchant_id, provider) = path.into_inner();

    match TelephoneOrderProvider::from_str(&provider) {
        Ok(TelephoneOrderProvider::Twilio) => {
            charge_with::<TwilioGenericPay>(state, req, body, merchant_id).await
        }
        Err(error) => {
            logger::warn!("Unknown telephone order provider {provider}: {error:?}");
            api::log_and_return_error_response(errors::ApiErrorResponse::InvalidRequestUrl.into())
        }
    }
}

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

    match serde_json::from_slice::<A::Request>(&body) {
        Ok(payload) => {
            let payment_id = A::resolve_payment_id(&payload).ok().flatten();

            tracing::Span::current().record(
                "payment_id",
                payment_id
                    .as_ref()
                    .map(|payment_id| payment_id.get_string_repr())
                    .unwrap_or_default(),
            );

            let locking_action = telephone_order::locking_action(payment_id.as_ref(), flow.clone());

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
        Err(error) => {
            logger::warn!("Telephone order request body could not be deserialized: {error:?}");
            api::log_and_return_error_response(
                errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "request body".to_string(),
                    expected_format: "a telephone order charge request for this provider"
                        .to_string(),
                }
                .into(),
            )
        }
    }
}
