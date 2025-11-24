use actix_web::{web, HttpRequest, HttpResponse};
use api_models::profile_acquirer::{ProfileAcquirerCreate, ProfileAcquirerUpdate};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::api_locking,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileAcquirerCreate))]
pub async fn create_profile_acquirer(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<ProfileAcquirerCreate>,
) -> HttpResponse {
    let flow = Flow::ProfileAcquirerCreate;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state: super::SessionState, auth_data, req, _| {
            let platform = auth_data.into();
            crate::core::profile_acquirer::create_profile_acquirer(state, req, platform)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: true,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileAcquirerUpdate))]
pub async fn profile_acquirer_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::ProfileId,
        common_utils::id_type::ProfileAcquirerId,
    )>,
    json_payload: web::Json<ProfileAcquirerUpdate>,
) -> HttpResponse {
    let flow = Flow::ProfileAcquirerUpdate;
    let (profile_id, profile_acquirer_id) = path.into_inner();
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state: super::SessionState, auth_data, req, _| {
            let platform = auth_data.into();
            crate::core::profile_acquirer::update_profile_acquirer_config(
                state,
                profile_id.clone(),
                profile_acquirer_id.clone(),
                req,
                platform,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: true,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
