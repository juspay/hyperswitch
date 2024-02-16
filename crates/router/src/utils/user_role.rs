use api_models::user_role as user_role_api;

use crate::{
    consts,
    services::authorization::{permissions::Permission, roles::RoleInfo},
};

pub fn is_internal_role(role_id: &str) -> bool {
    role_id == consts::user_role::ROLE_ID_INTERNAL_ADMIN
        || role_id == consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER
}

pub fn get_role_name_and_permission_response(
    role_info: &RoleInfo,
) -> (Vec<user_role_api::Permission>, String) {
    (
        role_info
            .get_permission_groups()
            .iter()
            .flat_map(|permission_group| {
                permission_group
                    .get_permissions_vec()
                    .iter()
                    .cloned()
                    .map(Into::into)
            })
            .collect::<Vec<user_role_api::Permission>>(),
        role_info.get_role_name().to_string(),
    )
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
