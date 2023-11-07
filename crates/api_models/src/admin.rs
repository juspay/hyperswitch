use common_utils::{
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

    /// Merchant related details
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be  used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    #[serde(
        default,
        deserialize_with = "payout_routing_algorithm::deserialize_option"
    )]
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

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response
    /// If the value is not provided, a default value is used
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: Option<bool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "city": "NY", "unit": "245" }"#)]
    pub metadata: Option<MerchantAccountMetadata>,

    /// API key that will be used for server side API access
    #[schema(example = "AH3423bkjbkjdsfbkj")]
    pub publishable_key: Option<String>,

    /// An identifier for the vault used to store payment method information.
    #[schema(example = "locker_abc123")]
    pub locker_id: Option<String>,

    ///Default business details for connector routing
    #[schema(value_type = Option<PrimaryBusinessDetails>)]
    pub primary_business_details: Option<Vec<PrimaryBusinessDetails>>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    #[schema(example = 900)]
    pub intent_fulfillment_time: Option<u32>,

    /// The id of the organization to which the merchant belongs to
    pub organization_id: Option<String>,

    pub payment_link_config: Option<PaymentLinkConfig>,
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

    /// Merchant related details
    pub merchant_details: Option<MerchantDetails>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// Webhook related details
    pub webhook_details: Option<WebhookDetails>,

    /// The routing algorithm to be used for routing payments to desired connectors
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "stripe"}))]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    #[serde(
        default,
        deserialize_with = "payout_routing_algorithm::deserialize_option"
    )]
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

    /// Refers to the hash key used for payment response
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

    ///Default business details for connector routing
    pub primary_business_details: Option<Vec<PrimaryBusinessDetails>>,

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    ///Will be used to expire client secret after certain amount of time to be supplied in seconds
    ///(900) for 15 mins
    pub intent_fulfillment_time: Option<u32>,

    /// The default business profile that must be used for creating merchant accounts and payments
    /// To unset this field, pass an empty string
    #[schema(max_length = 64)]
    pub default_profile: Option<String>,

    pub payment_link_config: Option<serde_json::Value>,
}

#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct MerchantAccountResponse {
    /// The identifier for the Merchant Account
    #[schema(max_length = 255, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// Name of the Merchant Account
    #[schema(value_type = Option<String>,example = "NewAge Retailer")]
    pub merchant_name: OptionalEncryptableName,

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
    pub merchant_details: Option<Encryptable<pii::SecretSerdeValue>>,

    /// Webhook related details
    #[schema(value_type = Option<WebhookDetails>)]
    pub webhook_details: Option<serde_json::Value>,

    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    #[schema(value_type = Option<RoutingAlgorithm>, max_length = 255, example = "custom")]
    pub routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    #[serde(
        default,
        deserialize_with = "payout_routing_algorithm::deserialize_option"
    )]
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
    ///Default business details for connector routing
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

    /// A enum value to indicate the status of recon service. By default it is not_requested.
    #[schema(value_type = ReconStatus, example = "not_requested")]
    pub recon_status: enums::ReconStatus,

    pub payment_link_config: Option<serde_json::Value>,
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
#[cfg(feature = "payouts")]
pub mod payout_routing_algorithm {
    use std::{fmt, str::FromStr};

    use serde::{
        de::{self, Visitor},
        Deserializer,
    };
    use serde_json::Map;

    use super::PayoutRoutingAlgorithm;
    use crate::enums::PayoutConnectors;
    struct RoutingAlgorithmVisitor;
    struct OptionalRoutingAlgorithmVisitor;

    impl<'de> Visitor<'de> for RoutingAlgorithmVisitor {
        type Value = serde_json::Value;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("routing algorithm")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut output = serde_json::Value::Object(Map::new());
            let mut routing_data: String = "".to_string();
            let mut routing_type: String = "".to_string();

