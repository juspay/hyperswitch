use super::permissions::Permission;
use once_cell::sync::Lazy;

pub enum PermissionGroup {
    OperationsRead,
    OperationsWrite,
    UnnamedRead,
    UnnamedWrite,
}

impl PermissionGroup {
    pub fn get_permissions_groups(&self) -> &Lazy<Vec<Permission>> {
        match self {
            PermissionGroup::OperationsRead => &OPERATIONS_READ,
            PermissionGroup::OperationsWrite => &OPERATIONS_WRITE,
            PermissionGroup::UnnamedRead => &UNNAMED_READ,
            PermissionGroup::UnnamedWrite => &UNNAMED_WRITE,
        }
    }
}

pub static OPERATIONS_READ: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::PaymentRead,
        Permission::RefundRead,
        Permission::MandateRead,
        Permission::DisputeRead,
    ]
});

pub static OPERATIONS_WRITE: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::PaymentWrite,
        Permission::RefundWrite,
        Permission::MandateWrite,
        Permission::DisputeWrite,
    ]
});

pub static UNNAMED_READ: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::PaymentRead,
        Permission::RefundRead,
        Permission::MandateRead,
        Permission::DisputeRead,
    ]
});

pub static UNNAMED_WRITE: Lazy<Vec<Permission>> = Lazy::new(|| {
    vec![
        Permission::PaymentWrite,
        Permission::RefundWrite,
        Permission::MandateWrite,
        Permission::DisputeWrite,
    ]
});

impl From<diesel_models::enums::PermissionGroup> for PermissionGroup {
    fn from(value: diesel_models::enums::PermissionGroup) -> Self {
        match value {
            diesel_models::enums::PermissionGroup::OperationsRead => Self::OperationsRead,
            diesel_models::enums::PermissionGroup::OperationsWrite => Self::OperationsWrite,
            diesel_models::enums::PermissionGroup::UnnamedRead => Self::UnnamedRead,
            diesel_models::enums::PermissionGroup::UnnamedWrite => Self::UnnamedWrite,
        }
    }
}
