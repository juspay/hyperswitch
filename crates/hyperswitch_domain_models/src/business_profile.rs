use std::borrow::Cow;

use common_enums::enums as api_enums;
use common_types::{domain::AcquirerConfig, primitive_wrappers};
use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::{OptionExt, ValueExt},
    pii, type_name,
    types::keymanager,
};
use common_types::business_profile_types::{
    AuthenticationConnectorDetails, BusinessPaymentLinkConfig, BusinessPayoutLinkConfig,
    CardTestingGuardConfig, ExternalVaultConnectorDetails, PaymentMethodBlockingConfig,
    WebhookDetails,
};
#[cfg(feature = "v2")]
use common_types::business_profile_types::RevenueRecoveryAlgorithmData;
use error_stack::ResultExt;
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use router_env::logger;

use crate::{
    errors::api_error_response,
    merchant_key_store::MerchantKeyStore,
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};
#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct Profile {
    profile_id: common_utils::id_type::ProfileId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: bool,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: bool,
    pub is_l2_l3_enabled: bool,
    pub version: common_enums::ApiVersion,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: bool,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization:
        Option<primitive_wrappers::AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_clear_pan_retries_enabled: bool,
    pub force_3ds_challenge: bool,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: bool,
    pub three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    pub acquirer_config_map: Option<common_types::domain::AcquirerConfigMap>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub external_vault_details: ExternalVaultDetails,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub network_tokenization_credentials: OptionalEncryptableValue,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub enum ExternalVaultDetails {
    ExternalVaultEnabled(ExternalVaultConnectorDetails),
    Skip,
}

#[cfg(feature = "v1")]
impl ExternalVaultDetails {
    pub fn is_external_vault_enabled(&self) -> bool {
        match self {
            Self::ExternalVaultEnabled(_) => true,
            Self::Skip => false,
        }
    }
}

#[cfg(feature = "v1")]
impl
    TryFrom<(
        Option<common_enums::ExternalVaultEnabled>,
        Option<ExternalVaultConnectorDetails>,
    )> for ExternalVaultDetails
{
    type Error = error_stack::Report<ValidationError>;
    fn try_from(
        item: (
            Option<common_enums::ExternalVaultEnabled>,
            Option<ExternalVaultConnectorDetails>,
        ),
    ) -> Result<Self, Self::Error> {
        match item {
            (is_external_vault_enabled, external_vault_connector_details)
                if is_external_vault_enabled
                    .unwrap_or(common_enums::ExternalVaultEnabled::Skip)
                    == common_enums::ExternalVaultEnabled::Enable =>
            {
                Ok(Self::ExternalVaultEnabled(
                    external_vault_connector_details
                        .get_required_value("ExternalVaultConnectorDetails")?,
                ))
            }
            _ => Ok(Self::Skip),
        }
    }
}

#[cfg(feature = "v1")]
impl TryFrom<(Option<bool>, Option<ExternalVaultConnectorDetails>)> for ExternalVaultDetails {
    type Error = error_stack::Report<ValidationError>;
    fn try_from(
        item: (Option<bool>, Option<ExternalVaultConnectorDetails>),
    ) -> Result<Self, Self::Error> {
        match item {
            (is_external_vault_enabled, external_vault_connector_details)
                if is_external_vault_enabled.unwrap_or(false) =>
            {
                Ok(Self::ExternalVaultEnabled(
                    external_vault_connector_details
                        .get_required_value("ExternalVaultConnectorDetails")?,
                ))
            }
            _ => Ok(Self::Skip),
        }
    }
}

#[cfg(feature = "v1")]
impl From<ExternalVaultDetails>
    for (
        Option<common_enums::ExternalVaultEnabled>,
        Option<ExternalVaultConnectorDetails>,
    )
{
    fn from(external_vault_details: ExternalVaultDetails) -> Self {
        match external_vault_details {
            ExternalVaultDetails::ExternalVaultEnabled(connector_details) => (
                Some(common_enums::ExternalVaultEnabled::Enable),
                Some(connector_details),
            ),
            ExternalVaultDetails::Skip => (Some(common_enums::ExternalVaultEnabled::Skip), None),
        }
    }
}

