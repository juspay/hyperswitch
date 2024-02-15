use super::permissions::Permission;
use once_cell::sync::Lazy;

pub enum PermissionGroup {
    OperationsView,
    OperationsManage,
    ConnectorsView,
    ConnectorsManage,
    WorkflowsView,
    WorkflowsManage,
    AnalyticsView,
    UsersView,
    UsersManage,
    MerchantDetailsView,
    MerchantDetailsManage,
    OrganizationManage,
}

impl PermissionGroup {
    pub fn get_permissions_groups(&self) -> &Lazy<Vec<Permission>> {
        match self {
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

impl From<diesel_models::enums::PermissionGroup> for PermissionGroup {
    fn from(value: diesel_models::enums::PermissionGroup) -> Self {
        match value {
            diesel_models::enums::PermissionGroup::OperationsView => Self::OperationsView,
            diesel_models::enums::PermissionGroup::OperationsManage => Self::OperationsManage,
            diesel_models::enums::PermissionGroup::ConnectorsView => Self::ConnectorsView,
            diesel_models::enums::PermissionGroup::ConnectorsManage => Self::ConnectorsManage,
            diesel_models::enums::PermissionGroup::WorkflowsView => Self::WorkflowsView,
            diesel_models::enums::PermissionGroup::WorkflowsManage => Self::WorkflowsManage,
            diesel_models::enums::PermissionGroup::AnalyticsView => Self::AnalyticsView,
            diesel_models::enums::PermissionGroup::UsersView => Self::UsersView,
            diesel_models::enums::PermissionGroup::UsersManage => Self::UsersManage,
            diesel_models::enums::PermissionGroup::MerchantDetailsView => Self::MerchantDetailsView,
            diesel_models::enums::PermissionGroup::MerchantDetailsManage => {
                Self::MerchantDetailsManage
            }
            diesel_models::enums::PermissionGroup::OrganizationManage => Self::OrganizationManage,
        }
    }
}
