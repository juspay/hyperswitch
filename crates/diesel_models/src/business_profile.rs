use std::collections::{HashMap, HashSet};

use common_enums::{AuthenticationConnectors, UIWidgetFormLayout};
use common_utils::{encryption::Encryption, pii, types::AlwaysRequestExtendedAuthorization};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;

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
    pub always_request_extended_authorization: Option<AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
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
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
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
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: Option<bool>,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization: Option<AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: Option<bool>,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
}

#[cfg(feature = "v1")]
impl ProfileUpdateInternal {
    pub fn apply_changeset(self, source: Profile) -> Profile {
        let Self {
            profile_name,
            modified_at,
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
            is_recon_enabled,
            applepay_verified_domains,
            payment_link_config,
            session_expiry,
            authentication_connector_details,
            payout_link_config,
            is_extended_card_info_enabled,
            extended_card_info_config,
            is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers,
            always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector,
            tax_connector_id,
            is_tax_connector_enabled,
            dynamic_routing_algorithm,
            is_network_tokenization_enabled,
            is_auto_retries_enabled,
            max_auto_retries_enabled,
            always_request_extended_authorization,
            is_click_to_pay_enabled,
            authentication_product_ids,
        } = self;
        Profile {
            profile_id: source.profile_id,
            merchant_id: source.merchant_id,
            profile_name: profile_name.unwrap_or(source.profile_name),
            created_at: source.created_at,
            modified_at,
            return_url: return_url.or(source.return_url),
            enable_payment_response_hash: enable_payment_response_hash
                .unwrap_or(source.enable_payment_response_hash),
            payment_response_hash_key: payment_response_hash_key
                .or(source.payment_response_hash_key),
            redirect_to_merchant_with_http_post: redirect_to_merchant_with_http_post
                .unwrap_or(source.redirect_to_merchant_with_http_post),
            webhook_details: webhook_details.or(source.webhook_details),
            metadata: metadata.or(source.metadata),
            routing_algorithm: routing_algorithm.or(source.routing_algorithm),
            intent_fulfillment_time: intent_fulfillment_time.or(source.intent_fulfillment_time),
            frm_routing_algorithm: frm_routing_algorithm.or(source.frm_routing_algorithm),
            payout_routing_algorithm: payout_routing_algorithm.or(source.payout_routing_algorithm),
            is_recon_enabled: is_recon_enabled.unwrap_or(source.is_recon_enabled),
            applepay_verified_domains: applepay_verified_domains
                .or(source.applepay_verified_domains),
            payment_link_config: payment_link_config.or(source.payment_link_config),
            session_expiry: session_expiry.or(source.session_expiry),
            authentication_connector_details: authentication_connector_details
                .or(source.authentication_connector_details),
            payout_link_config: payout_link_config.or(source.payout_link_config),
            is_extended_card_info_enabled: is_extended_card_info_enabled
                .or(source.is_extended_card_info_enabled),
            is_connector_agnostic_mit_enabled: is_connector_agnostic_mit_enabled
                .or(source.is_connector_agnostic_mit_enabled),
            extended_card_info_config: extended_card_info_config
                .or(source.extended_card_info_config),
            use_billing_as_payment_method_billing: use_billing_as_payment_method_billing
                .or(source.use_billing_as_payment_method_billing),
            collect_shipping_details_from_wallet_connector:
                collect_shipping_details_from_wallet_connector
                    .or(source.collect_shipping_details_from_wallet_connector),
            collect_billing_details_from_wallet_connector:
                collect_billing_details_from_wallet_connector
                    .or(source.collect_billing_details_from_wallet_connector),
            outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                .or(source.outgoing_webhook_custom_http_headers),
            always_collect_billing_details_from_wallet_connector:
                always_collect_billing_details_from_wallet_connector
                    .or(source.always_collect_billing_details_from_wallet_connector),
            always_collect_shipping_details_from_wallet_connector:
                always_collect_shipping_details_from_wallet_connector
                    .or(source.always_collect_shipping_details_from_wallet_connector),
            tax_connector_id: tax_connector_id.or(source.tax_connector_id),
            is_tax_connector_enabled: is_tax_connector_enabled.or(source.is_tax_connector_enabled),
            version: source.version,
            dynamic_routing_algorithm: dynamic_routing_algorithm
                .or(source.dynamic_routing_algorithm),
            is_network_tokenization_enabled: is_network_tokenization_enabled
                .unwrap_or(source.is_network_tokenization_enabled),
            is_auto_retries_enabled: is_auto_retries_enabled.or(source.is_auto_retries_enabled),
            max_auto_retries_enabled: max_auto_retries_enabled.or(source.max_auto_retries_enabled),
            always_request_extended_authorization: always_request_extended_authorization
                .or(source.always_request_extended_authorization),
            is_click_to_pay_enabled: is_click_to_pay_enabled
                .unwrap_or(source.is_click_to_pay_enabled),
            authentication_product_ids: authentication_product_ids
                .or(source.authentication_product_ids),
        }
    }
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
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub should_collect_cvv_during_payment: bool,
    pub id: common_utils::id_type::ProfileId,
    pub version: common_enums::ApiVersion,
    pub dynamic_routing_algorithm: Option<serde_json::Value>,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub always_request_extended_authorization: Option<AlwaysRequestExtendedAuthorization>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
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
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub should_collect_cvv_during_payment: bool,
    pub id: common_utils::id_type::ProfileId,
    pub version: common_enums::ApiVersion,
    pub is_network_tokenization_enabled: bool,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub is_click_to_pay_enabled: bool,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
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
    pub routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<common_enums::OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
    pub should_collect_cvv_during_payment: Option<bool>,
    pub is_network_tokenization_enabled: Option<bool>,
    pub is_auto_retries_enabled: Option<bool>,
    pub max_auto_retries_enabled: Option<i16>,
    pub is_click_to_pay_enabled: Option<bool>,
    pub authentication_product_ids:
        Option<common_types::payments::AuthenticationConnectorAccountMap>,
    pub three_ds_decision_manager_config: Option<common_types::payments::DecisionManagerRecord>,
}

