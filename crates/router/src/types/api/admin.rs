pub use api_models::admin::{
    BusinessProfileCreate, BusinessProfileResponse, BusinessProfileUpdate, MerchantAccountCreate,
    MerchantAccountDeleteResponse, MerchantAccountResponse, MerchantAccountUpdate,
    MerchantConnectorCreate, MerchantConnectorDeleteResponse, MerchantConnectorDetails,
    MerchantConnectorDetailsWrap, MerchantConnectorId, MerchantConnectorResponse, MerchantDetails,
    MerchantId, PaymentMethodsEnabled, ToggleAllKVRequest, ToggleAllKVResponse, ToggleKVRequest,
    ToggleKVResponse, WebhookDetails,
};
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};

use crate::{
    core::errors,
    types::{domain, storage, transformers::ForeignTryFrom},
};

impl TryFrom<domain::MerchantAccount> for MerchantAccountResponse {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: domain::MerchantAccount) -> Result<Self, Self::Error> {
        let primary_business_details: Vec<api_models::admin::PrimaryBusinessDetails> = item
            .primary_business_details
            .parse_value("primary_business_details")?;

        let pm_collect_link_config: Option<api_models::admin::BusinessCollectLinkConfig> = item
            .pm_collect_link_config
            .map(|config| config.parse_value("pm_collect_link_config"))
            .transpose()?;

        Ok(Self {
            merchant_id: item.merchant_id,
            merchant_name: item.merchant_name,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            merchant_details: item.merchant_details,
            webhook_details: item.webhook_details,
            routing_algorithm: item.routing_algorithm,
            sub_merchants_enabled: item.sub_merchants_enabled,
            parent_merchant_id: item.parent_merchant_id,
            publishable_key: item.publishable_key,
            metadata: item.metadata,
            locker_id: item.locker_id,
            primary_business_details,
            frm_routing_algorithm: item.frm_routing_algorithm,
            #[cfg(feature = "payouts")]
            payout_routing_algorithm: item.payout_routing_algorithm,
            organization_id: item.organization_id,
            is_recon_enabled: item.is_recon_enabled,
            default_profile: item.default_profile,
            recon_status: item.recon_status,
            pm_collect_link_config,
        })
    }
}

impl ForeignTryFrom<storage::business_profile::BusinessProfile> for BusinessProfileResponse {
    type Error = error_stack::Report<errors::ParsingError>;

    fn foreign_try_from(
        item: storage::business_profile::BusinessProfile,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            profile_name: item.profile_name,
            return_url: item.return_url,
            enable_payment_response_hash: item.enable_payment_response_hash,
            payment_response_hash_key: item.payment_response_hash_key,
            redirect_to_merchant_with_http_post: item.redirect_to_merchant_with_http_post,
            webhook_details: item.webhook_details.map(Secret::new),
            metadata: item.metadata,
            routing_algorithm: item.routing_algorithm,
            intent_fulfillment_time: item.intent_fulfillment_time,
            frm_routing_algorithm: item.frm_routing_algorithm,
            #[cfg(feature = "payouts")]
            payout_routing_algorithm: item.payout_routing_algorithm,
            applepay_verified_domains: item.applepay_verified_domains,
            payment_link_config: item.payment_link_config,
            session_expiry: item.session_expiry,
            authentication_connector_details: item
                .authentication_connector_details
                .map(|authentication_connector_details| {
                    authentication_connector_details.parse_value("AuthenticationDetails")
                })
                .transpose()?,
            payout_link_config: item
                .payout_link_config
                .map(|payout_link_config| {
                    payout_link_config.parse_value("BusinessPayoutLinkConfig")
                })
                .transpose()?,
            use_billing_as_payment_method_billing: item.use_billing_as_payment_method_billing,
            extended_card_info_config: item
                .extended_card_info_config
                .map(|config| config.expose().parse_value("ExtendedCardInfoConfig"))
                .transpose()?,
            collect_shipping_details_from_wallet_connector: item
                .collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector: item
                .collect_billing_details_from_wallet_connector,
            is_connector_agnostic_mit_enabled: item.is_connector_agnostic_mit_enabled,
        })
    }
}

