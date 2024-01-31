use api_models::user_role as user_role_api;
use diesel_models::{enums::UserStatus, user_role::UserRole};
use error_stack::ResultExt;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization::{
        permissions::Permission,
        predefined_permissions::{self, RoleInfo},
    },
};

pub fn is_internal_role(role_id: &str) -> bool {
    role_id == consts::user_role::ROLE_ID_INTERNAL_ADMIN
        || role_id == consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER
}

pub async fn get_active_user_roles_for_user(
    state: &AppState,
    user_id: &str,
) -> UserResult<Vec<UserRole>> {
    Ok(state
        .store
        .list_user_roles_by_user_id(user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter(|ele| ele.status == UserStatus::Active)
        .collect())
}

pub fn validate_role_id(role_id: &str) -> UserResult<()> {
    if predefined_permissions::is_role_invitable(role_id) {
        return Ok(());
    }
    Err(UserErrors::InvalidRoleId.into())
}

pub fn get_role_name_and_permission_response(
    role_info: &RoleInfo,
) -> Option<(Vec<user_role_api::Permission>, &'static str)> {
    role_info.get_name().map(|name| {
        (
            role_info
                .get_permissions()
                .iter()
                .map(|&per| per.into())
                .collect::<Vec<user_role_api::Permission>>(),
            name,
        )
    })
}

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
            Permission::ForexRead => Self::ForexRead,
            Permission::RoutingRead => Self::RoutingRead,
            Permission::RoutingWrite => Self::RoutingWrite,
            Permission::DisputeRead => Self::DisputeRead,
            Permission::DisputeWrite => Self::DisputeWrite,
            Permission::MandateRead => Self::MandateRead,
            Permission::MandateWrite => Self::MandateWrite,
            Permission::CustomerRead => Self::CustomerRead,
            Permission::CustomerWrite => Self::CustomerWrite,
            Permission::FileRead => Self::FileRead,
            Permission::FileWrite => Self::FileWrite,
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
