use std::{borrow::Cow, collections::HashSet};

use common_enums::enums as api_enums;
use common_types::{domain::AcquirerConfig, primitive_wrappers};
use common_utils::{
    crypto::{OptionalEncryptableName, OptionalEncryptableValue},
    errors::{CustomResult, ValidationError},
    ext_traits::{OptionExt, ValueExt},
    pii,
};
#[cfg(feature = "v2")]
use diesel_models::business_profile::RevenueRecoveryAlgorithmData;
use diesel_models::business_profile::{
    self as storage_types, AuthenticationConnectorDetails, BusinessPaymentLinkConfig,
    BusinessPayoutLinkConfig, CardTestingGuardConfig, ExternalVaultConnectorDetails,
    PaymentMethodBlockingConfig, SurchargeConnectorDetails,
};
use error_stack::ResultExt;
use hyperswitch_masking::{ExposeInterface, Secret};
use router_env::logger;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MultipleWebhookDetail {
    pub webhook_endpoint_id: common_utils::id_type::WebhookEndpointId,
    pub webhook_url: Secret<String>,
    pub events: HashSet<common_enums::EventType>,
    pub status: common_enums::OutgoingWebhookEndpointStatus,
    pub is_legacy_url: bool,
}

impl ForeignFrom<storage_types::MultipleWebhookDetail> for MultipleWebhookDetail {
    fn foreign_from(item: storage_types::MultipleWebhookDetail) -> Self {
        Self {
            webhook_endpoint_id: item.webhook_endpoint_id,
            webhook_url: item.webhook_url,
            events: item.events,
            status: item.status,
            is_legacy_url: item.is_legacy_url,
        }
    }
}

