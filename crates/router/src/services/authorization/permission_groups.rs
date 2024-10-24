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
            PermissionGroup::OperationsView
            | PermissionGroup::ConnectorsView
            | PermissionGroup::WorkflowsView
            | PermissionGroup::AnalyticsView
            | PermissionGroup::UsersView
            | PermissionGroup::MerchantDetailsView
            | PermissionGroup::ReconOps => PermissionScope::Read,

            PermissionGroup::OperationsManage
            | PermissionGroup::ConnectorsManage
            | PermissionGroup::WorkflowsManage
            | PermissionGroup::UsersManage
            | PermissionGroup::MerchantDetailsManage
            | PermissionGroup::OrganizationManage => PermissionScope::Write,
        }
    }

    fn parent(&self) -> ParentGroup {
        match self {
            PermissionGroup::OperationsView | PermissionGroup::OperationsManage => {
                ParentGroup::Operations
            }
            PermissionGroup::ConnectorsView | PermissionGroup::ConnectorsManage => {
                ParentGroup::Connectors
            }
            PermissionGroup::WorkflowsView | PermissionGroup::WorkflowsManage => {
                ParentGroup::Workflows
            }
            PermissionGroup::AnalyticsView => ParentGroup::Analytics,
            PermissionGroup::UsersView | PermissionGroup::UsersManage => ParentGroup::Users,
            PermissionGroup::MerchantDetailsView | PermissionGroup::MerchantDetailsManage => {
                ParentGroup::Merchant
            }
            PermissionGroup::OrganizationManage => ParentGroup::Organization,
            PermissionGroup::ReconOps => ParentGroup::Recon,
        }
    }

    fn resources(&self) -> Vec<Resource> {
        self.parent().resources()
    }

    fn accessible_groups(&self) -> Vec<PermissionGroup> {
        match self {
            PermissionGroup::OperationsView => vec![PermissionGroup::OperationsView],
            PermissionGroup::OperationsManage => vec![
                PermissionGroup::OperationsView,
                PermissionGroup::OperationsManage,
            ],

            PermissionGroup::ConnectorsView => vec![PermissionGroup::ConnectorsView],
            PermissionGroup::ConnectorsManage => vec![
                PermissionGroup::ConnectorsView,
                PermissionGroup::ConnectorsManage,
            ],

            PermissionGroup::WorkflowsView => vec![PermissionGroup::WorkflowsView],
            PermissionGroup::WorkflowsManage => vec![
                PermissionGroup::WorkflowsView,
                PermissionGroup::WorkflowsManage,
            ],

            PermissionGroup::AnalyticsView => vec![PermissionGroup::AnalyticsView],

            PermissionGroup::UsersView => vec![PermissionGroup::UsersView],
            PermissionGroup::UsersManage => {
                vec![PermissionGroup::UsersView, PermissionGroup::UsersManage]
            }

            PermissionGroup::ReconOps => vec![PermissionGroup::ReconOps],

            PermissionGroup::MerchantDetailsView => vec![PermissionGroup::MerchantDetailsView],
            PermissionGroup::MerchantDetailsManage => vec![
                PermissionGroup::MerchantDetailsView,
                PermissionGroup::MerchantDetailsManage,
            ],

            PermissionGroup::OrganizationManage => vec![PermissionGroup::OrganizationManage],
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
            ParentGroup::Operations => OPERATIONS.to_vec(),
            ParentGroup::Connectors => CONNECTORS.to_vec(),
            ParentGroup::Workflows => WORKFLOWS.to_vec(),
            ParentGroup::Analytics => ANALYTICS.to_vec(),
            ParentGroup::Users => USERS.to_vec(),
            ParentGroup::Merchant | ParentGroup::Organization => ACCOUNT.to_vec(),
            ParentGroup::Recon => RECON.to_vec(),
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
