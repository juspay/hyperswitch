use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
#[cfg(feature = "v1")]
use crate::core::superposition_sdk_config;
use crate::{
    core::{api_locking, errors},
    headers::CLIENT_SECRET,
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::GetSuperpositionSdkConfig))]
pub async fn get_sdk_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::GetSuperpositionSdkConfig;
    let _platform = path.into_inner();
    let client_secret =
        match auth::get_header_value_by_key(CLIENT_SECRET.to_string(), req.headers()) {
            Ok(Some(client_secret)) => client_secret.to_string(),
            Ok(None) => {
                return api::log_and_return_error_response(error_stack::report!(
                    errors::ApiErrorResponse::MissingRequiredField {
                        field_name: CLIENT_SECRET,
                    }
                ));
            }
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
                client_secret.clone(),
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
