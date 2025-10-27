use std::{collections::HashMap, ops::Not};

use common_enums::{EntityType, ParentGroup, PermissionGroup, PermissionScope, Resource};
use strum::IntoEnumIterator;

use super::permissions;

pub trait PermissionGroupExt {
    fn scope(&self) -> PermissionScope;
    fn parent(&self) -> ParentGroup;
    fn resources(&self) -> Vec<Resource>;
    fn accessible_groups(&self) -> Vec<PermissionGroup>;
}

impl PermissionGroupExt for PermissionGroup {
    fn scope(&self) -> PermissionScope {
        match self {
            Self::OperationsView
            | Self::ConnectorsView
            | Self::WorkflowsView
            | Self::AnalyticsView
            | Self::UsersView
            | Self::AccountView
            | Self::ReconOpsView
            | Self::ReconReportsView
            | Self::ThemeView => PermissionScope::Read,

            Self::OperationsManage
            | Self::ConnectorsManage
            | Self::WorkflowsManage
            | Self::UsersManage
            | Self::AccountManage
            | Self::ReconOpsManage
            | Self::ReconReportsManage
            | Self::InternalManage
            | Self::ThemeManage => PermissionScope::Write,
        }
    }

    fn parent(&self) -> ParentGroup {
        match self {
            Self::OperationsView | Self::OperationsManage => ParentGroup::Operations,
            Self::ConnectorsView | Self::ConnectorsManage => ParentGroup::Connectors,
            Self::WorkflowsView | Self::WorkflowsManage => ParentGroup::Workflows,
            Self::AnalyticsView => ParentGroup::Analytics,
            Self::UsersView | Self::UsersManage => ParentGroup::Users,
            Self::AccountView | Self::AccountManage => ParentGroup::Account,

            Self::ThemeView | Self::ThemeManage => ParentGroup::Theme,
            Self::ReconOpsView | Self::ReconOpsManage => ParentGroup::ReconOps,
            Self::ReconReportsView | Self::ReconReportsManage => ParentGroup::ReconReports,
            Self::InternalManage => ParentGroup::Internal,
        }
    }

    fn resources(&self) -> Vec<Resource> {
        self.parent().resources()
    }

    fn accessible_groups(&self) -> Vec<Self> {
        match self {
            Self::OperationsView => vec![Self::OperationsView, Self::ConnectorsView],
            Self::OperationsManage => vec![
                Self::OperationsView,
                Self::OperationsManage,
                Self::ConnectorsView,
            ],

            Self::ConnectorsView => vec![Self::ConnectorsView],
            Self::ConnectorsManage => vec![Self::ConnectorsView, Self::ConnectorsManage],

            Self::WorkflowsView => vec![Self::WorkflowsView, Self::ConnectorsView],
            Self::WorkflowsManage => vec![
                Self::WorkflowsView,
                Self::WorkflowsManage,
                Self::ConnectorsView,
            ],

            Self::AnalyticsView => vec![Self::AnalyticsView, Self::OperationsView],

            Self::UsersView => vec![Self::UsersView],
            Self::UsersManage => {
                vec![Self::UsersView, Self::UsersManage]
            }

            Self::ReconOpsView => vec![Self::ReconOpsView],
            Self::ReconOpsManage => vec![Self::ReconOpsView, Self::ReconOpsManage],

            Self::ReconReportsView => vec![Self::ReconReportsView],
            Self::ReconReportsManage => vec![Self::ReconReportsView, Self::ReconReportsManage],

            Self::AccountView => vec![Self::AccountView],
            Self::AccountManage => vec![Self::AccountView, Self::AccountManage],

            Self::InternalManage => vec![Self::InternalManage],
            Self::ThemeView => vec![Self::ThemeView, Self::AccountView],
            Self::ThemeManage => vec![Self::ThemeManage, Self::AccountView],
        }
    }
}

pub trait ParentGroupExt {
    fn resources(&self) -> Vec<Resource>;
    fn get_descriptions_for_groups(
        entity_type: EntityType,
        groups: Vec<PermissionGroup>,
    ) -> Option<HashMap<ParentGroup, String>>;
    fn get_available_scopes(&self) -> Vec<PermissionScope>;
}

impl ParentGroupExt for ParentGroup {
    fn resources(&self) -> Vec<Resource> {
        match self {
            Self::Operations => OPERATIONS.to_vec(),
            Self::Connectors => CONNECTORS.to_vec(),
            Self::Workflows => WORKFLOWS.to_vec(),
            Self::Analytics => ANALYTICS.to_vec(),
            Self::Users => USERS.to_vec(),
            Self::Account => ACCOUNT.to_vec(),
            Self::ReconOps => RECON_OPS.to_vec(),
            Self::ReconReports => RECON_REPORTS.to_vec(),
            Self::Internal => INTERNAL.to_vec(),
            Self::Theme => THEME.to_vec(),
        }
    }

    fn get_descriptions_for_groups(
        entity_type: EntityType,
        groups: Vec<PermissionGroup>,
    ) -> Option<HashMap<Self, String>> {
        let descriptions_map = Self::iter()
            .filter_map(|parent| {
                if !groups.iter().any(|group| group.parent() == parent) {
                    return None;
                }
                let filtered_resources =
                    permissions::filter_resources_by_entity_type(parent.resources(), entity_type)?;

                let description = filtered_resources
                    .iter()
                    .map(|res| permissions::get_resource_name(*res, entity_type))
                    .collect::<Option<Vec<_>>>()?
                    .join(", ");

                Some((parent, description))
            })
            .collect::<HashMap<_, _>>();

        descriptions_map
            .is_empty()
            .not()
            .then_some(descriptions_map)
    }

    fn get_available_scopes(&self) -> Vec<PermissionScope> {
        PermissionGroup::iter()
            .filter(|group| group.parent() == *self)
            .map(|group| group.scope())
            .collect()
    }
}

pub static OPERATIONS: [Resource; 8] = [
    Resource::Payment,
    Resource::Refund,
    Resource::Mandate,
    Resource::Dispute,
    Resource::Customer,
    Resource::Payout,
    Resource::Report,
    Resource::Account,
];

pub static CONNECTORS: [Resource; 2] = [Resource::Connector, Resource::Account];

pub static WORKFLOWS: [Resource; 5] = [
    Resource::Routing,
    Resource::ThreeDsDecisionManager,
    Resource::SurchargeDecisionManager,
    Resource::Account,
    Resource::RevenueRecovery,
];

pub static ANALYTICS: [Resource; 3] = [Resource::Analytics, Resource::Report, Resource::Account];

pub static USERS: [Resource; 2] = [Resource::User, Resource::Account];

pub static ACCOUNT: [Resource; 3] = [Resource::Account, Resource::ApiKey, Resource::WebhookEvent];

pub static RECON_OPS: [Resource; 8] = [
    Resource::ReconToken,
    Resource::ReconFiles,
    Resource::ReconUpload,
    Resource::RunRecon,
    Resource::ReconConfig,
    Resource::ReconAndSettlementAnalytics,
    Resource::ReconReports,
    Resource::Account,
];

pub static INTERNAL: [Resource; 1] = [Resource::InternalConnector];

pub static RECON_REPORTS: [Resource; 4] = [
    Resource::ReconToken,
    Resource::ReconAndSettlementAnalytics,
    Resource::ReconReports,
    Resource::Account,
];

pub static THEME: [Resource; 1] = [Resource::Theme];