impl ForeignFrom<MultipleWebhookDetail> for storage_types::MultipleWebhookDetail {
    fn foreign_from(item: MultipleWebhookDetail) -> Self {
        Self {
            webhook_endpoint_id: item.webhook_endpoint_id,
            webhook_url: item.webhook_url,
            events: item.events,
            status: item.status,
            is_legacy_url: item.is_legacy_url,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WebhookUrls(pub Vec<MultipleWebhookDetail>);

impl From<Vec<MultipleWebhookDetail>> for WebhookUrls {
    fn from(item: Vec<MultipleWebhookDetail>) -> Self {
        Self(item)
    }
}

impl WebhookUrls {
    pub fn get_legacy_url(&self) -> Option<Secret<String>> {
        self.0
            .iter()
            .find(|webhook_detail| webhook_detail.is_legacy_url)
            .map(|webhook_detail| webhook_detail.webhook_url.clone())
    }

    pub fn update_legacy_webhook_url(&mut self, new_url: Secret<String>) {
        if let Some(legacy_webhook) = self
            .0
            .iter_mut()
            .find(|webhook_detail| webhook_detail.is_legacy_url)
        {
            legacy_webhook.webhook_url = new_url;
        }
    }

    pub fn get_multiple_webhook_urls(
        legacy_url: Option<Secret<String>>,
        multiple_urls: Option<Vec<storage_types::MultipleWebhookDetail>>,
    ) -> Self {
        let mut urls = Vec::new();
        let mut processed_endpoint_ids = HashSet::new();
        let existing_legacy_entry = multiple_urls
            .as_ref()
            .and_then(|list| list.iter().find(|detail| detail.is_legacy_url));

        if let Some(legacy_url_value) = legacy_url {
            let webhook_endpoint_id = existing_legacy_entry
                .map(|entry| entry.webhook_endpoint_id.clone())
                .unwrap_or_else(common_utils::generate_webhook_endpoint_id_of_default_length);

            if processed_endpoint_ids.insert(webhook_endpoint_id.clone()) {
                urls.push(MultipleWebhookDetail {
                    webhook_endpoint_id,
                    webhook_url: legacy_url_value,
                    events: existing_legacy_entry
                        .map(|entry| entry.events.clone())
                        .unwrap_or_else(|| common_enums::EventType::iter().collect()),
                    status: existing_legacy_entry
                        .map(|entry| entry.status)
                        .unwrap_or(common_enums::OutgoingWebhookEndpointStatus::Active),
                    is_legacy_url: true,
                });
            }
        }

        if let Some(multiple_urls_list) = multiple_urls {
            for detail in multiple_urls_list {
                if detail.is_legacy_url {
                    continue;
                }
                if processed_endpoint_ids.insert(detail.webhook_endpoint_id.clone()) {
                    urls.push(detail.foreign_into());
                }
            }
        }
        Self(urls)
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WebhookDetails {
    pub webhook_version: Option<String>,
    pub webhook_username: Option<String>,
    pub webhook_password: Option<Secret<String>>,
    pub payment_created_enabled: Option<bool>,
    pub payment_succeeded_enabled: Option<bool>,
    pub payment_failed_enabled: Option<bool>,
    pub payment_statuses_enabled: Option<Vec<common_enums::IntentStatus>>,
    pub refund_statuses_enabled: Option<Vec<common_enums::RefundStatus>>,
    pub payout_statuses_enabled: Option<Vec<common_enums::PayoutStatus>>,
    pub multiple_webhooks_list: Option<WebhookUrls>,
}

impl ForeignFrom<storage_types::WebhookDetails> for WebhookDetails {
    fn foreign_from(item: storage_types::WebhookDetails) -> Self {
        let webhook_urls =
            WebhookUrls::get_multiple_webhook_urls(item.webhook_url, item.multiple_webhooks_list);

        Self {
            webhook_version: item.webhook_version,
            webhook_username: item.webhook_username,
            webhook_password: item.webhook_password,
            payment_created_enabled: item.payment_created_enabled,
            payment_succeeded_enabled: item.payment_succeeded_enabled,
            payment_failed_enabled: item.payment_failed_enabled,
            payment_statuses_enabled: item.payment_statuses_enabled,
            refund_statuses_enabled: item.refund_statuses_enabled,
            payout_statuses_enabled: item.payout_statuses_enabled,
            multiple_webhooks_list: Some(webhook_urls),
        }
    }
}

impl WebhookDetails {
    pub fn merge(self, other: Self) -> Self {
        Self {
            webhook_version: other.webhook_version.or(self.webhook_version),
            webhook_username: other.webhook_username.or(self.webhook_username),
            webhook_password: other.webhook_password.or(self.webhook_password),
            payment_created_enabled: other
                .payment_created_enabled
                .or(self.payment_created_enabled),
            payment_succeeded_enabled: other
                .payment_succeeded_enabled
                .or(self.payment_succeeded_enabled),
            payment_failed_enabled: other.payment_failed_enabled.or(self.payment_failed_enabled),
            payment_statuses_enabled: other
                .payment_statuses_enabled
                .or(self.payment_statuses_enabled),
            refund_statuses_enabled: other
                .refund_statuses_enabled
                .or(self.refund_statuses_enabled),
            payout_statuses_enabled: other
                .payout_statuses_enabled
                .or(self.payout_statuses_enabled),
            multiple_webhooks_list: other.multiple_webhooks_list.or(self.multiple_webhooks_list),
        }
    }

    pub fn update_from_api(
        existing: Option<Self>,
        api_webhook: api_models::admin::WebhookDetails,
    ) -> Self {
        let mut existing_webhook_urls = existing
            .as_ref()
            .and_then(|d| d.multiple_webhooks_list.clone())
            .unwrap_or_else(|| WebhookUrls::get_multiple_webhook_urls(None, None));

        if let Some(new_url) = api_webhook.webhook_url {
            existing_webhook_urls.update_legacy_webhook_url(new_url);
        }

        let api_webhook_as_domain = Self {
            webhook_version: api_webhook.webhook_version,
            webhook_username: api_webhook.webhook_username,
            webhook_password: api_webhook.webhook_password,
            payment_created_enabled: api_webhook.payment_created_enabled,
            payment_failed_enabled: api_webhook.payment_failed_enabled,
            payment_succeeded_enabled: api_webhook.payment_succeeded_enabled,
            payment_statuses_enabled: api_webhook.payment_statuses_enabled,
            refund_statuses_enabled: api_webhook.refund_statuses_enabled,
            payout_statuses_enabled: api_webhook.payout_statuses_enabled,
            multiple_webhooks_list: Some(existing_webhook_urls),
        };

        match existing {
            Some(existing_details) => existing_details.merge(api_webhook_as_domain),
            None => api_webhook_as_domain,
        }
    }
}

impl ForeignFrom<WebhookDetails> for storage_types::WebhookDetails {
    fn foreign_from(item: WebhookDetails) -> Self {
        let webhook_url = item
            .multiple_webhooks_list
            .as_ref()
            .and_then(|list| list.get_legacy_url());
        Self {
            webhook_version: item.webhook_version,
            webhook_username: item.webhook_username,
            webhook_password: item.webhook_password,
            webhook_url,
            payment_created_enabled: item.payment_created_enabled,
            payment_succeeded_enabled: item.payment_succeeded_enabled,
            payment_failed_enabled: item.payment_failed_enabled,
            payment_statuses_enabled: item.payment_statuses_enabled,
            refund_statuses_enabled: item.refund_statuses_enabled,
            payout_statuses_enabled: item.payout_statuses_enabled,
            multiple_webhooks_list: item
                .multiple_webhooks_list
                .map(|urls| urls.0.into_iter().map(ForeignFrom::foreign_from).collect()),
        }
    }
}

use crate::{
    errors::api_error_response,
    merchant_key_store::MerchantKeyStore,
    payments,
    transformers::{ForeignFrom, ForeignInto},
};
#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    #[serde(with = "common_utils::crypto::encryptable_exact::optional")]
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
    #[serde(with = "common_utils::crypto::encryptable_exact::optional")]
    pub card_testing_secret_key: OptionalEncryptableName,
    pub is_clear_pan_retries_enabled: bool,
    pub force_3ds_challenge: bool,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: bool,
    pub three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    pub acquirer_config_map: Option<common_types::domain::AcquirerConfigBucket>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub external_vault_details: ExternalVaultDetails,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
    #[serde(with = "common_utils::crypto::encryptable_exact::optional")]
    pub network_tokenization_credentials: OptionalEncryptableValue,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

    /// Returns the external vault connector account id when external vault is enabled.
    pub fn get_vault_connector_id(
        &self,
    ) -> Option<common_utils::id_type::MerchantConnectorAccountId> {
        match self {
            Self::ExternalVaultEnabled(details) => Some(details.vault_connector_id.clone()),
            Self::Skip => None,
        }
    }

    /// Returns true when the configured external vault is the hyperswitch vault (`HyperswitchSdk`).
    pub fn is_hyperswitch_vault(&self) -> bool {
        matches!(
            self,
            Self::ExternalVaultEnabled(details)
                if details.vault_sdk == Some(common_enums::VaultSdk::HyperswitchSdk)
        )
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
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
    pub network_tokenization_credentials: OptionalEncryptableValue,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
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
            three_ds_decision_rule_algorithm: None,
            acquirer_config_map: None,
            merchant_category_code: value.merchant_category_code,
            merchant_country_code: value.merchant_country_code,
            dispute_polling_interval: value.dispute_polling_interval,
            is_manual_retry_enabled: value.is_manual_retry_enabled,
            always_enable_overcapture: value.always_enable_overcapture,
            external_vault_details: value.external_vault_details,
            billing_processor_id: value.billing_processor_id,
            surcharge_connector_details: value.surcharge_connector_details,
            network_tokenization_credentials: value.network_tokenization_credentials,
            payment_method_blocking: value.payment_method_blocking,
            default_fallback_routing: value.default_fallback_routing,
        }
    }
}

#[cfg(feature = "v1")]
pub struct ProfileDbBuilder {
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
    pub merchant_business_country: Option<api_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: bool,
    pub three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    pub acquirer_config_map: Option<common_types::domain::AcquirerConfigBucket>,
    pub merchant_category_code: Option<api_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub external_vault_details: ExternalVaultDetails,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
    pub network_tokenization_credentials: OptionalEncryptableValue,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v1")]
impl From<ProfileDbBuilder> for Profile {
    fn from(value: ProfileDbBuilder) -> Self {
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
            version: value.version,
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
            three_ds_decision_rule_algorithm: value.three_ds_decision_rule_algorithm,
            acquirer_config_map: value.acquirer_config_map,
            merchant_category_code: value.merchant_category_code,
            merchant_country_code: value.merchant_country_code,
            dispute_polling_interval: value.dispute_polling_interval,
            is_manual_retry_enabled: value.is_manual_retry_enabled,
            always_enable_overcapture: value.always_enable_overcapture,
            external_vault_details: value.external_vault_details,
            billing_processor_id: value.billing_processor_id,
            surcharge_connector_details: value.surcharge_connector_details,
            network_tokenization_credentials: value.network_tokenization_credentials,
            payment_method_blocking: value.payment_method_blocking,
            default_fallback_routing: value.default_fallback_routing,
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
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
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
    AcquirerConfigBucketUpdate {
        acquirer_config_map: Option<common_types::domain::AcquirerConfigBucket>,
    },
    DefaultRoutingFallbackUpdate {
        default_fallback_routing: Option<pii::SecretSerdeValue>,
    },
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    #[serde(with = "common_utils::crypto::encryptable_exact::optional")]
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
    #[serde(with = "common_utils::crypto::encryptable_exact::optional")]
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
    pub is_manual_retry_enabled: Option<bool>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
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
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
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
            is_manual_retry_enabled: None,
            billing_processor_id: value.billing_processor_id,
            surcharge_connector_details: value.surcharge_connector_details,
        }
    }
}

#[cfg(feature = "v2")]
pub struct ProfileDbBuilder {
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
    pub is_manual_retry_enabled: Option<bool>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
}

#[cfg(feature = "v2")]
impl From<ProfileDbBuilder> for Profile {
    fn from(value: ProfileDbBuilder) -> Self {
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
            version: value.version,
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
            is_manual_retry_enabled: value.is_manual_retry_enabled,
            billing_processor_id: value.billing_processor_id,
            surcharge_connector_details: value.surcharge_connector_details,
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
    pub fn get_is_tax_calculation_enabled(&self, payment_intent: &payments::PaymentIntent) -> bool {
        self.get_is_tax_connector_enabled()
            && !payment_intent
                .skip_external_tax_calculation
                .unwrap_or(false)
    }

    #[cfg(feature = "v1")]
    pub fn get_order_fulfillment_time(&self) -> Option<i64> {
        self.intent_fulfillment_time
    }

    #[cfg(feature = "v2")]
    pub fn get_order_fulfillment_time(&self) -> Option<i64> {
        self.order_fulfillment_time
    }

    #[cfg(feature = "v2")]
    pub fn get_order_fulfillment_time_or_default(&self) -> i64 {
        self.get_order_fulfillment_time()
            .unwrap_or(common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME)
    }

    pub fn get_webhook_url_from_profile(&self) -> CustomResult<String, ValidationError> {
        self.webhook_details
            .as_ref()
            .and_then(|details| details.multiple_webhooks_list.as_ref())
            .and_then(|list| list.get_legacy_url())
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
        // Flatten all buckets and search across them for an AcquirerConfig matching the network.
        self.acquirer_config_map
            .as_ref()
            .and_then(|acquirer_config_map| {
                acquirer_config_map
                    .configs
                    .values()
                    .flat_map(|bucket| bucket.iter())
                    .find(|cfg| cfg.network == network)
                    .cloned()
            })
    }

    /// Resolve an `AcquirerConfig` for a specific `profile_acquirer_id` bucket and `network`.
    /// Use this when the authentication record already has a `profile_acquirer_id` scoped to a
    /// particular acquirer, so the lookup is restricted to that bucket only.
    #[cfg(feature = "v1")]
    pub fn get_acquirer_details_for_profile_acquirer(
        &self,
        profile_acquirer_id: &common_utils::id_type::ProfileAcquirerId,
        network: common_enums::CardNetwork,
    ) -> Option<AcquirerConfig> {
        self.acquirer_config_map
            .as_ref()
            .and_then(|map| map.configs.get(profile_acquirer_id))
            .and_then(|bucket| bucket.iter().find(|cfg| cfg.network == network).cloned())
    }

    /// Resolve an `AcquirerConfig` from the default bucket or fallback to the first bucket.
    #[cfg(feature = "v1")]
    pub fn get_default_acquirer_details_from_network(
        &self,
        network: common_enums::CardNetwork,
    ) -> Option<AcquirerConfig> {
        self.acquirer_config_map.as_ref().and_then(|map| {
            // Get the default bucket from the default_acquirer_config identifier, or fallback to the first bucket
            let default_bucket = map
                .default_acquirer_config
                .as_ref()
                .and_then(|id| map.configs.get(id))
                .or_else(|| map.configs.values().next());

            default_bucket
                .and_then(|bucket| bucket.iter().find(|cfg| cfg.network == network).cloned())
        })
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

    /// As per RBI guidelines, Alt-ID is applicable for merchants based in India
    pub fn is_alt_id_eligible_merchant(&self) -> bool {
        matches!(
            self.merchant_business_country,
            Some(api_enums::CountryAlpha2::IN)
        )
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
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
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
pub trait ProfileInterface {
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
