use common_utils::pii;
use masking::{Secret, StrongSecret};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub use self::CreateMerchantAccount as MerchantAccountResponse;
use super::payments::AddressDetails;
use crate::{enums as api_enums, payment_methods};

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
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

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[schema(value_type = Option<RoutingAlgorithm>, max_length = 255, example = "custom")]
    pub routing_algorithm: Option<api_enums::RoutingAlgorithm>,

    /// The custom routing rules to be used for various payment methods and conditions
    pub custom_routing_rules: Option<Vec<CustomRoutingRules>>,

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

#[derive(Default, Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRoutingRules {
    /// The List of payment methods to include for this routing rule
    #[schema(value_type = Option<Vec<PaymentMethodType>>, example = json!(["card", "upi"]))]
    pub payment_methods_incl: Option<Vec<api_enums::PaymentMethodType>>,

    /// The List of payment methods to exclude for this routing rule. If there is conflict between include and exclude lists, include list overrides the exclude list.
    #[schema(value_type = Option<Vec<PaymentMethodType>>, example = json!(["card", "upi"]))]
    pub payment_methods_excl: Option<Vec<api_enums::PaymentMethodType>>,

    /// The List of payment method types to include for this routing rule
    #[schema(value_type = Option<Vec<PaymentMethodSubType>>, example = json!(["credit_card", "debit_card"]))]
    pub payment_method_types_incl: Option<Vec<api_enums::PaymentMethodSubType>>,

    /// The List of payment method types to exclude for this routing rule. If there is conflict between include and exclude lists, include list overrides the exclude list.
    #[schema(value_type = Option<Vec<PaymentMethodSubType>>, example = json!(["credit_card", "debit_card"]))]
    pub payment_method_types_excl: Option<Vec<api_enums::PaymentMethodSubType>>,

    /// The List of payment method issuers to include for this routing rule
    #[schema(example = json!(["Citibank", "JPMorgan"]))]
    pub payment_method_issuers_incl: Option<Vec<String>>,

    /// The List of payment method issuers to exclude for this routing rule. If there is conflict between include and exclude lists, include list overrides the exclude list.
    #[schema(example = json!(["Citibank", "JPMorgan"]))]
    pub payment_method_issuers_excl: Option<Vec<String>>,

    /// The List of countries to include for this routing rule
    #[schema(example = json!(["US", "UK"]))]
    pub countries_incl: Option<Vec<String>>,

    /// The List of countries to exclude for this routing rule. If there is conflict between include and exclude lists, include list overrides the exclude list.
    #[schema(example = json!(["US", "UK"]))]
    pub countries_excl: Option<Vec<String>>,

    /// The List of currencies to include for this routing rule
    #[schema(value_type = Option<Vec<Currency>>, example = json!(["EUR","USD"]))]
    pub currencies_incl: Option<Vec<api_enums::Currency>>,

    /// The List of currencies to exclude for this routing rule. If there is conflict between include and exclude lists, include list overrides the exclude list.
    #[schema(value_type = Option<Vec<Currency>>, example = json!(["EUR","USD"]))]
    pub currencies_excl: Option<Vec<api_enums::Currency>>,

    /// List of Metadata Filter keys to apply for the Routing Rule. The filters are presented as 2 arrays of keys and value. This property contains all the keys.
    #[schema(example = json!(["platform","Category"]))]
    pub metadata_filters_keys: Option<Vec<String>>,

    /// List of Metadata Filters to apply for the Routing Rule. The filters are presented as 2 arrays of keys and value. This property contains all the values.
    #[schema(example = json!(["android", "Category_Electronics"]))]
    pub metadata_filters_values: Option<Vec<String>>,

    /// The pecking order of payment connectors (or processors) to be used for routing. The first connector in the array will be attempted for routing. If it fails, the second connector will be used till the list is exhausted.
    #[schema(example = json!([ "stripe", "adyen", "brain_tree"]))]
    pub connectors_pecking_order: Option<Vec<String>>,

    ///An Array of Connectors (as Keys) with the associated percentage of traffic to be routed through the given connector (Expressed as an array of values)
    #[schema(example = json!([ "stripe", "adyen", "brain_tree"]))]
    pub connectors_traffic_weightage_key: Option<Vec<String>>,

    /// An Array of Weightage (expressed in percentage) that needs to be associated with the respective connectors (Expressed as an array of keys)
    #[schema(example = json!([ 50, 30, 20 ]))]
    pub connectors_traffic_weightage_value: Option<Vec<i32>>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub merchant_id: String,
    pub deleted: bool,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MerchantId {
    pub merchant_id: String,
}

#[derive(Default, Debug, Deserialize, ToSchema, Serialize)]
pub struct MerchantConnectorId {
    pub merchant_id: String,
    pub merchant_connector_id: i32,
}
//Merchant Connector Account CRUD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentConnectorCreate {
    pub connector_type: api_enums::ConnectorType,
    pub connector_name: String,
    pub merchant_connector_id: Option<i32>,
    pub connector_account_details: Option<Secret<serde_json::Value>>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub payment_methods_enabled: Option<Vec<PaymentMethods>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethods {
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_types: Option<Vec<api_enums::PaymentMethodSubType>>,
    pub payment_method_issuers: Option<Vec<String>>,
    pub payment_schemes: Option<Vec<String>>,
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,
    pub accepted_countries: Option<Vec<String>>,
    pub minimum_amount: Option<i32>,
    pub maximum_amount: Option<i32>,
    pub recurring_enabled: bool,
    pub installment_payment_enabled: bool,
    pub payment_experience: Option<Vec<payment_methods::PaymentExperience>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMcaResponse {
    pub merchant_id: String,
    pub merchant_connector_id: i32,
    pub deleted: bool,
}
