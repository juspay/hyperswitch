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
    pub fn get_id(&self) -> &id_type::PaymentId {
        &self.id
    }
}

#[cfg(all(feature = "v2", feature = "payment_v2"))]
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PaymentIntent {
    pub merchant_id: id_type::MerchantId,
    pub status: storage_enums::IntentStatus,
    pub amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub amount_captured: Option<MinorUnit>,
    pub customer_id: Option<id_type::CustomerId>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub statement_descriptor: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced: Option<PrimitiveDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: String,
    pub active_attempt: RemoteStorageObject<PaymentAttempt>,
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,
    pub allowed_payment_method_types: Option<serde_json::Value>,
    pub connector_metadata: Option<pii::SecretSerdeValue>,
    pub feature_metadata: Option<pii::SecretSerdeValue>,
    pub attempt_count: i16,
    pub profile_id: id_type::ProfileId,
    pub payment_link_id: Option<String>,
    // Denotes the action(approve or reject) taken by merchant in case of manual review.
    // Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub frm_merchant_decision: Option<String>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub updated_by: String,
    pub surcharge_applicable: Option<bool>,
    pub request_incremental_authorization: Option<storage_enums::RequestIncrementalAuthorization>,
    pub authorization_count: Option<i32>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub session_expiry: Option<PrimitiveDateTime>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub charges: Option<pii::SecretSerdeValue>,
    pub frm_metadata: Option<pii::SecretSerdeValue>,
    pub customer_details: Option<Encryptable<Secret<serde_json::Value>>>,
    pub merchant_order_reference_id: Option<String>,
    pub is_payment_processor_token_flow: Option<bool>,
    pub shipping_cost: Option<MinorUnit>,
    pub tax_details: Option<TaxDetails>,
    pub merchant_reference_id: String,
    pub billing_address: Option<Encryptable<Secret<serde_json::Value>>>,
    pub shipping_address: Option<Encryptable<Secret<serde_json::Value>>>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub id: id_type::PaymentId,
    pub authentication_type: Option<common_enums::AuthenticationType>,
    pub amount_to_capture: Option<MinorUnit>,
    pub prerouting_algorithm: Option<serde_json::Value>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_on_surcharge: Option<MinorUnit>,
    pub organization_id: id_type::OrganizationId,
    pub skip_external_tax_calculation: Option<bool>,
    pub enable_payment_link: Option<bool>,
}
