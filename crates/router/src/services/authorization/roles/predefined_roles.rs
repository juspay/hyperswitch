use std::collections::HashMap;

use common_enums::{EntityType, PermissionGroup, RoleScope};
use once_cell::sync::Lazy;

use super::RoleInfo;
use crate::consts;

pub static PREDEFINED_ROLES: Lazy<HashMap<&'static str, RoleInfo>> = Lazy::new(|| {
    let mut roles = HashMap::new();

    // Internal Roles
    roles.insert(
        common_utils::consts::ROLE_ID_INTERNAL_ADMIN,
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
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
                PermissionGroup::OrganizationManage,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconOpsManage,
                PermissionGroup::ReconReportsView,
                PermissionGroup::ReconReportsManage,
            ],
            role_id: common_utils::consts::ROLE_ID_INTERNAL_ADMIN.to_string(),
            role_name: "internal_admin".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
            is_internal: true,
        },
    );
    roles.insert(
        common_utils::consts::ROLE_ID_INTERNAL_VIEW_ONLY_USER,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::AccountView,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconReportsView,
            ],
            role_id: common_utils::consts::ROLE_ID_INTERNAL_VIEW_ONLY_USER.to_string(),
            role_name: "internal_view_only".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
            is_internal: true,
        },
    );

    // Tenant Roles
    roles.insert(
        common_utils::consts::ROLE_ID_TENANT_ADMIN,
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
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
                PermissionGroup::OrganizationManage,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconOpsManage,
                PermissionGroup::ReconReportsView,
                PermissionGroup::ReconReportsManage,
            ],
            role_id: common_utils::consts::ROLE_ID_TENANT_ADMIN.to_string(),
            role_name: "tenant_admin".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Tenant,
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
            is_internal: false,
        },
    );

    // Organization Roles
    roles.insert(
        common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN,
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
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
                PermissionGroup::OrganizationManage,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconOpsManage,
                PermissionGroup::ReconReportsView,
                PermissionGroup::ReconReportsManage,
            ],
            role_id: common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            role_name: "organization_admin".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Organization,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
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
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconOpsManage,
                PermissionGroup::ReconReportsView,
                PermissionGroup::ReconReportsManage,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_ADMIN.to_string(),
            role_name: "merchant_admin".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
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
                PermissionGroup::AccountView,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconReportsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_VIEW_ONLY.to_string(),
            role_name: "merchant_view_only".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
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
                PermissionGroup::AccountView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_IAM_ADMIN.to_string(),
            role_name: "merchant_iam".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
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
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconReportsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_DEVELOPER.to_string(),
            role_name: "merchant_developer".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
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
                PermissionGroup::AccountView,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconOpsManage,
                PermissionGroup::ReconReportsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_OPERATOR.to_string(),
            role_name: "merchant_operator".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
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
                PermissionGroup::AccountView,
                PermissionGroup::ReconOpsView,
                PermissionGroup::ReconReportsView,
            ],
            role_id: consts::user_role::ROLE_ID_MERCHANT_CUSTOMER_SUPPORT.to_string(),
            role_name: "customer_support".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Merchant,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );

    // Profile Roles
    roles.insert(
        consts::user_role::ROLE_ID_PROFILE_ADMIN,
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
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
            ],
            role_id: consts::user_role::ROLE_ID_PROFILE_ADMIN.to_string(),
            role_name: "profile_admin".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Profile,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_PROFILE_VIEW_ONLY,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::AccountView,
            ],
            role_id: consts::user_role::ROLE_ID_PROFILE_VIEW_ONLY.to_string(),
            role_name: "profile_view_only".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Profile,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_PROFILE_IAM_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::UsersManage,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::AccountView,
            ],
            role_id: consts::user_role::ROLE_ID_PROFILE_IAM_ADMIN.to_string(),
            role_name: "profile_iam".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Profile,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_PROFILE_DEVELOPER,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::ConnectorsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::AccountView,
                PermissionGroup::MerchantDetailsManage,
                PermissionGroup::AccountManage,
            ],
            role_id: consts::user_role::ROLE_ID_PROFILE_DEVELOPER.to_string(),
            role_name: "profile_developer".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Profile,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_PROFILE_OPERATOR,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
                PermissionGroup::ConnectorsView,
                PermissionGroup::WorkflowsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::AccountView,
            ],
            role_id: consts::user_role::ROLE_ID_PROFILE_OPERATOR.to_string(),
            role_name: "profile_operator".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Profile,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_PROFILE_CUSTOMER_SUPPORT,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsView,
                PermissionGroup::AnalyticsView,
                PermissionGroup::UsersView,
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::AccountView,
            ],
            role_id: consts::user_role::ROLE_ID_PROFILE_CUSTOMER_SUPPORT.to_string(),
            role_name: "profile_customer_support".to_string(),
            scope: RoleScope::Organization,
            entity_type: EntityType::Profile,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        },
    );
    roles
});
