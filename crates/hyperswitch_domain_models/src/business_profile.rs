#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
use common_enums::OrderFulfillmentTimeOrigin;
use common_utils::{
    crypto::OptionalEncryptableValue,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
    types::keymanager,
};
use diesel_models::business_profile::BusinessProfileUpdateInternal;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};

use crate::type_encryption::{decrypt_optional, AsyncLift};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[derive(Clone, Debug)]
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
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[derive(Debug)]
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
        outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
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
        let now = date_time::now();

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
                is_connector_agnostic_mit_enabled,
                use_billing_as_payment_method_billing,
                collect_shipping_details_from_wallet_connector,
                collect_billing_details_from_wallet_connector,
                outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                    .map(Encryption::from),
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "business_profile_v2")
))]
#[async_trait::async_trait]
impl super::behaviour::Conversion for BusinessProfile {
    type DstType = diesel_models::business_profile::BusinessProfile;
    type NewDstType = diesel_models::business_profile::BusinessProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::business_profile::BusinessProfile {
            profile_id: self.profile_id,
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
                outgoing_webhook_custom_http_headers: item
                    .outgoing_webhook_custom_http_headers
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::business_profile::BusinessProfileNew {
            profile_id: self.profile_id,
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
        })
    }
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
#[derive(Clone, Debug)]
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
    pub outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
    pub routing_algorithm_id: Option<String>,
    pub order_fulfillment_time: Option<i64>,
    pub order_fulfillment_time_origin: Option<OrderFulfillmentTimeOrigin>,
    pub frm_routing_algorithm_id: Option<String>,
    pub payout_routing_algorithm_id: Option<String>,
    pub default_fallback_routing: Option<pii::SecretSerdeValue>,
}

#[cfg(all(feature = "v2", feature = "business_profile_v2"))]
#[derive(Debug)]
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
        outgoing_webhook_custom_http_headers: OptionalEncryptableValue,
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
        let now = date_time::now();

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
                is_connector_agnostic_mit_enabled,
                use_billing_as_payment_method_billing,
                collect_shipping_details_from_wallet_connector,
                collect_billing_details_from_wallet_connector,
                outgoing_webhook_custom_http_headers: outgoing_webhook_custom_http_headers
                    .map(Encryption::from),
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
#[async_trait::async_trait]
impl super::behaviour::Conversion for BusinessProfile {
    type DstType = diesel_models::business_profile::BusinessProfile;
    type NewDstType = diesel_models::business_profile::BusinessProfileNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::business_profile::BusinessProfile {
            profile_id: self.profile_id,
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
            order_fulfillment_time: self.order_fulfillment_time,
            order_fulfillment_time_origin: self.order_fulfillment_time_origin,
            frm_routing_algorithm_id: self.frm_routing_algorithm_id,
            payout_routing_algorithm_id: self.payout_routing_algorithm_id,
            default_fallback_routing: self.default_fallback_routing,
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
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
                routing_algorithm_id: item.routing_algorithm_id,
                order_fulfillment_time: item.order_fulfillment_time,
                order_fulfillment_time_origin: item.order_fulfillment_time_origin,
                frm_routing_algorithm_id: item.frm_routing_algorithm_id,
                payout_routing_algorithm_id: item.payout_routing_algorithm_id,
                default_fallback_routing: item.default_fallback_routing,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting business profile data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::business_profile::BusinessProfileNew {
            profile_id: self.profile_id,
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
            order_fulfillment_time: self.order_fulfillment_time,
            order_fulfillment_time_origin: self.order_fulfillment_time_origin,
            frm_routing_algorithm_id: self.frm_routing_algorithm_id,
            payout_routing_algorithm_id: self.payout_routing_algorithm_id,
            default_fallback_routing: self.default_fallback_routing,
        })
    }
}
