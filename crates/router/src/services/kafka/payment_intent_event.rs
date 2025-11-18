#[cfg(feature = "v2")]
use ::common_types::{
    payments,
    primitive_wrappers::{EnablePartialAuthorizationBool, RequestExtendedAuthorizationBool},
};
#[cfg(feature = "v2")]
use common_enums::{self, RequestIncrementalAuthorization};
use common_utils::{
    crypto::Encryptable, hashing::HashedString, id_type, pii, types as common_types,
};
use diesel_models::enums as storage_enums;
#[cfg(feature = "v2")]
use diesel_models::{types as diesel_types, PaymentLinkConfigRequestForPayments};
#[cfg(feature = "v2")]
use diesel_models::{types::OrderDetailsWithAmount, TaxDetails};
use hyperswitch_domain_models::payments::PaymentIntent;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{address, routing};
use masking::{PeekInterface, Secret};
use serde_json::Value;
use time::OffsetDateTime;

#[cfg(feature = "v1")]
#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentIntentEvent<'a> {
    pub payment_id: &'a id_type::PaymentId,
    pub merchant_id: &'a id_type::MerchantId,
    pub status: storage_enums::IntentStatus,
    pub amount: common_types::MinorUnit,
    pub currency: Option<storage_enums::Currency>,
    pub amount_captured: Option<common_types::MinorUnit>,
    pub customer_id: Option<&'a id_type::CustomerId>,
    pub description: Option<&'a String>,
    pub return_url: Option<&'a String>,
    pub metadata: Option<String>,
    pub connector_id: Option<&'a String>,
    pub statement_descriptor_name: Option<&'a String>,
    pub statement_descriptor_suffix: Option<&'a String>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub modified_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub last_synced: Option<OffsetDateTime>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<&'a String>,
    pub active_attempt_id: String,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<&'a String>,
    pub attempt_count: i16,
    pub profile_id: Option<&'a id_type::ProfileId>,
    pub payment_confirm_source: Option<storage_enums::PaymentSource>,
    pub billing_details: Option<Encryptable<Secret<Value>>>,
    pub shipping_details: Option<Encryptable<Secret<Value>>>,
    pub customer_email: Option<HashedString<pii::EmailStrategy>>,
    pub feature_metadata: Option<&'a Value>,
    pub merchant_order_reference_id: Option<&'a String>,
    pub organization_id: &'a id_type::OrganizationId,
    #[serde(flatten)]
    pub infra_values: Option<Value>,
}

