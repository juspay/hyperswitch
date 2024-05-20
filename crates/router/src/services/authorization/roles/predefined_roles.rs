use std::collections::HashMap;

use common_enums::{PermissionGroup, RoleScope};
use once_cell::sync::Lazy;

use super::RoleInfo;
use crate::consts;

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
            role_name: "internal_admin".to_string(),
            scope: RoleScope::Organization,
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
            role_name: "internal_view_only".to_string(),
            scope: RoleScope::Organization,
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
            role_name: "organization_admin".to_string(),
            scope: RoleScope::Organization,
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
            role_name: "admin".to_string(),
            scope: RoleScope::Organization,
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
            role_name: "view_only".to_string(),
            scope: RoleScope::Organization,
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
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_IAM_ADMIN.to_string(),
            role_name: "iam".to_string(),
            scope: RoleScope::Organization,
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
                PermissionGroup::ConnectorsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::MerchantDetailsManage,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_DEVELOPER.to_string(),
            role_name: "developer".to_string(),
            scope: RoleScope::Organization,
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
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_OPERATOR.to_string(),
            role_name: "operator".to_string(),
            scope: RoleScope::Organization,
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
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_CUSTOMER_SUPPORT.to_string(),
            role_name: "customer_support".to_string(),
            scope: RoleScope::Organization,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles
});
