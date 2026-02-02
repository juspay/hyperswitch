#[cfg(feature = "v2")]
use common_types::payments;
#[cfg(feature = "v2")]
use common_utils::types;
use common_utils::{id_type, types::MinorUnit};
use diesel_models::enums as storage_enums;
#[cfg(feature = "v2")]
use diesel_models::payment_attempt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{
    address, payments::payment_attempt::PaymentAttemptFeatureMetadata,
    router_response_types::RedirectForm,
};
use hyperswitch_domain_models::{
    mandates::MandateDetails, payments::payment_attempt::PaymentAttempt,
};
use time::OffsetDateTime;

#[cfg(feature = "v1")]
#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentAttempt<'a> {
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
    pub routing_approach: Option<storage_enums::RoutingApproach>,
    pub debit_routing_savings: Option<MinorUnit>,
    pub signature_network: Option<common_enums::CardNetwork>,
    pub is_issuer_regulated: Option<bool>,
}

#[cfg(feature = "v1")]
impl<'a> KafkaPaymentAttempt<'a> {
    pub fn from_storage(attempt: &'a PaymentAttempt) -> Self {
        let card_payment_method_data = attempt
            .get_payment_method_data()
            .and_then(|data| data.get_additional_card_info());
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
            routing_approach: attempt.routing_approach.clone(),
            debit_routing_savings: attempt.debit_routing_savings,
            signature_network: card_payment_method_data
                .as_ref()
                .and_then(|data| data.signature_network.clone()),
            is_issuer_regulated: card_payment_method_data.and_then(|data| data.is_regulated),
        }
    }
}

#[cfg(feature = "v2")]
#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentAttempt<'a> {
    pub payment_id: &'a id_type::GlobalPaymentId,
    pub merchant_id: &'a id_type::MerchantId,
    pub attempt_id: &'a id_type::GlobalAttemptId,
    pub attempts_group_id: Option<&'a id_type::GlobalAttemptGroupId>,
    pub status: storage_enums::AttemptStatus,
    pub amount: MinorUnit,
    pub connector: Option<&'a String>,
    pub error_message: Option<&'a String>,
    pub surcharge_amount: Option<MinorUnit>,
    pub tax_amount: Option<MinorUnit>,
    pub payment_method_id: Option<&'a id_type::GlobalPaymentMethodId>,
    pub payment_method: storage_enums::PaymentMethod,
    pub connector_transaction_id: Option<&'a String>,
    pub authentication_type: storage_enums::AuthenticationType,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub modified_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::option")]
    pub last_synced: Option<OffsetDateTime>,
    pub cancellation_reason: Option<&'a String>,
    pub amount_to_capture: Option<MinorUnit>,
    pub browser_info: Option<&'a types::BrowserInformation>,
    pub error_code: Option<&'a String>,
    pub connector_metadata: Option<String>,
    // TODO: These types should implement copy ideally
    pub payment_experience: Option<&'a storage_enums::PaymentExperience>,
    pub payment_method_type: &'a storage_enums::PaymentMethodType,
    pub payment_method_data: Option<String>,
    pub error_reason: Option<&'a String>,
    pub multiple_capture_count: Option<i16>,
    pub amount_capturable: MinorUnit,
    pub merchant_connector_id: Option<&'a id_type::MerchantConnectorAccountId>,
    pub net_amount: MinorUnit,
    pub unified_code: Option<&'a String>,
    pub unified_message: Option<&'a String>,
    pub client_source: Option<&'a String>,
    pub client_version: Option<&'a String>,
    pub profile_id: &'a id_type::ProfileId,
    pub organization_id: &'a id_type::OrganizationId,
    pub card_network: Option<String>,
    pub card_discovery: Option<String>,

    pub connector_payment_id: Option<types::ConnectorTransactionId>,
    pub payment_token: Option<String>,
    pub preprocessing_step_id: Option<String>,
    pub connector_response_reference_id: Option<String>,
    pub updated_by: &'a String,
    pub encoded_data: Option<&'a masking::Secret<String>>,
    pub external_three_ds_authentication_attempted: Option<bool>,
    pub authentication_connector: Option<String>,
    pub authentication_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub customer_acceptance: Option<&'a masking::Secret<payments::CustomerAcceptance>>,
    pub shipping_cost: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
    pub charges: Option<payments::ConnectorChargeResponseData>,
    pub processor_merchant_id: &'a id_type::MerchantId,
    pub created_by: Option<&'a types::CreatedBy>,
    pub payment_method_type_v2: storage_enums::PaymentMethod,
    pub payment_method_subtype: storage_enums::PaymentMethodType,
    pub routing_result: Option<serde_json::Value>,
    pub authentication_applied: Option<common_enums::AuthenticationType>,
    pub external_reference_id: Option<String>,
    pub tax_on_surcharge: Option<MinorUnit>,
    pub payment_method_billing_address: Option<masking::Secret<&'a address::Address>>, // adjusted from Encryption
    pub redirection_data: Option<&'a RedirectForm>,
    pub connector_payment_data: Option<String>,
    pub connector_token_details: Option<&'a payment_attempt::ConnectorTokenDetails>,
    pub feature_metadata: Option<&'a PaymentAttemptFeatureMetadata>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
    pub connector_request_reference_id: Option<String>,
}

