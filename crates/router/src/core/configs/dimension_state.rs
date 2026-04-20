use std::marker::PhantomData;

use common_enums::{
    connector_enums::Connector,
    enums::{CardNetwork, Currency},
    PayoutRetryType,
};
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
    #[error("currency not available in dimension state")]
    MissingCurrency,
    #[error("payout_retry_type not available in dimension state")]
    MissingPayoutRetryType,
    #[error("network not available in dimension state")]
    MissingNetwork,
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

/// Marker for state WITH merchant_id
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

/// Marker for state WITHOUT currency
#[derive(Clone)]
pub struct NoCurrency;

/// Marker for state WITH currency
#[derive(Clone)]
pub struct HasCurrency;

/// Marker for state WITHOUT payout_retry_type
#[derive(Clone)]
pub struct NoPayoutRetryType;

/// Marker for state WITH payout_retry_type
#[derive(Clone)]
pub struct HasPayoutRetryType;

/// Marker for state WITH network
#[derive(Clone)]
pub struct HasNetwork;

// Dimensional State with type parameters

/// Dimensional state with type-level guarantees about which dimensions are present.
///
/// Uses the type-state pattern where type parameters indicate which fields are available.
///
/// # Type Parameters
/// * `Pm` - Provider Merchant ID type: `HasProviderMerchantId` or `NoProviderMerchantId`
/// * `M` - Processor Merchant ID type: `HasProcessorMerchantId` or `NoProcessorMerchantId`
/// * `O` - Organization ID type: `HasOrgId` or `NoOrgId`
/// * `P` - Profile ID type: `HasProfileId` or `NoProfileId`
/// * `Cn` - Connector type: `HasConnector` or `NoConnector`
/// * `Cu` - Currency type: `HasCurrency` or `NoCurrency`
/// * `PRT` - Payout retry type / Network: `HasPayoutRetryType`, `HasNetwork`, or `NoPayoutRetryType`
#[derive(Clone)]
pub struct Dimensions<Pm, M, O, P, Cn, Cu = NoCurrency, PRT = NoPayoutRetryType> {
    provider_merchant_id: Option<ProviderMerchantId>,
    processor_merchant_id: Option<ProcessorMerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    currency: Option<Currency>,
    payout_retry_type: Option<PayoutRetryType>,
    network: Option<CardNetwork>,
    _phantom: PhantomData<(Pm, M, O, P, Cn, Cu, PRT)>,
}

impl
    Dimensions<
        NoProviderMerchantId,
        NoProcessorMerchantId,
        NoOrgId,
        NoProfileId,
        NoConnector,
        NoCurrency,
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
            currency: None,
            payout_retry_type: None,
            network: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add provider_merchant_id if not already present
