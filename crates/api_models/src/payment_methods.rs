use std::collections::HashSet;

use common_utils::pii;
use serde::de;
use utoipa::ToSchema;

use crate::enums as api_enums;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentMethod {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethodType,example = "card")]
    pub payment_method: api_enums::PaymentMethodType,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodSubType>,example = "credit_card")]
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,

    /// The name of the bank/ provider issuing the payment method to the end user
    #[schema(example = "Citibank")]
    pub payment_method_issuer: Option<String>,

    /// A standard code representing the issuer of payment method
    #[schema(value_type = Option<PaymentMethodIssuerCode>,example = "jp_applepay")]
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,

    /// Card Details
    #[schema(example = json!({
    "card_number": "4111111145551142",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "John Doe"}))]
    pub card: Option<CardDetail>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,

    /// The unique identifier of the customer.
    #[schema(example = "cus_meowerunwiuwiwqw")]
    pub customer_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdatePaymentMethod {
    /// Card Details
    #[schema(example = json!({
    "card_number": "4111111145551142",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "John Doe"}))]
    pub card: Option<CardDetail>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardDetail {
    /// Card Number
    #[schema(value_type = String,example = "4111111145551142")]
    pub card_number: masking::Secret<String, pii::CardNumber>,

    /// Card Expiry Month
    #[schema(value_type = String,example = "10")]
    pub card_exp_month: masking::Secret<String>,

    /// Card Expiry Year
    #[schema(value_type = String,example = "25")]
    pub card_exp_year: masking::Secret<String>,

    /// Card Holder Name
    #[schema(value_type = String,example = "John Doe")]
    pub card_holder_name: Option<masking::Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentMethodResponse {
    /// Unique identifier for a merchant
    #[schema(example = "merchant_1671528864")]
    pub merchant_id: String,

    /// The unique identifier of the customer.
    #[schema(example = "cus_meowerunwiuwiwqw")]
    pub customer_id: Option<String>,

    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIy")]
    pub payment_method_id: String,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethodType,example = "card")]
    pub payment_method: api_enums::PaymentMethodType,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodSubType>,example = "credit_card")]
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,

    /// The name of the bank/ provider issuing the payment method to the end user
    #[schema(example = "Citibank")]
    pub payment_method_issuer: Option<String>,

    /// A standard code representing the issuer of payment method
    #[schema(value_type = Option<PaymentMethodIssuerCode>,example = "jp_applepay")]
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,

    /// Card details from card locker
    #[schema(example = json!({"last4": "1142","exp_month": "03","exp_year": "2030"}))]
    pub card: Option<CardDetailFromLocker>,

    /// Indicates whether the payment method is eligible for recurring payments
    #[schema(example = true)]
    pub recurring_enabled: bool,

    /// Indicates whether the payment method is eligible for installment payments
    #[schema(example = true)]
    pub installment_payment_enabled: bool,

    /// Type of payment experience enabled with the connector
    #[schema(value_type = Option<Vec<PaymentExperience>>,example = json!(["redirect_to_url"]))]
    pub payment_experience: Option<Vec<api_enums::PaymentExperience>>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,

    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = Option<PrimitiveDateTime>,example = "2023-01-18T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct CardDetailFromLocker {
    pub scheme: Option<String>,
    pub issuer_country: Option<String>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    #[schema(value_type=Option<String>)]
    pub card_number: Option<masking::Secret<String, pii::CardNumber>>,

    #[schema(value_type=Option<String>)]
    pub expiry_month: Option<masking::Secret<String>>,

    #[schema(value_type=Option<String>)]
    pub expiry_year: Option<masking::Secret<String>>,

    #[schema(value_type=Option<String>)]
    pub card_token: Option<masking::Secret<String>>,

    #[schema(value_type=Option<String>)]
    pub card_holder_name: Option<masking::Secret<String>>,

    #[schema(value_type=Option<String>)]
    pub card_fingerprint: Option<masking::Secret<String>>,
}

//List Payment Method
#[derive(Debug, serde::Serialize, Default, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListPaymentMethodRequest {
    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    #[schema(max_length = 30, min_length = 30, example = "secret_k2uj3he2893ein2d")]
    pub client_secret: Option<String>,

    /// The two-letter ISO currency code
    #[schema(example = json!(["US", "UK", "IN"]))]
    pub accepted_countries: Option<Vec<String>>,

    /// The three-letter ISO currency code
    #[schema(value_type = Option<Vec<Currency>>,example = json!(["USD", "EUR"]))]
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,

    /// Filter by amount
    #[schema(example = 60)]
    pub amount: Option<i64>,

    /// Indicates whether the payment method is eligible for recurring payments
    #[schema(example = true)]
    pub recurring_enabled: Option<bool>,

    /// Indicates whether the payment method is eligible for installment payments
    #[schema(example = true)]
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

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ListPaymentMethodResponse {
    /// Redirect URL of the merchant
    #[schema(example = "https://www.google.com")]
    pub redirect_url: Option<String>,

    /// Information about the payment method
    #[schema(value_type = Vec<ListPaymentMethod>,example = json!(
    [
        {
            "payment_method": "wallet",
            "payment_experience": null,
            "payment_method_issuers": [
                "labore magna ipsum",
                "aute"
            ]
        }
    ]
    ))]
    pub payment_methods: HashSet<ListPaymentMethod>,
}

