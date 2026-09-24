use std::marker::PhantomData;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::{connector_enums::Connector, PaymentMethodType, PayoutRetryType};
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
    #[error("payment_method_type not available in dimension state")]
    MissingPaymentMethodType,
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

/// Marker for state WITHOUT incoming_webhook_event
#[derive(Clone)]
pub struct NoWebhookEvent;

/// Marker for state WITH incoming_webhook_event
#[derive(Clone)]
pub struct HasWebhookEvent;

/// Marker for state WITHOUT payment_method_type
#[derive(Clone)]
pub struct NoPaymentMethodType;

/// Marker for state WITH payment_method_type
#[derive(Clone)]
pub struct HasPaymentMethodType;

// Dimensional State with type parameters

/// Dimensional state with type-level guarantees about which dimensions are present.
///
/// Uses the type-state pattern where type parameters indicate which fields are available.
///
/// # Type Parameters
/// * `Pm`  - Provider Merchant ID: `HasProviderMerchantId` or `NoProviderMerchantId`
/// * `M`   - Processor Merchant ID: `HasProcessorMerchantId` or `NoProcessorMerchantId`
/// * `O`   - Organization ID: `HasOrgId` or `NoOrgId`
/// * `P`   - Profile ID: `HasProfileId` or `NoProfileId`
/// * `Cn`  - Connector: `HasConnector` or `NoConnector`
/// * `PRT` - Payout Retry Type: `HasPayoutRetryType` or `NoPayoutRetryType`
/// * `Ev`  - Webhook Event type: `HasWebhookEvent` (present) or `NoWebhookEvent` (absent)
/// * `Pmt` - Payment Method Type: `HasPaymentMethodType` or `NoPaymentMethodType`
#[derive(Clone)]
pub struct Dimensions<Pm, M, O, P, Cn, PRT, Ev, Pmt> {
    provider_merchant_id: Option<ProviderMerchantId>,
    processor_merchant_id: Option<ProcessorMerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    payout_retry_type: Option<PayoutRetryType>,
    incoming_webhook_event: Option<IncomingWebhookEvent>,
    payment_method_type: Option<PaymentMethodType>,
    _phantom: PhantomData<(Pm, M, O, P, Cn, PRT, Ev, Pmt)>,
}

impl
    Dimensions<
        NoProviderMerchantId,
        NoProcessorMerchantId,
        NoOrgId,
        NoProfileId,
        NoConnector,
        NoPayoutRetryType,
        NoWebhookEvent,
        NoPaymentMethodType,
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
            incoming_webhook_event: None,
            payment_method_type: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add provider_merchant_id if not already present
