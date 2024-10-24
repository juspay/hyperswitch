use super::permissions::{self, ResourceExt};
use common_enums::{EntityType, ParentGroup, PermissionGroup, PermissionScope, Resource};
use std::collections::HashMap;
use strum::IntoEnumIterator;

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
            | Self::MerchantDetailsView => PermissionScope::Read,

            Self::OperationsManage
            | Self::ConnectorsManage
            | Self::WorkflowsManage
            | Self::UsersManage
            | Self::MerchantDetailsManage
            | Self::OrganizationManage
            | Self::ReconOps => PermissionScope::Write,
        }
    }

    fn parent(&self) -> ParentGroup {
        match self {
            Self::OperationsView | Self::OperationsManage => ParentGroup::Operations,
            Self::ConnectorsView | Self::ConnectorsManage => ParentGroup::Connectors,
            Self::WorkflowsView | Self::WorkflowsManage => ParentGroup::Workflows,
            Self::AnalyticsView => ParentGroup::Analytics,
            Self::UsersView | Self::UsersManage => ParentGroup::Users,
            Self::MerchantDetailsView | Self::MerchantDetailsManage => ParentGroup::Merchant,
            Self::OrganizationManage => ParentGroup::Organization,
            Self::ReconOps => ParentGroup::Recon,
        }
    }

    fn resources(&self) -> Vec<Resource> {
        self.parent().resources()
    }

    fn accessible_groups(&self) -> Vec<Self> {
        match self {
            Self::OperationsView => vec![Self::OperationsView],
            Self::OperationsManage => vec![Self::OperationsView, Self::OperationsManage],

            Self::ConnectorsView => vec![Self::ConnectorsView],
            Self::ConnectorsManage => vec![Self::ConnectorsView, Self::ConnectorsManage],

            Self::WorkflowsView => vec![Self::WorkflowsView],
            Self::WorkflowsManage => vec![Self::WorkflowsView, Self::WorkflowsManage],

            Self::AnalyticsView => vec![Self::AnalyticsView],

            Self::UsersView => vec![Self::UsersView],
            Self::UsersManage => {
                vec![Self::UsersView, Self::UsersManage]
            }

            Self::ReconOps => vec![Self::ReconOps],

            Self::MerchantDetailsView => vec![Self::MerchantDetailsView],
            Self::MerchantDetailsManage => {
                vec![Self::MerchantDetailsView, Self::MerchantDetailsManage]
            }

            Self::OrganizationManage => vec![Self::OrganizationManage],
        }
    }
}

pub trait ParentGroupExt {
    fn resources(&self) -> Vec<Resource>;
    fn get_descriptions(
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
            Self::Merchant | Self::Organization => ACCOUNT.to_vec(),
            Self::Recon => RECON.to_vec(),
        }
    }

    fn get_descriptions(
        entity_type: EntityType,
        groups: Vec<PermissionGroup>,
    ) -> HashMap<Self, String> {
        ParentGroup::iter()
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
                    .map(|res| permissions::get_resource_name(res, &entity_type))
                    .collect::<Vec<_>>()
                    .join(", ");

                Some((
                    parent,
                    format!("{} {}", permissions::get_scope_name(&scopes), resources),
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

pub static WORKFLOWS: [Resource; 5] = [
    Resource::Routing,
    Resource::ThreeDsDecisionManager,
    Resource::SurchargeDecisionManager,
    Resource::Connector,
    Resource::Account,
];

pub static ANALYTICS: [Resource; 3] = [Resource::Analytics, Resource::Report, Resource::Account];

pub static USERS: [Resource; 2] = [Resource::User, Resource::Account];

pub static ACCOUNT: [Resource; 3] = [Resource::Account, Resource::ApiKey, Resource::WebhookEvent];

pub static RECON: [Resource; 1] = [Resource::Recon];
