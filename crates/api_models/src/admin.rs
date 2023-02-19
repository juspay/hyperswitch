use common_utils::pii;
use masking::{Secret, StrongSecret};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::payments::AddressDetails;
use crate::enums as api_enums;

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateMerchantAccount {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// Name of the Merchant Account
    #[schema(example = "NewAge Retailer")]
    pub merchant_name: Option<String>,

    /// API key that will be used for server side API access
    #[schema(value_type = Option<String>, example = "Ah2354543543523")]
    pub api_key: Option<StrongSecret<String>>,

    /// Merchant related details
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub parent_merchant_id: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for payment response
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<serde_json::Value>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,
}

#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct MerchantAccountResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// Name of the Merchant Account
    #[schema(example = "NewAge Retailer")]
    pub merchant_name: Option<String>,

    /// API key that will be used for server side API access
    #[schema(value_type = Option<String>, example = "Ah2354543543523")]
    pub api_key: Option<StrongSecret<String>>,

    /// The URL to redirect after the completion of the operation
    #[schema(max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Merchant related details
    #[schema(value_type = Option<MerchantDetails>)]
    pub merchant_details: Option<serde_json::Value>,

    /// Webhook related details
    #[schema(value_type = Option<WebhookDetails>)]
    pub webhook_details: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[schema(value_type = Option<RoutingAlgorithm>, max_length = 255, example = "custom")]
    pub routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub parent_merchant_id: Option<String>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<serde_json::Value>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantDetails {
    /// The merchant's primary contact name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Doe")]
    pub primary_contact_person: Option<Secret<String>>,

    /// The merchant's primary phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "999999999")]
    pub primary_phone: Option<Secret<String>>,

    /// The merchant's primary email address
    #[schema(value_type = Option<String>, max_length = 255, example = "johndoe@test.com")]
    pub primary_email: Option<Secret<String, pii::Email>>,

    /// The merchant's secondary contact name
    #[schema(value_type = Option<String>, max_length= 255, example = "John Doe2")]
    pub secondary_contact_person: Option<Secret<String>>,

    /// The merchant's secondary phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "999999988")]
    pub secondary_phone: Option<Secret<String>>,

    /// The merchant's secondary email address
    #[schema(value_type = Option<String>, max_length = 255, example = "johndoe2@test.com")]
    pub secondary_email: Option<Secret<String, pii::Email>>,

    /// The business website of the merchant
    #[schema(max_length = 255, example = "www.example.com")]
    pub website: Option<String>,

    /// A brief description about merchant's business
    #[schema(
        max_length = 255,
        example = "Online Retail with a wide selection of organic products for North America"
    )]
    pub about_business: Option<String>,

    /// The merchant's address details
    pub address: Option<AddressDetails>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RoutingAlgorithm {
    Single(api_enums::RoutableConnectors),
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookDetails {
    ///The version for Webhook
    #[schema(max_length = 255, max_length = 255, example = "1.0.2")]
    pub webhook_version: Option<String>,

    ///The user name for Webhook login
    #[schema(max_length = 255, max_length = 255, example = "ekart_retail")]
    pub webhook_username: Option<String>,

    ///The password for Webhook login
    #[schema(value_type = Option<String>, max_length = 255, example = "ekart@123")]
    pub webhook_password: Option<Secret<String>>,

    ///The url for the webhook endpoint
    #[schema(value_type = Option<String>, example = "www.ekart.com/webhooks")]
    pub webhook_url: Option<Secret<String>>,

    /// If this property is true, a webhook message is posted whenever a new payment is created
    #[schema(example = true)]
    pub payment_created_enabled: Option<bool>,

    /// If this property is true, a webhook message is posted whenever a payment is successful
    #[schema(example = true)]
    pub payment_succeeded_enabled: Option<bool>,

    /// If this property is true, a webhook message is posted whenever a payment fails
    #[schema(example = true)]
    pub payment_failed_enabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteMerchantAccountResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,
    /// If the connector is deleted or not
    #[schema(example = false)]
    pub deleted: bool,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MerchantId {
    pub merchant_id: String,
}

#[derive(Default, Debug, Deserialize, ToSchema, Serialize)]
pub struct MerchantConnectorId {
    pub merchant_id: String,
    pub merchant_connector_id: String,
}

/// Create a new Payment Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentConnectorCreate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(example = "stripe")]
    pub connector_name: String,
    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR")]
    pub merchant_connector_id: Option<String>,
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<Secret<serde_json::Value>>,
    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,
    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,
    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(example = json!([
        {
            "payment_method": "wallet",
            "payment_method_types": [
                "upi_collect",
                "upi_intent"
            ],
            "payment_method_issuers": [
                "labore magna ipsum",
                "aute"
            ],
            "payment_schemes": [
                "Discover",
                "Discover"
            ],
            "accepted_currencies": [
                "AED",
                "AED"
            ],
            "accepted_countries": [
                "in",
                "us"
            ],
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethods>>,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,
}
/// Details of all the payment methods enabled for the connector for the given merchant account
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethods {
    /// Type of payment method.
    #[schema(value_type = PaymentMethodType,example = "card")]
    pub payment_method: api_enums::PaymentMethodType,
    /// Subtype of payment method
    #[schema(value_type = Option<Vec<PaymentMethodSubType>>,example = json!(["credit"]))]
    pub payment_method_types: Option<Vec<api_enums::PaymentMethodSubType>>,
    /// List of payment method issuers to be enabled for this payment method
    #[schema(example = json!(["HDFC"]))]
    pub payment_method_issuers: Option<Vec<String>>,
    /// List of payment schemes accepted or has the processing capabilities of the processor
    #[schema(example = json!(["MASTER","VISA","DINERS"]))]
    pub payment_schemes: Option<Vec<String>>,
    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(value_type = Option<Vec<Currency>>,example = json!(["USD","EUR","AED"]))]
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,
    ///  List of Countries accepted or has the processing capabilities of the processor
    #[schema(example = json!(["US","IN"]))]
    pub accepted_countries: Option<Vec<String>>,
    /// Minimum amount supported by the processor. To be represented in the lowest denomination of the target currency (For example, for USD it should be in cents)
    #[schema(example = 1)]
    pub minimum_amount: Option<i32>,
    /// Maximum amount supported by the processor. To be represented in the lowest denomination of
    /// the target currency (For example, for USD it should be in cents)
    #[schema(example = 1313)]
    pub maximum_amount: Option<i32>,
    /// Boolean to enable recurring payments / mandates. Default is true.
    #[schema(default = true, example = false)]
    pub recurring_enabled: bool,
    /// Boolean to enable installment / EMI / BNPL payments. Default is true.
    #[schema(default = true, example = false)]
    pub installment_payment_enabled: bool,
    /// Type of payment experience enabled with the connector
    #[schema(value_type = Option<Vec<PaymentExperience>>,example = json!(["redirect_to_url"]))]
    pub payment_experience: Option<Vec<api_enums::PaymentExperience>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteMcaResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,
    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR")]
    pub merchant_connector_id: String,
    /// If the connector is deleted or not
    #[schema(example = false)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleKVResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleKVRequest {
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}
