use std::marker::PhantomData;

use common_enums::{connector_enums::Connector, PaymentMethodType};
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
    #[error("payment_method_type not available in dimension state")]
    MissingPaymentMethodType,
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

/// Marker for state WITHOUT payment_method_type
pub struct NoPaymentMethodType;

/// Marker for state WITH payment_method_type
pub struct HasPaymentMethodType;

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
/// * `Pmt` - Payment method type: `HasPaymentMethodType` (present) or `NoPaymentMethodType` (absent)
pub struct Dimensions<M, O, P, Cn, Pmt> {
    merchant_id: Option<id_type::MerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    payment_method_type: Option<PaymentMethodType>,
    _phantom: PhantomData<(M, O, P, Cn, Pmt)>,
}

impl Dimensions<NoMerchantId, NoOrgId, NoProfileId, NoConnector, NoPaymentMethodType> {
    pub fn new() -> Self {
        Self {
            merchant_id: None,
            organization_id: None,
            profile_id: None,
            connector: None,
            payment_method_type: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add merchant_id if not already present
impl<O, P, Cn, Pmt> Dimensions<NoMerchantId, O, P, Cn, Pmt> {
    pub fn with_merchant_id(
        &self,
        id: id_type::MerchantId,
    ) -> Dimensions<HasMerchantId, O, P, Cn, Pmt> {
        Dimensions {
            merchant_id: Some(id),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<M, P, Cn, Pmt> Dimensions<M, NoOrgId, P, Cn, Pmt> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<M, HasOrgId, P, Cn, Pmt> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: Some(id),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<M, O, Cn, Pmt> Dimensions<M, O, NoProfileId, Cn, Pmt> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<M, O, HasProfileId, Cn, Pmt> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: Some(id),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<M, O, P, Pmt> Dimensions<M, O, P, NoConnector, Pmt> {
    pub fn with_connector(
        &self,
        connector: Connector,
    ) -> Dimensions<M, O, P, HasConnector, Pmt> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payment_method_type if not already present
impl<M, O, P, Cn> Dimensions<M, O, P, Cn, NoPaymentMethodType> {
    pub fn with_payment_method_type(
        &self,
        pmt: PaymentMethodType,
    ) -> Dimensions<M, O, P, Cn, HasPaymentMethodType> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: Some(pmt),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove merchant_id if currently present
impl<O, P, Cn, Pmt> Dimensions<HasMerchantId, O, P, Cn, Pmt> {
    pub fn without_merchant_id(&self) -> Dimensions<NoMerchantId, O, P, Cn, Pmt> {
        Dimensions {
            merchant_id: None,
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove organization_id if currently present
impl<M, P, Cn, Pmt> Dimensions<M, HasOrgId, P, Cn, Pmt> {
    pub fn without_organization_id(&self) -> Dimensions<M, NoOrgId, P, Cn, Pmt> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: None,
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove profile_id if currently present
impl<M, O, Cn, Pmt> Dimensions<M, O, HasProfileId, Cn, Pmt> {
    pub fn without_profile_id(&self) -> Dimensions<M, O, NoProfileId, Cn, Pmt> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: None,
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove connector if currently present
impl<M, O, P, Pmt> Dimensions<M, O, P, HasConnector, Pmt> {
    pub fn without_connector(&self) -> Dimensions<M, O, P, NoConnector, Pmt> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: None,
            payment_method_type: self.payment_method_type,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove payment_method_type if currently present
impl<M, O, P, Cn> Dimensions<M, O, P, Cn, HasPaymentMethodType> {
    pub fn without_payment_method_type(&self) -> Dimensions<M, O, P, Cn, NoPaymentMethodType> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: None,
            _phantom: PhantomData,
        }
    }
}

/// merchant_id getter - only available if HasMerchantId
impl<O, P, Cn, Pmt> Dimensions<HasMerchantId, O, P, Cn, Pmt> {
    pub fn merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.merchant_id
            .as_ref()
            .ok_or(DimensionError::MissingMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<M, P, Cn, Pmt> Dimensions<M, HasOrgId, P, Cn, Pmt> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<M, O, Cn, Pmt> Dimensions<M, O, HasProfileId, Cn, Pmt> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<M, O, P, Pmt> Dimensions<M, O, P, HasConnector, Pmt> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

/// payment_method_type getter - only available if HasPaymentMethodType
impl<M, O, P, Cn> Dimensions<M, O, P, Cn, HasPaymentMethodType> {
    pub fn payment_method_type(&self) -> Result<PaymentMethodType, DimensionError> {
        self.payment_method_type
            .ok_or(DimensionError::MissingPaymentMethodType)
    }
}

// Optional getters (available in any state)
impl<M, O, P, Cn, Pmt> Dimensions<M, O, P, Cn, Pmt> {
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

    pub fn get_payment_method_type(&self) -> Option<PaymentMethodType> {
        self.payment_method_type
    }
}

// Superposition context conversion
impl<M, O, P, Cn, Pmt> Dimensions<M, O, P, Cn, Pmt> {
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

        if let Some(pmt) = self.payment_method_type {
            ctx = ctx.with("payment_method_type", pmt.to_string().as_str());
        }

        Some(ctx)
    }
}

impl Default
    for Dimensions<NoMerchantId, NoOrgId, NoProfileId, NoConnector, NoPaymentMethodType>
{
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

    /// Get payment_method_type (if available)
    fn get_payment_method_type(&self) -> Option<PaymentMethodType>;
}

impl<M, O, P, Cn, Pmt> DimensionsBase for Dimensions<M, O, P, Cn, Pmt> {
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

    fn get_payment_method_type(&self) -> Option<PaymentMethodType> {
        self.get_payment_method_type()
    }
}

pub type DimensionsWithMerchantId =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, NoConnector, NoPaymentMethodType>;
pub type DimensionsWithOrgIdAndMerchantId =
    Dimensions<HasMerchantId, HasOrgId, NoProfileId, NoConnector, NoPaymentMethodType>;
pub type DimensionsWithOrgIdAndMerchantIdAndProfileId =
    Dimensions<HasMerchantId, HasOrgId, HasProfileId, NoConnector, NoPaymentMethodType>;
pub type DimensionsWithMerchantIdAndProfileId =
    Dimensions<HasMerchantId, NoOrgId, HasProfileId, NoConnector, NoPaymentMethodType>;
pub type DimensionsWithMerchantIdAndConnector =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, HasConnector, NoPaymentMethodType>;
pub type DimensionsWithMerchantIdAndProfileIdAndConnector =
    Dimensions<HasMerchantId, NoOrgId, HasProfileId, HasConnector, NoPaymentMethodType>;
pub type DimensionsWithMerchantIdAndPaymentMethodType =
    Dimensions<HasMerchantId, NoOrgId, NoProfileId, NoConnector, HasPaymentMethodType>;
pub type DimensionsWithMerchantIdAndProfileIdAndPaymentMethodType =
    Dimensions<HasMerchantId, NoOrgId, HasProfileId, NoConnector, HasPaymentMethodType>;