#[cfg(feature = "v2")]
#[serde_with::skip_serializing_none]
#[derive(serde::Serialize, Debug)]
pub struct KafkaPaymentIntentEvent<'a> {
    pub payment_id: &'a id_type::GlobalPaymentId,
    pub merchant_id: &'a id_type::MerchantId,
    pub status: storage_enums::IntentStatus,
    pub amount: common_types::MinorUnit,
    pub currency: storage_enums::Currency,
    pub amount_captured: Option<common_types::MinorUnit>,
    pub customer_id: Option<&'a id_type::GlobalCustomerId>,
    pub description: Option<&'a common_types::Description>,
    pub return_url: Option<&'a common_types::Url>,
    pub metadata: Option<&'a Secret<Value>>,
    pub statement_descriptor: Option<&'a common_types::StatementDescriptor>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub modified_at: OffsetDateTime,
    #[serde(default, with = "time::serde::timestamp::nanoseconds::option")]
    pub last_synced: Option<OffsetDateTime>,
    pub setup_future_usage: storage_enums::FutureUsage,
    pub off_session: bool,
    pub active_attempt_id: Option<&'a id_type::GlobalAttemptId>,
    pub active_attempt_id_type: common_enums::ActiveAttemptIDType,
    pub active_attempts_group_id: Option<&'a id_type::GlobalAttemptGroupId>,
    pub attempt_count: i16,
    pub profile_id: &'a id_type::ProfileId,
    pub customer_email: Option<HashedString<pii::EmailStrategy>>,
    pub feature_metadata: Option<&'a diesel_types::FeatureMetadata>,
    pub organization_id: &'a id_type::OrganizationId,
    pub order_details: Option<&'a Vec<Secret<OrderDetailsWithAmount>>>,

    pub allowed_payment_method_types: Option<&'a Vec<common_enums::PaymentMethodType>>,
    pub connector_metadata: Option<&'a api_models::payments::ConnectorMetadata>,
    pub payment_link_id: Option<&'a String>,
    pub updated_by: &'a String,
    pub surcharge_applicable: Option<bool>,
    pub request_incremental_authorization: RequestIncrementalAuthorization,
    pub split_txns_enabled: common_enums::SplitTxnsEnabled,
    pub authorization_count: Option<i32>,
    #[serde(with = "time::serde::timestamp::nanoseconds")]
    pub session_expiry: OffsetDateTime,
    pub request_external_three_ds_authentication: common_enums::External3dsAuthenticationRequest,
    pub frm_metadata: Option<Secret<&'a Value>>,
    pub customer_details: Option<Secret<&'a Value>>,
    pub shipping_cost: Option<common_types::MinorUnit>,
    pub tax_details: Option<TaxDetails>,
    pub skip_external_tax_calculation: bool,
    pub request_extended_authorization: Option<RequestExtendedAuthorizationBool>,
    pub psd2_sca_exemption_type: Option<storage_enums::ScaExemptionType>,
    pub split_payments: Option<&'a payments::SplitPaymentsRequest>,
    pub platform_merchant_id: Option<&'a id_type::MerchantId>,
    pub force_3ds_challenge: Option<bool>,
    pub force_3ds_challenge_trigger: Option<bool>,
    pub processor_merchant_id: &'a id_type::MerchantId,
    pub created_by: Option<&'a common_types::CreatedBy>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub merchant_reference_id: Option<&'a id_type::PaymentReferenceId>,
    pub capture_method: storage_enums::CaptureMethod,
    pub authentication_type: Option<common_enums::AuthenticationType>,
    pub prerouting_algorithm: Option<&'a routing::PaymentRoutingInfo>,
    pub surcharge_amount: Option<common_types::MinorUnit>,
    pub billing_address: Option<Secret<&'a address::Address>>,
    pub shipping_address: Option<Secret<&'a address::Address>>,
    pub tax_on_surcharge: Option<common_types::MinorUnit>,
    pub frm_merchant_decision: Option<common_enums::MerchantDecision>,
    pub enable_payment_link: common_enums::EnablePaymentLinkRequest,
    pub apply_mit_exemption: common_enums::MitExemptionRequest,
    pub customer_present: common_enums::PresenceOfCustomerDuringPayment,
    pub routing_algorithm_id: Option<&'a id_type::RoutingId>,
    pub payment_link_config: Option<&'a PaymentLinkConfigRequestForPayments>,
    pub enable_partial_authorization: Option<EnablePartialAuthorizationBool>,

    #[serde(flatten)]
    infra_values: Option<Value>,
}

impl KafkaPaymentIntentEvent<'_> {
    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &id_type::PaymentId {
        self.payment_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &id_type::GlobalPaymentId {
        self.payment_id
    }
}

#[cfg(feature = "v1")]
impl<'a> KafkaPaymentIntentEvent<'a> {
    pub fn from_storage(intent: &'a PaymentIntent, infra_values: Option<Value>) -> Self {
        Self {
            payment_id: &intent.payment_id,
            merchant_id: &intent.merchant_id,
            status: intent.status,
            amount: intent.amount,
            currency: intent.currency,
            amount_captured: intent.amount_captured,
            customer_id: intent.customer_id.as_ref(),
            description: intent.description.as_ref(),
            return_url: intent.return_url.as_ref(),
            metadata: intent.metadata.as_ref().map(|x| x.to_string()),
            connector_id: intent.connector_id.as_ref(),
            statement_descriptor_name: intent.statement_descriptor_name.as_ref(),
            statement_descriptor_suffix: intent.statement_descriptor_suffix.as_ref(),
            created_at: intent.created_at.assume_utc(),
            modified_at: intent.modified_at.assume_utc(),
            last_synced: intent.last_synced.map(|i| i.assume_utc()),
            setup_future_usage: intent.setup_future_usage,
            off_session: intent.off_session,
            client_secret: intent.client_secret.as_ref(),
            active_attempt_id: intent.active_attempt.get_id(),
            business_country: intent.business_country,
            business_label: intent.business_label.as_ref(),
            attempt_count: intent.attempt_count,
            profile_id: intent.profile_id.as_ref(),
            payment_confirm_source: intent.payment_confirm_source,
            // TODO: use typed information here to avoid PII logging
            billing_details: None,
            shipping_details: None,
            customer_email: intent
                .customer_details
                .as_ref()
                .and_then(|value| value.get_inner().peek().as_object())
                .and_then(|obj| obj.get("email"))
                .and_then(|email| email.as_str())
                .map(|email| HashedString::from(Secret::new(email.to_string()))),
            feature_metadata: intent.feature_metadata.as_ref(),
            merchant_order_reference_id: intent.merchant_order_reference_id.as_ref(),
            organization_id: &intent.organization_id,
            infra_values: infra_values.clone(),
        }
    }
}

