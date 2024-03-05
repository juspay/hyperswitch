use api_models::user_role as user_role_api;
use common_enums::PermissionGroup;
use diesel_models::user_role::UserRole;
use error_stack::{IntoReport, ResultExt};
use router_env::logger;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization::{self as authz, permissions::Permission, roles},
    types::domain,
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

pub fn validate_role_groups(groups: &[PermissionGroup]) -> UserResult<()> {
    if groups.is_empty() {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("Role groups cannot be empty");
    }

    if groups.contains(&PermissionGroup::OrganizationManage) {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("Organization manage group cannot be added to role");
    }

    Ok(())
}

pub async fn validate_role_name(
    state: &AppState,
    role_name: &domain::RoleName,
    merchant_id: &str,
    org_id: &str,
) -> UserResult<()> {
    let role_name_str = role_name.clone().get_role_name();

    let is_present_in_predefined_roles = roles::predefined_roles::PREDEFINED_ROLES
        .iter()
        .any(|(_, role_info)| role_info.get_role_name() == role_name_str);

    // TODO: Create and use find_by_role_name to make this efficient
    let is_present_in_custom_roles = state
        .store
        .list_all_roles(merchant_id, org_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .iter()
        .any(|role| role.role_name == role_name_str);

    if is_present_in_predefined_roles || is_present_in_custom_roles {
        return Err(UserErrors::RoleNameAlreadyExists.into());
    }

    Ok(())
}

pub async fn set_role_permissions_in_cache_by_user_role(
    state: &AppState,
    user_role: &UserRole,
) -> bool {
    set_role_permissions_in_cache_if_required(
        state,
        user_role.role_id.as_str(),
        user_role.merchant_id.as_str(),
        user_role.org_id.as_str(),
    )
    .await
    .map_err(|e| logger::error!("Error setting permissions in cache {:?}", e))
    .is_ok()
}

pub async fn set_role_permissions_in_cache_if_required(
    state: &AppState,
    role_id: &str,
    merchant_id: &str,
    org_id: &str,
) -> UserResult<()> {
    let role_info = roles::RoleInfo::from_role_id(state, role_id, merchant_id, org_id)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error getting role_info from role_id")?;

    if roles::predefined_roles::PREDEFINED_ROLES.contains_key(role_id) {
        return Ok(());
    }

    authz::set_permissions_in_cache(
        state,
        role_id,
        &role_info.get_permissions_set().into_iter().collect(),
        consts::JWT_TOKEN_TIME_IN_SECS
            .try_into()
            .into_report()
            .change_context(UserErrors::InternalServerError)?,
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Error setting permissions in redis")
}
