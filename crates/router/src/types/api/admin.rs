pub use api_models::admin::{
    MerchantAccountCreate, MerchantAccountDeleteResponse, MerchantAccountResponse,
    MerchantAccountUpdate, MerchantConnector, MerchantConnectorDeleteResponse,
    MerchantConnectorDetails, MerchantConnectorDetailsWrap, MerchantConnectorId, MerchantDetails,
    MerchantId, PaymentMethodsEnabled, RoutingAlgorithm, ToggleKVRequest, ToggleKVResponse,
    WebhookDetails,
};

use crate::types::{storage, transformers::ForeignFrom};

impl ForeignFrom<storage::MerchantAccount> for MerchantAccountResponse {
    fn foreign_from(value: storage::MerchantAccount) -> Self {
        let item = value;
        Self {
            merchant_id: item.merchant_id,
            merchant_name: item.merchant_name,
            api_key: item.api_key,
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
        }
    }
}
