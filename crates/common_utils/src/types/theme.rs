use common_enums::EntityType;

use crate::{
    events::{ApiEventMetric, ApiEventsType},
    id_type, impl_api_event_type,
};

/// Enum for having all the required lineage for every level.
/// Currently being used for theme related APIs and queries.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "entity_type")]
#[serde(rename_all = "snake_case")]
pub enum ThemeLineage {
    // TODO: Add back Tenant variant when we introduce Tenant Variant in EntityType
    // /// Tenant lineage variant
    // Tenant {
    //     /// tenant_id: String
    //     tenant_id: String,
    // },
    /// Org lineage variant
    Organization {
        /// tenant_id: String
        tenant_id: String,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
    },
    /// Merchant lineage variant
    Merchant {
        /// tenant_id: String
        tenant_id: String,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
    },
    /// Profile lineage variant
    Profile {
        /// tenant_id: String
        tenant_id: String,
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
    /// Get the entity_type from the lineage
    pub fn entity_type(&self) -> EntityType {
        match self {
            Self::Organization { .. } => EntityType::Organization,
            Self::Merchant { .. } => EntityType::Merchant,
            Self::Profile { .. } => EntityType::Profile,
        }
    }

    /// Get the tenant_id from the lineage
    pub fn tenant_id(&self) -> &str {
        match self {
            Self::Organization { tenant_id, .. }
            | Self::Merchant { tenant_id, .. }
            | Self::Profile { tenant_id, .. } => tenant_id,
        }
    }

    /// Get the org_id from the lineage
    pub fn org_id(&self) -> Option<&id_type::OrganizationId> {
        match self {
            Self::Organization { org_id, .. }
            | Self::Merchant { org_id, .. }
            | Self::Profile { org_id, .. } => Some(org_id),
        }
    }

    /// Get the merchant_id from the lineage
    pub fn merchant_id(&self) -> Option<&id_type::MerchantId> {
        match self {
            Self::Organization { .. } => None,
            Self::Merchant { merchant_id, .. } | Self::Profile { merchant_id, .. } => {
                Some(merchant_id)
            }
        }
    }

    /// Get the profile_id from the lineage
    pub fn profile_id(&self) -> Option<&id_type::ProfileId> {
        match self {
            Self::Organization { .. } | Self::Merchant { .. } => None,
            Self::Profile { profile_id, .. } => Some(profile_id),
        }
    }
}
