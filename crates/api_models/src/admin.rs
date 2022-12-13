use common_utils::pii;
use masking::{Secret, StrongSecret};
use serde::{Deserialize, Serialize};

pub use self::CreateMerchantAccount as MerchantAccountResponse;
use super::payments::AddressDetails;
use crate::enums as api_enums;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreateMerchantAccount {
    pub merchant_id: String,
    pub merchant_name: Option<String>,
    pub api_key: Option<StrongSecret<String>>,
    pub merchant_details: Option<MerchantDetails>,
    pub return_url: Option<String>,
    pub webhook_details: Option<WebhookDetails>,
    pub routing_algorithm: Option<api_enums::RoutingAlgorithm>,
    pub custom_routing_rules: Option<Vec<CustomRoutingRules>>,
    pub sub_merchants_enabled: Option<bool>,
    pub parent_merchant_id: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub publishable_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantDetails {
    pub primary_contact_person: Option<Secret<String>>,
    pub primary_phone: Option<Secret<String>>,
    pub primary_email: Option<Secret<String, pii::Email>>,
    pub secondary_contact_person: Option<Secret<String>>,
    pub secondary_phone: Option<Secret<String>>,
    pub secondary_email: Option<Secret<String, pii::Email>>,
    pub website: Option<String>,
    pub about_business: Option<String>,
    pub address: Option<AddressDetails>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookDetails {
    pub webhook_version: Option<String>,
    pub webhook_username: Option<String>,
    pub webhook_password: Option<Secret<String>>,
    pub webhook_url: Option<Secret<String>>,
    pub payment_created_enabled: Option<bool>,
    pub payment_succeeded_enabled: Option<bool>,
    pub payment_failed_enabled: Option<bool>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRoutingRules {
    pub payment_methods_incl: Option<Vec<api_enums::PaymentMethodType>>, //FIXME Add enums for all PM enums
    pub payment_methods_excl: Option<Vec<api_enums::PaymentMethodType>>,
    pub payment_method_types_incl: Option<Vec<api_enums::PaymentMethodSubType>>,
    pub payment_method_types_excl: Option<Vec<api_enums::PaymentMethodSubType>>,
    pub payment_method_issuers_incl: Option<Vec<String>>,
    pub payment_method_issuers_excl: Option<Vec<String>>,
    pub countries_incl: Option<Vec<String>>,
    pub countries_excl: Option<Vec<String>>,
    pub currencies_incl: Option<Vec<api_enums::Currency>>,
    pub currencies_excl: Option<Vec<api_enums::Currency>>,
    pub metadata_filters_keys: Option<Vec<String>>,
    pub metadata_filters_values: Option<Vec<String>>,
    pub connectors_pecking_order: Option<Vec<String>>,
    pub connectors_traffic_weightage_key: Option<Vec<String>>,
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

#[derive(Default, Debug, Deserialize, Serialize)]
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
    pub payment_experience: Option<Vec<String>>, //TODO
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMcaResponse {
    pub merchant_id: String,
    pub merchant_connector_id: i32,
    pub deleted: bool,
}
