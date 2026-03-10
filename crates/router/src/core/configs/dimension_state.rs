use std::marker::PhantomData;

use common_enums::{connector_enums::Connector, PaymentMethod, PaymentMethodType, PayoutRetryType};
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
    #[error("payout_retry_type not available in dimension state")]
    MissingPayoutRetryType,
    #[error("payment_method not available in dimension state")]
    MissingPaymentMethod,
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

/// Marker for state WITHOUT payout_retry_type
pub struct NoPayoutRetryType;

/// Marker for state WITH payout_retry_type
pub struct HasPayoutRetryType;

/// Marker for state WITHOUT payment_method
pub struct NoPaymentMethod;

/// Marker for state WITH payment_method
pub struct HasPaymentMethod;

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
/// * `Pr` - Payout retry type: `HasPayoutRetryType` (present) or `NoPayoutRetryType` (absent)
/// * `Pm` - Payment method type: `HasPaymentMethod` (present) or `NoPaymentMethod` (absent)
pub struct Dimensions<M, O, P, Cn, Pmt, Pr, Pm> {
    merchant_id: Option<id_type::MerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    connector: Option<Connector>,
    payment_method_type: Option<PaymentMethodType>,
    payout_retry_type: Option<PayoutRetryType>,
    payment_method: Option<PaymentMethod>,
    _phantom: PhantomData<(M, O, P, Cn, Pmt, Pr, Pm)>,
}

