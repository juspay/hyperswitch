use super::permissions::Permission;
use common_enums::PermissionGroup;
use once_cell::sync::Lazy;

pub fn get_permissions_vec(permission_group: &PermissionGroup) -> &Lazy<Vec<Permission>> {
    match permission_group {
        PermissionGroup::OperationsView => &OPERATIONS_VIEW,
        PermissionGroup::OperationsManage => &OPERATIONS_MANAGE,
        PermissionGroup::ConnectorsView => &CONNECTORS_VIEW,
        PermissionGroup::ConnectorsManage => &CONNECTORS_MANAGE,
        PermissionGroup::WorkflowsView => &WORKFLOWS_VIEW,
        PermissionGroup::WorkflowsManage => &WORKFLOWS_MANAGE,
        PermissionGroup::AnalyticsView => &ANALYTICS_VIEW,
        PermissionGroup::UsersView => &USERS_VIEW,
        PermissionGroup::UsersManage => &USERS_MANAGE,
        PermissionGroup::MerchantDetailsView => &MERCHANT_DETAILS_VIEW,
        PermissionGroup::MerchantDetailsManage => &MERCHANT_DETAILS_MANAGE,
        PermissionGroup::OrganizationManage => &ORGANIZATION_MANAGE,
    }
}

pub static OPERATIONS_VIEW: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::PaymentRead,
        Permission::RefundRead,
        Permission::MandateRead,
        Permission::DisputeRead,
        Permission::CustomerRead,
    ]
});

pub static OPERATIONS_MANAGE: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::PaymentWrite,
        Permission::RefundWrite,
        Permission::MandateWrite,
        Permission::DisputeWrite,
        Permission::CustomerWrite,
    ]
});

pub static CONNECTORS_VIEW: Lazy<Vec<Permission>> =
    Lazy::new(|| vec![Permission::MerchantConnectorAccountRead]);

pub static CONNECTORS_MANAGE: Lazy<Vec<Permission>> =
    Lazy::new(|| vec![Permission::MerchantConnectorAccountWrite]);

pub static WORKFLOWS_VIEW: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::RoutingRead,
        Permission::ThreeDsDecisionManagerRead,
        Permission::SurchargeDecisionManagerRead,
        Permission::MerchantConnectorAccountRead,
    ]
});

pub static WORKFLOWS_MANAGE: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::RoutingWrite,
        Permission::ThreeDsDecisionManagerWrite,
        Permission::SurchargeDecisionManagerWrite,
        Permission::MerchantConnectorAccountRead,
    ]
});

pub static ANALYTICS_VIEW: Lazy<Vec<Permission>> = Lazy::new(|| vec![Permission::Analytics]);

pub static USERS_VIEW: Lazy<Vec<Permission>> = Lazy::new(|| vec![Permission::UsersRead]);

pub static USERS_MANAGE: Lazy<Vec<Permission>> = Lazy::new(|| vec![Permission::UsersWrite]);

pub static MERCHANT_DETAILS_VIEW: Lazy<Vec<Permission>> =
    Lazy::new(|| vec![Permission::MerchantAccountRead, Permission::ApiKeyRead]);

pub static MERCHANT_DETAILS_MANAGE: Lazy<Vec<Permission>> =
    Lazy::new(|| vec![Permission::MerchantAccountWrite, Permission::ApiKeyWrite]);

pub static ORGANIZATION_MANAGE: Lazy<Vec<Permission>> =
    Lazy::new(|| vec![Permission::MerchantAccountCreate]);
