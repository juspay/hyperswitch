use std::ops::Not;

use api_models::user_role::GroupInfo;
use common_enums::{ParentGroup, PermissionGroup};
use strum::IntoEnumIterator;

// TODO: To be deprecated
pub fn get_group_authorization_info() -> Option<Vec<GroupInfo>> {
    let groups = PermissionGroup::iter()
        .filter_map(get_group_info_from_permission_group)
        .collect::<Vec<_>>();

    groups.is_empty().not().then_some(groups)
}

// TODO: To be deprecated
fn get_group_info_from_permission_group(group: PermissionGroup) -> Option<GroupInfo> {
    let description = get_group_description(group)?;
    Some(GroupInfo { group, description })
}

// TODO: To be deprecated
fn get_group_description(group: PermissionGroup) -> Option<&'static str> {
    match group {
        PermissionGroup::OperationsView => {
            Some("View Payments, Refunds, Payouts, Mandates, Disputes and Customers")
        }
        PermissionGroup::OperationsManage => {
            Some("Create, modify and delete Payments, Refunds, Payouts, Mandates, Disputes and Customers")
        }
        PermissionGroup::ConnectorsView => {
            Some("View connected Payment Processors, Payout Processors and Fraud & Risk Manager details")
        }
        PermissionGroup::ConnectorsManage => Some("Create, modify and delete connectors like Payment Processors, Payout Processors and Fraud & Risk Manager"),
        PermissionGroup::WorkflowsView => {
            Some("View Routing, 3DS Decision Manager, Surcharge Decision Manager")
        }
        PermissionGroup::WorkflowsManage => {
            Some("Create, modify and delete Routing, 3DS Decision Manager, Surcharge Decision Manager")
        }
        PermissionGroup::AnalyticsView => Some("View Analytics"),
        PermissionGroup::UsersView => Some("View Users"),
        PermissionGroup::UsersManage => Some("Manage and invite Users to the Team"),
        PermissionGroup::AccountView => Some("View Merchant Details"),
        PermissionGroup::AccountManage => Some("Create, modify and delete Merchant Details like api keys, webhooks, etc"),
        PermissionGroup::ReconReportsView => Some("View reconciliation reports and analytics"),
        PermissionGroup::ReconReportsManage => Some("Manage reconciliation reports"),
        PermissionGroup::ReconOpsView => Some("View and access all reconciliation operations including reports and analytics"),
        PermissionGroup::ReconOpsManage => Some("Manage all reconciliation operations including reports and analytics"),
        PermissionGroup::ThemeView => Some("View Themes"),
        PermissionGroup::ThemeManage => Some("Manage Themes"),
        PermissionGroup::InternalManage => None, // Internal group, no user-facing description
    }
}

pub fn get_parent_group_description(group: ParentGroup) -> Option<&'static str> {
    match group {
        ParentGroup::Operations => Some("Payments, Refunds, Payouts, Mandates, Disputes and Customers"),
        ParentGroup::Connectors => Some("Create, modify and delete connectors like Payment Processors, Payout Processors and Fraud & Risk Manager"),
        ParentGroup::Workflows => Some("Create, modify and delete Routing, 3DS Decision Manager, Surcharge Decision Manager"),
        ParentGroup::Analytics => Some("View Analytics"),
        ParentGroup::Users =>  Some("Manage and invite Users to the Team"),
        ParentGroup::Account => Some("Create, modify and delete Merchant Details like api keys, webhooks, etc"),
        ParentGroup::ReconOps => Some("View, manage reconciliation operations like upload and process files, run reconciliation etc"),
        ParentGroup::ReconReports => Some("View, manage reconciliation reports and analytics"),
        ParentGroup::Theme => Some("Manage and view themes for the organization"),
        ParentGroup::Internal => None, // Internal group, no user-facing description
    }
}