impl ForeignTryFrom<(domain::MerchantAccount, BusinessProfileCreate)>
    for storage::business_profile::BusinessProfileNew
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        (merchant_account, request): (domain::MerchantAccount, BusinessProfileCreate),
    ) -> Result<Self, Self::Error> {
        // Generate a unique profile id
        let profile_id = common_utils::generate_id_with_default_len("pro");

        let current_time = common_utils::date_time::now();

        let webhook_details = request
            .webhook_details
            .as_ref()
            .map(|webhook_details| {
                webhook_details.encode_to_value().change_context(
                    errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "webhook details",
                    },
                )
            })
            .transpose()?;

        let payment_response_hash_key = request
            .payment_response_hash_key
            .or(merchant_account.payment_response_hash_key)
            .unwrap_or(common_utils::crypto::generate_cryptographically_secure_random_string(64));

        let payment_link_config_value = request
            .payment_link_config
            .map(|pl_config| {
                pl_config.encode_to_value().change_context(
                    errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "payment_link_config_value",
                    },
                )
            })
            .transpose()?;

        Ok(Self {
            profile_id,
            merchant_id: merchant_account.merchant_id,
            profile_name: request.profile_name.unwrap_or("default".to_string()),
            created_at: current_time,
            modified_at: current_time,
            return_url: request
                .return_url
                .map(|return_url| return_url.to_string())
                .or(merchant_account.return_url),
            enable_payment_response_hash: request
                .enable_payment_response_hash
                .unwrap_or(merchant_account.enable_payment_response_hash),
            payment_response_hash_key: Some(payment_response_hash_key),
            redirect_to_merchant_with_http_post: request
                .redirect_to_merchant_with_http_post
                .unwrap_or(merchant_account.redirect_to_merchant_with_http_post),
            webhook_details: webhook_details.or(merchant_account.webhook_details),
            metadata: request.metadata,
            routing_algorithm: Some(serde_json::json!({
                "algorithm_id": null,
                "timestamp": 0
            })),
            intent_fulfillment_time: request
                .intent_fulfillment_time
                .map(i64::from)
                .or(merchant_account.intent_fulfillment_time)
                .or(Some(common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME)),
            frm_routing_algorithm: request
                .frm_routing_algorithm
                .or(merchant_account.frm_routing_algorithm),
            #[cfg(feature = "payouts")]
            payout_routing_algorithm: request
                .payout_routing_algorithm
                .or(merchant_account.payout_routing_algorithm),
            #[cfg(not(feature = "payouts"))]
            payout_routing_algorithm: None,
            is_recon_enabled: merchant_account.is_recon_enabled,
            applepay_verified_domains: request.applepay_verified_domains,
            payment_link_config: payment_link_config_value,
            session_expiry: request
                .session_expiry
                .map(i64::from)
                .or(Some(common_utils::consts::DEFAULT_SESSION_EXPIRY)),
            authentication_connector_details: request
                .authentication_connector_details
                .as_ref()
                .map(Encode::encode_to_value)
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "authentication_connector_details",
                })?,
            payout_link_config: request
                .payout_link_config
                .as_ref()
                .map(Encode::encode_to_value)
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payout_link_config",
                })?,
            is_connector_agnostic_mit_enabled: request.is_connector_agnostic_mit_enabled,
            is_extended_card_info_enabled: None,
            extended_card_info_config: None,
            use_billing_as_payment_method_billing: request
                .use_billing_as_payment_method_billing
                .or(Some(true)),
            collect_shipping_details_from_wallet_connector: request
                .collect_shipping_details_from_wallet_connector
                .or(Some(false)),
            collect_billing_details_from_wallet_connector: request
                .collect_billing_details_from_wallet_connector
                .or(Some(false)),
        })
    }
}
