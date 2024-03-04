use common_enums::PermissionGroup;
use error_stack::{IntoReport, ResultExt};
use router_env::logger;

use super::authentication::AuthToken;
use crate::{
    consts,
    core::errors::{ApiErrorResponse, RouterResult, StorageErrorExt},
    routes::app::AppStateInfo,
};

#[cfg(feature = "olap")]
pub mod info;
pub mod permission_groups;
pub mod permissions;
pub mod roles;

pub async fn get_permissions<A>(
    state: &A,
    token: &AuthToken,
) -> RouterResult<Vec<permissions::Permission>>
where
    A: AppStateInfo + Sync,
{
    if let Some(permissions) = get_permissions_from_predefined_roles(&token.role_id) {
        Ok(permissions)
    } else if let Some(permissions) = get_permissions_from_cache(state, &token.role_id).await {
        Ok(permissions)
    } else {
        let permissions =
            get_permissions_from_db(state, &token.role_id, &token.merchant_id, &token.org_id)
                .await?;
        let token_expiry: i64 = token
            .exp
            .try_into()
            .into_report()
            .change_context(ApiErrorResponse::InternalServerError)?;
        let cache_ttl = token_expiry - common_utils::date_time::now_unix_timestamp();
        set_permissions_in_cache(state, &token.role_id, &permissions, cache_ttl).await?;

        Ok(permissions)
    }
}

async fn get_permissions_from_cache<A>(
    state: &A,
    role_id: &str,
) -> Option<Vec<permissions::Permission>>
where
    A: AppStateInfo + Sync,
{
    let redis_conn = state
        .store()
        .get_redis_conn()
        .map_err(|e| logger::error!("Error eshtablishing redis connection {:?}", e))
        .ok()?;

    redis_conn
        .get_and_deserialize_key(&get_cache_key_from_role_id(role_id), "Vec<Permission>")
        .await
        .map_err(|e| logger::error!("Error getting permissions from cache {:?}", e))
        .ok()
}

pub fn get_cache_key_from_role_id(role_id: &str) -> String {
    format!("{}{}", consts::ROLE_CACHE_PREFIX, role_id)
}

fn get_permissions_from_predefined_roles(role_id: &str) -> Option<Vec<permissions::Permission>> {
    roles::predefined_roles::PREDEFINED_ROLES
        .get(role_id)
        .map(|role_info| get_permissions_from_groups(role_info.get_permission_groups()))
}

async fn get_permissions_from_db<A>(
    state: &A,
    role_id: &str,
    merchant_id: &str,
    org_id: &str,
) -> RouterResult<Vec<permissions::Permission>>
where
    A: AppStateInfo + Sync,
{
    state
        .store()
        .find_role_by_role_id_in_merchant_scope(role_id, merchant_id, org_id)
        .await
        .map(|role| get_permissions_from_groups(&role.groups))
        .to_not_found_response(ApiErrorResponse::InvalidJwtToken)
}

pub async fn set_permissions_in_cache<A>(
    state: &A,
    role_id: &str,
    permissions: &Vec<permissions::Permission>,
    expiry: i64,
) -> RouterResult<()>
where
    A: AppStateInfo + Sync,
{
    let redis_conn = state
        .store()
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    redis_conn
        .serialize_and_set_key_with_expiry(
            &get_cache_key_from_role_id(role_id),
            permissions,
            expiry,
        )
        .await
        .change_context(ApiErrorResponse::InternalServerError)
}

pub fn get_permissions_from_groups(groups: &[PermissionGroup]) -> Vec<permissions::Permission> {
    groups
        .iter()
        .flat_map(|group| {
            permission_groups::get_permissions_vec(group)
                .iter()
                .cloned()
        })
        .collect()
}

pub fn check_authorization(
    required_permission: &permissions::Permission,
    permissions: &[permissions::Permission],
) -> RouterResult<()> {
    permissions
        .contains(required_permission)
        .then_some(())
        .ok_or(
            ApiErrorResponse::AccessForbidden {
                resource: required_permission.to_string(),
            }
            .into(),
        )
}