impl<M, O, P, Cn, Cu, PRT> Dimensions<NoProviderMerchantId, M, O, P, Cn, Cu, PRT> {
    pub fn with_provider_merchant_id(
        &self,
        id: ProviderMerchantId,
    ) -> Dimensions<HasProviderMerchantId, M, O, P, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: Some(id),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add processor_merchant_id if not already present
impl<Pm, O, P, Cn, Cu, PRT> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, Cu, PRT> {
    pub fn with_processor_merchant_id(
        &self,
        id: ProcessorMerchantId,
    ) -> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: Some(id),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<Pm, M, P, Cn, Cu, PRT> Dimensions<Pm, M, NoOrgId, P, Cn, Cu, PRT> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<Pm, M, HasOrgId, P, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: Some(id),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<Pm, M, O, Cn, Cu, PRT> Dimensions<Pm, M, O, NoProfileId, Cn, Cu, PRT> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<Pm, M, O, HasProfileId, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: Some(id),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<Pm, M, O, P, Cu, PRT> Dimensions<Pm, M, O, P, NoConnector, Cu, PRT> {
    pub fn with_connector(
        &self,
        connector: Connector,
    ) -> Dimensions<Pm, M, O, P, HasConnector, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add currency if not already present
impl<Pm, M, O, P, Cn, PRT> Dimensions<Pm, M, O, P, Cn, NoCurrency, PRT> {
    pub fn with_currency(
        &self,
        currency: Currency,
    ) -> Dimensions<Pm, M, O, P, Cn, HasCurrency, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: Some(currency),
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only add payout_retry_type if in the neutral state
impl<Pm, M, O, P, Cn, Cu> Dimensions<Pm, M, O, P, Cn, Cu, NoPayoutRetryType> {
    pub fn with_payout_retry_type(
        &self,
        payout_retry_type: PayoutRetryType,
    ) -> Dimensions<Pm, M, O, P, Cn, Cu, HasPayoutRetryType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: Some(payout_retry_type),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn with_network(
        &self,
        network: CardNetwork,
    ) -> Dimensions<Pm, M, O, P, Cn, Cu, HasNetwork> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: Some(network),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove provider_merchant_id if currently present
impl<M, O, P, Cn, Cu, PRT> Dimensions<HasProviderMerchantId, M, O, P, Cn, Cu, PRT> {
    pub fn without_provider_merchant_id(
        &self,
    ) -> Dimensions<NoProviderMerchantId, M, O, P, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: None,
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove processor_merchant_id if currently present
impl<Pm, O, P, Cn, Cu, PRT> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, Cu, PRT> {
    pub fn without_processor_merchant_id(
        &self,
    ) -> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: None,
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove organization_id if currently present
impl<Pm, M, P, Cn, Cu, PRT> Dimensions<Pm, M, HasOrgId, P, Cn, Cu, PRT> {
    pub fn without_organization_id(&self) -> Dimensions<Pm, M, NoOrgId, P, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: None,
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove profile_id if currently present
impl<Pm, M, O, Cn, Cu, PRT> Dimensions<Pm, M, O, HasProfileId, Cn, Cu, PRT> {
    pub fn without_profile_id(&self) -> Dimensions<Pm, M, O, NoProfileId, Cn, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: None,
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove connector if currently present
impl<Pm, M, O, P, Cu, PRT> Dimensions<Pm, M, O, P, HasConnector, Cu, PRT> {
    pub fn without_connector(&self) -> Dimensions<Pm, M, O, P, NoConnector, Cu, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: None,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove currency if currently present
impl<Pm, M, O, P, Cn, PRT> Dimensions<Pm, M, O, P, Cn, HasCurrency, PRT> {
    pub fn without_currency(&self) -> Dimensions<Pm, M, O, P, Cn, NoCurrency, PRT> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: None,
            payout_retry_type: self.payout_retry_type.clone(),
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove payout_retry_type if currently present
impl<Pm, M, O, P, Cn, Cu> Dimensions<Pm, M, O, P, Cn, Cu, HasPayoutRetryType> {
    pub fn without_payout_retry_type(&self) -> Dimensions<Pm, M, O, P, Cn, Cu, NoPayoutRetryType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: None,
            network: self.network.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove network if currently present
impl<Pm, M, O, P, Cn, Cu> Dimensions<Pm, M, O, P, Cn, Cu, HasNetwork> {
    pub fn without_network(&self) -> Dimensions<Pm, M, O, P, Cn, Cu, NoPayoutRetryType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            currency: self.currency,
            payout_retry_type: self.payout_retry_type.clone(),
            network: None,
            _phantom: PhantomData,
        }
    }
}

/// provider_merchant_id getter - only available if HasProviderMerchantId
impl<M, O, P, Cn, Cu, PRT> Dimensions<HasProviderMerchantId, M, O, P, Cn, Cu, PRT> {
    pub fn provider_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.provider_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProviderMerchantId)
    }
}

/// processor_merchant_id getter - only available if HasProcessorMerchantId
impl<Pm, O, P, Cn, Cu, PRT> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, Cu, PRT> {
    pub fn processor_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.processor_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProcessorMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<Pm, M, P, Cn, Cu, PRT> Dimensions<Pm, M, HasOrgId, P, Cn, Cu, PRT> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<Pm, M, O, Cn, Cu, PRT> Dimensions<Pm, M, O, HasProfileId, Cn, Cu, PRT> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<Pm, M, O, P, Cu, PRT> Dimensions<Pm, M, O, P, HasConnector, Cu, PRT> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

/// currency getter - only available if HasCurrency
impl<Pm, M, O, P, Cn, PRT> Dimensions<Pm, M, O, P, Cn, HasCurrency, PRT> {
    pub fn currency(&self) -> Result<Currency, DimensionError> {
        self.currency.ok_or(DimensionError::MissingCurrency)
    }
}

/// payout_retry_type getter - only available if HasPayoutRetryType
impl<Pm, M, O, P, Cn, Cu> Dimensions<Pm, M, O, P, Cn, Cu, HasPayoutRetryType> {
    pub fn payout_retry_type(&self) -> Result<PayoutRetryType, DimensionError> {
        self.payout_retry_type
            .clone()
            .ok_or(DimensionError::MissingPayoutRetryType)
    }
}

/// network getter - only available if HasNetwork
impl<Pm, M, O, P, Cn, Cu> Dimensions<Pm, M, O, P, Cn, Cu, HasNetwork> {
    pub fn network(&self) -> Result<CardNetwork, DimensionError> {
        self.network.clone().ok_or(DimensionError::MissingNetwork)
    }
}

// Optional getters (available in any state)
impl<Pm, M, O, P, Cn, Cu, PRT> Dimensions<Pm, M, O, P, Cn, Cu, PRT> {
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

    pub fn get_currency(&self) -> Option<Currency> {
        self.currency
    }

    pub fn get_payout_retry_type(&self) -> Option<PayoutRetryType> {
        self.payout_retry_type.clone()
    }

    pub fn get_network(&self) -> Option<CardNetwork> {
        self.network.clone()
    }
}

// Superposition context conversion
impl<Pm, M, O, P, Cn, Cu, PRT> Dimensions<Pm, M, O, P, Cn, Cu, PRT> {
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

        if let Some(ref pid) = &self.profile_id {
            ctx = ctx.with("profile_id", pid.get_string_repr());
        }

        if let Some(conn) = self.connector {
            ctx = ctx.with("connector", conn.to_string().as_str());
        }

        if let Some(cur) = self.currency {
            ctx = ctx.with("currency", cur.to_string().as_str());
        }

        if let Some(ref prt) = self.payout_retry_type {
            ctx = ctx.with("payout_retry_type", prt.to_string().as_str());
        }

        if let Some(ref net) = self.network {
            ctx = ctx.with("network", net.to_string().as_str());
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
        NoCurrency,
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

    /// Get currency (if available)
    fn get_currency(&self) -> Option<Currency>;

    /// Get payout_retry_type (if available)
    fn get_payout_retry_type(&self) -> Option<PayoutRetryType>;

    /// Get network (if available)
    fn get_network(&self) -> Option<CardNetwork>;
}

impl<Pm, M, O, P, Cn, Cu, PRT> DimensionsBase for Dimensions<Pm, M, O, P, Cn, Cu, PRT> {
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

    fn get_currency(&self) -> Option<Currency> {
        self.get_currency()
    }

    fn get_payout_retry_type(&self) -> Option<PayoutRetryType> {
        self.get_payout_retry_type()
    }

    fn get_network(&self) -> Option<CardNetwork> {
        self.get_network()
    }
}

// Type aliases
pub type DimensionsWithProviderMerchantId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
// Type aliases - both provider and processor merchant IDs present
pub type DimensionsWithProcessorAndProviderMerchantId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithOrgId = Dimensions<
    NoProviderMerchantId,
    NoProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndConnector = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileIdAndConnector = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    HasConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndOrgId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndOrgIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    HasProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndOrgIdAndConnectorAndCurrency = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    HasConnector,
    HasCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndPayoutRetryType = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    HasPayoutRetryType,
>;
pub type DimensionsWithProcessorMerchantIdAndOrgId = Dimensions<
    NoProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithConnectorAndCurrencyAndNetwork = Dimensions<
    NoProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    HasCurrency,
    HasNetwork,
>;
pub type DimensionsWithProcessorMerchantId = Dimensions<
    NoProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    NoPayoutRetryType,
>;
pub type DimensionsWithNetwork = Dimensions<
    NoProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoCurrency,
    HasNetwork,
>;
