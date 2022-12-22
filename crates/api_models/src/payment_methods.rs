use common_utils::pii;
use serde::de;

use crate::enums as api_enums;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentMethod {
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,
    pub card: Option<CardDetail>,
    pub metadata: Option<serde_json::Value>,
    pub customer_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UpdatePaymentMethod {
    pub card: Option<CardDetail>,
    // Add more payment method update field in future
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CardDetail {
    pub card_number: masking::Secret<String, pii::CardNumber>,
    pub card_exp_month: masking::Secret<String>,
    pub card_exp_year: masking::Secret<String>,
    pub card_holder_name: Option<masking::Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentMethodResponse {
    pub merchant_id: String,
    pub customer_id: Option<String>,
    pub payment_method_id: String,
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,
    pub card: Option<CardDetailFromLocker>,
    pub recurring_enabled: bool,
    pub installment_payment_enabled: bool,
    pub payment_experience: Option<Vec<PaymentExperience>>,
    pub metadata: Option<serde_json::Value>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct CardDetailFromLocker {
    pub scheme: Option<String>,
    pub issuer_country: Option<String>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    pub card_number: Option<masking::Secret<String, pii::CardNumber>>,
    pub expiry_month: Option<masking::Secret<String>>,
    pub expiry_year: Option<masking::Secret<String>>,
    pub card_token: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
    pub card_fingerprint: Option<masking::Secret<String>>,
}

//List Payment Method
#[derive(Debug, serde::Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListPaymentMethodRequest {
    pub client_secret: Option<String>,
    pub accepted_countries: Option<Vec<String>>,
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,
    pub amount: Option<i32>,
    pub recurring_enabled: Option<bool>,
    pub installment_payment_enabled: Option<bool>,
}

impl<'de> serde::Deserialize<'de> for ListPaymentMethodRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> de::Visitor<'de> for FieldVisitor {
            type Value = ListPaymentMethodRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("Failed while deserializing as map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut output = ListPaymentMethodRequest::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        "client_secret" => {
                            set_or_reject_duplicate(
                                &mut output.client_secret,
                                "client_secret",
                                map.next_value()?,
                            )?;
                        }
                        "accepted_countries" => match output.accepted_countries.as_mut() {
                            Some(inner) => inner.push(map.next_value()?),
                            None => {
                                output.accepted_countries = Some(vec![map.next_value()?]);
                            }
                        },
                        "accepted_currencies" => match output.accepted_currencies.as_mut() {
                            Some(inner) => inner.push(map.next_value()?),
                            None => {
                                output.accepted_currencies = Some(vec![map.next_value()?]);
                            }
                        },
                        "amount" => {
                            set_or_reject_duplicate(
                                &mut output.amount,
                                "amount",
                                map.next_value()?,
                            )?;
                        }
                        "recurring_enabled" => {
                            set_or_reject_duplicate(
                                &mut output.recurring_enabled,
                                "recurring_enabled",
                                map.next_value()?,
                            )?;
                        }
                        "installment_payment_enabled" => {
                            set_or_reject_duplicate(
                                &mut output.installment_payment_enabled,
                                "installment_payment_enabled",
                                map.next_value()?,
                            )?;
                        }
                        _ => {}
                    }
                }

                Ok(output)
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

// Try to set the provided value to the data otherwise throw an error
fn set_or_reject_duplicate<T, E: de::Error>(
    data: &mut Option<T>,
    name: &'static str,
    value: T,
) -> Result<(), E> {
    match data {
        Some(_inner) => Err(de::Error::duplicate_field(name)),
        None => {
            *data = Some(value);
            Ok(())
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
    pub payment_experience: Option<Vec<PaymentExperience>>,
}

#[derive(Debug, serde::Serialize)]
pub struct ListCustomerPaymentMethodsResponse {
    pub enabled_payment_methods: Vec<ListPaymentMethodResponse>,
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
}

#[derive(Debug, serde::Serialize)]
pub struct DeletePaymentMethodResponse {
    pub payment_method_id: String,
    pub deleted: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct CustomerPaymentMethod {
    pub payment_token: String,
    pub customer_id: String,
    pub payment_method: api_enums::PaymentMethodType,
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,
    pub recurring_enabled: bool,
    pub installment_payment_enabled: bool,
    pub payment_experience: Option<Vec<PaymentExperience>>,
    pub card: Option<CardDetailFromLocker>,
    pub metadata: Option<serde_json::Value>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PaymentExperience {
    RedirectToUrl,
    InvokeSdkClient,
    DisplayQrCode,
    OneClick,
    LinkWallet,
    InvokePaymentApp,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodId {
    pub payment_method_id: String,
}

//------------------------------------------------TokenizeService------------------------------------------------
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizePayloadEncrypted {
    pub payload: String,
    pub key_id: String,
    pub version: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizePayloadRequest {
    pub value1: String,
    pub value2: String,
    pub lookup_key: String,
    pub service_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GetTokenizePayloadRequest {
    pub lookup_key: String,
    pub get_value2: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct DeleteTokenizeByTokenRequest {
    pub lookup_key: String,
}

#[derive(Debug, serde::Serialize)] // Blocked: Yet to be implemented by `basilisk`
pub struct DeleteTokenizeByDateRequest {
    pub buffer_minutes: i32,
    pub service_name: String,
    pub max_rows: i32,
}

#[derive(Debug, serde::Deserialize)]
pub struct GetTokenizePayloadResponse {
    pub lookup_key: String,
    pub get_value2: Option<bool>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCardValue2 {
    pub card_security_code: Option<String>,
    pub card_fingerprint: Option<String>,
    pub external_id: Option<String>,
    pub customer_id: Option<String>,
}
