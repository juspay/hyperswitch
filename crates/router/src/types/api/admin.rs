pub use api_models::admin::{
    CreateMerchantAccount, CustomRoutingRules, DeleteMcaResponse, DeleteResponse,
    MerchantConnectorId, MerchantDetails, MerchantId, PaymentConnectorCreate, PaymentMethods,
    WebhookDetails,
};

//use serde::{Serialize, Deserialize};
pub use self::CreateMerchantAccount as MerchantAccountResponse;

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