#[cfg(feature = "v2")]
impl<'a> KafkaPaymentAttempt<'a> {
    pub fn from_storage(attempt: &'a PaymentAttempt) -> Self {
        use masking::PeekInterface;
        let PaymentAttempt {
            payment_id,
            merchant_id,
            attempts_group_id,
            amount_details,
            status,
            connector,
            error,
            authentication_type,
            created_at,
            modified_at,
            last_synced,
            cancellation_reason,
            browser_info,
            payment_token,
            connector_metadata,
            payment_experience,
            payment_method_data,
            routing_result,
            preprocessing_step_id,
            multiple_capture_count,
            connector_response_reference_id,
            updated_by,
            redirection_data,
            encoded_data,
            merchant_connector_id,
            external_three_ds_authentication_attempted,
            authentication_connector,
            authentication_id,
            fingerprint_id,
            client_source,
            client_version,
            customer_acceptance,
            profile_id,
            organization_id,
            payment_method_type,
            payment_method_id,
            connector_payment_id,
            payment_method_subtype,
            authentication_applied,
            external_reference_id,
            payment_method_billing_address,
            id,
            connector_token_details,
            card_discovery,
            charges,
            feature_metadata,
            processor_merchant_id,
            created_by,
            connector_request_reference_id,
            network_transaction_id: _,
            authorized_amount: _,
        } = attempt;

        let (connector_payment_id, connector_payment_data) = connector_payment_id
            .clone()
            .map(types::ConnectorTransactionId::form_id_and_data)
            .map(|(txn_id, txn_data)| (Some(txn_id), txn_data))
            .unwrap_or((None, None));

        Self {
            payment_id,
            merchant_id,
            attempt_id: id,
            attempts_group_id: attempts_group_id.as_ref(),
            status: *status,
            amount: amount_details.get_net_amount(),
            connector: connector.as_ref(),
            error_message: error.as_ref().map(|error_details| &error_details.message),
            surcharge_amount: amount_details.get_surcharge_amount(),
            tax_amount: amount_details.get_tax_on_surcharge(),
            payment_method_id: payment_method_id.as_ref(),
            payment_method: *payment_method_type,
            connector_transaction_id: connector_response_reference_id.as_ref(),
            authentication_type: *authentication_type,
            created_at: created_at.assume_utc(),
            modified_at: modified_at.assume_utc(),
            last_synced: last_synced.map(|i| i.assume_utc()),
            cancellation_reason: cancellation_reason.as_ref(),
            amount_to_capture: amount_details.get_amount_to_capture(),
            browser_info: browser_info.as_ref(),
            error_code: error.as_ref().map(|error_details| &error_details.code),
            connector_metadata: connector_metadata.as_ref().map(|v| v.peek().to_string()),
            payment_experience: payment_experience.as_ref(),
            payment_method_type: payment_method_subtype,
            payment_method_data: payment_method_data.as_ref().map(|v| v.peek().to_string()),
            error_reason: error
                .as_ref()
                .and_then(|error_details| error_details.reason.as_ref()),
            multiple_capture_count: *multiple_capture_count,
            amount_capturable: amount_details.get_amount_capturable(),
            merchant_connector_id: merchant_connector_id.as_ref(),
            net_amount: amount_details.get_net_amount(),
            unified_code: error
                .as_ref()
                .and_then(|error_details| error_details.unified_code.as_ref()),
            unified_message: error
                .as_ref()
                .and_then(|error_details| error_details.unified_message.as_ref()),
            client_source: client_source.as_ref(),
            client_version: client_version.as_ref(),
            profile_id,
            organization_id,
            card_network: payment_method_data
                .as_ref()
                .map(|data| data.peek())
                .and_then(|data| data.as_object())
                .and_then(|pm| pm.get("card"))
                .and_then(|data| data.as_object())
                .and_then(|card| card.get("card_network"))
                .and_then(|network| network.as_str())
                .map(|network| network.to_string()),
            card_discovery: card_discovery.map(|discovery| discovery.to_string()),
            payment_token: payment_token.clone(),
            preprocessing_step_id: preprocessing_step_id.clone(),
            connector_response_reference_id: connector_response_reference_id.clone(),
            updated_by,
            encoded_data: encoded_data.as_ref(),
            external_three_ds_authentication_attempted: *external_three_ds_authentication_attempted,
            authentication_connector: authentication_connector.clone(),
            authentication_id: authentication_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string().clone()),
            fingerprint_id: fingerprint_id.clone(),
            customer_acceptance: customer_acceptance.as_ref(),
            shipping_cost: amount_details.get_shipping_cost(),
            order_tax_amount: amount_details.get_order_tax_amount(),
            charges: charges.clone(),
            processor_merchant_id,
            created_by: created_by.as_ref(),
            payment_method_type_v2: *payment_method_type,
            connector_payment_id: connector_payment_id.as_ref().cloned(),
            payment_method_subtype: *payment_method_subtype,
            routing_result: routing_result.clone(),
            authentication_applied: *authentication_applied,
            external_reference_id: external_reference_id.clone(),
            tax_on_surcharge: amount_details.get_tax_on_surcharge(),
            payment_method_billing_address: payment_method_billing_address
                .as_ref()
                .map(|v| masking::Secret::new(v.get_inner())),
            redirection_data: redirection_data.as_ref(),
            connector_payment_data,
            connector_token_details: connector_token_details.as_ref(),
            feature_metadata: feature_metadata.as_ref(),
            network_advice_code: error
                .as_ref()
                .and_then(|details| details.network_advice_code.clone()),
            network_decline_code: error
                .as_ref()
                .and_then(|details| details.network_decline_code.clone()),
            network_error_message: error
                .as_ref()
                .and_then(|details| details.network_error_message.clone()),
            connector_request_reference_id: connector_request_reference_id.clone(),
        }
    }
}

impl super::KafkaMessage for KafkaPaymentAttempt<'_> {
    #[cfg(feature = "v1")]
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.attempt_id
        )
    }
    #[cfg(feature = "v2")]
    fn key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.merchant_id.get_string_repr(),
            self.payment_id.get_string_repr(),
            self.attempt_id.get_string_repr()
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::PaymentAttempt
    }
}
