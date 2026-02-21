use std::marker::PhantomData;

use common_enums::{PaymentMethod, PaymentMethodType, PayoutRetryType};
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
    #[error("payout_retry_type not available in dimension state")]
    MissingPayoutRetryType,
    #[error("payment_method_type not available in dimension state")]
    MissingPaymentMethodType,
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

/// Marker for state WITHOUT payout_retry_type
pub struct NoPayoutRetryType;

/// Marker for state WITH payout_retry_type
pub struct HasPayoutRetryType;

/// Marker for state WITHOUT payment_method_type
pub struct NoPaymentMethodType;

/// Marker for state WITH payment_method_type
pub struct HasPaymentMethodType;

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
/// * `R` - Payout Retry Type: `HasPayoutRetryType` (present) or `NoPayoutRetryType` (absent)
/// * `T` - Payment Method Type: `HasPaymentMethodType` (present) or `NoPaymentMethodType` (absent)
/// * `PM` - Payment Method: `HasPaymentMethod` (present) or `NoPaymentMethod` (absent)
pub struct Dimensions<M, O, P, R, T, PM> {
    merchant_id: Option<id_type::MerchantId>,
    organization_id: Option<id_type::OrganizationId>,
    profile_id: Option<id_type::ProfileId>,
    payout_retry_type: Option<PayoutRetryType>,
    payment_method_type: Option<PaymentMethodType>,
    payment_method: Option<PaymentMethod>,
    _phantom: PhantomData<(M, O, P, R, T, PM)>,
}

impl
    Dimensions<
        NoMerchantId,
        NoOrgId,
        NoProfileId,
        NoPayoutRetryType,
        NoPaymentMethodType,
        NoPaymentMethod,
    >
{
    pub fn new() -> Self {
        Self {
            merchant_id: None,
            organization_id: None,
            profile_id: None,
            payout_retry_type: None,
            payment_method_type: None,
            payment_method: None,
            _phantom: PhantomData,
        }
    }
}

