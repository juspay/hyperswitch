use std::collections::{HashMap, HashSet};

use common_enums::{AuthenticationConnectors, UIWidgetFormLayout, VaultSdk};
use common_types::primitive_wrappers;
use common_utils::{encryption::Encryption, pii};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use hyperswitch_masking::Secret;
use time::Duration;

#[cfg(feature = "v1")]
use crate::schema::business_profile;
#[cfg(feature = "v2")]
use crate::schema_v2::business_profile;

/// Note: The order of fields in the struct is important.
/// This should be in the same order as the fields in the schema.rs file, otherwise the code will
/// not compile
/// If two adjacent columns have the same type, then the compiler will not throw any error, but the
/// fields read / written will be interchanged
#[cfg(feature = "v1")]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(profile_id), check_for_backend(diesel::pg::Pg))]
pub struct Profile {
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
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
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
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub version: common_enums::ApiVersion,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization:
        Option<primitive_wrappers::AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: Option<Encryption>,
    pub is_clear_pan_retries_enabled: bool,
    pub force_3ds_challenge: Option<bool>,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub id: Option<common_utils::id_type::ProfileId>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: Option<bool>,
    pub three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    pub acquirer_config_map: Option<AcquirerConfigBucket>,
    pub merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub is_l2_l3_enabled: Option<bool>,
    pub network_tokenization_credentials: Option<Encryption>,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(profile_id))]
