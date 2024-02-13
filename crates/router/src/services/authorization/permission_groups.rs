use once_cell::sync::Lazy;
use std::collections::HashSet;

use super::permissions::Permission;

pub enum PermissionGroup {
    OperationsRead,
    OperationsWrite,
    UnnamedRead,
    UnnamedWrite,
}

impl PermissionGroup {
    pub fn get_permissions_set(&self) -> &'static HashSet<Permission> {
        match self {
            Self::OperationsRead => &OPERATIONS_READ,
            Self::OperationsWrite => &OPERATIONS_WRITE,
            Self::UnnamedRead => &UNNAMED_READ,
            Self::UnnamedWrite => &UNNAMED_WRITE,
        }
    }

    pub fn get_permissions_vec(&self) -> Vec<Permission> {
        self.get_permissions_set().iter().cloned().collect()
    }
}

pub static OPERATIONS_READ: Lazy<HashSet<Permission>> = Lazy::new(|| {
    HashSet::from([
        Permission::PaymentRead,
        Permission::RefundRead,
        Permission::MandateRead,
        Permission::DisputeRead,
    ])
});

pub static OPERATIONS_WRITE: Lazy<HashSet<Permission>> = Lazy::new(|| {
    HashSet::from([
        Permission::PaymentWrite,
        Permission::RefundWrite,
        Permission::MandateWrite,
        Permission::DisputeWrite,
    ])
});

pub static UNNAMED_READ: Lazy<HashSet<Permission>> = Lazy::new(|| {
    HashSet::from([
        Permission::PaymentRead,
        Permission::RefundRead,
        Permission::MandateRead,
        Permission::DisputeRead,
    ])
});

pub static UNNAMED_WRITE: Lazy<HashSet<Permission>> = Lazy::new(|| {
    HashSet::from([
        Permission::PaymentWrite,
        Permission::RefundWrite,
        Permission::MandateWrite,
        Permission::DisputeWrite,
    ])
});
