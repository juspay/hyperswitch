use std::collections::HashMap;

use once_cell::sync::Lazy;

use super::permissions::Permission;
use crate::consts;

pub struct RoleInfo {
    permissions: Vec<Permission>,
    name: Option<&'static str>,
    is_invitable: bool,
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
        consts::ROLE_ID_ORGANIZATION_ADMIN,
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
                Permission::FileRead,
                Permission::FileWrite,
                Permission::Analytics,
                Permission::UsersRead,
                Permission::UsersWrite,
                Permission::MerchantAccountCreate,
            ],
            name: Some("Organization Admin"),
            is_invitable: false,
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
