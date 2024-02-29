use api_models::user_role as user_role_api;
use common_enums::PermissionGroup;
use error_stack::ResultExt;

use crate::{
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization::{permissions::Permission, roles},
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

pub fn validate_role_groups(groups: &Vec<PermissionGroup>) -> UserResult<()> {
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
        .find(|(_, role_info)| role_info.get_role_name() == &role_name_str)
        .is_some();

    // TODO: Create and use find_by_role_name to make this efficient
    let is_present_in_custom_roles = state
        .store
        .list_all_roles(merchant_id, org_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .iter()
        .map(|role| &role.role_name)
        .find(|name| name == &role_name_str.as_str())
        .is_some();

    if is_present_in_predefined_roles || is_present_in_custom_roles {
        return Err(UserErrors::RoleNameAlreadyExists.into());
    }

    Ok(())
}
