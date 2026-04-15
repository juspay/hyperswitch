use std::marker::PhantomData;

use common_enums::{connector_enums::Connector, PayoutRetryType};

use common_utils::id_type;
use external_services::superposition;
pub use hyperswitch_domain_models::platform::{ProcessorMerchantId, ProviderMerchantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DimensionError {
    #[error("provider_merchant_id not available in dimension state")]
    MissingProviderMerchantId,
    #[error("processor_merchant_id not available in dimension state")]
    MissingProcessorMerchantId,
    #[error("organization_id not available in dimension state")]
    MissingOrganizationId,
    #[error("profile_id not available in dimension state")]
    MissingProfileId,
    #[error("connector not available in dimension state")]
    MissingConnector,
    #[error("payout_retry_type not available in dimension state")]
    MissingPayoutRetryType,
}

/// Marker for state WITHOUT provider_merchant_id
#[derive(Clone)]
pub struct NoProviderMerchantId;

/// Marker for state WITH provider_merchant_id
#[derive(Clone)]
pub struct HasProviderMerchantId;

/// Marker for state WITHOUT processor_merchant_id
#[derive(Clone)]
pub struct NoProcessorMerchantId;

/// Marker for state WITH processor_merchant_id
#[derive(Clone)]
pub struct HasProcessorMerchantId;

/// Marker for state WITHOUT organization_id
#[derive(Clone)]
pub struct NoOrgId;

/// Marker for state WITH organization_id
#[derive(Clone)]
pub struct HasOrgId;

/// Marker for state WITHOUT profile_id
#[derive(Clone)]
pub struct NoProfileId;

/// Marker for state WITH profile_id
#[derive(Clone)]
pub struct HasProfileId;

/// Marker for state WITHOUT connector
#[derive(Clone)]
pub struct NoConnector;

/// Marker for state WITH connector
#[derive(Clone)]
pub struct HasConnector;

/// Marker for state WITHOUT payout_retry_type
#[derive(Clone)]
pub struct NoPayoutRetryType;

/// Marker for state WITH payout_retry_type
#[derive(Clone)]
pub struct HasPayoutRetryType;

// Dimensional State with type parameters

/// Dimensional state with type-level guarantees about which dimensions are present.
///
/// Uses the type-state pattern where type parameters indicate which fields are available.
///
/// # Type Parameters
/// * `Pm` - Provider Merchant ID type: `HasProviderMerchantId` (present) or `NoProviderMerchantId` (absent)
/// * `M` - Processor Merchant ID type: `HasProcessorMerchantId` (present) or `NoProcessorMerchantId` (absent)
/// * `O` - Organization ID type: `HasOrgId` (present) or `NoOrgId` (absent)
/// * `P` - Profile ID type: `HasProfileId` (present) or `NoProfileId` (absent)
/// * `Cn` - Connector type: `HasConnector` (present) or `NoConnector` (absent)
/// * `Pr` - Payout retry type: `HasPayoutRetryType` (present) or `NoPayoutRetryType` (absent)
#[derive(Clone)]
pub struct Dimensions<Pm, M, O, P, Cn, Pr> {
    provider_merchant_id: Option<ProviderMerchantId>,
    processor_merchant_id: Option<ProcessorMerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    payout_retry_type: Option<PayoutRetryType>,
    _phantom: PhantomData<(Pm, M, O, P, Cn, Pr)>,
}

impl
    Dimensions<
        NoProviderMerchantId,
        NoProcessorMerchantId,
        NoOrgId,
        NoProfileId,
        NoConnector,
        NoPayoutRetryType,
    >
{
    pub fn new() -> Self {
        Self {
            provider_merchant_id: None,
            processor_merchant_id: None,
            organization_id: None,
            profile_id: None,
            connector: None,
            payout_retry_type: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add provider_merchant_id if not already present
impl<M, O, P, Cn, Pr> Dimensions<NoProviderMerchantId, M, O, P, Cn, Pr> {
    pub fn with_provider_merchant_id(
        &self,
        id: ProviderMerchantId,
    ) -> Dimensions<HasProviderMerchantId, M, O, P, Cn, Pr> {
        Dimensions {
            provider_merchant_id: Some(id),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add processor_merchant_id if not already present
impl<Pm, O, P, Cn, Pr> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, Pr> {
    pub fn with_processor_merchant_id(
        &self,
        id: ProcessorMerchantId,
    ) -> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: Some(id),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<Pm, M, P, Cn, Pr> Dimensions<Pm, M, NoOrgId, P, Cn, Pr> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<Pm, M, HasOrgId, P, Cn, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: Some(id),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<Pm, M, O, Cn, Pr> Dimensions<Pm, M, O, NoProfileId, Cn, Pr> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<Pm, M, O, HasProfileId, Cn, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: Some(id),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<Pm, M, O, P, Pr> Dimensions<Pm, M, O, P, NoConnector, Pr> {
    pub fn with_connector(
        &self,
        connector: Connector,
    ) -> Dimensions<Pm, M, O, P, HasConnector, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add payout_retry_type if not already present
impl<Pm, M, O, P, Cn> Dimensions<Pm, M, O, P, Cn, NoPayoutRetryType> {
    pub fn with_payout_retry_type(
        &self,
        prt: PayoutRetryType,
    ) -> Dimensions<Pm, M, O, P, Cn, HasPayoutRetryType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: Some(prt),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove provider_merchant_id if currently present
impl<M, O, P, Cn, Pr> Dimensions<HasProviderMerchantId, M, O, P, Cn, Pr> {
    pub fn without_provider_merchant_id(
        &self,
    ) -> Dimensions<NoProviderMerchantId, M, O, P, Cn, Pr> {
        Dimensions {
            provider_merchant_id: None,
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove processor_merchant_id if currently present
impl<Pm, O, P, Cn, Pr> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, Pr> {
    pub fn without_processor_merchant_id(
        &self,
    ) -> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: None,
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove organization_id if currently present
impl<Pm, M, P, Cn, Pr> Dimensions<Pm, M, HasOrgId, P, Cn, Pr> {
    pub fn without_organization_id(&self) -> Dimensions<Pm, M, NoOrgId, P, Cn, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: None,
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove profile_id if currently present
impl<Pm, M, O, Cn, Pr> Dimensions<Pm, M, O, HasProfileId, Cn, Pr> {
    pub fn without_profile_id(&self) -> Dimensions<Pm, M, O, NoProfileId, Cn, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: None,
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove connector if currently present
impl<Pm, M, O, P, Pr> Dimensions<Pm, M, O, P, HasConnector, Pr> {
    pub fn without_connector(&self) -> Dimensions<Pm, M, O, P, NoConnector, Pr> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: None,
            payout_retry_type: self.payout_retry_type.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove payout_retry_type if currently present
impl<Pm, M, O, P, Cn> Dimensions<Pm, M, O, P, Cn, HasPayoutRetryType> {
    pub fn without_payout_retry_type(
        &self,
    ) -> Dimensions<Pm, M, O, P, Cn, NoPayoutRetryType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: None,
            _phantom: PhantomData,
        }
    }
}

/// provider_merchant_id getter - only available if HasProviderMerchantId
impl<M, O, P, Cn, Pr> Dimensions<HasProviderMerchantId, M, O, P, Cn, Pr> {
    pub fn provider_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.provider_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProviderMerchantId)
    }
}

/// processor_merchant_id getter - only available if HasProcessorMerchantId
impl<Pm, O, P, Cn, Pr> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, Pr> {
    pub fn processor_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.processor_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProcessorMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<Pm, M, P, Cn, Pr> Dimensions<Pm, M, HasOrgId, P, Cn, Pr> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<Pm, M, O, Cn, Pr> Dimensions<Pm, M, O, HasProfileId, Cn, Pr> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<Pm, M, O, P, Pr> Dimensions<Pm, M, O, P, HasConnector, Pr> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

/// payout_retry_type getter - only available if HasPayoutRetryType
impl<Pm, M, O, P, Cn> Dimensions<Pm, M, O, P, Cn, HasPayoutRetryType> {
    pub fn payout_retry_type(&self) -> Result<PayoutRetryType, DimensionError> {
        self.payout_retry_type
            .clone()
            .ok_or(DimensionError::MissingPayoutRetryType)
    }
}

// Optional getters (available in any state)
impl<Pm, M, O, P, Cn, Pr> Dimensions<Pm, M, O, P, Cn, Pr> {
    pub fn get_provider_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.provider_merchant_id.as_ref().map(|id| id.inner())
    }

    pub fn get_processor_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.processor_merchant_id.as_ref().map(|id| id.inner())
    }

    pub fn get_organization_id(&self) -> Option<&id_type::OrganizationId> {
        self.organization_id.as_ref()
    }

    pub fn get_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.profile_id.as_ref()
    }

    pub fn get_connector(&self) -> Option<Connector> {
        self.connector
    }

    pub fn get_payout_retry_type(&self) -> Option<&PayoutRetryType> {
        self.payout_retry_type.as_ref()
    }
}

// Superposition context conversion
impl<Pm, M, O, P, Cn, Pr> Dimensions<Pm, M, O, P, Cn, Pr> {
    /// Converts dimension state to Superposition config context
    pub fn to_superposition_context(&self) -> Option<superposition::ConfigContext> {
        let mut ctx = superposition::ConfigContext::new();

        if let Some(ref pm_id) = self.provider_merchant_id {
            ctx = ctx.with("provider_merchant_id", pm_id.inner().get_string_repr());
        }

        if let Some(ref mid) = self.processor_merchant_id {
            ctx = ctx.with("processor_merchant_id", mid.inner().get_string_repr());
        }
        if let Some(ref oid) = self.organization_id {
            ctx = ctx.with("organization_id", oid.get_string_repr());
        }
        if let Some(ref pid) = self.profile_id {
            ctx = ctx.with("profile_id", pid.get_string_repr());
        }

        if let Some(conn) = self.connector {
            ctx = ctx.with("connector", conn.to_string().as_str());
        }

        if let Some(ref prt) = self.payout_retry_type {
            ctx = ctx.with("payout_retry_type", prt.to_string().as_str());
        }

        Some(ctx)
    }
}

impl Default
    for Dimensions<
        NoProviderMerchantId,
        NoProcessorMerchantId,
        NoOrgId,
        NoProfileId,
        NoConnector,
        NoPayoutRetryType,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

/// Base trait for all Dimensions types - enables polymorphic access to dimension methods
pub trait DimensionsBase {
    /// Converts dimension state to Superposition config context
    fn to_superposition_context(&self) -> Option<superposition::ConfigContext>;

    /// Get provider_merchant_id (if available)
    fn get_provider_merchant_id(&self) -> Option<&id_type::MerchantId>;

    /// Get processor_merchant_id (if available)
    fn get_processor_merchant_id(&self) -> Option<&id_type::MerchantId>;

    /// Get organization_id (if available)
    fn get_organization_id(&self) -> Option<&id_type::OrganizationId>;

    /// Get profile_id (if available)
    fn get_profile_id(&self) -> Option<&id_type::ProfileId>;

    /// Get connector (if available)
    fn get_connector(&self) -> Option<Connector>;

    /// Get payout_retry_type (if available)
    fn get_payout_retry_type(&self) -> Option<&PayoutRetryType>;
}

impl<Pm, M, O, P, Cn, Pr> DimensionsBase for Dimensions<Pm, M, O, P, Cn, Pr> {
    fn to_superposition_context(&self) -> Option<superposition::ConfigContext> {
        self.to_superposition_context()
    }

    fn get_provider_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.get_provider_merchant_id()
    }

    fn get_processor_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.get_processor_merchant_id()
    }

    fn get_organization_id(&self) -> Option<&id_type::OrganizationId> {
        self.get_organization_id()
    }

    fn get_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.get_profile_id()
    }

    fn get_connector(&self) -> Option<Connector> {
        self.get_connector()
    }

    fn get_payout_retry_type(&self) -> Option<&PayoutRetryType> {
        self.get_payout_retry_type()
    }
}

pub type DimensionsWithProviderMerchantId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndOrgId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndOrgIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    HasOrgId,
    HasProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndConnector = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndProfileIdAndConnector = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    HasConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndPayoutRetryType = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    HasPayoutRetryType,
>;
pub type DimensionsWithProviderMerchantIdAndProfileIdAndPayoutRetryType = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    HasPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndPayoutRetryType = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    HasPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileIdAndPayoutRetryType = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    HasPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndOrgId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndConnector = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileIdAndConnector = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    HasConnector,
    NoPayoutRetryType,
>;
