use std::collections::HashMap;

use api_models::user_role::{GroupInfo, ParentGroup, ParentInfo, PermissionInfo};
use common_enums::PermissionGroup;
use strum::{EnumIter, IntoEnumIterator};

use super::{permission_groups::get_permissions_vec, permissions::Permission};

pub fn get_module_authorization_info() -> Vec<ModuleInfo> {
    PermissionModule::iter()
        .map(|module| ModuleInfo::new(&module))
        .collect()
}

pub fn get_group_authorization_info() -> Vec<GroupInfo> {
    PermissionGroup::iter()
        .map(get_group_info_from_permission_group)
        .collect()
}

pub fn get_group_authorization_info_with_group_tag() -> Vec<ParentInfo> {
    let parent_info_hash_map: HashMap<ParentGroup, Vec<String>> = PermissionGroup::iter()
        .map(|value| (get_parent_name(value), value))
        .fold(HashMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_default().push(value.to_string());
            acc
        });

    parent_info_hash_map
        .into_iter()
        .map(|(name, value)| ParentInfo {
            name: name.clone(),
            description: get_parent_group_description(name),
            groups: value,
        })
        .collect()
}

pub fn get_permission_info_from_permissions(permissions: &[Permission]) -> Vec<PermissionInfo> {
    permissions
        .iter()
        .map(|&per| PermissionInfo {
            description: Permission::get_permission_description(&per),
            enum_name: per.into(),
        })
        .collect()
}

// TODO: Deprecate once groups are stable
#[derive(PartialEq, EnumIter, Clone)]
pub enum PermissionModule {
    Payments,
    Refunds,
    MerchantAccount,
    Connectors,
    Routing,
    Analytics,
    Mandates,
    Customer,
    Disputes,
    ThreeDsDecisionManager,
    SurchargeDecisionManager,
    AccountCreate,
    Payouts,
}

impl PermissionModule {
    pub fn get_module_description(&self) -> &'static str {
        match self {
            Self::Payments => "Everything related to payments - like creating and viewing payment related information are within this module",
            Self::Refunds => "Refunds module encompasses everything related to refunds - like creating and viewing payment related information",
            Self::MerchantAccount => "Accounts module permissions allow the user to view and update account details, configure webhooks and much more",
            Self::Connectors => "All connector related actions - like configuring new connectors, viewing and updating connector configuration lies with this module",
            Self::Routing => "All actions related to new, active, and past routing stacks take place here",
            Self::Analytics => "Permission to view and analyse the data relating to payments, refunds, sdk etc.",
            Self::Mandates => "Everything related to mandates - like creating and viewing mandate related information are within this module",
            Self::Customer => "Everything related to customers - like creating and viewing customer related information are within this module",
            Self::Disputes => "Everything related to disputes - like creating and viewing dispute related information are within this module",
            Self::ThreeDsDecisionManager => "View and configure 3DS decision rules configured for a merchant",
            Self::SurchargeDecisionManager =>"View and configure surcharge decision rules configured for a merchant",
            Self::AccountCreate => "Create new account within your organization",
            Self::Payouts => "Everything related to payouts - like creating and viewing payout related information are within this module"
        }
    }
}

// TODO: Deprecate once groups are stable
pub struct ModuleInfo {
    pub module: PermissionModule,
    pub description: &'static str,
    pub permissions: Vec<PermissionInfo>,
}

impl ModuleInfo {
    pub fn new(module: &PermissionModule) -> Self {
        let module_name = module.clone();
        let description = module.get_module_description();

        match module {
            PermissionModule::Payments => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::PaymentRead,
                    Permission::PaymentWrite,
                ]),
            },
            PermissionModule::Refunds => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::RefundRead,
                    Permission::RefundWrite,
                ]),
            },
            PermissionModule::MerchantAccount => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::MerchantAccountRead,
                    Permission::MerchantAccountWrite,
                ]),
            },
            PermissionModule::Connectors => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::MerchantConnectorAccountRead,
                    Permission::MerchantConnectorAccountWrite,
                ]),
            },
            PermissionModule::Routing => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::RoutingRead,
                    Permission::RoutingWrite,
                ]),
            },
            PermissionModule::Analytics => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[Permission::Analytics]),
            },
            PermissionModule::Mandates => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::MandateRead,
                    Permission::MandateWrite,
                ]),
            },
            PermissionModule::Customer => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::CustomerRead,
                    Permission::CustomerWrite,
                ]),
            },
            PermissionModule::Disputes => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::DisputeRead,
                    Permission::DisputeWrite,
                ]),
            },
            PermissionModule::ThreeDsDecisionManager => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::ThreeDsDecisionManagerRead,
                    Permission::ThreeDsDecisionManagerWrite,
                ]),
            },

            PermissionModule::SurchargeDecisionManager => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::SurchargeDecisionManagerRead,
                    Permission::SurchargeDecisionManagerWrite,
                ]),
            },
            PermissionModule::AccountCreate => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::MerchantAccountCreate,
                ]),
            },
            PermissionModule::Payouts => Self {
                module: module_name,
                description,
                permissions: get_permission_info_from_permissions(&[
                    Permission::PayoutRead,
                    Permission::PayoutWrite,
                ]),
            },
        }
    }
}

fn get_group_info_from_permission_group(group: PermissionGroup) -> GroupInfo {
    let description = get_group_description(group);
    GroupInfo {
        group,
        description,
        permissions: get_permission_info_from_permissions(get_permissions_vec(&group)),
    }
}

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
    }
}

fn get_parent_name(group: PermissionGroup) -> ParentGroup {
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
    }
}

fn get_parent_group_description(group: ParentGroup) -> &'static str {
    match group {
        ParentGroup::Operations => "Payments, Refunds, Payouts, Mandates, Disputes and Customers",
        ParentGroup::Connectors => "Create, modify and delete connectors like Payment Processors, Payout Processors and Fraud & Risk Manager",
        ParentGroup::Workflows => "Create, modify and delete Routing, 3DS Decision Manager, Surcharge Decision Manager",
       ParentGroup::Analytics => "View Analytics",
        ParentGroup::Users =>  "Manage and invite Users to the Team",
       ParentGroup::Merchant => "Create, modify and delete Merchant Details like api keys, webhooks, etc",
       ParentGroup::Organization =>"Manage organization level tasks like create new Merchant accounts, Organization level roles, etc",
    }
}
