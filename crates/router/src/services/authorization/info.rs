use api_models::user_role::{GroupInfo, ParentGroup};
use common_enums::PermissionGroup;
use strum::IntoEnumIterator;

// TODO: To be deprecated
pub fn get_group_authorization_info() -> Vec<GroupInfo> {
    PermissionGroup::iter()
        .map(get_group_info_from_permission_group)
        .collect()
}

// TODO: To be deprecated
fn get_group_info_from_permission_group(group: PermissionGroup) -> GroupInfo {
    let description = get_group_description(group);
    GroupInfo { group, description }
}

// TODO: To be deprecated
fn get_group_description(group: PermissionGroup) -> &'static str {
    match group {
        PermissionGroup::OperationsView => {
            "View Payments, Refunds, Payouts, Mandates, Disputes and Customers"
        }
        PermissionGroup::OperationsManage => {
            "Create, modify and delete Payments, Refunds, Payouts, Mandates, Disputes and Customers"
        }
        PermissionGroup::ConnectorsView => {
            "View connected Payment Processors, Payout Processors and Fraud & Risk Manager details"
        }
        PermissionGroup::ConnectorsManage => "Create, modify and delete connectors like Payment Processors, Payout Processors and Fraud & Risk Manager",
        PermissionGroup::WorkflowsView => {
            "View Routing, 3DS Decision Manager, Surcharge Decision Manager"
        }
        PermissionGroup::WorkflowsManage => {
            "Create, modify and delete Routing, 3DS Decision Manager, Surcharge Decision Manager"
        }
        PermissionGroup::AnalyticsView => "View Analytics",
        PermissionGroup::UsersView => "View Users",
        PermissionGroup::UsersManage => "Manage and invite Users to the Team",
        PermissionGroup::MerchantDetailsView => "View Merchant Details",
        PermissionGroup::MerchantDetailsManage => "Create, modify and delete Merchant Details like api keys, webhooks, etc",
        PermissionGroup::OrganizationManage => "Manage organization level tasks like create new Merchant accounts, Organization level roles, etc",
        PermissionGroup::ReconOps => "View and manage reconciliation reports",
    }
}

pub fn get_parent_name(group: PermissionGroup) -> ParentGroup {
    match group {
        PermissionGroup::OperationsView | PermissionGroup::OperationsManage => {
            ParentGroup::Operations
        }
        PermissionGroup::ConnectorsView | PermissionGroup::ConnectorsManage => {
            ParentGroup::Connectors
        }
        PermissionGroup::WorkflowsView | PermissionGroup::WorkflowsManage => ParentGroup::Workflows,
        PermissionGroup::AnalyticsView => ParentGroup::Analytics,
        PermissionGroup::UsersView | PermissionGroup::UsersManage => ParentGroup::Users,
        PermissionGroup::MerchantDetailsView | PermissionGroup::MerchantDetailsManage => {
            ParentGroup::Merchant
        }
        PermissionGroup::OrganizationManage => ParentGroup::Organization,
        PermissionGroup::ReconOps => ParentGroup::Recon,
    }
}

pub fn get_parent_group_description(group: ParentGroup) -> &'static str {
    match group {
        ParentGroup::Operations => "Payments, Refunds, Payouts, Mandates, Disputes and Customers",
        ParentGroup::Connectors => "Create, modify and delete connectors like Payment Processors, Payout Processors and Fraud & Risk Manager",
        ParentGroup::Workflows => "Create, modify and delete Routing, 3DS Decision Manager, Surcharge Decision Manager",
        ParentGroup::Analytics => "View Analytics",
        ParentGroup::Users =>  "Manage and invite Users to the Team",
        ParentGroup::Merchant => "Create, modify and delete Merchant Details like api keys, webhooks, etc",
        ParentGroup::Organization =>"Manage organization level tasks like create new Merchant accounts, Organization level roles, etc",
        ParentGroup::Recon => "View and manage reconciliation reports",
    }
}