#[cfg(feature = "v1")]
impl From<ExternalVaultDetails> for (Option<bool>, Option<ExternalVaultConnectorDetails>) {
    fn from(external_vault_details: ExternalVaultDetails) -> Self {
        match external_vault_details {
            ExternalVaultDetails::ExternalVaultEnabled(connector_details) => {
                (Some(true), Some(connector_details))
            }
            ExternalVaultDetails::Skip => (Some(false), None),
        }
    }
}

#[cfg(feature = "v1")]
pub struct ProfileSetter {
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: bool,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: bool,
    pub is_l2_l3_enabled: bool,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: bool,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization:
        Option<primitive_wrappers::AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_clear_pan_retries_enabled: bool,
    pub force_3ds_challenge: bool,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: bool,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub external_vault_details: ExternalVaultDetails,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub network_tokenization_credentials: OptionalEncryptableValue,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
}

#[cfg(feature = "v1")]
impl From<ProfileSetter> for Profile {
    fn from(value: ProfileSetter) -> Self {
        Self {
            profile_id: value.profile_id,
            merchant_id: value.merchant_id,
            profile_name: value.profile_name,
            created_at: value.created_at,
            modified_at: value.modified_at,
            return_url: value.return_url,
            enable_payment_response_hash: value.enable_payment_response_hash,
            payment_response_hash_key: value.payment_response_hash_key,
            redirect_to_merchant_with_http_post: value.redirect_to_merchant_with_http_post,
            webhook_details: value.webhook_details,
            metadata: value.metadata,
            routing_algorithm: value.routing_algorithm,
            intent_fulfillment_time: value.intent_fulfillment_time,
            frm_routing_algorithm: value.frm_routing_algorithm,
            payout_routing_algorithm: value.payout_routing_algorithm,
            is_recon_enabled: value.is_recon_enabled,
            applepay_verified_domains: value.applepay_verified_domains,
            payment_link_config: value.payment_link_config,
            session_expiry: value.session_expiry,
            authentication_connector_details: value.authentication_connector_details,
            payout_link_config: value.payout_link_config,
            is_extended_card_info_enabled: value.is_extended_card_info_enabled,
            extended_card_info_config: value.extended_card_info_config,
            is_connector_agnostic_mit_enabled: value.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: value.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: value
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: value
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: value.outgoing_webhook_custom_http_headers,
            always_collect_billing_details_from_wallet_connector: value
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: value
                .always_collect_shipping_details_from_wallet_connector,
            tax_connector_id: value.tax_connector_id,
            is_tax_connector_enabled: value.is_tax_connector_enabled,
            is_l2_l3_enabled: value.is_l2_l3_enabled,
            version: common_types::consts::API_VERSION,
            dynamic_routing_algorithm: value.dynamic_routing_algorithm,
            is_network_tokenization_enabled: value.is_network_tokenization_enabled,
            is_auto_retries_enabled: value.is_auto_retries_enabled,
            max_auto_retries_enabled: value.max_auto_retries_enabled,
            always_request_extended_authorization: value.always_request_extended_authorization,
            is_click_to_pay_enabled: value.is_click_to_pay_enabled,
            authentication_product_ids: value.authentication_product_ids,
            card_testing_guard_config: value.card_testing_guard_config,
            card_testing_secret_key: value.card_testing_secret_key,
            is_clear_pan_retries_enabled: value.is_clear_pan_retries_enabled,
            force_3ds_challenge: value.force_3ds_challenge,
            is_debit_routing_enabled: value.is_debit_routing_enabled,
            merchant_business_country: value.merchant_business_country,
            is_iframe_redirection_enabled: value.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: value.is_pre_network_tokenization_enabled,
            three_ds_decision_rule_algorithm: None, // three_ds_decision_rule_algorithm is not yet created during profile creation
            acquirer_config_map: None,
            merchant_category_code: value.merchant_category_code,
            merchant_country_code: value.merchant_country_code,
            dispute_polling_interval: value.dispute_polling_interval,
            is_manual_retry_enabled: value.is_manual_retry_enabled,
            always_enable_overcapture: value.always_enable_overcapture,
            external_vault_details: value.external_vault_details,
            billing_processor_id: value.billing_processor_id,
            network_tokenization_credentials: value.network_tokenization_credentials,
            payment_method_blocking: value.payment_method_blocking,
        }
    }
}

