use common_utils::pii;
use serde::de;
use utoipa::ToSchema;

use crate::{
    admin, enums as api_enums,
    payments::{self, BankCodeResponse},
};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodCreate {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethodType,example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>,example = "credit")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

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
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The unique identifier of the customer.
    #[schema(example = "cus_meowerunwiuwiwqw")]
    pub customer_id: Option<String>,

    /// The card network
    #[schema(example = "Visa")]
    pub card_network: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodUpdate {
    /// Card Details
    #[schema(example = json!({
    "card_number": "4111111145551142",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "John Doe"}))]
    pub card: Option<CardDetail>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<CardNetwork>,example = "Visa")]
    pub card_network: Option<api_enums::CardNetwork>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
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
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>,example = "credit")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

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
    pub metadata: Option<pii::SecretSerdeValue>,

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct PaymentExperienceTypes {
    /// The payment experience enabled
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience_type: api_enums::PaymentExperience,

    /// The list of eligible connectors for a given payment experience
    #[schema(example = json!(["stripe", "adyen"]))]
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct CardNetworkTypes {
    /// The card network enabled
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    pub card_network: api_enums::CardNetwork,

    /// The list of eligible connectors for a given card network
    #[schema(example = json!(["stripe", "adyen"]))]
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct ResponsePaymentMethodTypes {
    /// The payment method type enabled
    #[schema(example = "klarna")]
    pub payment_method_type: api_enums::PaymentMethodType,

    /// The list of payment experiences enabled, if applicable for a payment method type
    pub payment_experience: Option<Vec<PaymentExperienceTypes>>,

    /// The list of card networks enabled, if applicable for a payment method type
    pub card_networks: Option<Vec<CardNetworkTypes>>,

    /// The list of banks enabled, if applicable for a payment method type
    pub bank_names: Option<Vec<BankCodeResponse>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ResponsePaymentMethodsEnabled {
    /// The payment method enabled
    #[schema(value_type = PaymentMethod)]
    pub payment_method: api_enums::PaymentMethod,

    /// The list of payment method types enabled for a connector account
    pub payment_method_types: Vec<ResponsePaymentMethodTypes>,
}

#[derive(Clone, Debug)]
pub struct ResponsePaymentMethodIntermediate {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub payment_experience: Option<api_enums::PaymentExperience>,
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
    pub payment_method: api_enums::PaymentMethod,
    pub connector: String,
}

impl ResponsePaymentMethodIntermediate {
    pub fn new(
        pm_type: RequestPaymentMethodTypes,
        connector: String,
        pm: api_enums::PaymentMethod,
    ) -> Self {
        Self {
            payment_method_type: pm_type.payment_method_type,
            payment_experience: pm_type.payment_experience,
            card_networks: pm_type.card_networks,
            payment_method: pm,
            connector,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub struct RequestPaymentMethodTypes {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub payment_experience: Option<api_enums::PaymentExperience>,
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(example = json!(
        {
        "enable_all":false,
        "disable_only": ["INR", "CAD", "AED","JPY"],
        "enable_only": ["EUR","USD"]
        }
    ))]
    pub accepted_currencies: Option<admin::AcceptedCurrencies>,

    ///  List of Countries accepted or has the processing capabilities of the processor
    #[schema(example = json!(
        {
            "enable_all":false,
            "disable_only": ["FR", "DE","IN"],
            "enable_only": ["UK","AU"]
        }
    ))]
    pub accepted_countries: Option<admin::AcceptedCountries>,

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
}

//List Payment Method
#[derive(Debug, serde::Serialize, Default, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodListRequest {
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

    /// Indicates whether the payment method is eligible for card netwotks
    #[schema(value_type = Option<Vec<CardNetwork>>, example = json!(["visa", "mastercard"]))]
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
}

impl<'de> serde::Deserialize<'de> for PaymentMethodListRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> de::Visitor<'de> for FieldVisitor {
            type Value = PaymentMethodListRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("Failed while deserializing as map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut output = PaymentMethodListRequest::default();

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
                        "card_network" => match output.card_networks.as_mut() {
                            Some(inner) => inner.push(map.next_value()?),
                            None => output.card_networks = Some(vec![map.next_value()?]),
                        },
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
pub struct PaymentMethodListResponse {
    /// Redirect URL of the merchant
    #[schema(example = "https://www.google.com")]
    pub redirect_url: Option<String>,

    /// Information about the payment method
    #[schema(value_type = Vec<PaymentMethodList>,example = json!(
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
    pub payment_methods: Vec<ResponsePaymentMethodsEnabled>,
}

#[derive(Eq, PartialEq, Hash, Debug, serde::Deserialize, ToSchema)]
pub struct PaymentMethodList {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<Vec<PaymentMethodType>>,example = json!(["credit"]))]
    pub payment_method_types: Option<Vec<RequestPaymentMethodTypes>>,
}

/// Currently if the payment method is Wallet or Paylater the relevant fields are `payment_method`
/// and `payment_method_issuers`. Otherwise only consider
/// `payment_method`,`payment_method_issuers`,`payment_method_types`,`payment_schemes` fields.
impl serde::Serialize for PaymentMethodList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PaymentMethodList", 4)?;
        state.serialize_field("payment_method", &self.payment_method)?;

        state.serialize_field("payment_method_types", &self.payment_method_types)?;

        state.end()
    }
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethodsListResponse {
    /// List of payment methods for customer
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentMethodDeleteResponse {
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
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>,example = "credit_card")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

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
    pub metadata: Option<pii::SecretSerdeValue>,

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
    pub service_name: String,
    pub get_value2: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DeleteTokenizeByTokenRequest {
    pub lookup_key: String,
    pub service_name: String,
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
    pub data: payments::WalletData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue2 {
    pub customer_id: Option<String>,
}