            while let Some(key) = map.next_key()? {
                match key {
                    "type" => {
                        routing_type = map.next_value()?;
                        output["type"] = serde_json::Value::String(routing_type.to_owned());
                    }
                    "data" => {
                        routing_data = map.next_value()?;
                        output["data"] = serde_json::Value::String(routing_data.to_owned());
                    }
                    f => {
                        output[f] = map.next_value()?;
                    }
                }
            }

            match routing_type.as_ref() {
                "single" => {
                    let routable_payout_connector = PayoutConnectors::from_str(&routing_data);
                    let routable_conn = match routable_payout_connector {
                        Ok(rpc) => Ok(rpc),
                        Err(_) => Err(de::Error::custom(format!(
                            "Unknown payout connector {routing_data}"
                        ))),
                    }?;
                    Ok(PayoutRoutingAlgorithm::Single(routable_conn))
                }
                u => Err(de::Error::custom(format!("Unknown routing algorithm {u}"))),
            }?;
            Ok(output)
        }
    }

    impl<'de> Visitor<'de> for OptionalRoutingAlgorithmVisitor {
        type Value = Option<serde_json::Value>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("routing algorithm")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer
                .deserialize_any(RoutingAlgorithmVisitor)
                .map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'a, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(RoutingAlgorithmVisitor)
    }

    pub(crate) fn deserialize_option<'a, D>(
        deserializer: D,
    ) -> Result<Option<serde_json::Value>, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_option(OptionalRoutingAlgorithmVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrimaryBusinessDetails {
    #[schema(value_type = CountryAlpha2)]
    pub country: api_enums::CountryAlpha2,
    #[schema(example = "food")]
    pub business: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PaymentLinkConfig {
    pub merchant_logo: Option<String>,
    pub color_scheme: Option<PaymentLinkColorSchema>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]

pub struct PaymentLinkColorSchema {
    pub primary_color: Option<String>,
    pub primary_accent_color: Option<String>,
    pub secondary_color: Option<String>,
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
    /// Connector label for a connector, this can serve as a field to identify the connector as per business details
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR")]
    pub merchant_connector_id: Option<String>,
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,
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
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// contains the frm configs for the merchant connector
    #[schema(example = json!(common_utils::consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    pub business_label: Option<String>,

    /// Business Sub label of the merchant
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,
    /// Identifier for the business profile, if not provided default will be chosen from merchant account
    pub profile_id: Option<String>,

    pub pm_auth_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorWebhookDetails {
    #[schema(value_type = String, example = "12345678900987654321")]
    pub merchant_secret: Secret<String>,
    #[schema(value_type = String, example = "12345678900987654321")]
    pub additional_secret: Option<Secret<String>>,
}

/// Response of creating a new Merchant Connector for the merchant account."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorResponse {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,
    /// Name of the Connector
    #[schema(example = "stripe")]
    pub connector_name: String,

    /// Connector label for a connector, this can serve as a field to identify the connector as per business details
    #[schema(example = "stripe_US_travel")]
    pub connector_label: Option<String>,

    /// Unique ID of the connector
    #[schema(example = "mca_5apGeP94tMts6rg3U3kR")]
    pub merchant_connector_id: String,
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: pii::SecretSerdeValue,
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
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Business Country of the connector
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    ///Business Type of the merchant
    #[schema(example = "travel")]
    pub business_label: Option<String>,

    /// Business Sub label of the merchant
    #[schema(example = "chase")]
    pub business_sub_label: Option<String>,

    /// contains the frm configs for the merchant connector
    #[schema(example = json!(common_utils::consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    /// The business profile this connector must be created in
    /// default value from merchant account is taken if not passed
    #[schema(max_length = 64)]
    pub profile_id: Option<String>,
    /// identifier for the verified domains of a particular connector account
    pub applepay_verified_domains: Option<Vec<String>>,

    pub pm_auth_config: Option<serde_json::Value>,
}

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantConnectorUpdate {
    /// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
    #[schema(value_type = ConnectorType, example = "payment_processor")]
    pub connector_type: api_enums::ConnectorType,

    /// Connector label for a connector, this can serve as a field to identify the connector as per business details
    pub connector_label: Option<String>,

    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "auth_type": "HeaderKey","api_key": "Basic MyVerySecretApiKey" }))]
    pub connector_account_details: Option<pii::SecretSerdeValue>,

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

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,max_length = 255,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// contains the frm configs for the merchant connector
    #[schema(example = json!(common_utils::consts::FRM_CONFIGS_EG))]
    pub frm_configs: Option<Vec<FrmConfigs>>,

    /// Webhook details of this merchant connector
    #[schema(example = json!({
        "connector_webhook_details": {
            "merchant_secret": "1234567890987654321"
        }
    }))]
    pub connector_webhook_details: Option<MerchantConnectorWebhookDetails>,

    pub pm_auth_config: Option<serde_json::Value>,
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
    ///payment method types(credit, debit) that can be used in the payment
    pub payment_method_types: Vec<FrmPaymentMethodType>,
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
    ///frm flow type to be used...can be pre/post
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
    #[schema(value_type = Option<Vec<PaymentMethodType>>,example = json!(["credit"]))]
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

