use common_utils::{self, crypto::Encryptable, id_type, pii, types::MinorUnit};
use diesel_models::payment_intent::TaxDetails;
use masking::Secret;
use time::PrimitiveDateTime;

pub mod payment_attempt;
pub mod payment_intent;

use common_enums as storage_enums;

use self::payment_attempt::PaymentAttempt;
use crate::RemoteStorageObject;

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "payment_v2")))]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaymentIntent {
    pub payment_id: id_type::PaymentId,
    pub merchant_id: id_type::MerchantId,
    pub status: storage_enums::IntentStatus,
    pub amount: MinorUnit,
    pub shipping_cost: Option<MinorUnit>,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<MinorUnit>,
    pub customer_id: Option<id_type::CustomerId>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub connector_id: Option<String>,
    pub shipping_address_id: Option<String>,
    pub billing_address_id: Option<String>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub active_attempt: RemoteStorageObject<PaymentAttempt>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<serde_json::Value>,
    pub feature_metadata: Option<serde_json::Value>,
    pub attempt_count: i16,
    pub profile_id: Option<id_type::ProfileId>,
    pub payment_link_id: Option<String>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,

    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub request_incremental_authorization: Option<storage_enums::RequestIncrementalAuthorization>,
    pub incremental_authorization_allowed: Option<bool>,
    pub authorization_count: Option<i32>,
    pub fingerprint_id: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub charges: Option<pii::SecretSerdeValue>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub billing_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub shipping_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub is_payment_processor_token_flow: Option<bool>,
    pub organization_id: id_type::OrganizationId,
    pub tax_details: Option<TaxDetails>,
    pub skip_external_tax_calculation: Option<bool>,
}

impl PaymentIntent {
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "payment_v2"),))]
    pub fn get_id(&self) -> &id_type::PaymentId {
        &self.payment_id
    }

    #[cfg(all(feature = "v2", feature = "payment_v2",))]
    pub fn get_id(&self) -> &id_type::PaymentGlobalId {
        &self.id
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum TaxCalculationOverride {
    /// Skip calling the external tax provider
    Skip,
    /// Calculate tax by calling the external tax provider
    Calculate,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum SurchargeCalculationOverride {
    /// Skip calculating surcharge
    Skip,
    /// Calculate surcharge
    Calculate,
}

#[cfg(feature = "v2")]
impl From<Option<bool>> for TaxCalculationOverride {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => TaxCalculationOverride::Calculate,
            _ => TaxCalculationOverride::Skip,
        }
    }
}

#[cfg(feature = "v2")]
impl From<Option<bool>> for SurchargeCalculationOverride {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => SurchargeCalculationOverride::Calculate,
            _ => SurchargeCalculationOverride::Skip,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AmountDetails {
    /// The amount of the order in the lowest denomination of currency
    order_amount: MinorUnit,
    /// The currency of the order
    currency: common_enums::Currency,
    /// The shipping cost of the order. This has to be collected from the merchant
    shipping_cost: Option<MinorUnit>,
    /// Tax details related to the order. This will be calculated by the external tax provider
    tax_details: Option<TaxDetails>,
    /// The action to whether calculate tax by calling external tax provider or not
    skip_external_tax_calculation: TaxCalculationOverride,
    /// The action to whether calculate surcharge or not
    skip_surcharge_calculation: SurchargeCalculationOverride,
    /// The surcharge amount to be added to the order, collected from the merchant
    surcharge_amount: Option<MinorUnit>,
    /// tax on surcharge amount
    tax_on_surcharge: Option<MinorUnit>,
}

#[cfg(feature = "v2")]
impl AmountDetails {
    /// Get the action to whether calculate surcharge or not as a boolean value
    fn get_surcharge_action_as_bool(&self) -> bool {
        match self.skip_surcharge_calculation {
            SurchargeCalculationOverride::Skip => false,
            SurchargeCalculationOverride::Calculate => true,
        }
    }

