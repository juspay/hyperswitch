pub use api_models::admin::{
    payout_routing_algorithm, BusinessProfileCreate, BusinessProfileResponse,
    BusinessProfileUpdate, MerchantAccountCreate, MerchantAccountDeleteResponse,
    MerchantAccountResponse, MerchantAccountUpdate, MerchantConnectorCreate,
    MerchantConnectorDeleteResponse, MerchantConnectorDetails, MerchantConnectorDetailsWrap,
    MerchantConnectorId, MerchantConnectorResponse, MerchantDetails, MerchantId,
    PaymentMethodsEnabled, PayoutRoutingAlgorithm, PayoutStraightThroughAlgorithm,
    RoutingAlgorithm, StraightThroughAlgorithm, ToggleKVRequest, ToggleKVResponse, WebhookDetails,
};
use common_utils::ext_traits::ValueExt;
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
        })
    }
}