#[cfg(feature = "v2")]
impl<'a> KafkaPaymentIntentEvent<'a> {
    pub fn from_storage(intent: &'a PaymentIntent, infra_values: Option<Value>) -> Self {
        let PaymentIntent {
            id,
            merchant_id,
            status,
            amount_details,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            statement_descriptor,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage,
            active_attempt_id,
            active_attempt_id_type,
            active_attempts_group_id,
            order_details,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count,
            profile_id,
            payment_link_id,
            frm_merchant_decision,
            updated_by,
            request_incremental_authorization,
            split_txns_enabled,
            authorization_count,
            session_expiry,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details,
            merchant_reference_id,
            billing_address,
            shipping_address,
            capture_method,
            authentication_type,
            prerouting_algorithm,
            organization_id,
            enable_payment_link,
            apply_mit_exemption,
            customer_present,
            payment_link_config,
            routing_algorithm_id,
            split_payments,
            force_3ds_challenge,
            force_3ds_challenge_trigger,
            processor_merchant_id,
            created_by,
            is_iframe_redirection_enabled,
            is_payment_id_from_merchant,
            enable_partial_authorization,
        } = intent;

        Self {
            payment_id: id,
            merchant_id,
            status: *status,
            amount: amount_details.order_amount,
            currency: amount_details.currency,
            amount_captured: *amount_captured,
            customer_id: customer_id.as_ref(),
            description: description.as_ref(),
            return_url: return_url.as_ref(),
            metadata: metadata.as_ref(),
            statement_descriptor: statement_descriptor.as_ref(),
            created_at: created_at.assume_utc(),
            modified_at: modified_at.assume_utc(),
            last_synced: last_synced.map(|t| t.assume_utc()),
            setup_future_usage: *setup_future_usage,
            off_session: setup_future_usage.is_off_session(),
            active_attempt_id: active_attempt_id.as_ref(),
            active_attempt_id_type: *active_attempt_id_type,
            active_attempts_group_id: active_attempts_group_id.as_ref(),
            attempt_count: *attempt_count,
            profile_id,
            customer_email: None,
            feature_metadata: feature_metadata.as_ref(),
            organization_id,
            order_details: order_details.as_ref(),
            allowed_payment_method_types: allowed_payment_method_types.as_ref(),
            connector_metadata: connector_metadata.as_ref(),
            payment_link_id: payment_link_id.as_ref(),
            updated_by,
            surcharge_applicable: None,
            request_incremental_authorization: *request_incremental_authorization,
            split_txns_enabled: *split_txns_enabled,
            authorization_count: *authorization_count,
            session_expiry: session_expiry.assume_utc(),
            request_external_three_ds_authentication: *request_external_three_ds_authentication,
            frm_metadata: frm_metadata
                .as_ref()
                .map(|frm_metadata| frm_metadata.as_ref()),
            customer_details: customer_details
                .as_ref()
                .map(|customer_details| customer_details.get_inner().as_ref()),
            shipping_cost: amount_details.shipping_cost,
            tax_details: amount_details.tax_details.clone(),
            skip_external_tax_calculation: amount_details.get_external_tax_action_as_bool(),
            request_extended_authorization: None,
            psd2_sca_exemption_type: None,
            split_payments: split_payments.as_ref(),
            platform_merchant_id: None,
            force_3ds_challenge: *force_3ds_challenge,
            force_3ds_challenge_trigger: *force_3ds_challenge_trigger,
            processor_merchant_id,
            created_by: created_by.as_ref(),
            is_iframe_redirection_enabled: *is_iframe_redirection_enabled,
            merchant_reference_id: merchant_reference_id.as_ref(),
            billing_address: billing_address
                .as_ref()
                .map(|billing_address| Secret::new(billing_address.get_inner())),
            shipping_address: shipping_address
                .as_ref()
                .map(|shipping_address| Secret::new(shipping_address.get_inner())),
            capture_method: *capture_method,
            authentication_type: *authentication_type,
            prerouting_algorithm: prerouting_algorithm.as_ref(),
            surcharge_amount: amount_details.surcharge_amount,
            tax_on_surcharge: amount_details.tax_on_surcharge,
            frm_merchant_decision: *frm_merchant_decision,
            enable_payment_link: *enable_payment_link,
            apply_mit_exemption: *apply_mit_exemption,
            customer_present: *customer_present,
            routing_algorithm_id: routing_algorithm_id.as_ref(),
            payment_link_config: payment_link_config.as_ref(),
            infra_values,
            enable_partial_authorization: *enable_partial_authorization,
        }
    }
}

impl super::KafkaMessage for KafkaPaymentIntentEvent<'_> {
    fn key(&self) -> String {
        format!(
            "{}_{}",
            self.merchant_id.get_string_repr(),
            self.get_id().get_string_repr(),
        )
    }

    fn event_type(&self) -> crate::events::EventType {
        crate::events::EventType::PaymentIntent
    }
}
