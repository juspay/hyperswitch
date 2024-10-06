use std::sync::Arc;

use common_utils::id_type;
use error_stack::ResultExt;
use redis_interface::RedisConnectionPool;
use router_env::logger;

use super::authentication::AuthToken;
use crate::{
    consts,
    core::errors::{ApiErrorResponse, RouterResult, StorageErrorExt},
    routes::app::SessionStateInfo,
};

#[cfg(feature = "olap")]
pub mod info;
pub mod permission_groups;
pub mod permissions;
pub mod roles;

pub async fn get_role_info<A>(state: &A, token: &AuthToken) -> RouterResult<roles::RoleInfo>
where
    A: SessionStateInfo + Sync,
{
    if let Some(role_info) = roles::predefined_roles::PREDEFINED_ROLES.get(token.role_id.as_str()) {
        return Ok(role_info.clone());
    }

    if let Ok(role_info) = get_role_info_from_cache(state, &token.role_id)
        .await
        .map_err(|e| logger::error!("Failed to get permissions from cache {e:?}"))
    {
        return Ok(role_info.clone());
    }

    let role_info =
        get_role_info_from_db(state, &token.role_id, &token.merchant_id, &token.org_id).await?;

    let token_expiry =
        i64::try_from(token.exp).change_context(ApiErrorResponse::InternalServerError)?;
    let cache_ttl = token_expiry - common_utils::date_time::now_unix_timestamp();

    set_role_info_in_cache(state, &token.role_id, &role_info, cache_ttl)
        .await
        .map_err(|e| logger::error!("Failed to set role info in cache {e:?}"))
        .ok();
    Ok(role_info)
}

async fn get_role_info_from_cache<A>(state: &A, role_id: &str) -> RouterResult<roles::RoleInfo>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    redis_conn
        .get_and_deserialize_key(&get_cache_key_from_role_id(role_id), "RoleInfo")
        .await
        .change_context(ApiErrorResponse::InternalServerError)
}

pub fn get_cache_key_from_role_id(role_id: &str) -> String {
    format!("{}{}", consts::ROLE_INFO_CACHE_PREFIX, role_id)
}

async fn get_role_info_from_db<A>(
    state: &A,
    role_id: &str,
    merchant_id: &id_type::MerchantId,
    org_id: &id_type::OrganizationId,
) -> RouterResult<roles::RoleInfo>
where
    A: SessionStateInfo + Sync,
{
    state
        .store()
        .find_role_by_role_id_in_merchant_scope(role_id, merchant_id, org_id)
        .await
        .map(roles::RoleInfo::from)
        .to_not_found_response(ApiErrorResponse::InvalidJwtToken)
}

pub async fn set_role_info_in_cache<A>(
    state: &A,
    role_id: &str,
    role_info: &roles::RoleInfo,
    expiry: i64,
) -> RouterResult<()>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    redis_conn
        .serialize_and_set_key_with_expiry(&get_cache_key_from_role_id(role_id), role_info, expiry)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
}

pub fn check_permission(
    required_permission: &permissions::Permission,
    role_info: &roles::RoleInfo,
) -> RouterResult<()> {
    role_info
        .check_permission_exists(required_permission)
        .then_some(())
        .ok_or(
            ApiErrorResponse::AccessForbidden {
                resource: required_permission.to_string(),
            }
            .into(),
        )
}

pub fn check_entity(
    required_minimum_entity: common_enums::EntityType,
    role_info: &roles::RoleInfo,
) -> RouterResult<()> {
    if required_minimum_entity > role_info.get_entity_type() {
        Err(ApiErrorResponse::AccessForbidden {
            resource: required_minimum_entity.to_string(),
        })?;
    }
    Ok(())
}

fn get_redis_connection<A: SessionStateInfo>(state: &A) -> RouterResult<Arc<RedisConnectionPool>> {
    state
        .store()
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")
}
