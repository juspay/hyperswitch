use actix_web::{web, HttpRequest, Responder};
use common_enums::EntityType;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_keys, api_locking},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api as api_types,
};

/// API Key - Create
///
/// Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
/// displayed only once on creation, so ensure you store it securely.
#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyCreate))]
pub async fn api_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
    json_payload: web::Json<api_types::CreateApiKeyRequest>,
) -> impl Responder {
    let flow = Flow::ApiKeyCreate;
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, payload, _| async {
            api_keys::create_api_key(state, payload, auth_data.key_store).await
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromRoute(merchant_id.clone()),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::ApiKeyWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyCreate))]
pub async fn api_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_types::CreateApiKeyRequest>,
) -> impl Responder {
    let flow = Flow::ApiKeyCreate;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data, payload, _| async {
            api_keys::create_api_key(state, payload, auth_data.key_store).await
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::ApiKeyWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// API Key - Retrieve
///
/// Retrieve information about the specified API Key.
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRetrieve))]
pub async fn api_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::ApiKeyRetrieve;
    let key_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        &key_id,
        |state, auth_data, key_id, _| {
            api_keys::retrieve_api_key(
                state,
                auth_data.merchant_account.get_id().to_owned(),
                key_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::ApiKeyRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v1")]
/// API Key - Retrieve
///
/// Retrieve information about the specified API Key.
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRetrieve))]
pub async fn api_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::ApiKeyRetrieve;
    let (merchant_id, key_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        (merchant_id.clone(), &key_id),
        |state, _, (merchant_id, key_id), _| api_keys::retrieve_api_key(state, merchant_id, key_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::ApiKeyRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v1")]
/// API Key - Update
///
/// Update information for the specified API Key.
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyUpdate))]
pub async fn api_key_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
    json_payload: web::Json<api_types::UpdateApiKeyRequest>,
) -> impl Responder {
    let flow = Flow::ApiKeyUpdate;
    let (merchant_id, key_id) = path.into_inner();
    let mut payload = json_payload.into_inner();
    payload.key_id = key_id;
    payload.merchant_id.clone_from(&merchant_id);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, payload, _| api_keys::update_api_key(state, payload),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::ApiKeyWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
pub async fn api_key_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
    json_payload: web::Json<api_types::UpdateApiKeyRequest>,
) -> impl Responder {
    let flow = Flow::ApiKeyUpdate;
    let (merchant_id, key_id) = path.into_inner();
    let mut payload = json_payload.into_inner();
    payload.key_id = key_id;
    payload.merchant_id.clone_from(&merchant_id);

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, payload, _| api_keys::update_api_key(state, payload),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::ApiKeyWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v1")]
/// API Key - Revoke
///
/// Revoke the specified API Key. Once revoked, the API Key can no longer be used for
/// authenticating with our APIs.
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRevoke))]
pub async fn api_key_revoke(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::ApiKeyRevoke;
    let (merchant_id, key_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        (&merchant_id, &key_id),
        |state, _, (merchant_id, key_id), _| api_keys::revoke_api_key(state, merchant_id, key_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::ApiKeyWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRevoke))]
pub async fn api_key_revoke(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::ApiKeyRevoke;
    let (merchant_id, key_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        (&merchant_id, &key_id),
        |state, _, (merchant_id, key_id), _| api_keys::revoke_api_key(state, merchant_id, key_id),
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::ApiKeyWrite,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
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
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("limit" = Option<i64>, Query, description = "The maximum number of API Keys to include in the response"),
        ("skip" = Option<i64>, Query, description = "The number of API Keys to skip when retrieving the list of API keys."),
    ),
    responses(
        (status = 200, description = "List of API Keys retrieved successfully", body = Vec<RetrieveApiKeyResponse>),
    ),
    tag = "API Key",
    operation_id = "List all API Keys associated with a merchant account",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyList))]
pub async fn api_key_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
    query: web::Query<api_types::ListApiKeyConstraints>,
) -> impl Responder {
    let flow = Flow::ApiKeyList;
    let list_api_key_constraints = query.into_inner();
    let limit = list_api_key_constraints.limit;
    let offset = list_api_key_constraints.skip;
    let merchant_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        (limit, offset, merchant_id.clone()),
        |state, _, (limit, offset, merchant_id), _| async move {
            api_keys::list_api_keys(state, merchant_id, limit, offset).await
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::ApiKeyRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