impl<M, O, P, Cn, PRT, Ev, Pmt> Dimensions<NoProviderMerchantId, M, O, P, Cn, PRT, Ev, Pmt> {
    pub fn with_provider_merchant_id(
        &self,
        id: ProviderMerchantId,
    ) -> Dimensions<HasProviderMerchantId, M, O, P, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: Some(id),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add processor_merchant_id if not already present
impl<Pm, O, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, PRT, Ev, Pmt> {
    pub fn with_processor_merchant_id(
        &self,
        id: ProcessorMerchantId,
    ) -> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: Some(id),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<Pm, M, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, NoOrgId, P, Cn, PRT, Ev, Pmt> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<Pm, M, HasOrgId, P, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: Some(id),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<Pm, M, O, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, O, NoProfileId, Cn, PRT, Ev, Pmt> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<Pm, M, O, HasProfileId, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: Some(id),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<Pm, M, O, P, PRT, Ev, Pmt> Dimensions<Pm, M, O, P, NoConnector, PRT, Ev, Pmt> {
    pub fn with_connector(
        &self,
        connector: Connector,
    ) -> Dimensions<Pm, M, O, P, HasConnector, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payout_retry_type if not already present
impl<Pm, M, O, P, Cn, Ev, Pmt> Dimensions<Pm, M, O, P, Cn, NoPayoutRetryType, Ev, Pmt> {
    pub fn with_payout_retry_type(
        &self,
        retry_type: PayoutRetryType,
    ) -> Dimensions<Pm, M, O, P, Cn, HasPayoutRetryType, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: Some(retry_type),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add incoming_webhook_event if not already present
impl<Pm, M, O, P, Cn, PRT, Pmt> Dimensions<Pm, M, O, P, Cn, PRT, NoWebhookEvent, Pmt> {
    pub fn with_incoming_webhook_event(
        &self,
        event: IncomingWebhookEvent,
    ) -> Dimensions<Pm, M, O, P, Cn, PRT, HasWebhookEvent, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: Some(event),
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payment_method_type if not already present
impl<Pm, M, O, P, Cn, PRT, Ev> Dimensions<Pm, M, O, P, Cn, PRT, Ev, NoPaymentMethodType> {
    pub fn with_payment_method_type(
        &self,
        payment_method_type: PaymentMethodType,
    ) -> Dimensions<Pm, M, O, P, Cn, PRT, Ev, HasPaymentMethodType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: Some(payment_method_type),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove provider_merchant_id if currently present
impl<M, O, P, Cn, PRT, Ev, Pmt> Dimensions<HasProviderMerchantId, M, O, P, Cn, PRT, Ev, Pmt> {
    pub fn without_provider_merchant_id(
        &self,
    ) -> Dimensions<NoProviderMerchantId, M, O, P, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: None,
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove processor_merchant_id if currently present
impl<Pm, O, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, PRT, Ev, Pmt> {
    pub fn without_processor_merchant_id(
        &self,
    ) -> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: None,
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove organization_id if currently present
impl<Pm, M, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, HasOrgId, P, Cn, PRT, Ev, Pmt> {
    pub fn without_organization_id(&self) -> Dimensions<Pm, M, NoOrgId, P, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: None,
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove profile_id if currently present
impl<Pm, M, O, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, O, HasProfileId, Cn, PRT, Ev, Pmt> {
    pub fn without_profile_id(&self) -> Dimensions<Pm, M, O, NoProfileId, Cn, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: None,
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove connector if currently present
impl<Pm, M, O, P, PRT, Ev, Pmt> Dimensions<Pm, M, O, P, HasConnector, PRT, Ev, Pmt> {
    pub fn without_connector(&self) -> Dimensions<Pm, M, O, P, NoConnector, PRT, Ev, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: None,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove incoming_webhook_event if currently present
impl<Pm, M, O, P, Cn, PRT, Pmt> Dimensions<Pm, M, O, P, Cn, PRT, HasWebhookEvent, Pmt> {
    pub fn without_incoming_webhook_event(
        &self,
    ) -> Dimensions<Pm, M, O, P, Cn, PRT, NoWebhookEvent, Pmt> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: None,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// provider_merchant_id getter - only available if HasProviderMerchantId
impl<M, O, P, Cn, PRT, Ev, Pmt> Dimensions<HasProviderMerchantId, M, O, P, Cn, PRT, Ev, Pmt> {
    pub fn provider_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.provider_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProviderMerchantId)
    }
}

/// processor_merchant_id getter - only available if HasProcessorMerchantId
impl<Pm, O, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, PRT, Ev, Pmt> {
    pub fn processor_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.processor_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProcessorMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<Pm, M, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, HasOrgId, P, Cn, PRT, Ev, Pmt> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<Pm, M, O, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, O, HasProfileId, Cn, PRT, Ev, Pmt> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<Pm, M, O, P, PRT, Ev, Pmt> Dimensions<Pm, M, O, P, HasConnector, PRT, Ev, Pmt> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

/// payment_method_type getter - only available if HasPaymentMethodType
impl<Pm, M, O, P, Cn, PRT, Ev> Dimensions<Pm, M, O, P, Cn, PRT, Ev, HasPaymentMethodType> {
    pub fn payment_method_type(&self) -> Result<PaymentMethodType, DimensionError> {
        self.payment_method_type
            .ok_or(DimensionError::MissingPaymentMethodType)
    }
}

// Optional getters (available in any state)
impl<Pm, M, O, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, O, P, Cn, PRT, Ev, Pmt> {
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

    pub fn get_incoming_webhook_event(&self) -> Option<IncomingWebhookEvent> {
        self.incoming_webhook_event
    }

    pub fn get_payment_method_type(&self) -> Option<PaymentMethodType> {
        self.payment_method_type
    }
}

// Superposition context conversion
impl<Pm, M, O, P, Cn, PRT, Ev, Pmt> Dimensions<Pm, M, O, P, Cn, PRT, Ev, Pmt> {
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

        if let Some(ref prt) = self.payout_retry_type {
            ctx = ctx.with("payout_retry_type", prt.to_string().as_str());
        }

        if let Some(event) = self.incoming_webhook_event {
            if let Ok(serde_json::Value::String(s)) = serde_json::to_value(event) {
                ctx = ctx.with("incoming_webhook_events", s.as_str());
            }
        }

        if let Some(pmt) = self.payment_method_type {
            ctx = ctx.with(
                "payment_method_type",
                pmt.superposition_dimension_value().as_str(),
            );
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
        NoWebhookEvent,
        NoPaymentMethodType,
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

    /// Get incoming_webhook_event (if available)
    fn get_incoming_webhook_event(&self) -> Option<IncomingWebhookEvent>;

    /// Get payment_method_type (if available)
    fn get_payment_method_type(&self) -> Option<PaymentMethodType>;
}

impl<Pm, M, O, P, Cn, PRT, Ev, Pmt> DimensionsBase for Dimensions<Pm, M, O, P, Cn, PRT, Ev, Pmt> {
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

    fn get_incoming_webhook_event(&self) -> Option<IncomingWebhookEvent> {
        self.get_incoming_webhook_event()
    }

    fn get_payment_method_type(&self) -> Option<PaymentMethodType> {
        self.get_payment_method_type()
    }
}

// Type aliases

// Global config scope: no Superposition dimensions are required.
pub type DimensionsGlobal = Dimensions<
    NoProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;

pub type DimensionsWithProviderMerchantId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;

// Type alias - provider merchant ID and organization ID present
pub type DimensionsWithProviderMerchantIdAndOrgId = Dimensions<
    HasProviderMerchantId,
    NoProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;

// Type alias - only processor merchant ID present
pub type DimensionsWithProcessorMerchantId = Dimensions<
    NoProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;

// Type alias - processor merchant ID and connector present
pub type DimensionsWithProcessorMerchantIdAndConnector = Dimensions<
    NoProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;

// Type aliases - both provider and processor merchant IDs present
pub type DimensionsWithProcessorAndProviderMerchantId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithOrgId = Dimensions<
    NoProviderMerchantId,
    NoProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndConnector = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndProfileIdAndConnector = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    HasProfileId,
    HasConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndOrgId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndOrgIdAndProfileId = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    HasOrgId,
    HasProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndPayoutRetryType = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    HasPayoutRetryType,
    NoWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorAndProviderMerchantIdAndConnectorAndWebhookEvent = Dimensions<
    HasProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPayoutRetryType,
    HasWebhookEvent,
    NoPaymentMethodType,
>;
pub type DimensionsWithProcessorMerchantIdAndPaymentMethodType = Dimensions<
    NoProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    HasPaymentMethodType,
>;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::collections::HashSet;

    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn payment_method_type_dimension_values_match_superposition_seed() {
        let seed_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/superposition_seed.toml"
        );
        let seeded_values = config::Config::builder()
            .add_source(config::File::new(seed_path, config::FileFormat::Toml))
            .build()
            .expect("Failed to read superposition seed")
            .get::<Vec<String>>("dimensions.payment_method_type.schema.enum")
            .expect("payment_method_type dimension missing from superposition seed");

        let seeded_values: HashSet<&str> = seeded_values.iter().map(String::as_str).collect();
        let dimension_values: HashSet<String> = PaymentMethodType::iter()
            .map(|payment_method_type| payment_method_type.superposition_dimension_value())
            .collect();
        let dimension_values: HashSet<&str> = dimension_values.iter().map(String::as_str).collect();

        // The seed is shared across API versions, so it also lists variants that only exist when
        // the `v2` feature is enabled.
        #[cfg(not(feature = "v2"))]
        let feature_gated_values: HashSet<&str> = HashSet::from(["Card"]);
        #[cfg(feature = "v2")]
        let feature_gated_values: HashSet<&str> = HashSet::new();

        let mut not_seeded: Vec<&str> = dimension_values
            .difference(&seeded_values)
            .copied()
            .collect();
        not_seeded.sort_unstable();
        assert!(
            not_seeded.is_empty(),
            "PaymentMethodType values missing from the superposition seed: {not_seeded:?}"
        );

        let mut unknown: Vec<&str> = seeded_values
            .difference(&dimension_values)
            .copied()
            .filter(|value| !feature_gated_values.contains(value))
            .collect();
        unknown.sort_unstable();
        assert!(
            unknown.is_empty(),
            "Seeded payment_method_type values not produced by any PaymentMethodType: {unknown:?}"
        );
    }
}
