use std::marker::PhantomData;

use common_utils::id_type;
use external_services::superposition;

use crate::types;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DimensionError {
    #[error("merchant_id not available in dimension state")]
    MissingMerchantId,
    #[error("organization_id not available in dimension state")]
    MissingOrganizationId,
    #[error("profile_id not available in dimension state")]
    MissingProfileId,
}

/// Marker for state WITHOUT merchant_id
pub struct NoMerchantId;

/// Marker for state WITH merchant_id
pub struct HasMerchantId;

/// Marker for state WITHOUT organization_id
pub struct NoOrgId;

/// Marker for state WITH organization_id
pub struct HasOrgId;

/// Marker for state WITHOUT profile_id
pub struct NoProfileId;

/// Marker for state WITH profile_id
pub struct HasProfileId;

/// Marker for state WITHOUT connector
pub struct NoConnector;

/// Marker for state WITH connector
pub struct HasConnector;

// Dimensional State with type parameters

/// Dimensional state with type-level guarantees about which dimensions are present.
///
/// Uses the type-state pattern where type parameters indicate which fields are available.
///
/// # Type Parameters
/// * `M` - Merchant ID type: `HasMerchantId` (present) or `NoMerchantId` (absent)
/// * `O` - Organization ID type: `HasOrgId` (present) or `NoOrgId` (absent)
/// * `P` - Profile ID type: `HasProfileId` (present) or `NoProfileId` (absent)
/// * `C` - Connector type: `HasConnector` (present) or `NoConnector` (absent)
pub struct Dimensions<M, O, P, C> {
    merchant_id: Option<id_type::MerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<types::Connector>,
    _phantom: PhantomData<(M, O, P, C)>,
}

impl Dimensions<NoMerchantId, NoOrgId, NoProfileId, NoConnector> {
    pub fn new() -> Self {
        Self {
            merchant_id: None,
            organization_id: None,
            profile_id: None,
            connector: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add merchant_id if not already present
impl<O, P, C> Dimensions<NoMerchantId, O, P, C> {
    pub fn with_merchant_id(self, id: id_type::MerchantId) -> Dimensions<HasMerchantId, O, P, C> {
        Dimensions {
            merchant_id: Some(id),
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            connector: self.connector,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<M, P, C> Dimensions<M, NoOrgId, P, C> {
    pub fn with_organization_id(
        self,
        id: id_type::OrganizationId,
    ) -> Dimensions<M, HasOrgId, P, C> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: Some(id),
            profile_id: self.profile_id,
            connector: self.connector,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<M, O, C> Dimensions<M, O, NoProfileId, C> {
    pub fn with_profile_id(self, id: id_type::ProfileId) -> Dimensions<M, O, HasProfileId, C> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: self.organization_id,
            profile_id: Some(id),
            connector: self.connector,
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<M, O, P> Dimensions<M, O, P, NoConnector> {
    pub fn with_connector(&self, connector: types::Connector) -> Dimensions<M, O, P, HasConnector> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            _phantom: PhantomData,
        }
    }
}

/// merchant_id getter - only available if HasMerchantId
impl<O, P, C> Dimensions<HasMerchantId, O, P, C> {
    pub fn merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.merchant_id
            .as_ref()
            .ok_or(DimensionError::MissingMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<M, P, C> Dimensions<M, HasOrgId, P, C> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<M, O, C> Dimensions<M, O, HasProfileId, C> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<M, O, P> Dimensions<M, O, P, HasConnector> {
    pub fn connector(&self) -> Result<types::Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingProfileId)
    }
}

// Optional getters (available in any state)
impl<M, O, P, C> Dimensions<M, O, P, C> {
    pub fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.merchant_id.as_ref()
    }

    pub fn get_organization_id(&self) -> Option<&id_type::OrganizationId> {
        self.organization_id.as_ref()
    }

    pub fn get_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.profile_id.as_ref()
    }

    pub fn get_connector(&self) -> Option<types::Connector> {
        self.connector
    }
}

// Superposition context conversion
impl<M, O, P, C> Dimensions<M, O, P, C> {
    /// Converts dimension state to Superposition config context
    pub fn to_superposition_context(&self) -> Option<superposition::ConfigContext> {
        let mut ctx = superposition::ConfigContext::new();

        if let Some(ref mid) = self.merchant_id {
            ctx = ctx.with("merchant_id", mid.get_string_repr());
        }

        if let Some(ref oid) = self.organization_id {
            ctx = ctx.with("organization_id", oid.get_string_repr());
        }

        if let Some(ref pid) = &self.profile_id {
            ctx = ctx.with("profile_id", pid.get_string_repr());
        }

        if let Some(connector) = self.connector {
            ctx = ctx.with("connector", &connector.to_string())
        }
        Some(ctx)
    }
}

impl Default for Dimensions<NoMerchantId, NoOrgId, NoProfileId, NoConnector> {
    fn default() -> Self {
        Self::new()
    }
}

/// Base trait for all Dimensions types - enables polymorphic access to dimension methods
pub trait DimensionsBase {
    /// Converts dimension state to Superposition config context
    fn to_superposition_context(&self) -> Option<superposition::ConfigContext>;

    /// Get merchant_id (if available)
    fn get_merchant_id(&self) -> Option<&id_type::MerchantId>;

    /// Get organization_id (if available)
    fn get_organization_id(&self) -> Option<&id_type::OrganizationId>;

    /// Get profile_id (if available)
    fn get_profile_id(&self) -> Option<&id_type::ProfileId>;

    fn get_connector(&self) -> Option<types::Connector>;
}

impl<M, O, P, C> DimensionsBase for Dimensions<M, O, P, C> {
    fn to_superposition_context(&self) -> Option<superposition::ConfigContext> {
        self.to_superposition_context()
    }

    fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.get_merchant_id()
    }

    fn get_organization_id(&self) -> Option<&id_type::OrganizationId> {
        self.get_organization_id()
    }

    fn get_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.get_profile_id()
    }

    fn get_connector(&self) -> Option<types::Connector> {
        self.get_connector()
    }
}

pub type DimensionsWithMerchantId = Dimensions<HasMerchantId, NoOrgId, NoProfileId, NoConnector>;
pub type DimensionsWithMerchantIdAndProfileId =
    Dimensions<HasMerchantId, NoOrgId, HasProfileId, NoConnector>;
pub type DimensionsWithMerchantIdAndConnector =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, HasConnector>;
