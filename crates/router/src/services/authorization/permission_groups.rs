use std::collections::HashMap;

use common_enums::{EntityType, ParentGroup, PermissionGroup, PermissionScope, Resource};
use strum::IntoEnumIterator;

use super::permissions::{self, ResourceExt};

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
            | Self::MerchantDetailsView
            | Self::AccountView
            | Self::ReconOpsView
            | Self::ReconReportsView => PermissionScope::Read,

            Self::OperationsManage
            | Self::ConnectorsManage
            | Self::WorkflowsManage
            | Self::UsersManage
            | Self::MerchantDetailsManage
            | Self::OrganizationManage
            | Self::AccountManage
            | Self::ReconOpsManage
            | Self::ReconReportsManage => PermissionScope::Write,
        }
    }

    fn parent(&self) -> ParentGroup {
        match self {
            Self::OperationsView | Self::OperationsManage => ParentGroup::Operations,
            Self::ConnectorsView | Self::ConnectorsManage => ParentGroup::Connectors,
            Self::WorkflowsView | Self::WorkflowsManage => ParentGroup::Workflows,
            Self::AnalyticsView => ParentGroup::Analytics,
            Self::UsersView | Self::UsersManage => ParentGroup::Users,
            Self::MerchantDetailsView
            | Self::OrganizationManage
            | Self::MerchantDetailsManage
            | Self::AccountView
            | Self::AccountManage => ParentGroup::Account,
            Self::ReconOpsView | Self::ReconOpsManage => ParentGroup::ReconOps,
            Self::ReconReportsView | Self::ReconReportsManage => ParentGroup::ReconReports,
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

            Self::MerchantDetailsView => vec![Self::MerchantDetailsView],
            Self::MerchantDetailsManage => {
                vec![Self::MerchantDetailsView, Self::MerchantDetailsManage]
            }

            Self::OrganizationManage => vec![Self::OrganizationManage],

            Self::AccountView => vec![Self::AccountView],
            Self::AccountManage => vec![Self::AccountView, Self::AccountManage],
        }
    }
}

pub trait ParentGroupExt {
    fn resources(&self) -> Vec<Resource>;
    fn get_descriptions_for_groups(
        entity_type: EntityType,
        groups: Vec<PermissionGroup>,
    ) -> HashMap<ParentGroup, String>;
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
        }
    }

    fn get_descriptions_for_groups(
        entity_type: EntityType,
        groups: Vec<PermissionGroup>,
    ) -> HashMap<Self, String> {
        Self::iter()
            .filter_map(|parent| {
                let scopes = groups
                    .iter()
                    .filter(|group| group.parent() == parent)
                    .map(|group| group.scope())
                    .max()?;

                let resources = parent
                    .resources()
                    .iter()
                    .filter(|res| res.entities().iter().any(|entity| entity <= &entity_type))
                    .map(|res| permissions::get_resource_name(*res, entity_type))
                    .collect::<Vec<_>>()
                    .join(", ");

                Some((
                    parent,
                    format!("{} {}", permissions::get_scope_name(scopes), resources),
                ))
            })
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

pub static WORKFLOWS: [Resource; 4] = [
    Resource::Routing,
    Resource::ThreeDsDecisionManager,
    Resource::SurchargeDecisionManager,
    Resource::Account,
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

pub static RECON_REPORTS: [Resource; 4] = [
    Resource::ReconToken,
    Resource::ReconAndSettlementAnalytics,
    Resource::ReconReports,
    Resource::Account,
];
