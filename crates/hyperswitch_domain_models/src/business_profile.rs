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
#[cfg(feature = "v2")]
use diesel_models::business_profile::RevenueRecoveryAlgorithmData;
use diesel_models::business_profile::{
    self as storage_types, AuthenticationConnectorDetails, BusinessPaymentLinkConfig,
    BusinessPayoutLinkConfig, CardTestingGuardConfig, ExternalVaultConnectorDetails,
    ProfileUpdateInternal, WebhookDetails,
};
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    behaviour::Conversion,
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
    },
    CardTestingSecretKeyUpdate {
        card_testing_secret_key: OptionalEncryptableName,
    },
    AcquirerConfigMapUpdate {
        acquirer_config_map: Option<common_types::domain::AcquirerConfigMap>,
    },
}

#[cfg(feature = "v1")]
impl From<ProfileUpdate> for ProfileUpdateInternal {
    fn from(profile_update: ProfileUpdate) -> Self {
        let now = date_time::now();

        match profile_update {
            ProfileUpdate::Update(update) => {
                let ProfileGeneralUpdate {
                    profile_name,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    routing_algorithm,
                    intent_fulfillment_time,
                    frm_routing_algorithm,
                    payout_routing_algorithm,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    extended_card_info_config,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    is_connector_agnostic_mit_enabled,
                    outgoing_webhook_custom_http_headers,
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    tax_connector_id,
                    is_tax_connector_enabled,
                    is_l2_l3_enabled,
                    dynamic_routing_algorithm,
                    is_network_tokenization_enabled,
                    is_auto_retries_enabled,
                    max_auto_retries_enabled,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    card_testing_guard_config,
                    card_testing_secret_key,
                    is_clear_pan_retries_enabled,
                    force_3ds_challenge,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    is_iframe_redirection_enabled,
                    is_pre_network_tokenization_enabled,
                    merchant_category_code,
                    merchant_country_code,
                    dispute_polling_interval,
                    always_request_extended_authorization,
                    is_manual_retry_enabled,
                    always_enable_overcapture,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    billing_processor_id,
                } = *update;

                let is_external_vault_enabled = match is_external_vault_enabled {
                    Some(external_vault_mode) => match external_vault_mode {
                        common_enums::ExternalVaultEnabled::Enable => Some(true),
                        common_enums::ExternalVaultEnabled::Skip => Some(false),
                    },
                    None => Some(false),
                };

                Self {
                    profile_name,
                    modified_at: now,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    routing_algorithm,
                    intent_fulfillment_time,
                    frm_routing_algorithm,
                    payout_routing_algorithm,
                    is_recon_enabled: None,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    is_extended_card_info_enabled: None,
                    extended_card_info_config,
                    is_connector_agnostic_mit_enabled,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                        .map(Encryption::from),
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    tax_connector_id,
                    is_tax_connector_enabled,
                    is_l2_l3_enabled,
                    dynamic_routing_algorithm,
                    is_network_tokenization_enabled,
                    is_auto_retries_enabled,
                    max_auto_retries_enabled,
                    always_request_extended_authorization,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    card_testing_guard_config,
                    card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                    is_clear_pan_retries_enabled,
                    force_3ds_challenge,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    is_iframe_redirection_enabled,
                    is_pre_network_tokenization_enabled,
                    three_ds_decision_rule_algorithm: None,
                    acquirer_config_map: None,
                    merchant_category_code,
                    merchant_country_code,
                    dispute_polling_interval,
                    is_manual_retry_enabled,
                    always_enable_overcapture,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    billing_processor_id,
                }
            }
            ProfileUpdate::RoutingAlgorithmUpdate {
                routing_algorithm,
                payout_routing_algorithm,
                three_ds_decision_rule_algorithm,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
            ProfileUpdate::DynamicRoutingAlgorithmUpdate {
                dynamic_routing_algorithm,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm: None,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
            ProfileUpdate::ExtendedCardInfoUpdate {
                is_extended_card_info_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: Some(is_extended_card_info_enabled),
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm: None,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
            ProfileUpdate::ConnectorAgnosticMitUpdate {
                is_connector_agnostic_mit_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: Some(is_connector_agnostic_mit_enabled),
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm: None,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
            ProfileUpdate::NetworkTokenizationUpdate {
                is_network_tokenization_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: Some(is_network_tokenization_enabled),
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm: None,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
            ProfileUpdate::CardTestingSecretKeyUpdate {
                card_testing_secret_key,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm: None,
                acquirer_config_map: None,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
            ProfileUpdate::AcquirerConfigMapUpdate {
                acquirer_config_map,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                routing_algorithm: None,
                intent_fulfillment_time: None,
                frm_routing_algorithm: None,
                payout_routing_algorithm: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                dynamic_routing_algorithm: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                always_request_extended_authorization: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                force_3ds_challenge: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                is_iframe_redirection_enabled: None,
                is_pre_network_tokenization_enabled: None,
                three_ds_decision_rule_algorithm: None,
                acquirer_config_map,
                merchant_category_code: None,
                merchant_country_code: None,
                dispute_polling_interval: None,
                is_manual_retry_enabled: None,
                always_enable_overcapture: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                billing_processor_id: None,
                is_l2_l3_enabled: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for Profile {
    type DstType = diesel_models::business_profile::Profile;
    type NewDstType = diesel_models::business_profile::ProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let (is_external_vault_enabled, external_vault_connector_details) =
            self.external_vault_details.into();

        Ok(diesel_models::business_profile::Profile {
            profile_id: self.profile_id.clone(),
            id: Some(self.profile_id),
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            is_l2_l3_enabled: Some(self.is_l2_l3_enabled),
            version: self.version,
            dynamic_routing_algorithm: self.dynamic_routing_algorithm,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: Some(self.is_auto_retries_enabled),
            max_auto_retries_enabled: self.max_auto_retries_enabled,
            always_request_extended_authorization: self.always_request_extended_authorization,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(|name| name.into()),
            is_clear_pan_retries_enabled: self.is_clear_pan_retries_enabled,
            force_3ds_challenge: Some(self.force_3ds_challenge),
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: Some(self.is_pre_network_tokenization_enabled),
            three_ds_decision_rule_algorithm: self.three_ds_decision_rule_algorithm,
            acquirer_config_map: self.acquirer_config_map,
            merchant_category_code: self.merchant_category_code,
            merchant_country_code: self.merchant_country_code,
            dispute_polling_interval: self.dispute_polling_interval,
            is_manual_retry_enabled: self.is_manual_retry_enabled,
            always_enable_overcapture: self.always_enable_overcapture,
            is_external_vault_enabled,
            external_vault_connector_details,
            billing_processor_id: self.billing_processor_id,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // Decrypt encrypted fields first
        let (outgoing_webhook_custom_http_headers, card_testing_secret_key) = async {
            let outgoing_webhook_custom_http_headers = item
                .outgoing_webhook_custom_http_headers
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let card_testing_secret_key = item
                .card_testing_secret_key
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            Ok::<_, error_stack::Report<common_utils::errors::CryptoError>>((
                outgoing_webhook_custom_http_headers,
                card_testing_secret_key,
            ))
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })?;

        let external_vault_details = ExternalVaultDetails::try_from((
            item.is_external_vault_enabled,
            item.external_vault_connector_details,
        ))?;

        // Construct the domain type
        Ok(Self {
            profile_id: item.profile_id,
            merchant_id: item.merchant_id,
            profile_name: item.profile_name,
            created_at: item.created_at,
            modified_at: item.modified_at,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            webhook_details: item.webhook_details,
            metadata: item.metadata,
            routing_algorithm: item.routing_algorithm,
            intent_fulfillment_time: item.intent_fulfillment_time,
            frm_routing_algorithm: item.frm_routing_algorithm,
            payout_routing_algorithm: item.payout_routing_algorithm,
            is_recon_enabled: item.is_recon_enabled,
            applepay_verified_domains: item.applepay_verified_domains,
            payment_link_config: item.payment_link_config,
            session_expiry: item.session_expiry,
            authentication_connector_details: item.authentication_connector_details,
            payout_link_config: item.payout_link_config,
            is_extended_card_info_enabled: item.is_extended_card_info_enabled,
            extended_card_info_config: item.extended_card_info_config,
            is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: item
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: item
                .collect_billing_details_from_wallet_connector,
            always_collect_billing_details_from_wallet_connector: item
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: item
                .always_collect_shipping_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers,
            tax_connector_id: item.tax_connector_id,
            is_tax_connector_enabled: item.is_tax_connector_enabled.unwrap_or(false),
            is_l2_l3_enabled: item.is_l2_l3_enabled.unwrap_or(false),
            version: item.version,
            dynamic_routing_algorithm: item.dynamic_routing_algorithm,
            is_network_tokenization_enabled: item.is_network_tokenization_enabled,
            is_auto_retries_enabled: item.is_auto_retries_enabled.unwrap_or(false),
            max_auto_retries_enabled: item.max_auto_retries_enabled,
            always_request_extended_authorization: item.always_request_extended_authorization,
            is_click_to_pay_enabled: item.is_click_to_pay_enabled,
            authentication_product_ids: item.authentication_product_ids,
            card_testing_guard_config: item.card_testing_guard_config,
            card_testing_secret_key,
            is_clear_pan_retries_enabled: item.is_clear_pan_retries_enabled,
            force_3ds_challenge: item.force_3ds_challenge.unwrap_or_default(),
            is_debit_routing_enabled: item.is_debit_routing_enabled,
            merchant_business_country: item.merchant_business_country,
            is_iframe_redirection_enabled: item.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: item
                .is_pre_network_tokenization_enabled
                .unwrap_or(false),
            three_ds_decision_rule_algorithm: item.three_ds_decision_rule_algorithm,
            acquirer_config_map: item.acquirer_config_map,
            merchant_category_code: item.merchant_category_code,
            merchant_country_code: item.merchant_country_code,
            dispute_polling_interval: item.dispute_polling_interval,
            is_manual_retry_enabled: item.is_manual_retry_enabled,
            always_enable_overcapture: item.always_enable_overcapture,
            external_vault_details,
            billing_processor_id: item.billing_processor_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let (is_external_vault_enabled, external_vault_connector_details) =
            self.external_vault_details.into();

        Ok(diesel_models::business_profile::ProfileNew {
            profile_id: self.profile_id.clone(),
            id: Some(self.profile_id),
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            is_l2_l3_enabled: Some(self.is_l2_l3_enabled),
            version: self.version,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: Some(self.is_auto_retries_enabled),
            max_auto_retries_enabled: self.max_auto_retries_enabled,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(Encryption::from),
            is_clear_pan_retries_enabled: self.is_clear_pan_retries_enabled,
            force_3ds_challenge: Some(self.force_3ds_challenge),
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_pre_network_tokenization_enabled: Some(self.is_pre_network_tokenization_enabled),
            merchant_category_code: self.merchant_category_code,
            merchant_country_code: self.merchant_country_code,
            dispute_polling_interval: self.dispute_polling_interval,
            is_manual_retry_enabled: self.is_manual_retry_enabled,
            is_external_vault_enabled,
            external_vault_connector_details,
            billing_processor_id: self.billing_processor_id,
        })
    }
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

    pub fn get_payment_webhook_statuses(&self) -> Cow<'_, [common_enums::IntentStatus]> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.payment_statuses_enabled.as_ref())
            .filter(|statuses_vec| !statuses_vec.is_empty())
            .map(|statuses_vec| Cow::Borrowed(statuses_vec.as_slice()))
            .unwrap_or_else(|| {
                Cow::Borrowed(common_types::consts::DEFAULT_PAYMENT_WEBHOOK_TRIGGER_STATUSES)
            })
    }

    pub fn get_refund_webhook_statuses(&self) -> Cow<'_, [common_enums::RefundStatus]> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.refund_statuses_enabled.as_ref())
            .filter(|statuses_vec| !statuses_vec.is_empty())
            .map(|statuses_vec| Cow::Borrowed(statuses_vec.as_slice()))
            .unwrap_or_else(|| {
                Cow::Borrowed(common_types::consts::DEFAULT_REFUND_WEBHOOK_TRIGGER_STATUSES)
            })
    }

    pub fn get_payout_webhook_statuses(&self) -> Cow<'_, [common_enums::PayoutStatus]> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.payout_statuses_enabled.as_ref())
            .filter(|statuses_vec| !statuses_vec.is_empty())
            .map(|statuses_vec| Cow::Borrowed(statuses_vec.as_slice()))
            .unwrap_or_else(|| {
                Cow::Borrowed(common_types::consts::DEFAULT_PAYOUT_WEBHOOK_TRIGGER_STATUSES)
            })
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

#[cfg(feature = "v2")]
impl From<ProfileUpdate> for ProfileUpdateInternal {
    fn from(profile_update: ProfileUpdate) -> Self {
        let now = date_time::now();

        match profile_update {
            ProfileUpdate::Update(update) => {
                let ProfileGeneralUpdate {
                    profile_name,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    extended_card_info_config,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    is_connector_agnostic_mit_enabled,
                    outgoing_webhook_custom_http_headers,
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    order_fulfillment_time,
                    order_fulfillment_time_origin,
                    is_network_tokenization_enabled,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    three_ds_decision_manager_config,
                    card_testing_guard_config,
                    card_testing_secret_key,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    is_iframe_redirection_enabled,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    merchant_category_code,
                    merchant_country_code,
                    revenue_recovery_retry_algorithm_type,
                    split_txns_enabled,
                    billing_processor_id,
                } = *update;
                Self {
                    profile_name,
                    modified_at: now,
                    return_url,
                    enable_payment_response_hash,
                    payment_response_hash_key,
                    redirect_to_merchant_with_http_post,
                    webhook_details,
                    metadata,
                    is_recon_enabled: None,
                    applepay_verified_domains,
                    payment_link_config,
                    session_expiry,
                    authentication_connector_details,
                    payout_link_config,
                    is_extended_card_info_enabled: None,
                    extended_card_info_config,
                    is_connector_agnostic_mit_enabled,
                    use_billing_as_payment_method_billing,
                    collect_shipping_details_from_wallet_connector,
                    collect_billing_details_from_wallet_connector,
                    outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                        .map(Encryption::from),
                    routing_algorithm_id: None,
                    always_collect_billing_details_from_wallet_connector,
                    always_collect_shipping_details_from_wallet_connector,
                    order_fulfillment_time,
                    order_fulfillment_time_origin,
                    frm_routing_algorithm_id: None,
                    payout_routing_algorithm_id: None,
                    default_fallback_routing: None,
                    should_collect_cvv_during_payment: None,
                    tax_connector_id: None,
                    is_tax_connector_enabled: None,
                    is_l2_l3_enabled: None,
                    is_network_tokenization_enabled,
                    is_auto_retries_enabled: None,
                    max_auto_retries_enabled: None,
                    is_click_to_pay_enabled,
                    authentication_product_ids,
                    three_ds_decision_manager_config,
                    card_testing_guard_config,
                    card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                    is_clear_pan_retries_enabled: None,
                    is_debit_routing_enabled,
                    merchant_business_country,
                    revenue_recovery_retry_algorithm_type,
                    revenue_recovery_retry_algorithm_data: None,
                    is_iframe_redirection_enabled,
                    is_external_vault_enabled,
                    external_vault_connector_details,
                    merchant_category_code,
                    merchant_country_code,
                    split_txns_enabled,
                    billing_processor_id,
                }
            }
            ProfileUpdate::RoutingAlgorithmUpdate {
                routing_algorithm_id,
                payout_routing_algorithm_id,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                routing_algorithm_id,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                payout_routing_algorithm_id,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_l2_l3_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::ExtendedCardInfoUpdate {
                is_extended_card_info_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: Some(is_extended_card_info_enabled),
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_l2_l3_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::ConnectorAgnosticMitUpdate {
                is_connector_agnostic_mit_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                is_l2_l3_enabled: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: Some(is_connector_agnostic_mit_enabled),
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::DefaultRoutingFallbackUpdate {
                default_fallback_routing,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                is_l2_l3_enabled: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::NetworkTokenizationUpdate {
                is_network_tokenization_enabled,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                is_l2_l3_enabled: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: Some(is_network_tokenization_enabled),
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::CollectCvvDuringPaymentUpdate {
                should_collect_cvv_during_payment,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: Some(should_collect_cvv_during_payment),
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                is_l2_l3_enabled: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::DecisionManagerRecordUpdate {
                three_ds_decision_manager_config,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: Some(three_ds_decision_manager_config),
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_l2_l3_enabled: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::CardTestingSecretKeyUpdate {
                card_testing_secret_key,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: card_testing_secret_key.map(Encryption::from),
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                is_l2_l3_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: None,
                revenue_recovery_retry_algorithm_data: None,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
            ProfileUpdate::RevenueRecoveryAlgorithmUpdate {
                revenue_recovery_retry_algorithm_type,
                revenue_recovery_retry_algorithm_data,
            } => Self {
                profile_name: None,
                modified_at: now,
                return_url: None,
                enable_payment_response_hash: None,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: None,
                webhook_details: None,
                metadata: None,
                is_recon_enabled: None,
                applepay_verified_domains: None,
                payment_link_config: None,
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                always_collect_billing_details_from_wallet_connector: None,
                always_collect_shipping_details_from_wallet_connector: None,
                routing_algorithm_id: None,
                is_l2_l3_enabled: None,
                payout_routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                default_fallback_routing: None,
                should_collect_cvv_during_payment: None,
                tax_connector_id: None,
                is_tax_connector_enabled: None,
                is_network_tokenization_enabled: None,
                is_auto_retries_enabled: None,
                max_auto_retries_enabled: None,
                is_click_to_pay_enabled: None,
                authentication_product_ids: None,
                three_ds_decision_manager_config: None,
                card_testing_guard_config: None,
                card_testing_secret_key: None,
                is_clear_pan_retries_enabled: None,
                is_debit_routing_enabled: None,
                merchant_business_country: None,
                revenue_recovery_retry_algorithm_type: Some(revenue_recovery_retry_algorithm_type),
                revenue_recovery_retry_algorithm_data,
                is_iframe_redirection_enabled: None,
                is_external_vault_enabled: None,
                external_vault_connector_details: None,
                merchant_category_code: None,
                merchant_country_code: None,
                split_txns_enabled: None,
                billing_processor_id: None,
            },
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for Profile {
    type DstType = diesel_models::business_profile::Profile;
    type NewDstType = diesel_models::business_profile::ProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::business_profile::Profile {
            id: self.id,
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            routing_algorithm_id: self.routing_algorithm_id,
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            payout_routing_algorithm_id: self.payout_routing_algorithm_id,
            order_fulfillment_time: self.order_fulfillment_time,
            order_fulfillment_time_origin: self.order_fulfillment_time_origin,
            frm_routing_algorithm_id: self.frm_routing_algorithm_id,
            default_fallback_routing: self.default_fallback_routing,
            should_collect_cvv_during_payment: self.should_collect_cvv_during_payment,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            version: self.version,
            dynamic_routing_algorithm: None,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: None,
            max_auto_retries_enabled: None,
            always_request_extended_authorization: None,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            three_ds_decision_manager_config: self.three_ds_decision_manager_config,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(|name| name.into()),
            is_clear_pan_retries_enabled: self.is_clear_pan_retries_enabled,
            force_3ds_challenge: None,
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            revenue_recovery_retry_algorithm_type: self.revenue_recovery_retry_algorithm_type,
            revenue_recovery_retry_algorithm_data: self.revenue_recovery_retry_algorithm_data,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_external_vault_enabled: self.is_external_vault_enabled,
            external_vault_connector_details: self.external_vault_connector_details,
            three_ds_decision_rule_algorithm: None,
            acquirer_config_map: None,
            merchant_category_code: self.merchant_category_code,
            merchant_country_code: self.merchant_country_code,
            dispute_polling_interval: None,
            split_txns_enabled: Some(self.split_txns_enabled),
            is_manual_retry_enabled: None,
            is_l2_l3_enabled: None,
            always_enable_overcapture: None,
            billing_processor_id: self.billing_processor_id,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                id: item.id,
                merchant_id: item.merchant_id,
                profile_name: item.profile_name,
                created_at: item.created_at,
                modified_at: item.modified_at,
                return_url: item.return_url,
                enable_payment_response_hash: item.enable_payment_response_hash,
                payment_response_hash_key: item.payment_response_hash_key,
                redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
                webhook_details: item.webhook_details,
                metadata: item.metadata,
                is_recon_enabled: item.is_recon_enabled,
                applepay_verified_domains: item.applepay_verified_domains,
                payment_link_config: item.payment_link_config,
                session_expiry: item.session_expiry,
                authentication_connector_details: item.authentication_connector_details,
                payout_link_config: item.payout_link_config,
                is_extended_card_info_enabled: item.is_extended_card_info_enabled,
                extended_card_info_config: item.extended_card_info_config,
                is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
                use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
                collect_shipping_details_from_wallet_connector: item
                    .collect_shipping_details_from_wallet_connector,
                collect_billing_details_from_wallet_connector: item
                    .collect_billing_details_from_wallet_connector,
                outgoing_webhook_custom_http_headers: item
                    .outgoing_webhook_custom_http_headers
                    .async_lift(|inner| async {
                        crypto_operation(
                            state,
                            type_name!(Self::DstType),
                            CryptoOperation::DecryptOptional(inner),
                            key_manager_identifier.clone(),
                            key.peek(),
                        )
                        .await
                        .and_then(|val| val.try_into_optionaloperation())
                    })
                    .await?,
                routing_algorithm_id: item.routing_algorithm_id,
                always_collect_billing_details_from_wallet_connector: item
                    .always_collect_billing_details_from_wallet_connector,
                always_collect_shipping_details_from_wallet_connector: item
                    .always_collect_shipping_details_from_wallet_connector,
                order_fulfillment_time: item.order_fulfillment_time,
                order_fulfillment_time_origin: item.order_fulfillment_time_origin,
                frm_routing_algorithm_id: item.frm_routing_algorithm_id,
                payout_routing_algorithm_id: item.payout_routing_algorithm_id,
                default_fallback_routing: item.default_fallback_routing,
                should_collect_cvv_during_payment: item.should_collect_cvv_during_payment,
                tax_connector_id: item.tax_connector_id,
                is_tax_connector_enabled: item.is_tax_connector_enabled.unwrap_or(false),
                version: item.version,
                is_network_tokenization_enabled: item.is_network_tokenization_enabled,
                is_click_to_pay_enabled: item.is_click_to_pay_enabled,
                authentication_product_ids: item.authentication_product_ids,
                three_ds_decision_manager_config: item.three_ds_decision_manager_config,
                card_testing_guard_config: item.card_testing_guard_config,
                card_testing_secret_key: match item.card_testing_secret_key {
                    Some(encrypted_value) => crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(Some(encrypted_value)),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                    .unwrap_or_default(),
                    None => None,
                },
                is_clear_pan_retries_enabled: item.is_clear_pan_retries_enabled,
                is_debit_routing_enabled: item.is_debit_routing_enabled,
                merchant_business_country: item.merchant_business_country,
                revenue_recovery_retry_algorithm_type: item.revenue_recovery_retry_algorithm_type,
                revenue_recovery_retry_algorithm_data: item.revenue_recovery_retry_algorithm_data,
                is_iframe_redirection_enabled: item.is_iframe_redirection_enabled,
                is_external_vault_enabled: item.is_external_vault_enabled,
                external_vault_connector_details: item.external_vault_connector_details,
                merchant_category_code: item.merchant_category_code,
                merchant_country_code: item.merchant_country_code,
                split_txns_enabled: item.split_txns_enabled.unwrap_or_default(),
                billing_processor_id: item.billing_processor_id,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::business_profile::ProfileNew {
            id: self.id,
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            created_at: self.created_at,
            modified_at: self.modified_at,
            return_url: self.return_url,
            enable_payment_response_hash: self.enable_payment_response_hash,
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self.redirect_to_merchant_with_http_post,
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            is_recon_enabled: self.is_recon_enabled,
            applepay_verified_domains: self.applepay_verified_domains,
            payment_link_config: self.payment_link_config,
            session_expiry: self.session_expiry,
            authentication_connector_details: self.authentication_connector_details,
            payout_link_config: self.payout_link_config,
            is_extended_card_info_enabled: self.is_extended_card_info_enabled,
            extended_card_info_config: self.extended_card_info_config,
            is_connector_agnostic_mit_enabled: self.is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing: self.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: self
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: self
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: self
                .outgoing_webhook_custom_http_headers
                .map(Encryption::from),
            routing_algorithm_id: self.routing_algorithm_id,
            always_collect_billing_details_from_wallet_connector: self
                .always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector: self
                .always_collect_shipping_details_from_wallet_connector,
            order_fulfillment_time: self.order_fulfillment_time,
            order_fulfillment_time_origin: self.order_fulfillment_time_origin,
            frm_routing_algorithm_id: self.frm_routing_algorithm_id,
            payout_routing_algorithm_id: self.payout_routing_algorithm_id,
            default_fallback_routing: self.default_fallback_routing,
            should_collect_cvv_during_payment: self.should_collect_cvv_during_payment,
            tax_connector_id: self.tax_connector_id,
            is_tax_connector_enabled: Some(self.is_tax_connector_enabled),
            version: self.version,
            is_network_tokenization_enabled: self.is_network_tokenization_enabled,
            is_auto_retries_enabled: None,
            max_auto_retries_enabled: None,
            is_click_to_pay_enabled: self.is_click_to_pay_enabled,
            authentication_product_ids: self.authentication_product_ids,
            three_ds_decision_manager_config: self.three_ds_decision_manager_config,
            card_testing_guard_config: self.card_testing_guard_config,
            card_testing_secret_key: self.card_testing_secret_key.map(Encryption::from),
            is_clear_pan_retries_enabled: Some(self.is_clear_pan_retries_enabled),
            is_debit_routing_enabled: self.is_debit_routing_enabled,
            merchant_business_country: self.merchant_business_country,
            revenue_recovery_retry_algorithm_type: self.revenue_recovery_retry_algorithm_type,
            revenue_recovery_retry_algorithm_data: self.revenue_recovery_retry_algorithm_data,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            is_external_vault_enabled: self.is_external_vault_enabled,
            external_vault_connector_details: self.external_vault_connector_details,
            merchant_category_code: self.merchant_category_code,
            is_l2_l3_enabled: None,
            merchant_country_code: self.merchant_country_code,
            split_txns_enabled: Some(self.split_txns_enabled),
            billing_processor_id: self.billing_processor_id,
        })
    }
}

#[async_trait::async_trait]
pub trait ProfileInterface
where
    Profile: Conversion<DstType = storage_types::Profile, NewDstType = storage_types::ProfileNew>,
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
