use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "v1")]
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::sdk_auth::SdkAuthorization;
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
#[cfg(feature = "v1")]
use crate::core::{payments, superposition_sdk_config};
use crate::{
    core::{api_locking, errors},
    headers::AUTHORIZATION,
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
fn get_payment_id_from_headers_or_payload(
    headers: &actix_web::http::header::HeaderMap,
    json_payload: &api_models::superposition_sdk_config::SdkConfigRequest,
) -> error_stack::Result<common_utils::id_type::PaymentId, errors::ApiErrorResponse> {
    if let Some(sdk_auth_val) = auth::get_header_value_by_key(AUTHORIZATION.to_string(), headers)? {
        let sdk_auth = SdkAuthorization::decode(sdk_auth_val)
            .change_context(errors::ApiErrorResponse::Unauthorized)?;
        sdk_auth.payment_id.ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_id",
            })
        })
    } else {
        let client_secret = json_payload
            .client_secret
            .as_ref()
            .map(|cs| cs.peek())
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "client_secret",
                })
            })?;

        let payment_id_str = payments::helpers::get_payment_id_from_client_secret(client_secret)?;
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
    json_payload: web::Query<api_models::superposition_sdk_config::SdkConfigRequest>,
) -> HttpResponse {
    let flow = Flow::GetSuperpositionSdkConfig;
    let _platform = path.into_inner();
    let payload = json_payload.into_inner();

    let payment_id = match get_payment_id_from_headers_or_payload(req.headers(), &payload) {
        Ok(payment_id) => payment_id,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let api_auth = auth::ApiKeyAuth {
        allow_connected_scope_operation: true,
        allow_platform_self_operation: true,
    };

    let (auth_type, _auth_flow) =
        match auth::check_sdk_auth_and_get_auth(req.headers(), &payload, api_auth) {
            Ok(auth) => auth,
            Err(e) => return api::log_and_return_error_response(e),
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
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::GetSuperpositionSdkConfig))]
pub async fn get_profile_sdk_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let flow = Flow::GetSuperpositionSdkConfig;
    let (_platform, profile_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state: super::SessionState, auth_data, _req, _| {
            superposition_sdk_config::get_profile_superposition_sdk_config(
                state,
                auth_data.platform,
                profile_id.clone(),
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
