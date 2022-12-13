use common_utils::pii;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::enums as api_enums;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentMethod {
    pub merchant_id: Option<String>,
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,
    pub card: Option<CardDetail>,
    pub metadata: Option<serde_json::Value>,
    pub customer_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CardDetail {
    pub card_number: Secret<String, pii::CardNumber>,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaymentMethodResponse {
    pub payment_method_id: String,
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,
    pub card: Option<CardDetailFromLocker>,
    //TODO: Populate this on request?
    // pub accepted_country: Option<Vec<String>>,
    // pub accepted_currency: Option<Vec<enums::Currency>>,
    // pub minimum_amount: Option<i32>,
    // pub maximum_amount: Option<i32>,
    pub recurring_enabled: bool,
    pub installment_payment_enabled: bool,
    pub payment_experience: Option<Vec<String>>, //TODO change it to enum
    pub metadata: Option<serde_json::Value>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CardDetailFromLocker {
    pub scheme: Option<String>,
    pub issuer_country: Option<String>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    pub card_number: Option<Secret<String, pii::CardNumber>>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    pub card_token: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_fingerprint: Option<Secret<String>>,
}

//List Payment Method
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListPaymentMethodRequest {
    pub accepted_countries: Option<Vec<String>>,
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,
    pub amount: Option<i32>,
    pub recurring_enabled: Option<bool>,
    pub installment_payment_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPaymentMethodResponse {
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_types: Option<Vec<api_enums::PaymentMethodSubType>>,
    pub payment_method_issuers: Option<Vec<String>>,
    pub payment_method_issuer_code: Option<Vec<api_enums::PaymentMethodIssuerCode>>,
    pub payment_schemes: Option<Vec<String>>,
    pub accepted_countries: Option<Vec<String>>,
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,
    pub minimum_amount: Option<i32>,
    pub maximum_amount: Option<i32>,
    pub recurring_enabled: bool,
    pub installment_payment_enabled: bool,
    pub payment_experience: Option<Vec<String>>, //TODO change it to enum
}

#[derive(Debug, Serialize)]
pub struct ListCustomerPaymentMethodsResponse {
    pub enabled_payment_methods: Vec<ListPaymentMethodResponse>,
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
}

#[derive(Debug, Serialize)]
pub struct DeletePaymentMethodResponse {
    pub payment_method_id: String,
    pub deleted: bool,
}

#[derive(Debug, Serialize)]
pub struct CustomerPaymentMethod {
    pub payment_token: String,
    pub customer_id: String,
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,
    //TODO: Populate this on request?
    // pub accepted_country: Option<Vec<String>>,
    // pub accepted_currency: Option<Vec<enums::Currency>>,
    // pub minimum_amount: Option<i32>,
    // pub maximum_amount: Option<i32>,
    pub recurring_enabled: bool,
    pub installment_payment_enabled: bool,
    pub payment_experience: Option<Vec<String>>, //TODO change it to enum
    pub card: Option<CardDetailFromLocker>,
    pub metadata: Option<serde_json::Value>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethodId {
    pub payment_method_id: String,
}

//------------------------------------------------TokenizeService------------------------------------------------
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizePayloadEncrypted {
    pub payload: String,
    pub key_id: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizePayloadRequest {
    pub value1: String,
    pub value2: String,
    pub lookup_key: String,
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTokenizePayloadRequest {
    pub lookup_key: String,
    pub get_value2: bool,
}

#[derive(Debug, Serialize)]
pub struct DeleteTokenizeByTokenRequest {
    pub lookup_key: String,
}

#[derive(Debug, Serialize)] //FIXME yet to be implemented
pub struct DeleteTokenizeByDateRequest {
    pub buffer_minutes: i32,
    pub service_name: String,
    pub max_rows: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetTokenizePayloadResponse {
    pub lookup_key: String,
    pub get_value2: Option<bool>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue1 {
    pub card_number: String,
    pub exp_year: String,
    pub exp_month: String,
    pub name_on_card: Option<String>,
    pub nickname: Option<String>,
    pub card_last_four: Option<String>,
    pub card_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue2 {
    pub card_security_code: Option<String>,
    pub card_fingerprint: Option<String>,
    pub external_id: Option<String>,
    pub customer_id: Option<String>,
}
