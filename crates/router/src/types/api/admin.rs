pub use api_models::admin::{
    CreateMerchantAccount, DeleteMcaResponse, DeleteMerchantAccountResponse,
    MerchantAccountResponse, MerchantConnectorId, MerchantDetails, MerchantId,
    PaymentConnectorCreate, PaymentMethods, RoutingAlgorithm, ToggleKVRequest, ToggleKVResponse,
    WebhookDetails,
};

use crate::types::{storage, transformers::Foreign};

impl From<Foreign<storage::MerchantAccount>> for Foreign<MerchantAccountResponse> {
    fn from(value: Foreign<storage::MerchantAccount>) -> Self {
        let item = value.0;
        MerchantAccountResponse {
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
        .into()
    }
}

//use serde::{Serialize, Deserialize};

//use crate::newtype;

//use api_models::admin;

//newtype!(
//pub CreateMerchantAccount = admin::CreateMerchantAccount,
//derives = (Clone, Debug, Deserialize, Serialize)
//);

//newtype!(
//pub MerchantDetails = admin::MerchantDetails,
//derives = (Clone, Debug, Deserialize, Serialize)
//);

//newtype!(
//pub WebhookDetails = admin::WebhookDetails,
//derives = (Clone, Debug, Deserialize, Serialize)
//);

//newtype!(
//pub CustomRoutingRules = admin::CustomRoutingRules,
//derives = (Default, Clone, Debug, Deserialize, Serialize)
//);

//newtype!(
//pub DeleteResponse = admin::DeleteResponse,
//derives = (Debug, Serialize)
//);

//newtype!(
//pub MerchantId = admin::MerchantId,
//derives = (Default, Debug, Deserialize, Serialize)
//);

//newtype!(
//pub MerchantConnectorId = admin::MerchantConnectorId,
//derives = (Default, Debug, Deserialize, Serialize)
//);

//newtype!(
//pub PaymentConnectorCreate = admin::PaymentConnectorCreate,
//derives = (Debug, Clone, Serialize, Deserialize)
//);

//newtype!(
//pub PaymentMethods = admin::PaymentMethods,
//derives = (Debug, Clone, Serialize, Deserialize)
//);

//newtype!(
//pub DeleteMcaResponse = admin::DeleteMcaResponse,
//derives = (Debug, Clone, Serialize, Deserialize)
//);
