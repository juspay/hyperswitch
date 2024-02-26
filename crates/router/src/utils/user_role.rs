use api_models::user_role as user_role_api;
use error_stack::ResultExt;

use crate::{
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization::permissions::Permission,
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
