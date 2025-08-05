use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_keys, api_locking},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api as api_types,
};

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
            &auth::PlatformOrgAdminAuthWithMerchantIdFromRoute {
                merchant_id_from_route: merchant_id.clone(),
                is_admin_auth_allowed: true,
            },
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantApiKeyWrite,
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
        |state, auth::AuthenticationDataWithoutProfile { key_store, .. }, payload, _| async {
            api_keys::create_api_key(state, payload, key_store).await
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantApiKeyWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRetrieve))]
pub async fn api_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ApiKeyId>,
) -> impl Responder {
    let flow = Flow::ApiKeyRetrieve;
    let key_id = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        &key_id,
        |state,
         auth::AuthenticationDataWithoutProfile {
             merchant_account, ..
         },
         key_id,
         _| {
            api_keys::retrieve_api_key(
                state,
                merchant_account.get_id().to_owned(),
                key_id.to_owned(),
            )
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantApiKeyRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRetrieve))]
pub async fn api_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ApiKeyId,
    )>,
) -> impl Responder {
    let flow = Flow::ApiKeyRetrieve;
    let (merchant_id, key_id) = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        (merchant_id.clone(), key_id.clone()),
        |state, _, (merchant_id, key_id), _| api_keys::retrieve_api_key(state, merchant_id, key_id),
        auth::auth_type(
            &auth::PlatformOrgAdminAuthWithMerchantIdFromRoute {
                merchant_id_from_route: merchant_id.clone(),
                is_admin_auth_allowed: true,
            },
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantApiKeyRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyUpdate))]
pub async fn api_key_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ApiKeyId,
    )>,
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
            &auth::PlatformOrgAdminAuthWithMerchantIdFromRoute {
                merchant_id_from_route: merchant_id.clone(),
                is_admin_auth_allowed: true,
            },
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantApiKeyWrite,
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
    key_id: web::Path<common_utils::id_type::ApiKeyId>,
    json_payload: web::Json<api_types::UpdateApiKeyRequest>,
) -> impl Responder {
    let flow = Flow::ApiKeyUpdate;
    let api_key_id = key_id.into_inner();
    let mut payload = json_payload.into_inner();
    payload.key_id = api_key_id;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state,
         auth::AuthenticationDataWithoutProfile {
             merchant_account, ..
         },
         mut payload,
         _| {
            payload.merchant_id = merchant_account.get_id().to_owned();
            api_keys::update_api_key(state, payload)
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantApiKeyRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRevoke))]
pub async fn api_key_revoke(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ApiKeyId,
    )>,
) -> impl Responder {
    let flow = Flow::ApiKeyRevoke;
    let (merchant_id, key_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (&merchant_id, &key_id),
        |state, _, (merchant_id, key_id), _| {
            api_keys::revoke_api_key(state, merchant_id.clone(), key_id)
        },
        auth::auth_type(
            &auth::PlatformOrgAdminAuthWithMerchantIdFromRoute {
                merchant_id_from_route: merchant_id.clone(),
                is_admin_auth_allowed: true,
            },
            &auth::JWTAuthMerchantFromRoute {
                merchant_id: merchant_id.clone(),
                required_permission: Permission::MerchantApiKeyWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyRevoke))]
pub async fn api_key_revoke(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ApiKeyId>,
) -> impl Responder {
    let flow = Flow::ApiKeyRevoke;
    let key_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        &key_id,
        |state,
         auth::AuthenticationDataWithoutProfile {
             merchant_account, ..
         },
         key_id,
         _| api_keys::revoke_api_key(state, merchant_account.get_id().to_owned(), key_id),
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantApiKeyWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
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

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (limit, offset, merchant_id.clone()),
        |state, _, (limit, offset, merchant_id), _| async move {
            api_keys::list_api_keys(state, merchant_id, limit, offset).await
        },
        auth::auth_type(
            &auth::PlatformOrgAdminAuthWithMerchantIdFromRoute {
                merchant_id_from_route: merchant_id.clone(),
                is_admin_auth_allowed: true,
            },
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantApiKeyRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::ApiKeyList))]
pub async fn api_key_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<api_types::ListApiKeyConstraints>,
) -> impl Responder {
    let flow = Flow::ApiKeyList;
    let payload = query.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state,
         auth::AuthenticationDataWithoutProfile {
             merchant_account, ..
         },
         payload,
         _| async move {
            let merchant_id = merchant_account.get_id().to_owned();
            api_keys::list_api_keys(state, merchant_id, payload.limit, payload.skip).await
        },
        auth::auth_type(
            &auth::AdminApiAuthWithMerchantIdFromHeader,
            &auth::JWTAuthMerchantFromHeader {
                required_permission: Permission::MerchantApiKeyRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