impl Profile {
    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &common_utils::id_type::ProfileId {
        &self.profile_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &common_utils::id_type::ProfileId {
        &self.id
    }
}

#[cfg(feature = "v1")]
#[derive(Debug)]
pub struct ProfileGeneralUpdate {
    pub profile_name: Option<String>,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub always_request_extended_authorization:
        Option<primitive_wrappers::AlwaysRequestExtendedAuthorization>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub is_l2_l3_enabled: Option<bool>,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: Option<bool>,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub is_click_to_pay_enabled: Option<bool>,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_clear_pan_retries_enabled: Option<bool>,
    pub force_3ds_challenge: Option<bool>,
    pub is_debit_routing_enabled: Option<bool>,
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: Option<bool>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub is_external_vault_enabled: Option<common_enums::ExternalVaultEnabled>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub network_tokenization_credentials: OptionalEncryptableValue,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
}

#[cfg(feature = "v1")]
#[derive(Debug)]
pub enum ProfileUpdate {
    Update(Box<ProfileGeneralUpdate>),
    RoutingAlgorithmUpdate {
        routing_algorithm: Option<serde_json::Value>,
        payout_routing_algorithm: Option<serde_json::Value>,
        three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    },
    DynamicRoutingAlgorithmUpdate {
        dynamic_routing_algorithm: Option<serde_json::Value>,
    },
    ExtendedCardInfoUpdate {
        is_extended_card_info_enabled: bool,
    },
    ConnectorAgnosticMitUpdate {
        is_connector_agnostic_mit_enabled: bool,
    },
    NetworkTokenizationUpdate {
        is_network_tokenization_enabled: bool,
        network_tokenization_credentials: OptionalEncryptableValue,
    },
    CardTestingSecretKeyUpdate {
        card_testing_secret_key: OptionalEncryptableName,
    },
    AcquirerConfigMapUpdate {
        acquirer_config_map: Option<common_types::domain::AcquirerConfigMap>,
    },
}


#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct Profile {
    id: common_utils::id_type::ProfileId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<common_utils::types::Url>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub is_recon_enabled: bool,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub should_collect_cvv_during_payment:
        Option<primitive_wrappers::ShouldCollectCvvDuringPayment>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: bool,
    pub version: common_enums::ApiVersion,
    pub is_network_tokenization_enabled: bool,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_clear_pan_retries_enabled: bool,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    pub revenue_recovery_retry_algorithm_type: Option<common_enums::RevenueRecoveryAlgorithmType>,
    pub revenue_recovery_retry_algorithm_data: Option<RevenueRecoveryAlgorithmData>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub split_txns_enabled: common_enums::SplitTxnsEnabled,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[cfg(feature = "v2")]
pub struct ProfileSetter {
    pub id: common_utils::id_type::ProfileId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<common_utils::types::Url>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub is_recon_enabled: bool,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub should_collect_cvv_during_payment:
        Option<primitive_wrappers::ShouldCollectCvvDuringPayment>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: bool,
    pub is_network_tokenization_enabled: bool,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_clear_pan_retries_enabled: bool,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    pub revenue_recovery_retry_algorithm_type: Option<common_enums::RevenueRecoveryAlgorithmType>,
    pub revenue_recovery_retry_algorithm_data: Option<RevenueRecoveryAlgorithmData>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub split_txns_enabled: common_enums::SplitTxnsEnabled,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[cfg(feature = "v2")]
impl From<ProfileSetter> for Profile {
    fn from(value: ProfileSetter) -> Self {
        Self {
            id: value.id,
            merchant_id: value.merchant_id,
            profile_name: value.profile_name,
            created_at: value.created_at,
            modified_at: value.modified_at,
            return_url: value.return_url,
            enable_payment_response_hash: value.enable_payment_response_hash,
            payment_response_hash_key: value.payment_response_hash_key,
            redirect_to_merchant_with_http_post: value.redirect_to_merchant_with_http_post,
            webhook_details: value.webhook_details,
            metadata: value.metadata,
            is_recon_enabled: value.is_recon_enabled,
            applepay_verified_domains: value.applepay_verified_domains,
            payment_link_config: value.payment_link_config,
            session_expiry: value.session_expiry,
            authentication_connector_details: value.authentication_connector_details,
            payout_link_config: value.payout_link_config,
            is_extended_card_info_enabled: value.is_extended_card_info_enabled,
            extended_card_info_config: value.extended_card_info_config,
            is_connector_agnostic_mit_enabled: value.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: value.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: value
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: value
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: value.outgoing_webhook_custom_http_headers,
            always_collect_billing_details_from_wallet_connector: value
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: value
                .always_collect_shipping_details_from_wallet_connector,
            routing_algorithm_id: value.routing_algorithm_id,
            order_fulfillment_time: value.order_fulfillment_time,
            order_fulfillment_time_origin: value.order_fulfillment_time_origin,
            frm_routing_algorithm_id: value.frm_routing_algorithm_id,
            payout_routing_algorithm_id: value.payout_routing_algorithm_id,
            default_fallback_routing: value.default_fallback_routing,
            should_collect_cvv_during_payment: value.should_collect_cvv_during_payment,
            tax_connector_id: value.tax_connector_id,
            is_tax_connector_enabled: value.is_tax_connector_enabled,
            version: common_types::consts::API_VERSION,
            is_network_tokenization_enabled: value.is_network_tokenization_enabled,
            is_click_to_pay_enabled: value.is_click_to_pay_enabled,
            authentication_product_ids: value.authentication_product_ids,
            three_ds_decision_manager_config: value.three_ds_decision_manager_config,
            card_testing_guard_config: value.card_testing_guard_config,
            card_testing_secret_key: value.card_testing_secret_key,
            is_clear_pan_retries_enabled: value.is_clear_pan_retries_enabled,
            is_debit_routing_enabled: value.is_debit_routing_enabled,
            merchant_business_country: value.merchant_business_country,
            revenue_recovery_retry_algorithm_type: value.revenue_recovery_retry_algorithm_type,
            revenue_recovery_retry_algorithm_data: value.revenue_recovery_retry_algorithm_data,
            is_iframe_redirection_enabled: value.is_iframe_redirection_enabled,
            is_external_vault_enabled: value.is_external_vault_enabled,
            external_vault_connector_details: value.external_vault_connector_details,
            merchant_category_code: value.merchant_category_code,
            merchant_country_code: value.merchant_country_code,
            split_txns_enabled: value.split_txns_enabled,
            billing_processor_id: value.billing_processor_id,
        }
    }
}

impl Profile {
    pub fn get_is_tax_connector_enabled(&self) -> bool {
        let is_tax_connector_enabled = self.is_tax_connector_enabled;
        match &self.tax_connector_id {
            Some(_id) => is_tax_connector_enabled,
            _ => false,
        }
    }

