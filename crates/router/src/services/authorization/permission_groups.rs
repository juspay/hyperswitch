use common_enums::PermissionGroup;

use super::permissions::Permission;

pub fn get_permissions_vec(permission_group: &PermissionGroup) -> &[Permission] {
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

pub static OPERATIONS_VIEW: [Permission; 7] = [
    Permission::PaymentRead,
    Permission::RefundRead,
    Permission::MandateRead,
    Permission::DisputeRead,
    Permission::CustomerRead,
    Permission::MerchantAccountRead,
    Permission::PayoutRead,
];

pub static OPERATIONS_MANAGE: [Permission; 7] = [
    Permission::PaymentWrite,
    Permission::RefundWrite,
    Permission::MandateWrite,
    Permission::DisputeWrite,
    Permission::CustomerWrite,
    Permission::MerchantAccountRead,
    Permission::PayoutWrite,
];

pub static CONNECTORS_VIEW: [Permission; 2] = [
    Permission::MerchantConnectorAccountRead,
    Permission::MerchantAccountRead,
];

pub static CONNECTORS_MANAGE: [Permission; 2] = [
    Permission::MerchantConnectorAccountWrite,
    Permission::MerchantAccountRead,
];

pub static WORKFLOWS_VIEW: [Permission; 5] = [
    Permission::RoutingRead,
    Permission::ThreeDsDecisionManagerRead,
    Permission::SurchargeDecisionManagerRead,
    Permission::MerchantConnectorAccountRead,
    Permission::MerchantAccountRead,
];

pub static WORKFLOWS_MANAGE: [Permission; 5] = [
    Permission::RoutingWrite,
    Permission::ThreeDsDecisionManagerWrite,
    Permission::SurchargeDecisionManagerWrite,
    Permission::MerchantConnectorAccountRead,
    Permission::MerchantAccountRead,
];

pub static ANALYTICS_VIEW: [Permission; 2] =
    [Permission::Analytics, Permission::MerchantAccountRead];

pub static USERS_VIEW: [Permission; 2] = [Permission::UsersRead, Permission::MerchantAccountRead];

pub static USERS_MANAGE: [Permission; 2] =
    [Permission::UsersWrite, Permission::MerchantAccountRead];

pub static MERCHANT_DETAILS_VIEW: [Permission; 1] = [Permission::MerchantAccountRead];

pub static MERCHANT_DETAILS_MANAGE: [Permission; 5] = [
    Permission::MerchantAccountWrite,
    Permission::ApiKeyRead,
    Permission::ApiKeyWrite,
    Permission::MerchantAccountRead,
    Permission::WebhookEventRead,
];

pub static ORGANIZATION_MANAGE: [Permission; 2] = [
    Permission::MerchantAccountCreate,
    Permission::MerchantAccountRead,
];