impl
    Dimensions<
        NoMerchantId,
        NoOrgId,
        NoProfileId,
        NoConnector,
        NoPaymentMethodType,
        NoPayoutRetryType,
        NoPaymentMethod,
    >
{
    pub fn new() -> Self {
        Self {
            merchant_id: None,
            organization_id: None,
            profile_id: None,
            connector: None,
            payment_method_type: None,
            payout_retry_type: None,
            payment_method: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add merchant_id if not already present
impl<O, P, Cn, Pmt, Pr, Pm> Dimensions<NoMerchantId, O, P, Cn, Pmt, Pr, Pm> {
    pub fn with_merchant_id(
        &self,
        id: id_type::MerchantId,
    ) -> Dimensions<HasMerchantId, O, P, Cn, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: Some(id),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<M, P, Cn, Pmt, Pr, Pm> Dimensions<M, NoOrgId, P, Cn, Pmt, Pr, Pm> {
    pub fn with_organization_id(
        &self,
        id: id_type::OrganizationId,
    ) -> Dimensions<M, HasOrgId, P, Cn, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: Some(id),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<M, O, Cn, Pmt, Pr, Pm> Dimensions<M, O, NoProfileId, Cn, Pmt, Pr, Pm> {
    pub fn with_profile_id(
        &self,
        id: id_type::ProfileId,
    ) -> Dimensions<M, O, HasProfileId, Cn, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: Some(id),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add connector if not already present
impl<M, O, P, Pmt, Pr, Pm> Dimensions<M, O, P, NoConnector, Pmt, Pr, Pm> {
    pub fn with_connector(
        &self,
        connector: Connector,
    ) -> Dimensions<M, O, P, HasConnector, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: Some(connector),
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payment_method_type if not already present
impl<M, O, P, Cn, Pr, Pm> Dimensions<M, O, P, Cn, NoPaymentMethodType, Pr, Pm> {
    pub fn with_payment_method_type(
        &self,
        pmt: PaymentMethodType,
    ) -> Dimensions<M, O, P, Cn, HasPaymentMethodType, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: Some(pmt),
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payout_retry_type if not already present
impl<M, O, P, Cn, Pmt, Pm> Dimensions<M, O, P, Cn, Pmt, NoPayoutRetryType, Pm> {
    pub fn with_payout_retry_type(
        &self,
        prt: PayoutRetryType,
    ) -> Dimensions<M, O, P, Cn, Pmt, HasPayoutRetryType, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: Some(prt),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payment_method if not already present
impl<M, O, P, Cn, Pmt, Pr> Dimensions<M, O, P, Cn, Pmt, Pr, NoPaymentMethod> {
    pub fn with_payment_method(
        &self,
        pm: PaymentMethod,
    ) -> Dimensions<M, O, P, Cn, Pmt, Pr, HasPaymentMethod> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: Some(pm),
            _phantom: PhantomData,
        }
    }
}

/// Can only remove merchant_id if currently present
impl<O, P, Cn, Pmt, Pr, Pm> Dimensions<HasMerchantId, O, P, Cn, Pmt, Pr, Pm> {
    pub fn without_merchant_id(&self) -> Dimensions<NoMerchantId, O, P, Cn, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: None,
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove organization_id if currently present
impl<M, P, Cn, Pmt, Pr, Pm> Dimensions<M, HasOrgId, P, Cn, Pmt, Pr, Pm> {
    pub fn without_organization_id(&self) -> Dimensions<M, NoOrgId, P, Cn, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: None,
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove profile_id if currently present
impl<M, O, Cn, Pmt, Pr, Pm> Dimensions<M, O, HasProfileId, Cn, Pmt, Pr, Pm> {
    pub fn without_profile_id(&self) -> Dimensions<M, O, NoProfileId, Cn, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: None,
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove connector if currently present
impl<M, O, P, Pmt, Pr, Pm> Dimensions<M, O, P, HasConnector, Pmt, Pr, Pm> {
    pub fn without_connector(&self) -> Dimensions<M, O, P, NoConnector, Pmt, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: None,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove payment_method_type if currently present
impl<M, O, P, Cn, Pr, Pm> Dimensions<M, O, P, Cn, HasPaymentMethodType, Pr, Pm> {
    pub fn without_payment_method_type(
        &self,
    ) -> Dimensions<M, O, P, Cn, NoPaymentMethodType, Pr, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: None,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove payout_retry_type if currently present
impl<M, O, P, Cn, Pmt, Pm> Dimensions<M, O, P, Cn, Pmt, HasPayoutRetryType, Pm> {
    pub fn without_payout_retry_type(
        &self,
    ) -> Dimensions<M, O, P, Cn, Pmt, NoPayoutRetryType, Pm> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: None,
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only remove payment_method if currently present
impl<M, O, P, Cn, Pmt, Pr> Dimensions<M, O, P, Cn, Pmt, Pr, HasPaymentMethod> {
    pub fn without_payment_method(&self) -> Dimensions<M, O, P, Cn, Pmt, Pr, NoPaymentMethod> {
        Dimensions {
            merchant_id: self.merchant_id.clone(),
            organization_id: self.organization_id.clone(),
            profile_id: self.profile_id.clone(),
            connector: self.connector,
            payment_method_type: self.payment_method_type,
            payout_retry_type: self.payout_retry_type.clone(),
            payment_method: None,
            _phantom: PhantomData,
        }
    }
}

/// merchant_id getter - only available if HasMerchantId
impl<O, P, Cn, Pmt, Pr, Pm> Dimensions<HasMerchantId, O, P, Cn, Pmt, Pr, Pm> {
    pub fn merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.merchant_id
            .as_ref()
            .ok_or(DimensionError::MissingMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<M, P, Cn, Pmt, Pr, Pm> Dimensions<M, HasOrgId, P, Cn, Pmt, Pr, Pm> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<M, O, Cn, Pmt, Pr, Pm> Dimensions<M, O, HasProfileId, Cn, Pmt, Pr, Pm> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// connector getter - only available if HasConnector
impl<M, O, P, Pmt, Pr, Pm> Dimensions<M, O, P, HasConnector, Pmt, Pr, Pm> {
    pub fn connector(&self) -> Result<Connector, DimensionError> {
        self.connector.ok_or(DimensionError::MissingConnector)
    }
}

/// payment_method_type getter - only available if HasPaymentMethodType
impl<M, O, P, Cn, Pr, Pm> Dimensions<M, O, P, Cn, HasPaymentMethodType, Pr, Pm> {
    pub fn payment_method_type(&self) -> Result<PaymentMethodType, DimensionError> {
        self.payment_method_type
            .ok_or(DimensionError::MissingPaymentMethodType)
    }
}

/// payout_retry_type getter - only available if HasPayoutRetryType
impl<M, O, P, Cn, Pmt, Pm> Dimensions<M, O, P, Cn, Pmt, HasPayoutRetryType, Pm> {
    pub fn payout_retry_type(&self) -> Result<PayoutRetryType, DimensionError> {
        self.payout_retry_type
            .clone()
            .ok_or(DimensionError::MissingPayoutRetryType)
    }
}

/// payment_method getter - only available if HasPaymentMethod
impl<M, O, P, Cn, Pmt, Pr> Dimensions<M, O, P, Cn, Pmt, Pr, HasPaymentMethod> {
    pub fn payment_method(&self) -> Result<PaymentMethod, DimensionError> {
        self.payment_method
            .ok_or(DimensionError::MissingPaymentMethod)
    }
}

// Optional getters (available in any state)
impl<M, O, P, Cn, Pmt, Pr, Pm> Dimensions<M, O, P, Cn, Pmt, Pr, Pm> {
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

    pub fn get_payout_retry_type(&self) -> Option<&PayoutRetryType> {
        self.payout_retry_type.as_ref()
    }

    pub fn get_payment_method(&self) -> Option<PaymentMethod> {
        self.payment_method
    }
}

// Superposition context conversion
impl<M, O, P, Cn, Pmt, Pr, Pm> Dimensions<M, O, P, Cn, Pmt, Pr, Pm> {
    /// Converts dimension state to Superposition config context
    pub fn to_superposition_context(&self) -> Option<superposition::ConfigContext> {
        let mut ctx = superposition::ConfigContext::new();

        if let Some(ref mid) = self.merchant_id {
            ctx = ctx.with("merchant_id", mid.get_string_repr());
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
        if let Some(pmt) = self.payment_method_type {
            ctx = ctx.with("payment_method_type", pmt.to_string().as_str());
        }
        if let Some(ref prt) = self.payout_retry_type {
            ctx = ctx.with("payout_retry_type", prt.to_string().as_str());
        }
        if let Some(pm) = self.payment_method {
            ctx = ctx.with("payment_method", pm.to_string().as_str());
        }

        Some(ctx)
    }
}

impl Default
    for Dimensions<
        NoMerchantId,
        NoOrgId,
        NoProfileId,
        NoConnector,
        NoPaymentMethodType,
        NoPayoutRetryType,
        NoPaymentMethod,
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

    /// Get payout_retry_type (if available)
    fn get_payout_retry_type(&self) -> Option<&PayoutRetryType>;

    /// Get payment_method (if available)
    fn get_payment_method(&self) -> Option<PaymentMethod>;
}

impl<M, O, P, Cn, Pmt, Pr, Pm> DimensionsBase for Dimensions<M, O, P, Cn, Pmt, Pr, Pm> {
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

    fn get_payout_retry_type(&self) -> Option<&PayoutRetryType> {
        self.get_payout_retry_type()
    }

    fn get_payment_method(&self) -> Option<PaymentMethod> {
        self.get_payment_method()
    }
}

pub type DimensionsWithMerchantId = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithOrgIdAndMerchantId = Dimensions<
    HasMerchantId,
    HasOrgId,
    NoProfileId,
    NoConnector,
    NoPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithOrgIdAndMerchantIdAndProfileId = Dimensions<
    HasMerchantId,
    HasOrgId,
    HasProfileId,
    NoConnector,
    NoPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndProfileId = Dimensions<
    HasMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    NoPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndConnector = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    HasConnector,
    NoPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndProfileIdAndConnector = Dimensions<
    HasMerchantId,
    NoOrgId,
    HasProfileId,
    HasConnector,
    NoPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndPaymentMethodType = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    HasPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndProfileIdAndPaymentMethodType = Dimensions<
    HasMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    HasPaymentMethodType,
    NoPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndPayoutRetryType = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    NoConnector,
    NoPaymentMethodType,
    HasPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndProfileIdAndPayoutRetryType = Dimensions<
    HasMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    NoPaymentMethodType,
    HasPayoutRetryType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantIdAndProfileIdAndPaymentMethodAndPaymentMethodType = Dimensions<
    HasMerchantId,
    NoOrgId,
    HasProfileId,
    NoConnector,
    HasPaymentMethodType,
    NoPayoutRetryType,
    HasPaymentMethod,
>;