    #[cfg(feature = "v1")]
    pub fn get_order_fulfillment_time(&self) -> Option<i64> {
        self.intent_fulfillment_time
    }

    #[cfg(feature = "v2")]
    pub fn get_order_fulfillment_time(&self) -> Option<i64> {
        self.order_fulfillment_time
    }

    pub fn get_webhook_url_from_profile(&self) -> CustomResult<String, ValidationError> {
        self.webhook_details
            .clone()
            .and_then(|details| details.webhook_url)
            .get_required_value("webhook_details.webhook_url")
            .map(ExposeInterface::expose)
    }

    #[cfg(feature = "v2")]
    pub fn is_external_vault_enabled(&self) -> bool {
        self.is_external_vault_enabled.unwrap_or(false)
    }

    #[cfg(feature = "v2")]
    pub fn is_vault_sdk_enabled(&self) -> bool {
        self.external_vault_connector_details.is_some()
    }

    #[cfg(feature = "v1")]
    pub fn get_acquirer_details_from_network(
        &self,
        network: common_enums::CardNetwork,
    ) -> Option<AcquirerConfig> {
        // iterate over acquirer_config_map and find the acquirer config for the given network
        self.acquirer_config_map
            .as_ref()
            .and_then(|acquirer_config_map| {
                acquirer_config_map
                    .0
                    .iter()
                    .find(|&(_, acquirer_config)| acquirer_config.network == network)
            })
            .map(|(_, acquirer_config)| acquirer_config.clone())
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_routing_algorithm(
        &self,
    ) -> CustomResult<
        Option<api_models::routing::RoutingAlgorithmRef>,
        api_error_response::ApiErrorResponse,
    > {
        self.routing_algorithm
            .clone()
            .map(|val| {
                val.parse_value::<api_models::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef")
            })
            .transpose()
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from merchant account")
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_routing_algorithm_id(
        &self,
    ) -> CustomResult<Option<common_utils::id_type::RoutingId>, api_error_response::ApiErrorResponse>
    {
        Ok(self
            .routing_algorithm
            .clone()
            .map(|val| {
                val.parse_value::<api_models::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef")
            })
            .transpose()
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from business profile")?
            .and_then(|algorithm| algorithm.algorithm_id))
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_routing_algorithm_id(
        &self,
    ) -> CustomResult<Option<common_utils::id_type::RoutingId>, api_error_response::ApiErrorResponse>
    {
        Ok(self.routing_algorithm_id.clone())
    }

    #[cfg(feature = "v1")]
    pub fn get_three_ds_decision_rule_algorithm_id(
        &self,
    ) -> Option<common_utils::id_type::RoutingId> {
        self.three_ds_decision_rule_algorithm
            .clone()
            .map(|val| {
                val.parse_value::<api_models::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef")
            })
            .transpose()
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to deserialize three_ds_decision_rule_algorithm ref from profile",
            )
            .inspect_err(|err| {
                logger::error!(
                    "Error while parsing three_ds_decision_rule_algorithm ref from profile {:?}",
                    err
                )
            })
            .ok()
            .flatten()
            .and_then(|algorithm| algorithm.algorithm_id)
    }

    #[cfg(feature = "v1")]
    pub fn get_payout_routing_algorithm(
        &self,
    ) -> CustomResult<
        Option<api_models::routing::RoutingAlgorithmRef>,
        api_error_response::ApiErrorResponse,
    > {
        self.payout_routing_algorithm
            .clone()
            .map(|val| {
                val.parse_value::<api_models::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef")
            })
            .transpose()
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to deserialize payout routing algorithm ref from merchant account",
            )
    }

    #[cfg(feature = "v1")]
    pub fn get_frm_routing_algorithm(
        &self,
    ) -> CustomResult<
        Option<api_models::routing::RoutingAlgorithmRef>,
        api_error_response::ApiErrorResponse,
    > {
        self.frm_routing_algorithm
            .clone()
            .map(|val| {
                val.parse_value::<api_models::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef")
            })
            .transpose()
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to deserialize frm routing algorithm ref from merchant account",
            )
    }

