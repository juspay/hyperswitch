// use diesel_models::enums::MandateDetails;
use data_models::{mandates::MandateDetails, payments::payment_attempt::PaymentAttempt};
use diesel_models::enums as storage_enums;
use time::OffsetDateTime;

#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentAttempt<'a> {
    pub payment_id: &'a String,
    pub merchant_id: &'a String,
    pub attempt_id: &'a String,
    pub status: storage_enums::AttemptStatus,
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<&'a String>,
    pub error_message: Option<&'a String>,
    pub offer_amount: Option<i64>,
    pub surcharge_amount: Option<i64>,
    pub tax_amount: Option<i64>,
    pub payment_method_id: Option<&'a String>,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub connector_transaction_id: Option<&'a String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub capture_on: Option<OffsetDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub modified_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub last_synced: Option<OffsetDateTime>,
    pub cancellation_reason: Option<&'a String>,
    pub amount_to_capture: Option<i64>,
    pub mandate_id: Option<&'a String>,
    pub browser_info: Option<String>,
    pub error_code: Option<&'a String>,
    pub connector_metadata: Option<String>,
    // TODO: These types should implement copy ideally
    pub payment_experience: Option<&'a storage_enums::PaymentExperience>,
    pub payment_method_type: Option<&'a storage_enums::PaymentMethodType>,
    pub payment_method_data: Option<String>,
    pub error_reason: Option<&'a String>,
    pub multiple_capture_count: Option<i16>,
    pub amount_capturable: i64,
    pub merchant_connector_id: Option<&'a String>,
    pub net_amount: i64,
    pub unified_code: Option<&'a String>,
    pub unified_message: Option<&'a String>,
    pub mandate_data: Option<&'a MandateDetails>,
}

impl<'a> KafkaPaymentAttempt<'a> {
    pub fn from_storage(attempt: &'a PaymentAttempt) -> Self {
        Self {
            payment_id: &attempt.payment_id,
            merchant_id: &attempt.merchant_id,
            attempt_id: &attempt.attempt_id,
            status: attempt.status,
            amount: attempt.amount,
            currency: attempt.currency,
            save_to_locker: attempt.save_to_locker,
            connector: attempt.connector.as_ref(),
            error_message: attempt.error_message.as_ref(),
            offer_amount: attempt.offer_amount,
            surcharge_amount: attempt.surcharge_amount,
            tax_amount: attempt.tax_amount,
            payment_method_id: attempt.payment_method_id.as_ref(),
            payment_method: attempt.payment_method,
            connector_transaction_id: attempt.connector_transaction_id.as_ref(),
            capture_method: attempt.capture_method,
            capture_on: attempt.capture_on.map(|i| i.assume_utc()),
            confirm: attempt.confirm,
            authentication_type: attempt.authentication_type,
            created_at: attempt.created_at.assume_utc(),
            modified_at: attempt.modified_at.assume_utc(),
            last_synced: attempt.last_synced.map(|i| i.assume_utc()),
            cancellation_reason: attempt.cancellation_reason.as_ref(),
            amount_to_capture: attempt.amount_to_capture,
            mandate_id: attempt.mandate_id.as_ref(),
            browser_info: attempt.browser_info.as_ref().map(|v| v.to_string()),
            error_code: attempt.error_code.as_ref(),
            connector_metadata: attempt.connector_metadata.as_ref().map(|v| v.to_string()),
            payment_experience: attempt.payment_experience.as_ref(),
            payment_method_type: attempt.payment_method_type.as_ref(),
            payment_method_data: attempt.payment_method_data.as_ref().map(|v| v.to_string()),
            error_reason: attempt.error_reason.as_ref(),
            multiple_capture_count: attempt.multiple_capture_count,
            amount_capturable: attempt.amount_capturable,
            merchant_connector_id: attempt.merchant_connector_id.as_ref(),
            net_amount: attempt.net_amount,
            unified_code: attempt.unified_code.as_ref(),
            unified_message: attempt.unified_message.as_ref(),
            mandate_data: attempt.mandate_data.as_ref(),
        }
    }
}

impl<'a> super::KafkaMessage for KafkaPaymentAttempt<'a> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id, self.payment_id, self.attempt_id
        )
    }

    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.modified_at.unix_timestamp())
    }
}
