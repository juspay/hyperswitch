use std::marker::PhantomData;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::connector_enums::Connector;
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
    #[error("connector not available in dimension state")]
    MissingConnector,
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

/// Marker for state WITHOUT incoming_webhook_event
pub struct NoWebhookEvent;

/// Marker for state WITH incoming_webhook_event
pub struct HasWebhookEvent;

// Dimensional State with type parameters

/// Dimensional state with type-level guarantees about which dimensions are present.
///
/// Uses the type-state pattern where type parameters indicate which fields are available.
///
/// # Type Parameters
/// * `M` - Merchant ID type: `HasMerchantId` (present) or `NoMerchantId` (absent)
/// * `O` - Organization ID type: `HasOrgId` (present) or `NoOrgId` (absent)
/// * `P` - Profile ID type: `HasProfileId` (present) or `NoProfileId` (absent)
/// * `Cn` - Connector type: `HasConnector` (present) or `NoConnector` (absent)
/// * `Ev` - Webhook Event type: `HasWebhookEvent` (present) or `NoWebhookEvent` (absent)
pub struct Dimensions<M, O, P, Cn, Ev> {
    merchant_id: Option<id_type::MerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    incoming_webhook_event: Option<IncomingWebhookEvent>,
    _phantom: PhantomData<(M, O, P, Cn, Ev)>,
}

impl Dimensions<NoMerchantId, NoOrgId, NoProfileId, NoConnector, NoWebhookEvent> {
    pub fn new() -> Self {
        Self {
            merchant_id: None,
            organization_id: None,
            profile_id: None,
            connector: None,
            incoming_webhook_event: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add merchant_id if not already present
impl<O, P, Cn, Ev> Dimensions<NoMerchantId, O, P, Cn, Ev> {
    pub fn with_merchant_id(
        &self,
        id: id_type::MerchantId,
    ) -> Dimensions<HasMerchantId, O, P, Cn, Ev> {
        Dimensions {
            merchant_id: Some(id),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<M, P, Cn, Ev> Dimensions<M, NoOrgId, P, Cn, Ev> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<M, HasOrgId, P, Cn, Ev> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: Some(id),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<M, O, Cn, Ev> Dimensions<M, O, NoProfileId, Cn, Ev> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<M, O, HasProfileId, Cn, Ev> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: Some(id),
            connector: self.connector,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<M, O, P, Ev> Dimensions<M, O, P, NoConnector, Ev> {
    pub fn with_connector(&self, connector: Connector) -> Dimensions<M, O, P, HasConnector, Ev> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only add incoming_webhook_event if not already present
impl<M, O, P, Cn> Dimensions<M, O, P, Cn, NoWebhookEvent> {
    pub fn with_incoming_webhook_event(
        &self,
        event: IncomingWebhookEvent,
    ) -> Dimensions<M, O, P, Cn, HasWebhookEvent> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            incoming_webhook_event: Some(event),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove merchant_id if currently present
impl<O, P, Cn, Ev> Dimensions<HasMerchantId, O, P, Cn, Ev> {
    pub fn without_merchant_id(&self) -> Dimensions<NoMerchantId, O, P, Cn, Ev> {
        Dimensions {
            merchant_id: None,
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove organization_id if currently present
impl<M, P, Cn, Ev> Dimensions<M, HasOrgId, P, Cn, Ev> {
    pub fn without_organization_id(&self) -> Dimensions<M, NoOrgId, P, Cn, Ev> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: None,
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove profile_id if currently present
impl<M, O, Cn, Ev> Dimensions<M, O, HasProfileId, Cn, Ev> {
    pub fn without_profile_id(&self) -> Dimensions<M, O, NoProfileId, Cn, Ev> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: None,
            connector: self.connector,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove connector if currently present
impl<M, O, P, Ev> Dimensions<M, O, P, HasConnector, Ev> {
    pub fn without_connector(&self) -> Dimensions<M, O, P, NoConnector, Ev> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: None,
            incoming_webhook_event: self.incoming_webhook_event,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove incoming_webhook_event if currently present
impl<M, O, P, Cn> Dimensions<M, O, P, Cn, HasWebhookEvent> {
    pub fn without_incoming_webhook_event(&self) -> Dimensions<M, O, P, Cn, NoWebhookEvent> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            incoming_webhook_event: None,
            _phantom: PhantomData,
        }
    }
}

/// merchant_id getter - only available if HasMerchantId
impl<O, P, Cn, Ev> Dimensions<HasMerchantId, O, P, Cn, Ev> {
    pub fn merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.merchant_id
            .as_ref()
            .ok_or(DimensionError::MissingMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<M, P, Cn, Ev> Dimensions<M, HasOrgId, P, Cn, Ev> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<M, O, Cn, Ev> Dimensions<M, O, HasProfileId, Cn, Ev> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<M, O, P, Ev> Dimensions<M, O, P, HasConnector, Ev> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

// Optional getters (available in any state)
impl<M, O, P, Cn, Ev> Dimensions<M, O, P, Cn, Ev> {
    pub fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.merchant_id.as_ref()
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

    pub fn get_incoming_webhook_event(&self) -> Option<IncomingWebhookEvent> {
        self.incoming_webhook_event
    }
}

// Superposition context conversion
impl<M, O, P, Cn, Ev> Dimensions<M, O, P, Cn, Ev> {
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

        if let Some(conn) = self.connector {
            ctx = ctx.with("connector", conn.to_string().as_str());
        }

        if let Some(event) = self.incoming_webhook_event {
            if let Ok(serde_json::Value::String(s)) = serde_json::to_value(event) {
                ctx = ctx.with("incoming_webhook_events", s.as_str());
            }
        }

        Some(ctx)
    }
}

impl Default for Dimensions<NoMerchantId, NoOrgId, NoProfileId, NoConnector, NoWebhookEvent> {
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

    /// Get connector (if available)
    fn get_connector(&self) -> Option<Connector>;

    /// Get incoming_webhook_event (if available)
    fn get_incoming_webhook_event(&self) -> Option<IncomingWebhookEvent>;
}

impl<M, O, P, Cn, Ev> DimensionsBase for Dimensions<M, O, P, Cn, Ev> {
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

    fn get_connector(&self) -> Option<Connector> {
        self.get_connector()
    }

    fn get_incoming_webhook_event(&self) -> Option<IncomingWebhookEvent> {
        self.get_incoming_webhook_event()
    }
}

pub type DimensionsWithMerchantId =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, NoConnector, NoWebhookEvent>;
pub type DimensionsWithOrgIdAndMerchantId =
    Dimensions<HasMerchantId, HasOrgId, NoProfileId, NoConnector, NoWebhookEvent>;
pub type DimensionsWithOrgIdAndMerchantIdAndProfileId =
    Dimensions<HasMerchantId, HasOrgId, HasProfileId, NoConnector, NoWebhookEvent>;
pub type DimensionsWithMerchantIdAndProfileId =
    Dimensions<HasMerchantId, NoOrgId, HasProfileId, NoConnector, NoWebhookEvent>;
pub type DimensionsWithMerchantIdAndConnector =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, HasConnector, NoWebhookEvent>;
pub type DimensionsWithMerchantIdAndProfileIdAndConnector =
    Dimensions<HasMerchantId, NoOrgId, HasProfileId, HasConnector, NoWebhookEvent>;
pub type DimensionsWithMerchantIdConnectorAndWebhookEvent =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, HasConnector, HasWebhookEvent>;