#[derive(Eq, PartialEq, Hash, Debug, serde::Deserialize, ToSchema)]
pub struct ListPaymentMethod {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethodType,example = "card")]
    pub payment_method: api_enums::PaymentMethodType,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<Vec<PaymentMethodSubType>>,example = json!(["credit_card"]))]
    pub payment_method_types: Option<Vec<api_enums::PaymentMethodSubType>>,

    /// The name of the bank/ provider issuing the payment method to the end user
    #[schema(example = json!(["Citibank"]))]
    pub payment_method_issuers: Option<Vec<String>>,

    /// A standard code representing the issuer of payment method
    #[schema(value_type = Option<Vec<PaymentMethodIssuerCode>>,example = json!(["jp_applepay"]))]
    pub payment_method_issuer_code: Option<Vec<api_enums::PaymentMethodIssuerCode>>,

    /// List of payment schemes accepted or has the processing capabilities of the processor
    #[schema(example = json!(["MASTER", "VISA", "DINERS"]))]
    pub payment_schemes: Option<Vec<String>>,

    /// List of Countries accepted or has the processing capabilities of the processor
    #[schema(example = json!(["US", "UK", "IN"]))]
    pub accepted_countries: Option<Vec<String>>,

    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(value_type = Option<Vec<Currency>>,example = json!(["USD", "EUR"]))]
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,

    /// Minimum amount supported by the processor. To be represented in the lowest denomination of
    /// the target currency (For example, for USD it should be in cents)
    #[schema(example = 60000)]
    pub minimum_amount: Option<i64>,

    /// Maximum amount supported by the processor. To be represented in the lowest denomination of
    /// the target currency (For example, for USD it should be in cents)
    #[schema(example = 1)]
    pub maximum_amount: Option<i64>,

    /// Boolean to enable recurring payments / mandates. Default is true.
    #[schema(example = true)]
    pub recurring_enabled: bool,

    /// Boolean to enable installment / EMI / BNPL payments. Default is true.
    #[schema(example = true)]
    pub installment_payment_enabled: bool,

    /// Type of payment experience enabled with the connector
    #[schema(value_type = Option<Vec<PaymentExperience>>, example = json!(["redirect_to_url"]))]
    pub payment_experience: Option<Vec<api_enums::PaymentExperience>>,
}

/// We need a custom serializer to only send relevant fields in ListPaymentMethodResponse
/// Currently if the payment method is Wallet or Paylater the relevant fields are `payment_method`
/// and `payment_method_issuers`. Otherwise only consider
/// `payment_method`,`payment_method_issuers`,`payment_method_types`,`payment_schemes` fields.
impl serde::Serialize for ListPaymentMethod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ListPaymentMethod", 4)?;
        state.serialize_field("payment_method", &self.payment_method)?;
        state.serialize_field("payment_experience", &self.payment_experience)?;
        match self.payment_method {
            api_enums::PaymentMethodType::Wallet | api_enums::PaymentMethodType::PayLater => {
                state.serialize_field("payment_method_issuers", &self.payment_method_issuers)?;
            }
            _ => {
                state.serialize_field("payment_method_issuers", &self.payment_method_issuers)?;
                state.serialize_field("payment_method_types", &self.payment_method_types)?;
                state.serialize_field("payment_schemes", &self.payment_schemes)?;
            }
        }
        state.end()
    }
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ListCustomerPaymentMethodsResponse {
    /// List of enabled payment methods for a customer
    #[schema(value_type = Vec<ListPaymentMethod>,example = json!(
        [
            {
                "payment_method": "wallet",
                "payment_experience": null,
                "payment_method_issuers": [
                    "labore magna ipsum",
                    "aute"
                ]
            }
        ]
    ))]
    pub enabled_payment_methods: HashSet<ListPaymentMethod>,

    /// List of payment methods for customer
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct DeletePaymentMethodResponse {
    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIy")]
    pub payment_method_id: String,

    /// Whether payment method was deleted or not
    #[schema(example = true)]
    pub deleted: bool,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethod {
    /// Token for payment method in temporary card locker which gets refreshed often
    #[schema(example = "7ebf443f-a050-4067-84e5-e6f6d4800aef")]
    pub payment_token: String,

    /// The unique identifier of the customer.
    #[schema(example = "cus_meowerunwiuwiwqw")]
    pub customer_id: String,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethodType,example = "card")]
    pub payment_method: api_enums::PaymentMethodType,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodSubType>,example = "credit_card")]
    pub payment_method_type: Option<api_enums::PaymentMethodSubType>,

    /// The name of the bank/ provider issuing the payment method to the end user
    #[schema(example = "Citibank")]
    pub payment_method_issuer: Option<String>,

    /// A standard code representing the issuer of payment method
    #[schema(value_type = Option<PaymentMethodIssuerCode>,example = "jp_applepay")]
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,

    /// Indicates whether the payment method is eligible for recurring payments
    #[schema(example = true)]
    pub recurring_enabled: bool,

    /// Indicates whether the payment method is eligible for installment payments
    #[schema(example = true)]
    pub installment_payment_enabled: bool,

    /// Type of payment experience enabled with the connector
    #[schema(value_type = Option<Vec<PaymentExperience>>,example = json!(["redirect_to_url"]))]
    pub payment_experience: Option<Vec<api_enums::PaymentExperience>>,

    /// Card details from card locker
    #[schema(example = json!({"last4": "1142","exp_month": "03","exp_year": "2030"}))]
    pub card: Option<CardDetailFromLocker>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<serde_json::Value>,

    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = Option<PrimitiveDateTime>,example = "2023-01-18T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
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
    pub payment_method_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue1 {
    pub issuer: String,
    pub token: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue2 {
    pub customer_id: Option<String>,
}
