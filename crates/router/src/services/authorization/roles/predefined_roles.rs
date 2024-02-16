use std::collections::HashMap;

#[cfg(feature = "olap")]
use error_stack::ResultExt;
use once_cell::sync::Lazy;

#[cfg(feature = "olap")]
use crate::core::errors::{UserErrors, UserResult};
use crate::{consts, services::authorization::permission_groups::PermissionGroup};

use super::RoleInfo;

pub static PREDEFINED_ROLES: Lazy<HashMap<&'static str, RoleInfo>> = Lazy::new(|| {
    let mut roles = HashMap::new();
    roles.insert(
        consts::user_role::ROLE_ID_INTERNAL_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
                PermissionGroup::ConnectorsView,
                PermissionGroup::ConnectorsManage,
                PermissionGroup::WorkflowsView,
                PermissionGroup::WorkflowsManage,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::OrganizationManage,
            ],
            role_id: consts::user_role::ROLE_ID_INTERNAL_ADMIN.to_string(),
            role_name: "Internal Admin".to_string(),
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
            is_internal: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER.to_string(),
            role_name: "Internal View Only".to_string(),
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
            is_internal: true,
        },
    );

    roles.insert(
        consts::user_role::ROLE_ID_ORGANIZATION_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
                PermissionGroup::ConnectorsView,
                PermissionGroup::ConnectorsManage,
                PermissionGroup::WorkflowsView,
                PermissionGroup::WorkflowsManage,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::OrganizationManage,
            ],
            role_id: consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            role_name: "Organization Admin".to_string(),
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
            is_internal: false,
        },
    );

    // MERCHANT ROLES
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
                PermissionGroup::ConnectorsView,
                PermissionGroup::ConnectorsManage,
                PermissionGroup::WorkflowsView,
                PermissionGroup::WorkflowsManage,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::MerchantDetailsManage,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_ADMIN.to_string(),
            role_name: "Admin".to_string(),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_VIEW_ONLY,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_VIEW_ONLY.to_string(),
            role_name: "View Only".to_string(),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_IAM_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_IAM_ADMIN.to_string(),
            role_name: "IAM".to_string(),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_DEVELOPER,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
                PermissionGroup::ConnectorsView,
                PermissionGroup::ConnectorsManage,
                PermissionGroup::WorkflowsView,
                PermissionGroup::WorkflowsManage,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::MerchantDetailsManage,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_DEVELOPER.to_string(),
            role_name: "Developer".to_string(),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_OPERATOR,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
                PermissionGroup::ConnectorsView,
                PermissionGroup::ConnectorsManage,
                PermissionGroup::WorkflowsView,
                PermissionGroup::WorkflowsManage,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_OPERATOR.to_string(),
            role_name: "Operator".to_string(),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_CUSTOMER_SUPPORT,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_CUSTOMER_SUPPORT.to_string(),
            role_name: "Customer Support".to_string(),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles
});

#[cfg(feature = "olap")]
pub fn is_role_invitable(role_id: &str) -> UserResult<bool> {
    PREDEFINED_ROLES
        .get(role_id)
        .map(|role_info| role_info.is_invitable)
        .ok_or(UserErrors::InvalidRoleId.into())
        .attach_printable(format!("role_id = {} doesn't exist", role_id))
}

#[cfg(feature = "olap")]
pub fn is_role_deletable(role_id: &str) -> UserResult<bool> {
    PREDEFINED_ROLES
        .get(role_id)
        .map(|role_info| role_info.is_deletable)
        .ok_or(UserErrors::InvalidRoleId.into())
        .attach_printable(format!("role_id = {} doesn't exist", role_id))
}

#[cfg(feature = "olap")]
pub fn is_role_updatable(role_id: &str) -> UserResult<bool> {
    PREDEFINED_ROLES
        .get(role_id)
        .map(|role_info| role_info.is_updatable)
        .ok_or(UserErrors::InvalidRoleId.into())
        .attach_printable(format!("role_id = {} doesn't exist", role_id))
}