    pub fn get_configured_payment_webhook_statuses(
        &self,
    ) -> Option<Cow<'_, [common_enums::IntentStatus]>> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.payment_statuses_enabled.as_ref())
            .filter(|statuses_vec| !statuses_vec.is_empty())
            .map(|statuses_vec| Cow::Borrowed(statuses_vec.as_slice()))
    }

    pub fn get_configured_refund_webhook_statuses(
        &self,
    ) -> Option<Cow<'_, [common_enums::RefundStatus]>> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.refund_statuses_enabled.as_ref())
            .filter(|statuses_vec| !statuses_vec.is_empty())
            .map(|statuses_vec| Cow::Borrowed(statuses_vec.as_slice()))
    }

    pub fn get_configured_payout_webhook_statuses(
        &self,
    ) -> Option<Cow<'_, [common_enums::PayoutStatus]>> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.payout_statuses_enabled.as_ref())
            .filter(|statuses_vec| !statuses_vec.is_empty())
            .map(|statuses_vec| Cow::Borrowed(statuses_vec.as_slice()))
    }

    pub fn get_billing_processor_id(
        &self,
    ) -> CustomResult<
        common_utils::id_type::MerchantConnectorAccountId,
        api_error_response::ApiErrorResponse,
    > {
        self.billing_processor_id
            .to_owned()
            .ok_or(error_stack::report!(
                api_error_response::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_processor_id"
                }
            ))
    }
}

