use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, payments::helpers},
    services::{api, authentication as auth},
};

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ClientSecretCreate))]
pub async fn client_secret_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::client_secret::ClientSecretCreateRequest>,
) -> HttpResponse {
    let flow = Flow::ClientSecretCreate;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, _| {
            helpers::make_client_secret(
                state,
                payload.resource_id.to_owned(),
                auth.platform,
                req.headers(),
            )
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: false,
            allow_platform_self_operation: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ClientSecretDelete))]
pub async fn client_secret_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::ClientSecretDelete;
    let payload = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _: auth::AuthenticationData, req, _| helpers::delete_client_secret(state, req),
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: false,
            allow_platform_self_operation: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
