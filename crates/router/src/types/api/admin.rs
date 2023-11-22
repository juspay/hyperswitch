pub use api_models::admin::{
    payout_routing_algorithm, BusinessProfileCreate, BusinessProfileResponse,
    BusinessProfileUpdate, MerchantAccountCreate, MerchantAccountDeleteResponse,
    MerchantAccountResponse, MerchantAccountUpdate, MerchantConnectorCreate,
    MerchantConnectorDeleteResponse, MerchantConnectorDetails, MerchantConnectorDetailsWrap,
    MerchantConnectorId, MerchantConnectorResponse, MerchantDetails, MerchantId,
    PaymentMethodsEnabled, PayoutRoutingAlgorithm, PayoutStraightThroughAlgorithm, ToggleKVRequest,
    ToggleKVResponse, WebhookDetails,
};
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use masking::Secret;

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
            intent_fulfillment_time: item.intent_fulfillment_time,
            payout_routing_algorithm: item.payout_routing_algorithm,
            organization_id: item.organization_id,
            is_recon_enabled: item.is_recon_enabled,
            default_profile: item.default_profile,
            recon_status: item.recon_status,
            payment_link_config: item.payment_link_config,
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
            payout_routing_algorithm: item.payout_routing_algorithm,
            applepay_verified_domains: item.applepay_verified_domains,
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
                common_utils::ext_traits::Encode::<WebhookDetails>::encode_to_value(webhook_details)
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "webhook details",
                    })
            })
            .transpose()?;

        let payment_response_hash_key = request
            .payment_response_hash_key
            .or(merchant_account.payment_response_hash_key)
            .unwrap_or(common_utils::crypto::generate_cryptographically_secure_random_string(64));

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
            routing_algorithm: request
                .routing_algorithm
                .or(merchant_account.routing_algorithm),
            intent_fulfillment_time: request
                .intent_fulfillment_time
                .map(i64::from)
                .or(merchant_account.intent_fulfillment_time),
            frm_routing_algorithm: request
                .frm_routing_algorithm
                .or(merchant_account.frm_routing_algorithm),
            payout_routing_algorithm: request
                .payout_routing_algorithm
                .or(merchant_account.payout_routing_algorithm),
            is_recon_enabled: merchant_account.is_recon_enabled,
            applepay_verified_domains: request.applepay_verified_domains,
        })
    }
}
