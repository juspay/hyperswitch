use std::collections::HashMap;

use once_cell::sync::Lazy;

use super::permissions::Permission;
use crate::consts;

pub struct RoleInfo {
    permissions: Vec<Permission>,
    name: Option<&'static str>,
    is_invitable: bool,
    is_deletable: bool,
}

impl RoleInfo {
    pub fn get_permissions(&self) -> &Vec<Permission> {
        &self.permissions
    }

    pub fn get_name(&self) -> Option<&'static str> {
        self.name
    }

    pub fn is_invitable(&self) -> bool {
        self.is_invitable
    }
}

pub static PREDEFINED_PERMISSIONS: Lazy<HashMap<&'static str, RoleInfo>> = Lazy::new(|| {
    let mut roles = HashMap::new();
    roles.insert(
        consts::user_role::ROLE_ID_INTERNAL_ADMIN,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::PaymentWrite,
                Permission::RefundRead,
                Permission::RefundWrite,
                Permission::ApiKeyRead,
                Permission::ApiKeyWrite,
                Permission::MerchantAccountRead,
                Permission::MerchantAccountWrite,
                Permission::MerchantConnectorAccountRead,
                Permission::MerchantConnectorAccountWrite,
                Permission::RoutingRead,
                Permission::RoutingWrite,
                Permission::ForexRead,
                Permission::ThreeDsDecisionManagerWrite,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerWrite,
                Permission::SurchargeDecisionManagerRead,
                Permission::DisputeRead,
                Permission::DisputeWrite,
                Permission::MandateRead,
                Permission::MandateWrite,
                Permission::CustomerRead,
                Permission::CustomerWrite,
                Permission::FileRead,
                Permission::FileWrite,
                Permission::Analytics,
                Permission::UsersRead,
                Permission::UsersWrite,
                Permission::MerchantAccountCreate,
            ],
            name: None,
            is_invitable: false,
            is_deletable: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::RefundRead,
                Permission::ApiKeyRead,
                Permission::MerchantAccountRead,
                Permission::MerchantConnectorAccountRead,
                Permission::RoutingRead,
                Permission::ForexRead,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerRead,
                Permission::Analytics,
                Permission::DisputeRead,
                Permission::MandateRead,
                Permission::CustomerRead,
                Permission::FileRead,
                Permission::UsersRead,
            ],
            name: None,
            is_invitable: false,
            is_deletable: false,
        },
    );

    roles.insert(
        consts::user_role::ROLE_ID_ORGANIZATION_ADMIN,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::PaymentWrite,
                Permission::RefundRead,
                Permission::RefundWrite,
                Permission::ApiKeyRead,
                Permission::ApiKeyWrite,
                Permission::MerchantAccountRead,
                Permission::MerchantAccountWrite,
                Permission::MerchantConnectorAccountRead,
                Permission::MerchantConnectorAccountWrite,
                Permission::RoutingRead,
                Permission::RoutingWrite,
                Permission::ForexRead,
                Permission::ThreeDsDecisionManagerWrite,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerWrite,
                Permission::SurchargeDecisionManagerRead,
                Permission::DisputeRead,
                Permission::DisputeWrite,
                Permission::MandateRead,
                Permission::MandateWrite,
                Permission::CustomerRead,
                Permission::CustomerWrite,
                Permission::FileRead,
                Permission::FileWrite,
                Permission::Analytics,
                Permission::UsersRead,
                Permission::UsersWrite,
                Permission::MerchantAccountCreate,
            ],
            name: Some("Organization Admin"),
            is_invitable: false,
            is_deletable: false,
        },
    );

    // MERCHANT ROLES
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_ADMIN,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::PaymentWrite,
                Permission::RefundRead,
                Permission::RefundWrite,
                Permission::ApiKeyRead,
                Permission::ApiKeyWrite,
                Permission::MerchantAccountRead,
                Permission::MerchantAccountWrite,
                Permission::MerchantConnectorAccountRead,
                Permission::ForexRead,
                Permission::MerchantConnectorAccountWrite,
                Permission::RoutingRead,
                Permission::RoutingWrite,
                Permission::ThreeDsDecisionManagerWrite,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerWrite,
                Permission::SurchargeDecisionManagerRead,
                Permission::DisputeRead,
                Permission::DisputeWrite,
                Permission::MandateRead,
                Permission::MandateWrite,
                Permission::CustomerRead,
                Permission::CustomerWrite,
                Permission::FileRead,
                Permission::FileWrite,
                Permission::Analytics,
                Permission::UsersRead,
                Permission::UsersWrite,
            ],
            name: Some("Admin"),
            is_invitable: true,
            is_deletable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_VIEW_ONLY,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::RefundRead,
                Permission::ApiKeyRead,
                Permission::MerchantAccountRead,
                Permission::ForexRead,
                Permission::MerchantConnectorAccountRead,
                Permission::RoutingRead,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerRead,
                Permission::DisputeRead,
                Permission::MandateRead,
                Permission::CustomerRead,
                Permission::FileRead,
                Permission::Analytics,
                Permission::UsersRead,
            ],
            name: Some("View Only"),
            is_invitable: true,
            is_deletable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_IAM_ADMIN,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::RefundRead,
                Permission::ApiKeyRead,
                Permission::MerchantAccountRead,
                Permission::ForexRead,
                Permission::MerchantConnectorAccountRead,
                Permission::RoutingRead,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerRead,
                Permission::DisputeRead,
                Permission::MandateRead,
                Permission::CustomerRead,
                Permission::FileRead,
                Permission::Analytics,
                Permission::UsersRead,
                Permission::UsersWrite,
            ],
            name: Some("IAM"),
            is_invitable: true,
            is_deletable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_DEVELOPER,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::RefundRead,
                Permission::ApiKeyRead,
                Permission::ApiKeyWrite,
                Permission::MerchantAccountRead,
                Permission::ForexRead,
                Permission::MerchantConnectorAccountRead,
                Permission::RoutingRead,
                Permission::ThreeDsDecisionManagerRead,
                Permission::SurchargeDecisionManagerRead,
                Permission::DisputeRead,
                Permission::MandateRead,
                Permission::CustomerRead,
                Permission::FileRead,
                Permission::Analytics,
                Permission::UsersRead,
            ],
            name: Some("Developer"),
            is_invitable: true,
            is_deletable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_OPERATOR,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::PaymentWrite,
                Permission::RefundRead,
                Permission::RefundWrite,
                Permission::ApiKeyRead,
                Permission::MerchantAccountRead,
                Permission::ForexRead,
                Permission::MerchantConnectorAccountRead,
                Permission::MerchantConnectorAccountWrite,
                Permission::RoutingRead,
                Permission::RoutingWrite,
                Permission::ThreeDsDecisionManagerRead,
                Permission::ThreeDsDecisionManagerWrite,
                Permission::SurchargeDecisionManagerRead,
                Permission::SurchargeDecisionManagerWrite,
                Permission::DisputeRead,
                Permission::MandateRead,
                Permission::CustomerRead,
                Permission::FileRead,
                Permission::Analytics,
                Permission::UsersRead,
            ],
            name: Some("Operator"),
            is_invitable: true,
            is_deletable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_CUSTOMER_SUPPORT,
        RoleInfo {
            permissions: vec![
                Permission::PaymentRead,
                Permission::RefundRead,
                Permission::RefundWrite,
                Permission::ForexRead,
                Permission::DisputeRead,
                Permission::DisputeWrite,
                Permission::MerchantAccountRead,
                Permission::MerchantConnectorAccountRead,
                Permission::MandateRead,
                Permission::CustomerRead,
                Permission::FileRead,
                Permission::FileWrite,
                Permission::Analytics,
            ],
            name: Some("Customer Support"),
            is_invitable: true,
            is_deletable: true,
        },
    );
    roles
});

pub fn get_role_name_from_id(role_id: &str) -> Option<&'static str> {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .and_then(|role_info| role_info.name)
}

pub fn is_role_invitable(role_id: &str) -> bool {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .map_or(false, |role_info| role_info.is_invitable)
}

pub fn is_role_deletable(role_id: &str) -> bool {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .map_or(false, |role_info| role_info.is_deletable)
}
