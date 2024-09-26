use api_models::user_role::ParentGroup;
use common_enums::PermissionGroup;

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