#[cfg(feature = "payouts")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum PayoutRoutingAlgorithm {
    Single(api_enums::PayoutConnectors),
}

#[cfg(feature = "payouts")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum PayoutStraightThroughAlgorithm {
    Single(api_enums::PayoutConnectors),
}

#[derive(Clone, Debug, Deserialize, ToSchema, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessProfileCreate {
    /// A short name to identify the business profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation, This will be applied to all the
    /// connector accounts under this profile
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response
    /// If the value is not provided, a default value is used
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

    /// The routing algorithm to be  used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    #[serde(
        default,
        deserialize_with = "payout_routing_algorithm::deserialize_option"
    )]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified applepay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,
}

#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct BusinessProfileResponse {
    /// The identifier for Merchant Account
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// The unique identifier for Business Profile
    #[schema(max_length = 64, example = "pro_abcdefghijklmnopqrstuvwxyz")]
    pub profile_id: String,

    /// A short name to identify the business profile
    #[schema(max_length = 64)]
    pub profile_name: String,

    /// The URL to redirect after the completion of the operation, This will be applied to all the
    /// connector accounts under this profile
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<String>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: bool,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response
    /// If the value is not provided, a default value is used
    pub payment_response_hash_key: Option<String>,

    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    #[schema(default = false, example = true)]
    pub redirect_to_merchant_with_http_post: bool,

    /// Webhook related details
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

    /// The frm routing algorithm to be used for routing payments to desired FRM's
    #[schema(value_type = Option<Object>,example = json!({"type": "single", "data": "signifyd"}))]
    pub frm_routing_algorithm: Option<serde_json::Value>,

    /// The routing algorithm to be  used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    #[serde(
        default,
        deserialize_with = "payout_routing_algorithm::deserialize_option"
    )]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified applepay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessProfileUpdate {
    /// A short name to identify the business profile
    #[schema(max_length = 64)]
    pub profile_name: Option<String>,

    /// The URL to redirect after the completion of the operation, This will be applied to all the
    /// connector accounts under this profile
    #[schema(value_type = Option<String>, max_length = 255, example = "https://www.example.com/success")]
    pub return_url: Option<url::Url>,

    /// A boolean value to indicate if payment response hash needs to be enabled
    #[schema(default = true, example = true)]
    pub enable_payment_response_hash: Option<bool>,

    /// Refers to the hash key used for calculating the signature for webhooks and redirect response
    /// If the value is not provided, a default value is used
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

    /// The routing algorithm to be  used for routing payouts to desired connectors
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<RoutingAlgorithm>,example = json!({"type": "single", "data": "wise"}))]
    #[serde(
        default,
        deserialize_with = "payout_routing_algorithm::deserialize_option"
    )]
    pub payout_routing_algorithm: Option<serde_json::Value>,

    /// Verified applepay domains for a particular profile
    pub applepay_verified_domains: Option<Vec<String>>,
}
