use std::collections::HashMap;

use common_utils::{
    consts,
    crypto::{Encryptable, OptionalEncryptableName},
    pii,
};
use masking::Secret;
use serde::{Deserialize, Serialize};
use url;
use utoipa::ToSchema;

use super::payments::AddressDetails;
use crate::{
    enums,
    enums::{self as api_enums},
    payment_methods,
};

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
pub struct MerchantAccountListRequest {
    pub organization_id: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantAccountCreate {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// Name of the Merchant Account
    #[schema(value_type= Option<String>,example = "NewAge Retailer")]
    pub merchant_name: Option<Secret<String>>,

    /// Details about the merchant
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[serde(skip)]
    #[schema(deprecated)]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be  used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub parent_merchant_id: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a default value is used.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled.
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<MerchantAccountMetadata>,

    /// API key that will be used for client side API access. A publishable key has to be always paired with a `client_secret`.
    /// A `client_secret` can be obtained by creating a payment with `confirm` set to false
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    /// Details about the primary business unit of the merchant account
    #[schema(value_type = Option<PrimaryBusinessDetails>)]
    pub primary_business_details: Option<Vec<PrimaryBusinessDetails>>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The id of the organization to which the merchant belongs to
    pub organization_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuthenticationConnectorDetails {
    /// List of authentication connectors
    #[schema(value_type = Vec<AuthenticationConnectors>)]
    pub authentication_connectors: Vec<enums::AuthenticationConnectors>,
    /// URL of the (customer service) website that will be shown to the shopper in case of technical errors during the 3D Secure 2 process.
    pub three_ds_requestor_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MerchantAccountMetadata {
    pub compatible_connector: Option<api_enums::Connector>,

    #[serde(flatten)]
    pub data: Option<pii::SecretSerdeValue>,
}
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantAccountUpdate {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// Name of the Merchant Account
    #[schema(example = "NewAge Retailer")]
    pub merchant_name: Option<String>,

    /// Details about the merchant
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[serde(skip)]
    #[schema(deprecated)]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    #[schema(default = false, example = false)]
    pub sub_merchants_enabled: Option<bool>,

    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub parent_merchant_id: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a default value is used.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    /// Details about the primary business unit of the merchant account
    pub primary_business_details: Option<Vec<PrimaryBusinessDetails>>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The default business profile that must be used for creating merchant accounts and payments
    /// To unset this field, pass an empty string
    #[schema(max_length = 64)]
    pub default_profile: Option<String>,
}

#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct MerchantAccountResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// Name of the Merchant Account
    #[schema(value_type = Option<String>,example = "NewAge Retailer")]
    pub merchant_name: OptionalEncryptableName,

    /// The URL to redirect after completion of the payment
    #[schema(max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = false, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a default value is used.
    #[schema(max_length = 255, example = "xkkdf909012sdjki2dkh5sdf")]
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Details about the merchant
    #[schema(value_type = Option<MerchantDetails>)]
    pub merchant_details: Option<Encryptable<pii::SecretSerdeValue>>,

    /// Webhook related details
    #[schema(value_type = Option<WebhookDetails>)]
    pub webhook_details: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[serde(skip)]
    #[schema(deprecated)]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

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
    pub metadata: Option<pii::SecretSerdeValue>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    /// Details about the primary business unit of the merchant account
    #[schema(value_type = Vec<PrimaryBusinessDetails>)]
    pub primary_business_details: Vec<PrimaryBusinessDetails>,

    /// The frm routing algorithm to be used to process the incoming request from merchant to outgoing payment FRM.
    #[schema(value_type = Option<RoutingAlgorithm>, max_length = 255, example = r#"{"type": "single", "data": "stripe" }"#)]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    pub intent_fulfillment_time: Option<i64>,

    /// The organization id merchant is associated with
    pub organization_id: String,

    ///  A boolean value to indicate if the merchant has recon service is enabled or not, by default value is false
    pub is_recon_enabled: bool,

    /// The default business profile that must be used for creating merchant accounts and payments
    #[schema(max_length = 64)]
    pub default_profile: Option<String>,

    /// Used to indicate the status of the recon module for a merchant account
    #[schema(value_type = ReconStatus, example = "not_requested")]
    pub recon_status: enums::ReconStatus,
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
    pub primary_email: Option<pii::Email>,

    /// The merchant's secondary contact name
    #[schema(value_type = Option<String>, max_length= 255, example = "John Doe2")]
    pub secondary_contact_person: Option<Secret<String>>,

    /// The merchant's secondary phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "999999988")]
    pub secondary_phone: Option<Secret<String>>,

    /// The merchant's secondary email address
    #[schema(value_type = Option<String>, max_length = 255, example = "johndoe2@test.com")]
    pub secondary_email: Option<pii::Email>,

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
#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrimaryBusinessDetails {
    #[schema(value_type = CountryAlpha2)]
    pub country: api_enums::CountryAlpha2,
    #[schema(example = "food")]
    pub business: String,
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
pub struct MerchantAccountDeleteResponse {
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

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorCreate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: api_enums::Connector,
    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is `default`, connector label can be `stripe_default`
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Identifier for the business profile, if not provided default will be chosen from merchant account
    pub profile_id: Option<String>,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
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
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// The business country to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    pub business_label: Option<String>,

    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR")]
    pub merchant_connector_id: Option<String>,

    pub pm_auth_config: Option<serde_json::Value>,

    #[schema(value_type = Option<ConnectorStatus>, example = "inactive")]
    pub status: Option<api_enums::ConnectorStatus>,
}

// Different patterns of authentication.
#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "auth_type")]
pub enum ConnectorAuthType {
    TemporaryAuth,
    HeaderKey {
        api_key: Secret<String>,
    },
    BodyKey {
        api_key: Secret<String>,
        key1: Secret<String>,
    },
    SignatureKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
    },
    MultiAuthKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
        key2: Secret<String>,
    },
    CurrencyAuthKey {
        auth_key_map: HashMap<common_enums::Currency, pii::SecretSerdeValue>,
    },
    CertificateAuth {
        certificate: Secret<String>,
        private_key: Secret<String>,
    },
    #[default]
    NoKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorWebhookDetails {
    #[schema(value_type = String, example = "12345678900987654321")]
    pub merchant_secret: Secret<String>,
    #[schema(value_type = String, example = "12345678900987654321")]
    pub additional_secret: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MerchantConnectorInfo {
    pub connector_label: String,
    pub merchant_connector_id: String,
}

/// Response of creating a new Merchant Connector for the merchant account."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorResponse {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(value_type = Connector, example = "stripe")]
    pub connector_name: String,

    /// A unique label to identify the connector account created under a business profile
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the merchant connector account
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR")]
    pub merchant_connector_id: String,

    /// Identifier for the business profile, if not provided default will be chosen from merchant account
    #[schema(max_length = 64)]
    pub profile_id: Option<String>,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: pii::SecretSerdeValue,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
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
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// The business country to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    ///The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "travel")]
    pub business_label: Option<String>,

    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// identifier for the verified domains of a particular connector account
    pub applepay_verified_domains: Option<Vec<String>>,

    pub pm_auth_config: Option<serde_json::Value>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: api_enums::ConnectorStatus,
}

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorUpdate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is `default`, connector label can be `stripe_default`
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// An object containing the required details/credentials for a Connector account.
    #[schema(value_type = Option<MerchantConnectorDetails>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

    /// An object containing the details about the payment methods that need to be enabled under this merchant connector account
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
            "accepted_currencies": {
                "type": "enable_only",
                "list": ["USD", "EUR"]
            },
            "accepted_countries": {
                "type": "disable_only",
                "list": ["FR", "DE","IN"]
            },
            "minimum_amount": 1,
            "maximum_amount": 68607706,
            "recurring_enabled": true,
            "installment_payment_enabled": true
        }
    ]))]
    pub payment_methods_enabled: Option<Vec<PaymentMethodsEnabled>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    #[schema(default = false, example = false)]
    pub test_mode: Option<bool>,

    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    #[schema(default = false, example = false)]
    pub disabled: Option<bool>,

    /// Contains the frm configs for the merchant connector
    #[schema(example = json!(consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    pub pm_auth_config: Option<serde_json::Value>,

    #[schema(value_type = ConnectorStatus, example = "inactive")]
    pub status: Option<api_enums::ConnectorStatus>,
}

