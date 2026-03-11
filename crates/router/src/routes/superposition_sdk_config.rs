use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::api_locking,
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::GetSuperpositionSdkConfig))]
pub async fn get_sdk_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::ProfileId, String)>,
) -> HttpResponse {
    let flow = Flow::GetSuperpositionSdkConfig;
    let (_profile_id, _sdk_config) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state: super::SessionState, auth_data, _req, _| {
            crate::core::superposition_sdk_config::get_superposition_sdk_config(
                state,
                auth_data.platform,
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
