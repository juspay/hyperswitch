use strum::{EnumIter, IntoEnumIterator};

use super::permissions::Permission;

pub fn get_authorization_info() -> Vec<ModuleInfo> {
    PermissionModule::iter()
        .map(|module| ModuleInfo::new(&module))
        .collect()
}

pub struct PermissionInfo {
    pub enum_name: Permission,
    pub description: &'static str,
}

impl PermissionInfo {
    pub fn new(permissions: &[Permission]) -> Vec<Self> {
        permissions
            .iter()
            .map(|&per| Self {
                description: Permission::get_permission_description(&per),
                enum_name: per,
            })
            .collect()
    }
}

#[derive(PartialEq, EnumIter, Clone)]
pub enum PermissionModule {
    Payments,
    Refunds,
    MerchantAccount,
    Connectors,
    Forex,
    Routing,
    Analytics,
    Mandates,
    Customer,
    Disputes,
    Files,
    ThreeDsDecisionManager,
    SurchargeDecisionManager,
    AccountCreate,
}

impl PermissionModule {
    pub fn get_module_description(&self) -> &'static str {
        match self {
            Self::Payments => "Everything related to payments - like creating and viewing payment related information are within this module",
            Self::Refunds => "Refunds module encompasses everything related to refunds - like creating and viewing payment related information",
            Self::MerchantAccount => "Accounts module permissions allow the user to view and update account details, configure webhooks and much more",
            Self::Connectors => "All connector related actions - like configuring new connectors, viewing and updating connector configuration lies with this module",
            Self::Routing => "All actions related to new, active, and past routing stacks take place here",
            Self::Forex => "Forex module permissions allow the user to view and query the forex rates",
            Self::Analytics => "Permission to view and analyse the data relating to payments, refunds, sdk etc.",
            Self::Mandates => "Everything related to mandates - like creating and viewing mandate related information are within this module",
            Self::Customer => "Everything related to customers - like creating and viewing customer related information are within this module",
            Self::Disputes => "Everything related to disputes - like creating and viewing dispute related information are within this module",
            Self::Files => "Permissions for uploading, deleting and viewing files for disputes",
            Self::ThreeDsDecisionManager => "View and configure 3DS decision rules configured for a merchant",
            Self::SurchargeDecisionManager =>"View and configure surcharge decision rules configured for a merchant",
            Self::AccountCreate => "Create new account within your organization"
        }
    }
}

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
                permissions: PermissionInfo::new(&[
                    Permission::PaymentRead,
                    Permission::PaymentWrite,
                ]),
            },
            PermissionModule::Refunds => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::RefundRead,
                    Permission::RefundWrite,
                ]),
            },
            PermissionModule::MerchantAccount => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::MerchantAccountRead,
                    Permission::MerchantAccountWrite,
                ]),
            },
            PermissionModule::Connectors => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::MerchantConnectorAccountRead,
                    Permission::MerchantConnectorAccountWrite,
                ]),
            },
            PermissionModule::Forex => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[Permission::ForexRead]),
            },
            PermissionModule::Routing => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::RoutingRead,
                    Permission::RoutingWrite,
                ]),
            },
            PermissionModule::Analytics => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[Permission::Analytics]),
            },
            PermissionModule::Mandates => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::MandateRead,
                    Permission::MandateWrite,
                ]),
            },
            PermissionModule::Customer => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::CustomerRead,
                    Permission::CustomerWrite,
                ]),
            },
            PermissionModule::Disputes => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::DisputeRead,
                    Permission::DisputeWrite,
                ]),
            },
            PermissionModule::Files => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[Permission::FileRead, Permission::FileWrite]),
            },
            PermissionModule::ThreeDsDecisionManager => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::ThreeDsDecisionManagerWrite,
                    Permission::ThreeDsDecisionManagerRead,
                ]),
            },

            PermissionModule::SurchargeDecisionManager => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[
                    Permission::SurchargeDecisionManagerWrite,
                    Permission::SurchargeDecisionManagerRead,
                ]),
            },
            PermissionModule::AccountCreate => Self {
                module: module_name,
                description,
                permissions: PermissionInfo::new(&[Permission::MerchantAccountCreate]),
            },
        }
    }
}
