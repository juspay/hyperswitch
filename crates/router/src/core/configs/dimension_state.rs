use std::marker::PhantomData;

use api_models::webhooks::IncomingWebhookEvent;
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
///
/// The dimension is optional in value even when present in type: the builder accepts an
/// `Option`, and an absent value is simply omitted from the Superposition context so the
/// lookup falls back to the default rather than resolving against a guessed subtype.
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
/// * `PMT` - Payment Method Type: `HasPaymentMethodType` or `NoPaymentMethodType`
#[derive(Clone)]
pub struct Dimensions<Pm, M, O, P, Cn, PRT, Ev, PMT> {
    provider_merchant_id: Option<ProviderMerchantId>,
    processor_merchant_id: Option<ProcessorMerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    payout_retry_type: Option<PayoutRetryType>,
    incoming_webhook_event: Option<IncomingWebhookEvent>,
    payment_method_type: Option<common_enums::PaymentMethodType>,
    _phantom: PhantomData<(Pm, M, O, P, Cn, PRT, Ev, PMT)>,
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
impl<M, O, P, Cn, PRT, Ev, PMT> Dimensions<NoProviderMerchantId, M, O, P, Cn, PRT, Ev, PMT> {
    pub fn with_provider_merchant_id(
        &self,
        id: ProviderMerchantId,
    ) -> Dimensions<HasProviderMerchantId, M, O, P, Cn, PRT, Ev, PMT> {
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
impl<Pm, O, P, Cn, PRT, Ev, PMT> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, PRT, Ev, PMT> {
    pub fn with_processor_merchant_id(
        &self,
        id: ProcessorMerchantId,
    ) -> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, PRT, Ev, PMT> {
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
impl<Pm, M, P, Cn, PRT, Ev, PMT> Dimensions<Pm, M, NoOrgId, P, Cn, PRT, Ev, PMT> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<Pm, M, HasOrgId, P, Cn, PRT, Ev, PMT> {
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
impl<Pm, M, O, Cn, PRT, Ev, PMT> Dimensions<Pm, M, O, NoProfileId, Cn, PRT, Ev, PMT> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<Pm, M, O, HasProfileId, Cn, PRT, Ev, PMT> {
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
impl<Pm, M, O, P, PRT, Ev, PMT> Dimensions<Pm, M, O, P, NoConnector, PRT, Ev, PMT> {
    pub fn with_connector(
        &self,
        connector: Connector,
    ) -> Dimensions<Pm, M, O, P, HasConnector, PRT, Ev, PMT> {
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
impl<Pm, M, O, P, Cn, Ev, PMT> Dimensions<Pm, M, O, P, Cn, NoPayoutRetryType, Ev, PMT> {
    pub fn with_payout_retry_type(
        &self,
        retry_type: PayoutRetryType,
    ) -> Dimensions<Pm, M, O, P, Cn, HasPayoutRetryType, Ev, PMT> {
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
impl<Pm, M, O, P, Cn, PRT, PMT> Dimensions<Pm, M, O, P, Cn, PRT, NoWebhookEvent, PMT> {
    pub fn with_incoming_webhook_event(
        &self,
        event: IncomingWebhookEvent,
    ) -> Dimensions<Pm, M, O, P, Cn, PRT, HasWebhookEvent, PMT> {
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
    /// Takes an `Option` because callers derive the subtype from data that may not carry it.
    /// `None` leaves the dimension out of the Superposition context entirely.
    pub fn with_payment_method_type(
        &self,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> Dimensions<Pm, M, O, P, Cn, PRT, Ev, HasPaymentMethodType> {
        Dimensions {
            provider_merchant_id: self.provider_merchant_id.clone(),
            processor_merchant_id: self.processor_merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payout_retry_type: self.payout_retry_type.clone(),
            incoming_webhook_event: self.incoming_webhook_event,
            payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove provider_merchant_id if currently present
impl<M, O, P, Cn, PRT, Ev, PMT> Dimensions<HasProviderMerchantId, M, O, P, Cn, PRT, Ev, PMT> {
    pub fn without_provider_merchant_id(
        &self,
    ) -> Dimensions<NoProviderMerchantId, M, O, P, Cn, PRT, Ev, PMT> {
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
impl<Pm, O, P, Cn, PRT, Ev, PMT> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, PRT, Ev, PMT> {
    pub fn without_processor_merchant_id(
        &self,
    ) -> Dimensions<Pm, NoProcessorMerchantId, O, P, Cn, PRT, Ev, PMT> {
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
impl<Pm, M, P, Cn, PRT, Ev, PMT> Dimensions<Pm, M, HasOrgId, P, Cn, PRT, Ev, PMT> {
    pub fn without_organization_id(&self) -> Dimensions<Pm, M, NoOrgId, P, Cn, PRT, Ev, PMT> {
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
impl<Pm, M, O, Cn, PRT, Ev, PMT> Dimensions<Pm, M, O, HasProfileId, Cn, PRT, Ev, PMT> {
    pub fn without_profile_id(&self) -> Dimensions<Pm, M, O, NoProfileId, Cn, PRT, Ev, PMT> {
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
impl<Pm, M, O, P, PRT, Ev, PMT> Dimensions<Pm, M, O, P, HasConnector, PRT, Ev, PMT> {
    pub fn without_connector(&self) -> Dimensions<Pm, M, O, P, NoConnector, PRT, Ev, PMT> {
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
impl<Pm, M, O, P, Cn, PRT, PMT> Dimensions<Pm, M, O, P, Cn, PRT, HasWebhookEvent, PMT> {
    pub fn without_incoming_webhook_event(
        &self,
    ) -> Dimensions<Pm, M, O, P, Cn, PRT, NoWebhookEvent, PMT> {
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
impl<M, O, P, Cn, PRT, Ev, PMT> Dimensions<HasProviderMerchantId, M, O, P, Cn, PRT, Ev, PMT> {
    pub fn provider_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.provider_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProviderMerchantId)
    }
}

/// processor_merchant_id getter - only available if HasProcessorMerchantId
impl<Pm, O, P, Cn, PRT, Ev, PMT> Dimensions<Pm, HasProcessorMerchantId, O, P, Cn, PRT, Ev, PMT> {
    pub fn processor_merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.processor_merchant_id
            .as_ref()
            .map(|id| id.inner())
            .ok_or(DimensionError::MissingProcessorMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<Pm, M, P, Cn, PRT, Ev, PMT> Dimensions<Pm, M, HasOrgId, P, Cn, PRT, Ev, PMT> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<Pm, M, O, Cn, PRT, Ev, PMT> Dimensions<Pm, M, O, HasProfileId, Cn, PRT, Ev, PMT> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<Pm, M, O, P, PRT, Ev, PMT> Dimensions<Pm, M, O, P, HasConnector, PRT, Ev, PMT> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

// Optional getters (available in any state)
impl<Pm, M, O, P, Cn, PRT, Ev, PMT> Dimensions<Pm, M, O, P, Cn, PRT, Ev, PMT> {
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

    pub fn get_payment_method_type(&self) -> Option<common_enums::PaymentMethodType> {
        self.payment_method_type
    }
}

// Superposition context conversion
impl<Pm, M, O, P, Cn, PRT, Ev, PMT> Dimensions<Pm, M, O, P, Cn, PRT, Ev, PMT> {
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

        if let Some(payment_method_type) = self.payment_method_type {
            ctx = ctx.with(
                "payment_method_type",
                payment_method_type.to_string().as_str(),
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
    fn get_payment_method_type(&self) -> Option<common_enums::PaymentMethodType>;
}

impl<Pm, M, O, P, Cn, PRT, Ev, PMT> DimensionsBase for Dimensions<Pm, M, O, P, Cn, PRT, Ev, PMT> {
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

    fn get_payment_method_type(&self) -> Option<common_enums::PaymentMethodType> {
        self.get_payment_method_type()
    }
}

// Type aliases
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

// Type alias - processor merchant ID, connector and payment method type present
pub type DimensionsWithProcessorMerchantIdAndConnectorAndPaymentMethodType = Dimensions<
    NoProviderMerchantId,
    HasProcessorMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPayoutRetryType,
    NoWebhookEvent,
    HasPaymentMethodType,
>;

#[cfg(test)]
mod tests {
    use super::*;

    fn merchant_id() -> id_type::MerchantId {
        id_type::MerchantId::try_from(std::borrow::Cow::from("merchant_1234"))
            .expect("merchant id should be valid")
    }

    fn pcr_dimensions(
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> DimensionsWithProcessorMerchantIdAndConnectorAndPaymentMethodType {
        Dimensions::new()
            .with_processor_merchant_id(merchant_id().into())
            .with_connector(Connector::Stripe)
            .with_payment_method_type(payment_method_type)
    }

    #[test]
    fn payment_method_type_is_sent_as_a_snake_case_dimension_when_present() {
        let context = pcr_dimensions(Some(common_enums::PaymentMethodType::GooglePay))
            .to_superposition_context()
            .expect("context should be built");

        assert_eq!(context.get("payment_method_type"), Some("google_pay"));
    }

    #[test]
    fn payment_method_type_is_omitted_when_absent_so_lookups_fall_back_to_the_default() {
        let context = pcr_dimensions(None)
            .to_superposition_context()
            .expect("context should be built");

        assert_eq!(context.get("payment_method_type"), None);
        // The dimensions it travels with are unaffected by the missing subtype.
        assert_eq!(context.get("connector"), Some("stripe"));
        assert_eq!(context.get("processor_merchant_id"), Some("merchant_1234"));
    }
}