pub struct ProfileNew {
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
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
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
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub version: common_enums::ApiVersion,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: Option<Encryption>,
    pub is_clear_pan_retries_enabled: bool,
    pub force_3ds_challenge: Option<bool>,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub id: Option<common_utils::id_type::ProfileId>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: Option<bool>,
    pub merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub is_l2_l3_enabled: Option<bool>,
    pub network_tokenization_credentials: Option<Encryption>,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
#[router_derive::apply_changeset(target = Profile)]
pub struct ProfileUpdateInternal {
    pub profile_name: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
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
    pub is_recon_enabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
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
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub is_l2_l3_enabled: Option<bool>,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: Option<bool>,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization:
        Option<primitive_wrappers::AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: Option<bool>,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: Option<Encryption>,
    pub is_clear_pan_retries_enabled: Option<bool>,
    pub force_3ds_challenge: Option<bool>,
    pub is_debit_routing_enabled: Option<bool>,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_pre_network_tokenization_enabled: Option<bool>,
    pub three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    pub acquirer_config_map: Option<AcquirerConfigBucket>,
    pub merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub network_tokenization_credentials: Option<Encryption>,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
}

/// Note: The order of fields in the struct is important.
/// This should be in the same order as the fields in the schema.rs file, otherwise the code will
/// not compile
/// If two adjacent columns have the same type, then the compiler will not throw any error, but the
/// fields read / written will be interchanged
#[cfg(feature = "v2")]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Profile {
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
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
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
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub version: common_enums::ApiVersion,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization:
        Option<primitive_wrappers::AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: Option<Encryption>,
    pub is_clear_pan_retries_enabled: bool,
    pub force_3ds_challenge: Option<bool>,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub id: common_utils::id_type::ProfileId,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub three_ds_decision_rule_algorithm: Option<serde_json::Value>,
    pub acquirer_config_map: Option<AcquirerConfigBucket>,
    pub merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub dispute_polling_interval: Option<primitive_wrappers::DisputePollingIntervalInHours>,
    pub is_manual_retry_enabled: Option<bool>,
    pub always_enable_overcapture: Option<primitive_wrappers::AlwaysEnableOvercaptureBool>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub is_l2_l3_enabled: Option<bool>,
    pub network_tokenization_credentials: Option<Encryption>,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
    pub should_collect_cvv_during_payment:
        Option<primitive_wrappers::ShouldCollectCvvDuringPayment>,
    pub revenue_recovery_retry_algorithm_type: Option<common_enums::RevenueRecoveryAlgorithmType>,
    pub revenue_recovery_retry_algorithm_data: Option<RevenueRecoveryAlgorithmData>,
    pub split_txns_enabled: Option<common_enums::SplitTxnsEnabled>,
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

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(profile_id))]
pub struct ProfileNew {
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
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
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
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub version: common_enums::ApiVersion,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: Option<Encryption>,
    pub is_clear_pan_retries_enabled: Option<bool>,
    pub is_debit_routing_enabled: bool,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
    pub should_collect_cvv_during_payment:
        Option<primitive_wrappers::ShouldCollectCvvDuringPayment>,
    pub id: common_utils::id_type::ProfileId,
    pub revenue_recovery_retry_algorithm_type: Option<common_enums::RevenueRecoveryAlgorithmType>,
    pub revenue_recovery_retry_algorithm_data: Option<RevenueRecoveryAlgorithmData>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub is_l2_l3_enabled: Option<bool>,
    pub split_txns_enabled: Option<common_enums::SplitTxnsEnabled>,
    pub payment_method_blocking: Option<PaymentMethodBlockingConfig>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
#[router_derive::apply_changeset(target = Profile)]
pub struct ProfileUpdateInternal {
    pub profile_name: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<common_utils::types::Url>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub webhook_details: Option<WebhookDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub is_recon_enabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
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
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub always_collect_billing_details_from_wallet_connector: Option<bool>,
    pub always_collect_shipping_details_from_wallet_connector: Option<bool>,
    pub tax_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub is_tax_connector_enabled: Option<bool>,
    pub is_network_tokenization_enabled: Option<bool>,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub is_click_to_pay_enabled: Option<bool>,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub card_testing_guard_config: Option<CardTestingGuardConfig>,
    pub card_testing_secret_key: Option<Encryption>,
    pub is_clear_pan_retries_enabled: Option<bool>,
    pub is_debit_routing_enabled: Option<bool>,
    pub merchant_business_country: Option<common_enums::CountryAlpha2>,
    pub merchant_category_code: Option<common_enums::MerchantCategoryCode>,
    pub merchant_country_code: Option<common_types::payments::MerchantCountryCode>,
    pub billing_processor_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub surcharge_connector_details: Option<SurchargeConnectorDetails>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
    pub should_collect_cvv_during_payment:
        Option<primitive_wrappers::ShouldCollectCvvDuringPayment>,
    pub revenue_recovery_retry_algorithm_type: Option<common_enums::RevenueRecoveryAlgorithmType>,
    pub revenue_recovery_retry_algorithm_data: Option<RevenueRecoveryAlgorithmData>,
    pub is_iframe_redirection_enabled: Option<bool>,
    pub is_external_vault_enabled: Option<bool>,
    pub external_vault_connector_details: Option<ExternalVaultConnectorDetails>,
    pub is_l2_l3_enabled: Option<bool>,
    pub split_txns_enabled: Option<common_enums::SplitTxnsEnabled>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct AuthenticationConnectorDetails {
    pub authentication_connectors: Vec<AuthenticationConnectors>,
    pub three_ds_requestor_url: String,
    pub three_ds_requestor_app_url: Option<String>,
}

common_utils::impl_to_sql_from_sql_json!(AuthenticationConnectorDetails);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct ExternalVaultConnectorDetails {
    pub vault_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub vault_sdk: Option<VaultSdk>,
    pub vault_token_selector: Option<Vec<VaultTokenField>>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct VaultTokenField {
    pub token_type: common_enums::VaultTokenType,
}

common_utils::impl_to_sql_from_sql_json!(ExternalVaultConnectorDetails);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct SurchargeConnectorDetails {
    pub surcharge_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

common_utils::impl_to_sql_from_sql_json!(SurchargeConnectorDetails);

fn default_guest_ip_blocking_status() -> bool {
    common_utils::consts::DEFAULT_GUEST_IP_BLOCKING_STATUS
}

fn default_guest_ip_blocking_threshold() -> i32 {
    common_utils::consts::DEFAULT_GUEST_IP_BLOCKING_THRESHOLD
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct CardTestingGuardConfig {
    pub is_card_ip_blocking_enabled: bool,
    pub card_ip_blocking_threshold: i32,
    pub is_guest_user_card_blocking_enabled: bool,
    pub guest_user_card_blocking_threshold: i32,
    pub is_customer_id_blocking_enabled: bool,
    pub customer_id_blocking_threshold: i32,
    pub card_testing_guard_expiry: i32,
    #[serde(default = "default_guest_ip_blocking_status")]
    pub is_guest_ip_blocking_enabled: bool,
    #[serde(default = "default_guest_ip_blocking_threshold")]
    pub guest_ip_blocking_threshold: i32,
}

common_utils::impl_to_sql_from_sql_json!(CardTestingGuardConfig);

impl Default for CardTestingGuardConfig {
    fn default() -> Self {
        Self {
            is_card_ip_blocking_enabled: common_utils::consts::DEFAULT_CARD_IP_BLOCKING_STATUS,
            card_ip_blocking_threshold: common_utils::consts::DEFAULT_CARD_IP_BLOCKING_THRESHOLD,
            is_guest_user_card_blocking_enabled:
                common_utils::consts::DEFAULT_GUEST_USER_CARD_BLOCKING_STATUS,
            guest_user_card_blocking_threshold:
                common_utils::consts::DEFAULT_GUEST_USER_CARD_BLOCKING_THRESHOLD,
            is_customer_id_blocking_enabled:
                common_utils::consts::DEFAULT_CUSTOMER_ID_BLOCKING_STATUS,
            customer_id_blocking_threshold:
                common_utils::consts::DEFAULT_CUSTOMER_ID_BLOCKING_THRESHOLD,
            card_testing_guard_expiry:
                common_utils::consts::DEFAULT_CARD_TESTING_GUARD_EXPIRY_IN_SECS,
            is_guest_ip_blocking_enabled: common_utils::consts::DEFAULT_GUEST_IP_BLOCKING_STATUS,
            guest_ip_blocking_threshold: common_utils::consts::DEFAULT_GUEST_IP_BLOCKING_THRESHOLD,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MultipleWebhookDetail {
    pub webhook_endpoint_id: common_utils::id_type::WebhookEndpointId,
    pub webhook_url: Secret<String>,
    pub events: HashSet<common_enums::EventType>,
    pub status: common_enums::OutgoingWebhookEndpointStatus,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct WebhookDetails {
    pub webhook_version: Option<String>,
    pub webhook_username: Option<String>,
    pub webhook_password: Option<Secret<String>>,
    pub webhook_url: Option<Secret<String>>,
    pub payment_created_enabled: Option<bool>,
    pub payment_succeeded_enabled: Option<bool>,
    pub payment_failed_enabled: Option<bool>,
    pub payment_statuses_enabled: Option<Vec<common_enums::IntentStatus>>,
    pub refund_statuses_enabled: Option<Vec<common_enums::RefundStatus>>,
    pub payout_statuses_enabled: Option<Vec<common_enums::PayoutStatus>>,
    pub multiple_webhooks_list: Option<Vec<MultipleWebhookDetail>>,
}

common_utils::impl_to_sql_from_sql_json!(WebhookDetails);

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, diesel::AsExpression, diesel::FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
#[serde(untagged)]
pub enum AcquirerConfigBucket {
    New(common_types::domain::AcquirerConfigBucket),
    Old(HashMap<common_utils::id_type::ProfileAcquirerId, common_types::domain::AcquirerConfig>),
}

common_utils::impl_to_sql_from_sql_json!(AcquirerConfigBucket);

impl From<AcquirerConfigBucket> for common_types::domain::AcquirerConfigBucket {
    fn from(item: AcquirerConfigBucket) -> Self {
        match item {
            AcquirerConfigBucket::New(new) => new,
            AcquirerConfigBucket::Old(old) => Self {
                default_acquirer_config: None,
                configs: old.into_iter().map(|(k, v)| (k, vec![v])).collect(),
            },
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct BusinessPaymentLinkConfig {
    pub domain_name: Option<String>,
    #[serde(flatten)]
    pub default_config: Option<PaymentLinkConfigRequest>,
    pub business_specific_configs: Option<HashMap<String, PaymentLinkConfigRequest>>,
    pub allowed_domains: Option<HashSet<String>>,
    pub branding_visibility: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentLinkConfigRequest {
    pub theme: Option<String>,
    pub logo: Option<String>,
    pub seller_name: Option<String>,
    pub sdk_layout: Option<String>,
    pub display_sdk_only: Option<bool>,
    pub enabled_saved_payment_method: Option<bool>,
    pub hide_card_nickname_field: Option<bool>,
    pub show_card_form_by_default: Option<bool>,
    pub background_image: Option<PaymentLinkBackgroundImageConfig>,
    pub details_layout: Option<common_enums::PaymentLinkDetailsLayout>,
    pub payment_button_text: Option<String>,
    pub custom_message_for_card_terms: Option<String>,
    pub custom_message_for_payment_method_types:
        Option<common_types::payments::PaymentMethodsConfig>,
    pub payment_button_colour: Option<String>,
    pub skip_status_screen: Option<bool>,
    pub payment_button_text_colour: Option<String>,
    pub background_colour: Option<String>,
    pub sdk_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub payment_link_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub enable_button_only_on_form_ready: Option<bool>,
    pub payment_form_header_text: Option<String>,
    pub payment_form_label_type: Option<common_enums::PaymentLinkSdkLabelType>,
    pub show_card_terms: Option<common_enums::PaymentLinkShowSdkTerms>,
    pub is_setup_mandate_flow: Option<bool>,
    pub color_icon_card_cvc_error: Option<String>,
    pub show_merchant_name: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PaymentLinkBackgroundImageConfig {
    pub url: common_utils::types::Url,
    pub position: Option<common_enums::ElementPosition>,
    pub size: Option<common_enums::ElementSize>,
}

common_utils::impl_to_sql_from_sql_json!(BusinessPaymentLinkConfig);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct BusinessPayoutLinkConfig {
    #[serde(flatten)]
    pub config: BusinessGenericLinkConfig,
    pub form_layout: Option<UIWidgetFormLayout>,
    pub payout_test_mode: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct BusinessGenericLinkConfig {
    pub domain_name: Option<String>,
    pub allowed_domains: HashSet<String>,
    #[serde(flatten)]
    pub ui_config: common_utils::link_utils::GenericLinkUiConfig,
}

common_utils::impl_to_sql_from_sql_json!(BusinessPayoutLinkConfig);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct RevenueRecoveryAlgorithmData {
    pub monitoring_configured_timestamp: time::PrimitiveDateTime,
}

impl RevenueRecoveryAlgorithmData {
    pub fn has_exceeded_monitoring_threshold(&self, monitoring_threshold_in_seconds: i64) -> bool {
        let total_threshold_time = self.monitoring_configured_timestamp
            + Duration::seconds(monitoring_threshold_in_seconds);
        common_utils::date_time::now() >= total_threshold_time
    }
}

common_utils::impl_to_sql_from_sql_json!(RevenueRecoveryAlgorithmData);

/// Configuration for payment method blocking based on card attributes
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct PaymentMethodBlockingConfig {
    pub card: Option<CardBlockingConfig>,
    pub wallet: Option<WalletBlockingConfig>,
}

/// Card-specific blocking configuration
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CardBlockingConfig {
    /// Set of issuing countries to block using ISO 3166-1 alpha-2 codes (e.g., ["IN", "US"])
    pub issuing_country: Option<HashSet<common_enums::CountryAlpha2>>,
    /// Set of card types to block (e.g., ["Credit", "Debit"])
    pub card_types: Option<HashSet<common_enums::CardType>>,
    /// Set of card subtypes to block
    pub card_subtypes: Option<HashSet<String>>,
    /// Set of card issuers to block (e.g., ["HDFC Bank", "ICICI Bank"])
    pub issuers: Option<HashSet<String>>,
    /// Whether to block if BIN is provided but no matching record found in cards_info table.
    /// Defaults to false (allow payment if BIN not found in database).
    pub block_if_bin_info_unavailable: Option<bool>,
}

/// Wallet-specific blocking configuration for Apple Pay and Google Pay
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WalletBlockingConfig {
    /// Set of card types to block (e.g., ["Credit", "Debit"])
    pub card_types: Option<HashSet<common_enums::CardType>>,
}

impl WalletBlockingConfig {
    pub fn is_credit_blocked(&self) -> bool {
        self.card_types
            .as_ref()
            .is_some_and(|types| types.contains(&common_enums::CardType::Credit))
    }

    pub fn is_debit_blocked(&self) -> bool {
        self.card_types
            .as_ref()
            .is_some_and(|types| types.contains(&common_enums::CardType::Debit))
    }
}

impl CardBlockingConfig {
    pub fn should_block_if_bin_info_unavailable(&self) -> bool {
        self.block_if_bin_info_unavailable.unwrap_or(false)
    }

    pub fn should_block_by_attribute<T>(blocked: &Option<HashSet<T>>, value: Option<&str>) -> bool
    where
        T: std::str::FromStr + std::hash::Hash + Eq,
    {
        blocked
            .as_ref()
            .zip(value)
            .and_then(|(set, s)| s.parse::<T>().ok().map(|v| (set, v)))
            .is_some_and(|(set, v)| set.contains(&v))
    }
}

common_utils::impl_to_sql_from_sql_json!(PaymentMethodBlockingConfig);