#[cfg(feature = "v2")]
impl ProfileUpdateInternal {
    pub fn apply_changeset(self, source: Profile) -> Profile {
        let Self {
            profile_name,
            modified_at,
            return_url,
            enable_payment_response_hash,
            payment_response_hash_key,
            redirect_to_merchant_with_http_post,
            webhook_details,
            metadata,
            is_recon_enabled,
            applepay_verified_domains,
            payment_link_config,
            session_expiry,
            authentication_connector_details,
            payout_link_config,
            is_extended_card_info_enabled,
            extended_card_info_config,
            is_connector_agnostic_mit_enabled,
            use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers,
            always_collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector,
            tax_connector_id,
            is_tax_connector_enabled,
            routing_algorithm_id,
            order_fulfillment_time,
            order_fulfillment_time_origin,
            frm_routing_algorithm_id,
            payout_routing_algorithm_id,
            default_fallback_routing,
            should_collect_cvv_during_payment,
            is_network_tokenization_enabled,
            is_auto_retries_enabled,
            max_auto_retries_enabled,
            is_click_to_pay_enabled,
            authentication_product_ids,
            three_ds_decision_manager_config,
        } = self;
        Profile {
            id: source.id,
            merchant_id: source.merchant_id,
            profile_name: profile_name.unwrap_or(source.profile_name),
            created_at: source.created_at,
            modified_at,
            return_url: return_url.or(source.return_url),
            enable_payment_response_hash: enable_payment_response_hash
                .unwrap_or(source.enable_payment_response_hash),
            payment_response_hash_key: payment_response_hash_key
                .or(source.payment_response_hash_key),
            redirect_to_merchant_with_http_post: redirect_to_merchant_with_http_post
                .unwrap_or(source.redirect_to_merchant_with_http_post),
            webhook_details: webhook_details.or(source.webhook_details),
            metadata: metadata.or(source.metadata),
            is_recon_enabled: is_recon_enabled.unwrap_or(source.is_recon_enabled),
            applepay_verified_domains: applepay_verified_domains
                .or(source.applepay_verified_domains),
            payment_link_config: payment_link_config.or(source.payment_link_config),
            session_expiry: session_expiry.or(source.session_expiry),
            authentication_connector_details: authentication_connector_details
                .or(source.authentication_connector_details),
            payout_link_config: payout_link_config.or(source.payout_link_config),
            is_extended_card_info_enabled: is_extended_card_info_enabled
                .or(source.is_extended_card_info_enabled),
            is_connector_agnostic_mit_enabled: is_connector_agnostic_mit_enabled
                .or(source.is_connector_agnostic_mit_enabled),
            extended_card_info_config: extended_card_info_config
                .or(source.extended_card_info_config),
            use_billing_as_payment_method_billing: use_billing_as_payment_method_billing
                .or(source.use_billing_as_payment_method_billing),
            collect_shipping_details_from_wallet_connector:
                collect_shipping_details_from_wallet_connector
                    .or(source.collect_shipping_details_from_wallet_connector),
            collect_billing_details_from_wallet_connector:
                collect_billing_details_from_wallet_connector
                    .or(source.collect_billing_details_from_wallet_connector),
            outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                .or(source.outgoing_webhook_custom_http_headers),
            always_collect_billing_details_from_wallet_connector:
                always_collect_billing_details_from_wallet_connector
                    .or(always_collect_billing_details_from_wallet_connector),
            always_collect_shipping_details_from_wallet_connector:
                always_collect_shipping_details_from_wallet_connector
                    .or(always_collect_shipping_details_from_wallet_connector),
            tax_connector_id: tax_connector_id.or(source.tax_connector_id),
            is_tax_connector_enabled: is_tax_connector_enabled.or(source.is_tax_connector_enabled),
            routing_algorithm_id: routing_algorithm_id.or(source.routing_algorithm_id),
            order_fulfillment_time: order_fulfillment_time.or(source.order_fulfillment_time),
            order_fulfillment_time_origin: order_fulfillment_time_origin
                .or(source.order_fulfillment_time_origin),
            frm_routing_algorithm_id: frm_routing_algorithm_id.or(source.frm_routing_algorithm_id),
            payout_routing_algorithm_id: payout_routing_algorithm_id
                .or(source.payout_routing_algorithm_id),
            default_fallback_routing: default_fallback_routing.or(source.default_fallback_routing),
            should_collect_cvv_during_payment: should_collect_cvv_during_payment
                .unwrap_or(source.should_collect_cvv_during_payment),
            version: source.version,
            dynamic_routing_algorithm: None,
            is_network_tokenization_enabled: is_network_tokenization_enabled
                .unwrap_or(source.is_network_tokenization_enabled),
            is_auto_retries_enabled: is_auto_retries_enabled.or(source.is_auto_retries_enabled),
            max_auto_retries_enabled: max_auto_retries_enabled.or(source.max_auto_retries_enabled),
            always_request_extended_authorization: None,
            is_click_to_pay_enabled: is_click_to_pay_enabled
                .unwrap_or(source.is_click_to_pay_enabled),
            authentication_product_ids: authentication_product_ids
                .or(source.authentication_product_ids),
            three_ds_decision_manager_config: three_ds_decision_manager_config
                .or(source.three_ds_decision_manager_config),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct AuthenticationConnectorDetails {
    pub authentication_connectors: Vec<AuthenticationConnectors>,
    pub three_ds_requestor_url: String,
}

common_utils::impl_to_sql_from_sql_json!(AuthenticationConnectorDetails);

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
}

common_utils::impl_to_sql_from_sql_json!(WebhookDetails);

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
    pub payment_button_colour: Option<String>,
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
