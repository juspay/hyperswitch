use std::collections::HashMap;

use common_utils::custom_serde;
use error_stack::report;
use literally::hmap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    core::errors::{self, RouterResult},
    pii::{self, Secret},
    types::storage::enums,
};

/// Static collection that contains valid Payment Method Type and Payment Method SubType
/// tuples. Used for validation.
static PAYMENT_METHOD_TYPE_SET: Lazy<
    HashMap<enums::PaymentMethodType, Vec<enums::PaymentMethodSubType>>,
> = Lazy::new(|| {
    use enums::{PaymentMethodSubType as ST, PaymentMethodType as T};

    hmap! {
        T::Card => vec![
            ST::Credit,
            ST::Debit
        ],
        T::BankTransfer => vec![],
        T::Netbanking => vec![],
        T::Upi => vec![
            ST::UpiIntent,
            ST::UpiCollect
        ],
        T::OpenBanking => vec![],
        T::ConsumerFinance => vec![],
        T::Wallet => vec![]
    }
});

/// Static collection that contains valid Payment Method Issuer and Payment Method Issuer
/// Type tuples. Used for validation.
static PAYMENT_METHOD_ISSUER_SET: Lazy<
    HashMap<enums::PaymentMethodType, Vec<enums::PaymentMethodIssuerCode>>,
> = Lazy::new(|| {
    use enums::{PaymentMethodIssuerCode as IC, PaymentMethodType as T};

    hmap! {
        T::Card => vec![
            IC::JpHdfc,
            IC::JpIcici,
        ],
        T::Upi => vec![
            IC::JpGooglepay,
            IC::JpPhonepay
        ],
        T::Netbanking => vec![
            IC::JpSofort,
            IC::JpGiropay
        ],
        T::Wallet => vec![
            IC::JpApplepay,
            IC::JpGooglepay,
            IC::JpWechat
        ],
        T::BankTransfer => vec![
            IC::JpSepa,
            IC::JpBacs
        ]
    }
});

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentMethod {
    pub merchant_id: Option<String>,
    pub payment_method: enums::PaymentMethodType,
    pub payment_method_type: Option<enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<enums::PaymentMethodIssuerCode>,
    pub card: Option<CardDetail>,
    pub metadata: Option<serde_json::Value>,
    pub customer_id: Option<String>,
}

impl CreatePaymentMethod {
    pub fn validate(&self) -> RouterResult<()> {
        let pm_subtype_map = Lazy::get(&PAYMENT_METHOD_TYPE_SET)
            .unwrap_or_else(|| Lazy::force(&PAYMENT_METHOD_TYPE_SET));
        if !Self::check_subtype_mapping(
            pm_subtype_map,
            self.payment_method,
            self.payment_method_type,
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_type' provided.".to_string()
            })
            .attach_printable("Invalid payment method type"));
        }

        let issuer_map = Lazy::get(&PAYMENT_METHOD_ISSUER_SET)
            .unwrap_or_else(|| Lazy::force(&PAYMENT_METHOD_ISSUER_SET));
        if !Self::check_subtype_mapping(
            issuer_map,
            self.payment_method,
            self.payment_method_issuer_code,
        ) {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid 'payment_method_issuer_code' provided.".to_string()
            })
            .attach_printable("Invalid payment method issuer code"));
        }

        Ok(())
    }

    fn check_subtype_mapping<T, U>(
        dict: &HashMap<T, Vec<U>>,
        the_type: T,
        the_subtype: Option<U>,
    ) -> bool
    where
        T: std::cmp::Eq + std::hash::Hash,
        U: std::cmp::PartialEq,
    {
        let the_subtype = match the_subtype {
            Some(st) => st,
            None => return true,
        };

        dict.get(&the_type)
            .map(|subtypes| subtypes.contains(&the_subtype))
            .unwrap_or(true)
    }
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
    pub payment_method: enums::PaymentMethodType,
    pub payment_method_type: Option<enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<enums::PaymentMethodIssuerCode>,
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
    #[serde(default, with = "custom_serde::iso8601::option")]
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
    pub accepted_currencies: Option<Vec<enums::Currency>>,
    pub amount: Option<i32>,
    pub recurring_enabled: Option<bool>,
    pub installment_payment_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPaymentMethodResponse {
    pub payment_method: enums::PaymentMethodType,
    pub payment_method_types: Option<Vec<enums::PaymentMethodSubType>>,
    pub payment_method_issuers: Option<Vec<String>>,
    pub payment_method_issuer_code: Option<Vec<enums::PaymentMethodIssuerCode>>,
    pub payment_schemes: Option<Vec<String>>,
    pub accepted_countries: Option<Vec<String>>,
    pub accepted_currencies: Option<Vec<enums::Currency>>,
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
    pub payment_method: enums::PaymentMethodType,
    pub payment_method_type: Option<enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<enums::PaymentMethodIssuerCode>,
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
    #[serde(default, with = "custom_serde::iso8601::option")]
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

#[derive(Debug, Serialize, Deserialize, router_derive::DebugAsDisplay)]
pub struct TokenizePayloadRequest {
    pub value1: String,
    pub value2: String,
    pub lookup_key: String,
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize, router_derive::DebugAsDisplay)]
pub struct GetTokenizePayloadRequest {
    pub lookup_key: String,
    pub get_value2: bool,
}

#[derive(Debug, Serialize, router_derive::DebugAsDisplay)]
pub struct DeleteTokenizeByTokenRequest {
    pub lookup_key: String,
}

#[derive(Debug, Serialize)] //FIXME yet to be implemented
pub struct DeleteTokenizeByDateRequest {
    pub buffer_minutes: i32,
    pub service_name: String,
    pub max_rows: i32,
}

#[derive(Debug, Deserialize, router_derive::DebugAsDisplay)]
pub struct GetTokenizePayloadResponse {
    pub lookup_key: String,
    pub get_value2: Option<bool>,
}
#[derive(Debug, Serialize, Deserialize, router_derive::DebugAsDisplay)]
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

#[derive(Debug, Serialize, Deserialize, router_derive::DebugAsDisplay)]
#[serde(rename_all = "camelCase")]

pub struct TokenizedCardValue2 {
    pub card_security_code: Option<String>,
    pub card_fingerprint: Option<String>,
    pub external_id: Option<String>,
    pub customer_id: Option<String>,
}
