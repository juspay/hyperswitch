use crate::services::authorization::{self as authz, roles::predefined_roles};
use api_models::user_role as user_role_api;
use diesel_models::user_role::UserRole;
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization::{permissions::Permission, roles},
};

impl From<Permission> for user_role_api::Permission {
    fn from(value: Permission) -> Self {
        match value {
            Permission::PaymentRead => Self::PaymentRead,
            Permission::PaymentWrite => Self::PaymentWrite,
            Permission::RefundRead => Self::RefundRead,
            Permission::RefundWrite => Self::RefundWrite,
            Permission::ApiKeyRead => Self::ApiKeyRead,
            Permission::ApiKeyWrite => Self::ApiKeyWrite,
            Permission::MerchantAccountRead => Self::MerchantAccountRead,
            Permission::MerchantAccountWrite => Self::MerchantAccountWrite,
            Permission::MerchantConnectorAccountRead => Self::MerchantConnectorAccountRead,
            Permission::MerchantConnectorAccountWrite => Self::MerchantConnectorAccountWrite,
            Permission::RoutingRead => Self::RoutingRead,
            Permission::RoutingWrite => Self::RoutingWrite,
            Permission::DisputeRead => Self::DisputeRead,
            Permission::DisputeWrite => Self::DisputeWrite,
            Permission::MandateRead => Self::MandateRead,
            Permission::MandateWrite => Self::MandateWrite,
            Permission::CustomerRead => Self::CustomerRead,
            Permission::CustomerWrite => Self::CustomerWrite,
            Permission::Analytics => Self::Analytics,
            Permission::ThreeDsDecisionManagerWrite => Self::ThreeDsDecisionManagerWrite,
            Permission::ThreeDsDecisionManagerRead => Self::ThreeDsDecisionManagerRead,
            Permission::SurchargeDecisionManagerWrite => Self::SurchargeDecisionManagerWrite,
            Permission::SurchargeDecisionManagerRead => Self::SurchargeDecisionManagerRead,
            Permission::UsersRead => Self::UsersRead,
            Permission::UsersWrite => Self::UsersWrite,
            Permission::MerchantAccountCreate => Self::MerchantAccountCreate,
        }
    }
}

pub async fn is_role_name_already_present_for_merchant(
    state: &AppState,
    role_name: &str,
    merchant_id: &str,
    org_id: &str,
) -> UserResult<()> {
    let role_name_list: Vec<String> = state
        .store
        .list_all_roles(merchant_id, org_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .iter()
        .map(|role| role.role_name.to_owned())
        .collect();

    if role_name_list.contains(&role_name.to_string()) {
        return Err(UserErrors::RoleNameAlreadyExists.into());
    }
    Ok(())
}

pub async fn set_role_permissions_in_cache_by_user_role(
    state: &AppState,
    user_role: &UserRole,
) -> bool {
    set_role_permissions_in_cache(
        state,
        user_role.role_id.as_str(),
        user_role.merchant_id.as_str(),
        user_role.org_id.as_str(),
    )
    .await
    .map_err(|e| logger::error!("Error setting permissions in cache {:?}", e))
    .is_ok()
}

pub async fn set_role_permissions_in_cache(
    state: &AppState,
    role_id: &str,
    merchant_id: &str,
    org_id: &str,
) -> UserResult<()> {
    let role_info = roles::RoleInfo::from_role_id(state, role_id, merchant_id, org_id)
        .await
        .change_context(UserErrors::InternalServerError)?;

    if predefined_roles::PREDEFINED_ROLES.contains_key(role_id) {
        return Ok(());
    }

    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(UserErrors::InternalServerError)?;

    redis_conn
        .serialize_and_set_key_with_expiry(
            &authz::get_cache_key_from_role_id(role_id),
            role_info.get_permissions_set(),
            consts::JWT_TOKEN_TIME_IN_SECS as i64,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}