///Details of FrmConfigs are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmConfigs {
    ///this is the connector that can be used for the payment
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub gateway: Option<api_enums::Connector>,
    ///payment methods that can be used in the payment
    pub payment_methods: Vec<FrmPaymentMethod>,
}

///Details of FrmPaymentMethod are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmPaymentMethod {
    ///payment methods(card, wallet, etc) that can be used in the payment
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: Option<common_enums::PaymentMethod>,
    ///payment method types(credit, debit) that can be used in the payment. This field is deprecated. It has not been removed to provide backward compatibility.
    pub payment_method_types: Option<Vec<FrmPaymentMethodType>>,
    ///frm flow type to be used, can be pre/post
    #[schema(value_type = Option<FrmPreferredFlowTypes>)]
    pub flow: Option<api_enums::FrmPreferredFlowTypes>,
}

///Details of FrmPaymentMethodType are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FrmPaymentMethodType {
    ///payment method types(credit, debit) that can be used in the payment
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    ///card networks(like visa mastercard) types that can be used in the payment
    #[schema(value_type = CardNetwork)]
    pub card_networks: Option<Vec<common_enums::CardNetwork>>,
    ///frm flow type to be used, can be pre/post
    #[schema(value_type = FrmPreferredFlowTypes)]
    pub flow: api_enums::FrmPreferredFlowTypes,
    ///action that the frm would take, in case fraud is detected
    #[schema(value_type = FrmAction)]
    pub action: api_enums::FrmAction,
}
/// Details of all the payment methods enabled for the connector for the given merchant account
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodsEnabled {
    /// Type of payment method.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: common_enums::PaymentMethod,

    /// Subtype of payment method
    #[schema(value_type = Option<Vec<RequestPaymentMethodTypes>>,example = json!(["credit"]))]
    pub payment_method_types: Option<Vec<payment_methods::RequestPaymentMethodTypes>>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
pub enum AcceptedCurrencies {
    #[schema(value_type = Vec<Currency>)]
    EnableOnly(Vec<api_enums::Currency>),
    #[schema(value_type = Vec<Currency>)]
    DisableOnly(Vec<api_enums::Currency>),
    AllAccepted,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
/// Object to filter the customer countries for which the payment method is displayed
pub enum AcceptedCountries {
    #[schema(value_type = Vec<CountryAlpha2>)]
    EnableOnly(Vec<api_enums::CountryAlpha2>),
    #[schema(value_type = Vec<CountryAlpha2>)]
    DisableOnly(Vec<api_enums::CountryAlpha2>),
    AllAccepted,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MerchantConnectorDeleteResponse {
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
    #[serde(skip_deserializing)]
    pub merchant_id: String,
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleAllKVRequest {
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleAllKVResponse {
    ///Total number of updated merchants
    #[schema(example = 20)]
    pub total_updated: usize,
    /// Status of KV for the specific merchant
    #[schema(example = true)]
    pub kv_enabled: bool,
}
#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MerchantConnectorDetailsWrap {
    /// Creds Identifier is to uniquely identify the credentials. Do not send any sensitive info in this field. And do not send the string "null".
    pub creds_identifier: String,
    /// Merchant connector details type type. Base64 Encode the credentials and send it in  this type and send as a string.
    #[schema(value_type = Option<MerchantConnectorDetails>, example = r#"{
        "connector_account_details": {
            "auth_type": "HeaderKey",
            "api_key":"sk_test_xxxxxexamplexxxxxx12345"
        },
        "metadata": {
            "user_defined_field_1": "sample_1",
            "user_defined_field_2": "sample_2",
        },
    }"#)]
    pub encoded_data: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MerchantConnectorDetails {
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: pii::SecretSerdeValue,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessProfileCreate {
    /// The name of business profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a default value is used.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<u32>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified applepay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this business profile
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Default Payment Link config for all payment links created under this business profile
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    /// Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// A boolean value to indicate if cusomter shipping details needs to be sent for wallets payments
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
}

#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct BusinessProfileResponse {
    /// The identifier for Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// The default business profile that must be used for creating merchant accounts and payments
    #[schema(max_length = 64, example = "pro_abcdefghijklmnopqrstuvwxyz")]
    pub profile_id: String,

    /// Name of the business profile
    #[schema(max_length = 64)]
    pub profile_name: String,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a default value is used.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Webhook related details
    #[schema(value_type = Option<WebhookDetails>)]
    pub webhook_details: Option<pii::SecretSerdeValue>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<i64>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified applepay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this business profile
    #[schema(example = 900)]
    pub session_expiry: Option<i64>,

    /// Default Payment Link config for all payment links created under this business profile
    pub payment_link_config: Option<serde_json::Value>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    // Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessProfileUpdate {
    /// The name of business profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a default value is used.
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<u32>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified applepay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,

    /// Client Secret Default expiry for all payments created under this business profile
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Default Payment Link config for all payment links created under this business profile
    pub payment_link_config: Option<BusinessPaymentLinkConfig>,

    /// External 3DS authentication details
    pub authentication_connector_details: Option<AuthenticationConnectorDetails>,

    /// Merchant's config to support extended card info feature
    pub extended_card_info_config: Option<ExtendedCardInfoConfig>,

    // Whether to use the billing details passed when creating the intent as payment method billing
    pub use_billing_as_payment_method_billing: Option<bool>,

    /// A boolean value to indicate if cusomter shipping details needs to be sent for wallets payments
    pub collect_shipping_details_from_wallet_connector: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct BusinessPaymentLinkConfig {
    pub domain_name: Option<String>,
    #[serde(flatten)]
    pub config: PaymentLinkConfigRequest,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkConfigRequest {
    /// custom theme for the payment link
    #[schema(value_type = Option<String>, max_length = 255, example = "#4E6ADD")]
    pub theme: Option<String>,
    /// merchant display logo
    #[schema(value_type = Option<String>, max_length = 255, example = "https://i.pinimg.com/736x/4d/83/5c/4d835ca8aafbbb15f84d07d926fda473.jpg")]
    pub logo: Option<String>,
    /// Custom merchant name for payment link
    #[schema(value_type = Option<String>, max_length = 255, example = "hyperswitch")]
    pub seller_name: Option<String>,
    /// Custom layout for sdk
    #[schema(value_type = Option<String>, max_length = 255, example = "accordion")]
    pub sdk_layout: Option<String>,
    /// Display only the sdk for payment link
    #[schema(default = false, example = true)]
    pub display_sdk_only: Option<bool>,
    /// Enable saved payment method option for payment link
    #[schema(default = false, example = true)]
    pub enabled_saved_payment_method: Option<bool>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, ToSchema)]
pub struct PaymentLinkConfig {
    /// custom theme for the payment link
    pub theme: String,
    /// merchant display logo
    pub logo: String,
    /// Custom merchant name for payment link
    pub seller_name: String,
    /// Custom layout for sdk
    pub sdk_layout: String,
    /// Display only the sdk for payment link
    pub display_sdk_only: bool,
    /// Enable saved payment method option for payment link
    pub enabled_saved_payment_method: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ExtendedCardInfoChoice {
    pub enabled: bool,
}

impl common_utils::events::ApiEventMetric for ExtendedCardInfoChoice {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ConnectorAgnosticMitChoice {
    pub enabled: bool,
}

impl common_utils::events::ApiEventMetric for ConnectorAgnosticMitChoice {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ExtendedCardInfoConfig {
    /// Merchant public key
    #[schema(value_type = String)]
    pub public_key: Secret<String>,
    /// TTL for extended card info
    #[schema(default = 900, maximum = 7200, value_type = u16)]
    #[serde(default)]
    pub ttl_in_secs: TtlForExtendedCardInfo,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct TtlForExtendedCardInfo(u16);

impl Default for TtlForExtendedCardInfo {
    fn default() -> Self {
        Self(consts::DEFAULT_TTL_FOR_EXTENDED_CARD_INFO)
    }
}

impl<'de> Deserialize<'de> for TtlForExtendedCardInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;

        // Check if value exceeds the maximum allowed
        if value > consts::MAX_TTL_FOR_EXTENDED_CARD_INFO {
            Err(serde::de::Error::custom(
                "ttl_in_secs must be less than or equal to 7200 (2hrs)",
            ))
        } else {
            Ok(Self(value))
        }
    }
}

impl std::ops::Deref for TtlForExtendedCardInfo {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
