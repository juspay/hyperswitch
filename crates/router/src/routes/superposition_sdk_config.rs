use actix_web::{web, HttpRequest, HttpResponse};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::sdk_auth::SdkAuthorization;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
#[cfg(feature = "v1")]
use crate::core::{payments, superposition_sdk_config};
use crate::{
    core::{api_locking, errors},
    headers::{CLIENT_SECRET, SDK_AUTHORIZATION},
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
fn get_payment_id_from_headers(
    headers: &actix_web::http::header::HeaderMap,
) -> error_stack::Result<common_utils::id_type::PaymentId, errors::ApiErrorResponse> {
    if let Some(sdk_auth_val) =
        auth::get_header_value_by_key(SDK_AUTHORIZATION.to_string(), headers)?
    {
        let sdk_auth = SdkAuthorization::decode(sdk_auth_val)
            .change_context(errors::ApiErrorResponse::Unauthorized)?;
        sdk_auth.payment_id.ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_id",
            })
        })
    } else {
        let client_secret = auth::get_header_value_by_key(CLIENT_SECRET.to_string(), headers)?
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: CLIENT_SECRET,
                })
            })?
            .to_string();

        let payment_id_str = payments::helpers::get_payment_id_from_client_secret(&client_secret)?;
        common_utils::id_type::PaymentId::wrap(payment_id_str).change_context(
            errors::ApiErrorResponse::InvalidDataValue {
                field_name: "payment_id",
            },
        )
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::GetSuperpositionSdkConfig))]
pub async fn get_sdk_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::GetSuperpositionSdkConfig;
    let _platform = path.into_inner();

    let payment_id = match get_payment_id_from_headers(req.headers()) {
        Ok(payment_id) => payment_id,
        Err(err) => return api::log_and_return_error_response(err),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state: super::SessionState, auth_data, _req, _| {
            superposition_sdk_config::get_superposition_sdk_config(
                state,
                auth_data.platform,
                payment_id.clone(),
            )
        },
        &auth::HeaderAuth(auth::PublishableKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
