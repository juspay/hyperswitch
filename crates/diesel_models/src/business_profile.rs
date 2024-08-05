#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
use common_enums::OrderFulfillmentTimeOrigin;
use common_utils::{encryption::Encryption, pii};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
use crate::schema::business_profile;
#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
use crate::schema_v2::business_profile;

/// Note: The order of fields in the struct is important.
/// This should be in the same order as the fields in the schema.rs file, otherwise the code will
/// not compile
/// If two adjacent columns have the same type, then the compiler will not throw any error, but the
/// fields read / written will be interchanged
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Identifiable,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = business_profile, primary_key(profile_id), check_for_backend(diesel::pg::Pg))]
pub struct BusinessProfile {
    pub profile_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<serde_json::Value>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: bool,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<serde_json::Value>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<serde_json::Value>,
    pub payout_link_config: Option<serde_json::Value>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(profile_id))]
pub struct BusinessProfileNew {
    pub profile_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<serde_json::Value>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: bool,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<serde_json::Value>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<serde_json::Value>,
    pub payout_link_config: Option<serde_json::Value>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfileUpdateInternal {
    pub profile_name: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub webhook_details: Option<serde_json::Value>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<serde_json::Value>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<serde_json::Value>,
    pub payout_link_config: Option<serde_json::Value>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BusinessProfileUpdate {
    Update {
        profile_name: Option<String>,
        return_url: Option<String>,
        enable_payment_response_hash: Option<bool>,
        payment_response_hash_key: Option<String>,
        redirect_to_merchant_with_http_post: Option<bool>,
        webhook_details: Option<serde_json::Value>,
        metadata: Option<pii::SecretSerdeValue>,
        routing_algorithm: Option<serde_json::Value>,
        intent_fulfillment_time: Option<i64>,
        frm_routing_algorithm: Option<serde_json::Value>,
        payout_routing_algorithm: Option<serde_json::Value>,
        is_recon_enabled: Option<bool>,
        applepay_verified_domains: Option<Vec<String>>,
        payment_link_config: Option<serde_json::Value>,
        session_expiry: Option<i64>,
        authentication_connector_details: Option<serde_json::Value>,
        payout_link_config: Option<serde_json::Value>,
        extended_card_info_config: Option<pii::SecretSerdeValue>,
        use_billing_as_payment_method_billing: Option<bool>,
        collect_shipping_details_from_wallet_connector: Option<bool>,
        collect_billing_details_from_wallet_connector: Option<bool>,
        is_connector_agnostic_mit_enabled: Option<bool>,
        outgoing_webhook_custom_http_headers: Option<Encryption>,
    },
    RoutingAlgorithmUpdate {
        routing_algorithm: Option<serde_json::Value>,
        payout_routing_algorithm: Option<serde_json::Value>,
    },
    ExtendedCardInfoUpdate {
        is_extended_card_info_enabled: Option<bool>,
    },
    ConnectorAgnosticMitUpdate {
        is_connector_agnostic_mit_enabled: Option<bool>,
    },
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
impl From<BusinessProfileUpdate> for BusinessProfileUpdateInternal {
    fn from(business_profile_update: BusinessProfileUpdate) -> Self {
        let now = common_utils::date_time::now();

        match business_profile_update {
            BusinessProfileUpdate::Update {
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
                is_recon_enabled,
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
            } => Self {
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
                is_recon_enabled,
                applepay_verified_domains,
                payment_link_config,
                session_expiry,
                authentication_connector_details,
                payout_link_config,
                is_extended_card_info_enabled: None,
                extended_card_info_config,
                use_billing_as_payment_method_billing,
                collect_shipping_details_from_wallet_connector,
                collect_billing_details_from_wallet_connector,
                is_connector_agnostic_mit_enabled,
                outgoing_webhook_custom_http_headers,
            },
            BusinessProfileUpdate::RoutingAlgorithmUpdate {
                routing_algorithm,
                payout_routing_algorithm,
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
            },
            BusinessProfileUpdate::ExtendedCardInfoUpdate {
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
                is_extended_card_info_enabled,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
            },
            BusinessProfileUpdate::ConnectorAgnosticMitUpdate {
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
                is_connector_agnostic_mit_enabled,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
            },
        }
    }
}

// This is being used only in the `BusinessProfileInterface` implementation for `MockDb`.
// This can be removed once the `BusinessProfileInterface` trait has been updated to use the domain
// model instead.
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
impl From<BusinessProfileNew> for BusinessProfile {
    fn from(new: BusinessProfileNew) -> Self {
        Self {
            profile_id: new.profile_id,
            merchant_id: new.merchant_id,
            profile_name: new.profile_name,
            created_at: new.created_at,
            modified_at: new.modified_at,
            return_url: new.return_url,
            enable_payment_response_hash: new.enable_payment_response_hash,
            payment_response_hash_key: new.payment_response_hash_key,
            redirect_to_merchant_with_http_post: new.redirect_to_merchant_with_http_post,
            webhook_details: new.webhook_details,
            metadata: new.metadata,
            routing_algorithm: new.routing_algorithm,
            intent_fulfillment_time: new.intent_fulfillment_time,
            frm_routing_algorithm: new.frm_routing_algorithm,
            payout_routing_algorithm: new.payout_routing_algorithm,
            is_recon_enabled: new.is_recon_enabled,
            applepay_verified_domains: new.applepay_verified_domains,
            payment_link_config: new.payment_link_config,
            session_expiry: new.session_expiry,
            authentication_connector_details: new.authentication_connector_details,
            payout_link_config: new.payout_link_config,
            is_connector_agnostic_mit_enabled: new.is_connector_agnostic_mit_enabled,
            is_extended_card_info_enabled: new.is_extended_card_info_enabled,
            extended_card_info_config: new.extended_card_info_config,
            use_billing_as_payment_method_billing: new.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: new
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: new
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: new.outgoing_webhook_custom_http_headers,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
impl BusinessProfileUpdate {
    pub fn apply_changeset(self, source: BusinessProfile) -> BusinessProfile {
        let BusinessProfileUpdateInternal {
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
        } = self.into();
        BusinessProfile {
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
        }
    }
}

/// Note: The order of fields in the struct is important.
/// This should be in the same order as the fields in the schema.rs file, otherwise the code will
/// not compile
/// If two adjacent columns have the same type, then the compiler will not throw any error, but the
/// fields read / written will be interchanged
#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Identifiable,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = business_profile, primary_key(profile_id), check_for_backend(diesel::pg::Pg))]
pub struct BusinessProfile {
    pub profile_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub is_recon_enabled: bool,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<pii::SecretSerdeValue>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<pii::SecretSerdeValue>,
    pub payout_link_config: Option<pii::SecretSerdeValue>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub routing_algorithm_id: Option<String>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<String>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(profile_id))]
pub struct BusinessProfileNew {
    pub profile_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub is_recon_enabled: bool,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<pii::SecretSerdeValue>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<pii::SecretSerdeValue>,
    pub payout_link_config: Option<pii::SecretSerdeValue>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub routing_algorithm_id: Option<String>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<String>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfileUpdateInternal {
    pub profile_name: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub webhook_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub is_recon_enabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub payment_link_config: Option<pii::SecretSerdeValue>,
    pub session_expiry: Option<i64>,
    pub authentication_connector_details: Option<pii::SecretSerdeValue>,
    pub payout_link_config: Option<pii::SecretSerdeValue>,
    pub is_extended_card_info_enabled: Option<bool>,
    pub extended_card_info_config: Option<pii::SecretSerdeValue>,
    pub is_connector_agnostic_mit_enabled: Option<bool>,
    pub use_billing_as_payment_method_billing: Option<bool>,
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
    pub collect_billing_details_from_wallet_connector: Option<bool>,
    pub outgoing_webhook_custom_http_headers: Option<Encryption>,
    pub routing_algorithm_id: Option<String>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<String>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BusinessProfileUpdate {
    Update {
        profile_name: Option<String>,
        return_url: Option<String>,
        enable_payment_response_hash: Option<bool>,
        payment_response_hash_key: Option<String>,
        redirect_to_merchant_with_http_post: Option<bool>,
        webhook_details: Option<pii::SecretSerdeValue>,
        metadata: Option<pii::SecretSerdeValue>,
        is_recon_enabled: Option<bool>,
        applepay_verified_domains: Option<Vec<String>>,
        payment_link_config: Option<pii::SecretSerdeValue>,
        session_expiry: Option<i64>,
        authentication_connector_details: Option<pii::SecretSerdeValue>,
        payout_link_config: Option<pii::SecretSerdeValue>,
        extended_card_info_config: Option<pii::SecretSerdeValue>,
        use_billing_as_payment_method_billing: Option<bool>,
        collect_shipping_details_from_wallet_connector: Option<bool>,
        collect_billing_details_from_wallet_connector: Option<bool>,
        is_connector_agnostic_mit_enabled: Option<bool>,
        outgoing_webhook_custom_http_headers: Option<Encryption>,
        routing_algorithm_id: Option<String>,
        order_fulfillment_time: Option<i64>,
        order_fulfillment_time_origin: Option<OrderFulfillmentTimeOrigin>,
        frm_routing_algorithm_id: Option<String>,
        payout_routing_algorithm_id: Option<String>,
        default_fallback_routing: Option<pii::SecretSerdeValue>,
    },
    RoutingAlgorithmUpdate {
        routing_algorithm_id: Option<String>,
        payout_routing_algorithm_id: Option<String>,
    },
    ExtendedCardInfoUpdate {
        is_extended_card_info_enabled: Option<bool>,
    },
    ConnectorAgnosticMitUpdate {
        is_connector_agnostic_mit_enabled: Option<bool>,
    },
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
impl From<BusinessProfileUpdate> for BusinessProfileUpdateInternal {
    fn from(business_profile_update: BusinessProfileUpdate) -> Self {
        let now = common_utils::date_time::now();

        match business_profile_update {
            BusinessProfileUpdate::Update {
                profile_name,
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
                extended_card_info_config,
                use_billing_as_payment_method_billing,
                collect_shipping_details_from_wallet_connector,
                collect_billing_details_from_wallet_connector,
                is_connector_agnostic_mit_enabled,
                outgoing_webhook_custom_http_headers,
                routing_algorithm_id,
                order_fulfillment_time,
                order_fulfillment_time_origin,
                frm_routing_algorithm_id,
                payout_routing_algorithm_id,
                default_fallback_routing,
            } => Self {
                profile_name,
                modified_at: now,
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
                is_extended_card_info_enabled: None,
                extended_card_info_config,
                use_billing_as_payment_method_billing,
                collect_shipping_details_from_wallet_connector,
                collect_billing_details_from_wallet_connector,
                is_connector_agnostic_mit_enabled,
                outgoing_webhook_custom_http_headers,
                routing_algorithm_id,
                order_fulfillment_time,
                order_fulfillment_time_origin,
                frm_routing_algorithm_id,
                payout_routing_algorithm_id,
                default_fallback_routing,
            },
            BusinessProfileUpdate::RoutingAlgorithmUpdate {
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
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                payout_routing_algorithm_id,
                default_fallback_routing: None,
            },
            BusinessProfileUpdate::ExtendedCardInfoUpdate {
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
                is_extended_card_info_enabled,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled: None,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                default_fallback_routing: None,
            },
            BusinessProfileUpdate::ConnectorAgnosticMitUpdate {
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
                session_expiry: None,
                authentication_connector_details: None,
                payout_link_config: None,
                is_extended_card_info_enabled: None,
                extended_card_info_config: None,
                is_connector_agnostic_mit_enabled,
                use_billing_as_payment_method_billing: None,
                collect_shipping_details_from_wallet_connector: None,
                collect_billing_details_from_wallet_connector: None,
                outgoing_webhook_custom_http_headers: None,
                routing_algorithm_id: None,
                order_fulfillment_time: None,
                order_fulfillment_time_origin: None,
                frm_routing_algorithm_id: None,
                payout_routing_algorithm_id: None,
                default_fallback_routing: None,
            },
        }
    }
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
impl BusinessProfileUpdate {
    pub fn apply_changeset(self, source: BusinessProfile) -> BusinessProfile {
        let BusinessProfileUpdateInternal {
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
            routing_algorithm_id,
            order_fulfillment_time,
            order_fulfillment_time_origin,
            frm_routing_algorithm_id,
            payout_routing_algorithm_id,
            default_fallback_routing,
        } = self.into();
        BusinessProfile {
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
            routing_algorithm_id: routing_algorithm_id.or(source.routing_algorithm_id),
            order_fulfillment_time: order_fulfillment_time.or(source.order_fulfillment_time),
            order_fulfillment_time_origin: order_fulfillment_time_origin
                .or(source.order_fulfillment_time_origin),
            frm_routing_algorithm_id: frm_routing_algorithm_id.or(source.frm_routing_algorithm_id),
            payout_routing_algorithm_id: payout_routing_algorithm_id
                .or(source.payout_routing_algorithm_id),
            default_fallback_routing: default_fallback_routing.or(source.default_fallback_routing),
        }
    }
}

// This is being used only in the `BusinessProfileInterface` implementation for `MockDb`.
// This can be removed once the `BusinessProfileInterface` trait has been updated to use the domain
// model instead.
#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
impl From<BusinessProfileNew> for BusinessProfile {
    fn from(new: BusinessProfileNew) -> Self {
        Self {
            profile_id: new.profile_id,
            merchant_id: new.merchant_id,
            profile_name: new.profile_name,
            created_at: new.created_at,
            modified_at: new.modified_at,
            return_url: new.return_url,
            enable_payment_response_hash: new.enable_payment_response_hash,
            payment_response_hash_key: new.payment_response_hash_key,
            redirect_to_merchant_with_http_post: new.redirect_to_merchant_with_http_post,
            webhook_details: new.webhook_details,
            metadata: new.metadata,
            is_recon_enabled: new.is_recon_enabled,
            applepay_verified_domains: new.applepay_verified_domains,
            payment_link_config: new.payment_link_config,
            session_expiry: new.session_expiry,
            authentication_connector_details: new.authentication_connector_details,
            payout_link_config: new.payout_link_config,
            is_connector_agnostic_mit_enabled: new.is_connector_agnostic_mit_enabled,
            is_extended_card_info_enabled: new.is_extended_card_info_enabled,
            extended_card_info_config: new.extended_card_info_config,
            use_billing_as_payment_method_billing: new.use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector: new
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: new
                .collect_billing_details_from_wallet_connector,
            outgoing_webhook_custom_http_headers: new.outgoing_webhook_custom_http_headers,
            routing_algorithm_id: new.routing_algorithm_id,
            order_fulfillment_time: new.order_fulfillment_time,
            order_fulfillment_time_origin: new.order_fulfillment_time_origin,
            frm_routing_algorithm_id: new.frm_routing_algorithm_id,
            payout_routing_algorithm_id: new.payout_routing_algorithm_id,
            default_fallback_routing: new.default_fallback_routing,
        }
    }
}