#[cfg(feature = "v2")]
#[derive(Debug)]
pub struct ProfileGeneralUpdate {
    pub profile_name: Option<String>,
    pub return_url: Option<common_utils::types::Url>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,
    pub payout_link_config: Option<BusinessPayoutLinkConfig>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub is_network_tokenization_enabled: Option<bool>,
    pub is_click_to_pay_enabled: Option<bool>,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_debit_routing_enabled: Option<bool>,
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub revenue_recovery_retry_algorithm_type: Option<common_enums::RevenueRecoveryAlgorithmType>,
    pub split_txns_enabled: Option<common_enums::SplitTxnsEnabled>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[cfg(feature = "v2")]
#[derive(Debug)]
pub enum ProfileUpdate {
    Update(Box<ProfileGeneralUpdate>),
    RoutingAlgorithmUpdate {
        routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
        payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    },
    DefaultRoutingFallbackUpdate {
        default_fallback_routing: Option<pii::SecretSerdeValue>,
    },
    ExtendedCardInfoUpdate {
        is_extended_card_info_enabled: bool,
    },
    ConnectorAgnosticMitUpdate {
        is_connector_agnostic_mit_enabled: bool,
    },
    NetworkTokenizationUpdate {
        is_network_tokenization_enabled: bool,
    },
    CollectCvvDuringPaymentUpdate {
        should_collect_cvv_during_payment: primitive_wrappers::ShouldCollectCvvDuringPayment,
    },
    DecisionManagerRecordUpdate {
        three_ds_decision_manager_config: common_types::payments::DecisionManagerRecord,
    },
    CardTestingSecretKeyUpdate {
        card_testing_secret_key: OptionalEncryptableName,
    },
    RevenueRecoveryAlgorithmUpdate {
        revenue_recovery_retry_algorithm_type: common_enums::RevenueRecoveryAlgorithmType,
        revenue_recovery_retry_algorithm_data: Option<RevenueRecoveryAlgorithmData>,
    },
}

#[async_trait::async_trait]
pub trait ProfileInterface
{
    type Error;
    async fn insert_business_profile(
        &self,
        merchant_key_store: &MerchantKeyStore,
        business_profile: Profile,
    ) -> CustomResult<Profile, Self::Error>;

    async fn find_business_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<Profile, Self::Error>;

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<Profile, Self::Error>;

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Profile, Self::Error>;

    async fn update_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        current_state: Profile,
        profile_update: ProfileUpdate,
    ) -> CustomResult<Profile, Self::Error>;

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    async fn list_profile_by_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<Profile>, Self::Error>;
}
