// use diesel_models::enums::MandateDetails;
use common_utils::{id_type, types::MinorUnit};
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::{
    mandates::MandateDetails, payments::payment_attempt::PaymentAttempt,
};
use time::OffsetDateTime;

#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentAttemptEvent<'a> {
    pub payment_id: &'a id_type::PaymentId,
    pub merchant_id: &'a id_type::MerchantId,
    pub attempt_id: &'a String,
    pub status: storage_enums::AttemptStatus,
    pub amount: MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub save_to_locker: Option<bool>,
    pub connector: Option<&'a String>,
    pub error_message: Option<&'a String>,
    pub offer_amount: Option<MinorUnit>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_amount: Option<MinorUnit>,
    pub payment_method_id: Option<&'a String>,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub connector_transaction_id: Option<&'a String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub capture_on: Option<OffsetDateTime>,
    pub confirm: bool,
    pub authentication_type: Option<storage_enums::AuthenticationType>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub modified_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub last_synced: Option<OffsetDateTime>,
    pub cancellation_reason: Option<&'a String>,
    pub amount_to_capture: Option<MinorUnit>,
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
    pub amount_capturable: MinorUnit,
    pub merchant_connector_id: Option<&'a id_type::MerchantConnectorAccountId>,
    pub net_amount: MinorUnit,
    pub unified_code: Option<&'a String>,
    pub unified_message: Option<&'a String>,
    pub mandate_data: Option<&'a MandateDetails>,
    pub client_source: Option<&'a String>,
    pub client_version: Option<&'a String>,
    pub profile_id: &'a id_type::ProfileId,
    pub organization_id: &'a id_type::OrganizationId,
    pub card_network: Option<String>,
    pub card_discovery: Option<String>,
}

#[cfg(feature = "v1")]
impl<'a> KafkaPaymentAttemptEvent<'a> {
    pub fn from_storage(attempt: &'a PaymentAttempt) -> Self {
        Self {
            payment_id: &attempt.payment_id,
            merchant_id: &attempt.merchant_id,
            attempt_id: &attempt.attempt_id,
            status: attempt.status,
            amount: attempt.net_amount.get_order_amount(),
            currency: attempt.currency,
            save_to_locker: attempt.save_to_locker,
            connector: attempt.connector.as_ref(),
            error_message: attempt.error_message.as_ref(),
            offer_amount: attempt.offer_amount,
            surcharge_amount: attempt.net_amount.get_surcharge_amount(),
            tax_amount: attempt.net_amount.get_tax_on_surcharge(),
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
            net_amount: attempt.net_amount.get_total_amount(),
            unified_code: attempt.unified_code.as_ref(),
            unified_message: attempt.unified_message.as_ref(),
            mandate_data: attempt.mandate_data.as_ref(),
            client_source: attempt.client_source.as_ref(),
            client_version: attempt.client_version.as_ref(),
            profile_id: &attempt.profile_id,
            organization_id: &attempt.organization_id,
            card_network: attempt
                .payment_method_data
                .as_ref()
                .and_then(|data| data.as_object())
                .and_then(|pm| pm.get("card"))
                .and_then(|data| data.as_object())
                .and_then(|card| card.get("card_network"))
                .and_then(|network| network.as_str())
                .map(|network| network.to_string()),
            card_discovery: attempt
                .card_discovery
                .map(|discovery| discovery.to_string()),
        }
    }
}

#[cfg(feature = "v2")]
impl<'a> KafkaPaymentAttemptEvent<'a> {
    pub fn from_storage(attempt: &'a PaymentAttempt) -> Self {
        todo!()
        // Self {
        //     payment_id: &attempt.payment_id,
        //     merchant_id: &attempt.merchant_id,
        //     attempt_id: &attempt.attempt_id,
        //     status: attempt.status,
        //     amount: attempt.amount,
        //     currency: attempt.currency,
        //     save_to_locker: attempt.save_to_locker,
        //     connector: attempt.connector.as_ref(),
        //     error_message: attempt.error_message.as_ref(),
        //     offer_amount: attempt.offer_amount,
        //     surcharge_amount: attempt.surcharge_amount,
        //     tax_amount: attempt.tax_amount,
        //     payment_method_id: attempt.payment_method_id.as_ref(),
        //     payment_method: attempt.payment_method,
        //     connector_transaction_id: attempt.connector_transaction_id.as_ref(),
        //     capture_method: attempt.capture_method,
        //     capture_on: attempt.capture_on.map(|i| i.assume_utc()),
        //     confirm: attempt.confirm,
        //     authentication_type: attempt.authentication_type,
        //     created_at: attempt.created_at.assume_utc(),
        //     modified_at: attempt.modified_at.assume_utc(),
        //     last_synced: attempt.last_synced.map(|i| i.assume_utc()),
        //     cancellation_reason: attempt.cancellation_reason.as_ref(),
        //     amount_to_capture: attempt.amount_to_capture,
        //     mandate_id: attempt.mandate_id.as_ref(),
        //     browser_info: attempt.browser_info.as_ref().map(|v| v.to_string()),
        //     error_code: attempt.error_code.as_ref(),
        //     connector_metadata: attempt.connector_metadata.as_ref().map(|v| v.to_string()),
        //     payment_experience: attempt.payment_experience.as_ref(),
        //     payment_method_type: attempt.payment_method_type.as_ref(),
        //     payment_method_data: attempt.payment_method_data.as_ref().map(|v| v.to_string()),
        //     error_reason: attempt.error_reason.as_ref(),
        //     multiple_capture_count: attempt.multiple_capture_count,
        //     amount_capturable: attempt.amount_capturable,
        //     merchant_connector_id: attempt.merchant_connector_id.as_ref(),
        //     net_amount: attempt.net_amount,
        //     unified_code: attempt.unified_code.as_ref(),
        //     unified_message: attempt.unified_message.as_ref(),
        //     mandate_data: attempt.mandate_data.as_ref(),
        //     client_source: attempt.client_source.as_ref(),
        //     client_version: attempt.client_version.as_ref(),
        //     profile_id: &attempt.profile_id,
        //     organization_id: &attempt.organization_id,
        //     card_network: attempt
        //         .payment_method_data
        //         .as_ref()
        //         .and_then(|data| data.as_object())
        //         .and_then(|pm| pm.get("card"))
        //         .and_then(|data| data.as_object())
        //         .and_then(|card| card.get("card_network"))
        //         .and_then(|network| network.as_str())
        //         .map(|network| network.to_string()),
        // }
    }
}

impl super::KafkaMessage for KafkaPaymentAttemptEvent<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.attempt_id
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::PaymentAttempt
    }
}