/// Can only add merchant_id if not already present
impl<O, P, R, T, PM> Dimensions<NoMerchantId, O, P, R, T, PM> {
    pub fn with_merchant_id(
        self,
        id: id_type::MerchantId,
    ) -> Dimensions<HasMerchantId, O, P, R, T, PM> {
        Dimensions {
            merchant_id: Some(id),
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            payout_retry_type: self.payout_retry_type,
            payment_method_type: self.payment_method_type,
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add organization_id if not already present
impl<M, P, R, T, PM> Dimensions<M, NoOrgId, P, R, T, PM> {
    pub fn with_organization_id(
        self,
        id: id_type::OrganizationId,
    ) -> Dimensions<M, HasOrgId, P, R, T, PM> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: Some(id),
            profile_id: self.profile_id,
            payout_retry_type: self.payout_retry_type,
            payment_method_type: self.payment_method_type,
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add profile_id if not already present
impl<M, O, R, T, PM> Dimensions<M, O, NoProfileId, R, T, PM> {
    pub fn with_profile_id(
        self,
        id: id_type::ProfileId,
    ) -> Dimensions<M, O, HasProfileId, R, T, PM> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: self.organization_id,
            profile_id: Some(id),
            payout_retry_type: self.payout_retry_type,
            payment_method_type: self.payment_method_type,
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payout_retry_type if not already present
impl<M, O, P, T, PM> Dimensions<M, O, P, NoPayoutRetryType, T, PM> {
    pub fn with_payout_retry_type(
        self,
        retry_type: PayoutRetryType,
    ) -> Dimensions<M, O, P, HasPayoutRetryType, T, PM> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            payout_retry_type: Some(retry_type),
            payment_method_type: self.payment_method_type,
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payment_method_type if not already present
impl<M, O, P, R, PM> Dimensions<M, O, P, R, NoPaymentMethodType, PM> {
    pub fn with_payment_method_type(
        self,
        pmt: PaymentMethodType,
    ) -> Dimensions<M, O, P, R, HasPaymentMethodType, PM> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            payout_retry_type: self.payout_retry_type,
            payment_method_type: Some(pmt),
            payment_method: self.payment_method,
            _phantom: PhantomData,
        }
    }
}

/// Can only add payment_method if not already present
impl<M, O, P, R, T> Dimensions<M, O, P, R, T, NoPaymentMethod> {
    pub fn with_payment_method(
        self,
        pm: PaymentMethod,
    ) -> Dimensions<M, O, P, R, T, HasPaymentMethod> {
        Dimensions {
            merchant_id: self.merchant_id,
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            payout_retry_type: self.payout_retry_type,
            payment_method_type: self.payment_method_type,
            payment_method: Some(pm),
            _phantom: PhantomData,
        }
    }
}

/// merchant_id getter - only available if HasMerchantId
impl<O, P, R, T, PM> Dimensions<HasMerchantId, O, P, R, T, PM> {
    pub fn merchant_id(&self) -> Result<&id_type::MerchantId, DimensionError> {
        self.merchant_id
            .as_ref()
            .ok_or(DimensionError::MissingMerchantId)
    }
}

/// organization_id getter - only available if HasOrgId
impl<M, P, R, T, PM> Dimensions<M, HasOrgId, P, R, T, PM> {
    pub fn organization_id(&self) -> Result<&id_type::OrganizationId, DimensionError> {
        self.organization_id
            .as_ref()
            .ok_or(DimensionError::MissingOrganizationId)
    }
}

/// profile_id getter - only available if HasProfileId
impl<M, O, R, T, PM> Dimensions<M, O, HasProfileId, R, T, PM> {
    pub fn profile_id(&self) -> Result<&id_type::ProfileId, DimensionError> {
        self.profile_id
            .as_ref()
            .ok_or(DimensionError::MissingProfileId)
    }
}

/// payout_retry_type getter - only available if HasPayoutRetryType
impl<M, O, P, T, PM> Dimensions<M, O, P, HasPayoutRetryType, T, PM> {
    pub fn payout_retry_type(&self) -> Result<&PayoutRetryType, DimensionError> {
        self.payout_retry_type
            .as_ref()
            .ok_or(DimensionError::MissingPayoutRetryType)
    }
}

/// payment_method_type getter - only available if HasPaymentMethodType
impl<M, O, P, R, PM> Dimensions<M, O, P, R, HasPaymentMethodType, PM> {
    pub fn payment_method_type(&self) -> Result<&PaymentMethodType, DimensionError> {
        self.payment_method_type
            .as_ref()
            .ok_or(DimensionError::MissingPaymentMethodType)
    }
}

/// payment_method getter - only available if HasPaymentMethod
impl<M, O, P, R, T> Dimensions<M, O, P, R, T, HasPaymentMethod> {
    pub fn payment_method(&self) -> Result<&PaymentMethod, DimensionError> {
        self.payment_method
            .as_ref()
            .ok_or(DimensionError::MissingPaymentMethod)
    }
}

// Optional getters (available in any state)
impl<M, O, P, R, T, PM> Dimensions<M, O, P, R, T, PM> {
    pub fn get_merchant_id(&self) -> Option<&id_type::MerchantId> {
        self.merchant_id.as_ref()
    }

    pub fn get_organization_id(&self) -> Option<&id_type::OrganizationId> {
        self.organization_id.as_ref()
    }

    pub fn get_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.profile_id.as_ref()
    }

    pub fn get_payout_retry_type(&self) -> Option<&PayoutRetryType> {
        self.payout_retry_type.as_ref()
    }

    pub fn get_payment_method_type(&self) -> Option<&PaymentMethodType> {
        self.payment_method_type.as_ref()
    }

    pub fn get_payment_method(&self) -> Option<&PaymentMethod> {
        self.payment_method.as_ref()
    }
}

// Superposition context conversion
impl<M, O, P, R, T, PM> Dimensions<M, O, P, R, T, PM> {
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

        if let Some(ref rt) = self.payout_retry_type {
            let rt_str = match rt {
                PayoutRetryType::SingleConnector => "single_connector",
                PayoutRetryType::MultiConnector => "multi_connector",
            };
            ctx = ctx.with("payout_retry_type", rt_str);
        }

        if let Some(ref pmt) = self.payment_method_type {
            ctx = ctx.with("payment_method_type", &pmt.to_string());
        }

        if let Some(ref pm) = self.payment_method {
            ctx = ctx.with("payment_method", &pm.to_string());
        }

        Some(ctx)
    }
}

impl Default
    for Dimensions<
        NoMerchantId,
        NoOrgId,
        NoProfileId,
        NoPayoutRetryType,
        NoPaymentMethodType,
        NoPaymentMethod,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

pub type DimensionsWithMerchantId = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    NoPayoutRetryType,
    NoPaymentMethodType,
    NoPaymentMethod,
>;
pub type DimensionsWithOrgId = Dimensions<
    NoMerchantId,
    HasOrgId,
    NoProfileId,
    NoPayoutRetryType,
    NoPaymentMethodType,
    NoPaymentMethod,
>;
pub type DimensionsWithProfileId = Dimensions<
    NoMerchantId,
    NoOrgId,
    HasProfileId,
    NoPayoutRetryType,
    NoPaymentMethodType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantAndOrgId = Dimensions<
    HasMerchantId,
    HasOrgId,
    NoProfileId,
    NoPayoutRetryType,
    NoPaymentMethodType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantAndPayoutRetryType = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    HasPayoutRetryType,
    NoPaymentMethodType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantAndPaymentMethodType = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    NoPayoutRetryType,
    HasPaymentMethodType,
    NoPaymentMethod,
>;
pub type DimensionsWithMerchantPaymentMethodAndPaymentMethodType = Dimensions<
    HasMerchantId,
    NoOrgId,
    NoProfileId,
    NoPayoutRetryType,
    HasPaymentMethodType,
    HasPaymentMethod,
>;
