use std::marker::PhantomData;

use common_utils::id_type;
use external_services::superposition;

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

// Dimensional State with type parameters

/// Dimensional state with type-level guarantees about which dimensions are present.
///
/// Uses the type-state pattern where type parameters indicate which fields are available.
///
/// # Type Parameters
/// * `M` - Merchant ID type: `HasMerchantId` (present) or `NoMerchantId` (absent)
/// * `O` - Organization ID type: `HasOrgId` (present) or `NoOrgId` (absent)
/// * `P` - Profile ID type: `HasProfileId` (present) or `NoProfileId` (absent)
pub struct Dimensions<M, O, P> {
    merchant_id: Option<id_type::MerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    _phantom: PhantomData<(M, O, P)>,
}

impl Dimensions<NoMerchantId, NoOrgId, NoProfileId> {
    pub fn new() -> Self {
        Self {
            merchant_id: None,
            organization_id: None,
            profile_id: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add merchant_id if not already present
impl<O, P> Dimensions<NoMerchantId, O, P> {
    pub fn with_merchant_id(self, id: id_type::MerchantId) -> Dimensions<HasMerchantId, O, P> {
        Dimensions {
            merchant_id: Some(id),
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<M, P> Dimensions<M, NoOrgId, P> {
    pub fn with_organization_id(self, id: id_type::OrganizationId) -> Dimensions<M, HasOrgId, P> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: Some(id),
            profile_id: self.profile_id,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<M, O> Dimensions<M, O, NoProfileId> {
    pub fn with_profile_id(self, id: id_type::ProfileId) -> Dimensions<M, O, HasProfileId> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: self.organization_id,
            profile_id: Some(id),
            _phantom: PhantomData,
        }
    }
}

/// merchant_id getter - only available if HasMerchantId
impl<O, P> Dimensions<HasMerchantId, O, P> {
    pub fn merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.merchant_id
            .as_ref()
            .ok_or(DimensionError::MissingMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<M, P> Dimensions<M, HasOrgId, P> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<M, O> Dimensions<M, O, HasProfileId> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

// Optional getters (available in any state)
impl<M, O, P> Dimensions<M, O, P> {
    pub fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.merchant_id.as_ref()
    }

    pub fn get_organization_id(&self) -> Option<&id_type::OrganizationId> {
        self.organization_id.as_ref()
    }

    pub fn get_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.profile_id.as_ref()
    }
}

// Superposition context conversion
impl<M, O, P> Dimensions<M, O, P> {
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
        Some(ctx)
    }
}

impl Default for Dimensions<NoMerchantId, NoOrgId, NoProfileId> {
    fn default() -> Self {
        Self::new()
    }
}

pub type DimensionsWithMerchantId = Dimensions<HasMerchantId, NoOrgId, NoProfileId>;
