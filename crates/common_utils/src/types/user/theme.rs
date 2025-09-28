use common_enums::EntityType;
use serde::{Deserialize, Serialize};

use crate::{
    events::{ApiEventMetric, ApiEventsType},
    id_type, impl_api_event_type,
};

/// Enum for having all the required lineage for every level.
/// Currently being used for theme related APIs and queries.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "entity_type", rename_all = "snake_case")]
pub enum ThemeLineage {
    /// Tenant lineage variant
    Tenant {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
    },
    /// Org lineage variant
    Organization {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
    },
    /// Merchant lineage variant
    Merchant {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
    },
    /// Profile lineage variant
    Profile {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
        /// profile_id: ProfileId
        profile_id: id_type::ProfileId,
    },
}

impl_api_event_type!(Miscellaneous, (ThemeLineage));

impl ThemeLineage {
    /// Constructor for ThemeLineage
    pub fn new(
        entity_type: EntityType,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
    ) -> Self {
        match entity_type {
            EntityType::Tenant => Self::Tenant { tenant_id },
            EntityType::Organization => Self::Organization { tenant_id, org_id },
            EntityType::Merchant => Self::Merchant {
                tenant_id,
                org_id,
                merchant_id,
            },
            EntityType::Profile => Self::Profile {
                tenant_id,
                org_id,
                merchant_id,
                profile_id,
            },
        }
    }

    /// Get the entity_type from the lineage
    pub fn entity_type(&self) -> EntityType {
        match self {
            Self::Tenant { .. } => EntityType::Tenant,
            Self::Organization { .. } => EntityType::Organization,
            Self::Merchant { .. } => EntityType::Merchant,
            Self::Profile { .. } => EntityType::Profile,
        }
    }

    /// Get the tenant_id from the lineage
    pub fn tenant_id(&self) -> &id_type::TenantId {
        match self {
            Self::Tenant { tenant_id }
            | Self::Organization { tenant_id, .. }
            | Self::Merchant { tenant_id, .. }
            | Self::Profile { tenant_id, .. } => tenant_id,
        }
    }

    /// Get the org_id from the lineage
    pub fn org_id(&self) -> Option<&id_type::OrganizationId> {
        match self {
            Self::Tenant { .. } => None,
            Self::Organization { org_id, .. }
            | Self::Merchant { org_id, .. }
            | Self::Profile { org_id, .. } => Some(org_id),
        }
    }

    /// Get the merchant_id from the lineage
    pub fn merchant_id(&self) -> Option<&id_type::MerchantId> {
        match self {
            Self::Tenant { .. } | Self::Organization { .. } => None,
            Self::Merchant { merchant_id, .. } | Self::Profile { merchant_id, .. } => {
                Some(merchant_id)
            }
        }
    }

    /// Get the profile_id from the lineage
    pub fn profile_id(&self) -> Option<&id_type::ProfileId> {
        match self {
            Self::Tenant { .. } | Self::Organization { .. } | Self::Merchant { .. } => None,
            Self::Profile { profile_id, .. } => Some(profile_id),
        }
    }

    /// Get higher lineages from the current lineage
    pub fn get_same_and_higher_lineages(self) -> Vec<Self> {
        match &self {
            Self::Tenant { .. } => vec![self],
            Self::Organization { tenant_id, .. } => vec![
                Self::Tenant {
                    tenant_id: tenant_id.clone(),
                },
                self,
            ],
            Self::Merchant {
                tenant_id, org_id, ..
            } => vec![
                Self::Tenant {
                    tenant_id: tenant_id.clone(),
                },
                Self::Organization {
                    tenant_id: tenant_id.clone(),
                    org_id: org_id.clone(),
                },
                self,
            ],
            Self::Profile {
                tenant_id,
                org_id,
                merchant_id,
                ..
            } => vec![
                Self::Tenant {
                    tenant_id: tenant_id.clone(),
                },
                Self::Organization {
                    tenant_id: tenant_id.clone(),
                    org_id: org_id.clone(),
                },
                Self::Merchant {
                    tenant_id: tenant_id.clone(),
                    org_id: org_id.clone(),
                    merchant_id: merchant_id.clone(),
                },
                self,
            ],
        }
    }
}

/// Struct for holding the theme settings for email
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EmailThemeConfig {
    /// The entity name to be used in the email
    pub entity_name: String,

    /// The URL of the entity logo to be used in the email
    pub entity_logo_url: String,

    /// The primary color to be used in the email
    pub primary_color: String,

    /// The foreground color to be used in the email
    pub foreground_color: String,

    /// The background color to be used in the email
    pub background_color: String,
}