    /// Get the action to whether calculate external tax or not as a boolean value
    fn get_external_tax_action_as_bool(&self) -> bool {
        match self.skip_external_tax_calculation {
            TaxCalculationOverride::Skip => false,
            TaxCalculationOverride::Calculate => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum External3dsAuthenticationRequest {
    /// Request for 3ds authentication
    Enable,
    /// Skip 3ds authentication
    Disable,
}

impl From<Option<bool>> for External3dsAuthenticationRequest {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => External3dsAuthenticationRequest::Enable,
            _ => External3dsAuthenticationRequest::Disable,
        }
    }
}

impl External3dsAuthenticationRequest {
    pub fn as_bool(&self) -> bool {
        match self {
            External3dsAuthenticationRequest::Enable => true,
            External3dsAuthenticationRequest::Disable => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum EnablePaymentLinkRequest {
    /// Request for enabling payment link
    Enable,
    /// Skip enabling payment link
    Disable,
}

impl EnablePaymentLinkRequest {
    pub fn as_bool(&self) -> bool {
        match self {
            EnablePaymentLinkRequest::Enable => true,
            EnablePaymentLinkRequest::Disable => false,
        }
    }
}

impl From<Option<bool>> for EnablePaymentLinkRequest {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => EnablePaymentLinkRequest::Enable,
            _ => EnablePaymentLinkRequest::Disable,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum MitExemptionRequest {
    /// Request for applying MIT exemption
    Apply,
    /// Skip applying MIT exemption
    DoNotApply,
}

impl From<Option<bool>> for MitExemptionRequest {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => MitExemptionRequest::Apply,
            _ => MitExemptionRequest::DoNotApply,
        }
    }
}

impl MitExemptionRequest {
    pub fn as_bool(&self) -> bool {
        match self {
            MitExemptionRequest::Apply => true,
            MitExemptionRequest::DoNotApply => false,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_v2"))]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaymentIntent {
    /// The global identifier for the payment intent. This is generated by the system.
    /// The format of the global id is `{cell_id:5}_pay_{time_ordered_uuid:32}`.
    pub id: id_type::PaymentGlobalId,
    /// The identifier for the merchant. This is automatically derived from the api key used to create the payment.
    pub merchant_id: id_type::MerchantId,
    /// The status of payment intent.
    pub status: storage_enums::IntentStatus,
    /// The amount related details of the payment
    pub amount_details: AmountDetails,
    /// The total amount captured for the order. This is the sum of all the captured amounts for the order.
    pub amount_captured: Option<MinorUnit>,
    /// The identifier for the customer. This is the identifier for the customer in the merchant's system.
    pub customer_id: Option<id_type::CustomerId>,
    /// The description of the order. This will be passed to connectors which support description.
    pub description: Option<String>,
    /// The return url for the payment. This is the url to which the user will be redirected after the payment is completed.
    pub return_url: Option<String>,
    /// The metadata for the payment intent. This is the metadata that will be passed to the connectors.
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The statement descriptor for the order, this will be displayed in the user's bank statement.
    pub statement_descriptor: Option<String>,
    /// The time at which the order was created
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    /// The time at which the order was last modified
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    /// The client secret that is generated for the payment. This is used to authenticate the payment from client facing apis.
    pub client_secret: String,
    /// The active attempt for the payment intent. This is the payment attempt that is currently active for the payment intent.
    pub active_attempt: RemoteStorageObject<PaymentAttempt>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    /// This is the list of payment method types that are allowed for the payment intent.
    /// This field allows the merchant to restrict the payment methods that can be used for the payment intent.
    pub allowed_payment_method_types: Option<serde_json::Value>,
    /// This metadata contains details about
    pub connector_metadata: Option<pii::SecretSerdeValue>,
    pub feature_metadata: Option<pii::SecretSerdeValue>,
    /// Number of attempts that have been made for the order
    pub attempt_count: i16,
    /// The profile id for the payment.
    pub profile_id: id_type::ProfileId,
    /// The payment link id for the payment. This is generated only if `enable_payment_link` is set to true.
    pub payment_link_id: Option<String>,
    /// This Denotes the action(approve or reject) taken by merchant in case of manual review.
    /// Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub frm_merchant_decision: Option<String>,
    /// Denotes the last instance which updated the payment
    pub updated_by: String,
    /// Denotes whether merchant requested for incremental authorization to be enabled for this payment.
    pub request_incremental_authorization: Option<storage_enums::RequestIncrementalAuthorization>,
    /// Denotes the number of authorizations that have been made for the payment.
    pub authorization_count: Option<i32>,
    /// Denotes the client secret expiry for the payment. This is the time at which the client secret will expire.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub session_expiry: Option<PrimitiveDateTime>,
    /// Denotes whether merchant requested for 3ds authentication to be enabled for this payment.
    pub request_external_three_ds_authentication: External3dsAuthenticationRequest,
    /// Metadata related to fraud and risk management
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    /// The details of the customer in a denormalized form. Only a subset of fields are stored.
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    /// The reference id for the order in the merchant's system. This value can be passed by the merchant.
    pub merchant_reference_id: String,
    /// The billing address for the order in a denormalized form.
    pub billing_address: Option<Encryptable<Secret<serde_json::Value>>>,
    /// The shipping address for the order in a denormalized form.
    pub shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
    /// Capture method for the payment
    pub capture_method: Option<storage_enums::CaptureMethod>,
    /// Authentication type that is requsted by the merchant for this payment.
    pub authentication_type: Option<common_enums::AuthenticationType>,
    /// This contains the pre routing results that are done when routing is done during listing the payment methods.
    pub prerouting_algorithm: Option<serde_json::Value>,
    /// The organization id for the payment. This is derived from the merchant account
    pub organization_id: id_type::OrganizationId,
    /// Denotes the request by the merchat whether to enable a payment link for this payment.
    pub enable_payment_link: EnablePaymentLinkRequest,
    /// Denotes the request by the merchant whether to apply MIT exemption for this payment
    pub apply_mit_exemption: MitExemptionRequest,
}
