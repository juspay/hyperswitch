use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::api_keys,
    services::{api, authentication as auth},
    types::api as api_types,
};

/// API Key - Create
///
/// Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
/// displayed only once on creation, so ensure you store it securely.
#[utoipa::path(
    post,
    path = "/api_keys/{merchant_id)",
    request_body= CreateApiKeyRequest,
    responses(
        (status = 200, description = "API Key created", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "API Key",
    operation_id = "Create an API Key"
)]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyCreate))]
pub async fn api_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<api_types::CreateApiKeyRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, _, payload| async {
            api_keys::create_api_key(&*state.store, payload, merchant_id.clone()).await
        },
        &auth::AdminApiAuth,
    )
    .await
}

/// API Key - Retrieve
///
/// Retrieve information about the specified API Key.
#[utoipa::path(
    get,
    path = "/api_keys/{merchant_id}/{key_id}",
    params (("key_id" = String, Path, description = "The unique identifier for the API Key")),
    responses(
        (status = 200, description = "API Key retrieved", body = RetrieveApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Retrieve an API Key"
)]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRetrieve))]
pub async fn api_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (_merchant_id, key_id) = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        &key_id,
        |state, _, key_id| api_keys::retrieve_api_key(&*state.store, key_id),
        &auth::AdminApiAuth,
    )
    .await
}

/// API Key - Update
///
/// Update information for the specified API Key.
#[utoipa::path(
    post,
    path = "/api_keys/{merchant_id}/{key_id}",
    request_body = UpdateApiKeyRequest,
    params (("key_id" = String, Path, description = "The unique identifier for the API Key")),
    responses(
        (status = 200, description = "API Key updated", body = RetrieveApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Update an API Key"
)]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyUpdate))]
pub async fn api_key_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    json_payload: web::Json<api_types::UpdateApiKeyRequest>,
) -> impl Responder {
    let (_merchant_id, key_id) = path.into_inner();
    let payload = json_payload.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        (&key_id, payload),
        |state, _, (key_id, payload)| api_keys::update_api_key(&*state.store, key_id, payload),
        &auth::AdminApiAuth,
    )
    .await
}

/// API Key - Revoke
///
/// Revoke the specified API Key. Once revoked, the API Key can no longer be used for
/// authenticating with our APIs.
#[utoipa::path(
    delete,
    path = "/api_keys/{merchant_id)/{key_id}",
    params (("key_id" = String, Path, description = "The unique identifier for the API Key")),
    responses(
        (status = 200, description = "API Key revoked", body = RevokeApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Revoke an API Key"
)]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRevoke))]
pub async fn api_key_revoke(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (_merchant_id, key_id) = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        &key_id,
        |state, _, key_id| api_keys::revoke_api_key(&*state.store, key_id),
        &auth::AdminApiAuth,
    )
    .await
}

/// API Key - List
///
/// List all API Keys associated with your merchant account.
#[utoipa::path(
    get,
    path = "/api_keys/{merchant_id}/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of API Keys to include in the response"),
        ("skip" = Option<i64>, Query, description = "The number of API Keys to skip when retrieving the list of API keys."),
    ),
    responses(
        (status = 200, description = "List of API Keys retrieved successfully", body = Vec<RetrieveApiKeyResponse>),
    ),
    tag = "API Key",
    operation_id = "List all API Keys associated with a merchant account"
)]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyList))]
pub async fn api_key_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<api_types::ListApiKeyConstraints>,
) -> impl Responder {
    let list_api_key_constraints = query.into_inner();
    let limit = list_api_key_constraints.limit;
    let offset = list_api_key_constraints.skip;
    let merchant_id = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        (limit, offset, merchant_id),
        |state, _, (limit, offset, merchant_id)| async move {
            api_keys::list_api_keys(&*state.store, merchant_id, limit, offset).await
        },
        &auth::AdminApiAuth,
    )
    .await
}
