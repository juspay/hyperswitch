use std::collections::HashSet;

use common_utils::pii;
use serde::de;
use utoipa::ToSchema;

use crate::{admin, enums as api_enums, payments};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentMethod {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct PaymentExperienceTypes {
    pub payment_experience_type: api_enums::PaymentExperience,
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct CardNetworkTypes {
    pub card_network: api_enums::CardNetwork,
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct ResponsePaymentMethodTypes {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub payment_experience: Option<Vec<PaymentExperienceTypes>>,
    pub card_networks: Option<Vec<CardNetworkTypes>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ResponsePaymentMethodsEnabled {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_types: Vec<ResponsePaymentMethodTypes>,
}

#[derive(Clone)]
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

    /// Indicates whether the payment method is eligible for card netwotks
    #[schema(value_type = Option<Vec<CardNetwork>>, example = json!(["visa", "mastercard"]))]
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
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
    pub payment_methods: Vec<ResponsePaymentMethodsEnabled>,
}

// impl ResponsePaymentMethodTypes {
//     pub fn new(
//         pm_type: RequestPaymentMethodTypes,
//         connector: String,
//         payment_method: api_enums::PaymentMethod,
//     ) -> Self {
//         Self {
//             payment_method_type: pm_type.payment_method_type,
//             payment_experience: pm_type.payment_experience,
//             connector,
//             card_networks: pm_type.card_networks,
//             payment_method,
//         }
//     }
// }

#[derive(Eq, PartialEq, Hash, Debug, serde::Deserialize, ToSchema)]
pub struct ListPaymentMethod {
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
impl serde::Serialize for ListPaymentMethod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ListPaymentMethod", 4)?;
        state.serialize_field("payment_method", &self.payment_method)?;

        state.serialize_field("payment_method_types", &self.payment_method_types)?;

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
    pub data: WalletData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WalletData {
    GpayWallet(GpayWalletData),
    ApplePayWallet(ApplePayWalletData),
    PayPalWallet(PayPalWalletData),
    Paypal(PaypalRedirection),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaypalRedirection {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GpayWalletData {
    pub pm_type: String,
    pub description: String,
    pub info: GpayPaymentMethodInfo,
    pub tokenization_data: GpayTokenizationData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GpayPaymentMethodInfo {
    pub card_network: String,
    pub card_details: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GpayTokenizationData {
    pub token_type: String,
    pub token: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApplePayWalletData {
    pub payment_data: ApplepayPaymentData,
    pub payment_method: ApplepayPaymentMethod,
    pub transaction_identifier: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApplepayPaymentData {
    pub data: String,
    pub signature: String,
    pub header: ApplepayHeader,
    pub version: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApplepayHeader {
    pub public_key_hash: String,
    pub ephemeral_public_key: String,
    pub transaction_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApplepayPaymentMethod {
    pub display_name: String,
    pub network: String,
    #[serde(rename = "type")]
    pub pm_type: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PayPalWalletData {
    pub token: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue2 {
    pub customer_id: Option<String>,
}

impl From<payments::WalletData> for WalletData {
    fn from(value: payments::WalletData) -> Self {
        match value {
            payments::WalletData::GooglePay(data) => Self::GpayWallet(data.into()),
            payments::WalletData::ApplePay(data) => Self::ApplePayWallet(data.into()),
            payments::WalletData::PaypalSdk(data) => Self::PayPalWallet(data.into()),
            payments::WalletData::PaypalRedirect(_) => Self::Paypal(PaypalRedirection {}),
        }
    }
}

impl From<GpayWalletData> for payments::GpayWalletData {
    fn from(value: GpayWalletData) -> Self {
        Self {
            pm_type: value.pm_type,
            description: value.description,
            info: value.info.into(),
            tokenization_data: value.tokenization_data.into(),
        }
    }
}

impl From<ApplePayWalletData> for payments::ApplePayWalletData {
    fn from(value: ApplePayWalletData) -> Self {
        Self {
            payment_data: value.payment_data.into(),
            payment_method: value.payment_method.into(),
            transaction_identifier: value.transaction_identifier,
        }
    }
}

impl From<ApplepayPaymentMethod> for payments::ApplepayPaymentMethod {
    fn from(value: ApplepayPaymentMethod) -> Self {
        Self {
            display_name: value.display_name,
            network: value.network,
            pm_type: value.pm_type,
        }
    }
}

impl From<ApplepayPaymentData> for payments::ApplepayPaymentData {
    fn from(value: ApplepayPaymentData) -> Self {
        Self {
            data: value.data,
            signature: value.signature,
            header: value.header.into(),
            version: value.version,
        }
    }
}

impl From<ApplepayHeader> for payments::ApplepayHeader {
    fn from(value: ApplepayHeader) -> Self {
        Self {
            public_key_hash: value.public_key_hash,
            ephemeral_public_key: value.ephemeral_public_key,
            transaction_id: value.transaction_id,
        }
    }
}

impl From<PayPalWalletData> for payments::PayPalWalletData {
    fn from(value: PayPalWalletData) -> Self {
        Self { token: value.token }
    }
}

impl From<GpayPaymentMethodInfo> for payments::GpayPaymentMethodInfo {
    fn from(value: GpayPaymentMethodInfo) -> Self {
        Self {
            card_network: value.card_network,
            card_details: value.card_details,
        }
    }
}

impl From<GpayTokenizationData> for payments::GpayTokenizationData {
    fn from(value: GpayTokenizationData) -> Self {
        Self {
            token_type: value.token_type,
            token: value.token,
        }
    }
}

impl From<payments::GpayWalletData> for GpayWalletData {
    fn from(value: payments::GpayWalletData) -> Self {
        Self {
            pm_type: value.pm_type,
            description: value.description,
            info: value.info.into(),
            tokenization_data: value.tokenization_data.into(),
        }
    }
}

impl From<payments::ApplePayWalletData> for ApplePayWalletData {
    fn from(value: payments::ApplePayWalletData) -> Self {
        Self {
            payment_data: value.payment_data.into(),
            payment_method: value.payment_method.into(),
            transaction_identifier: value.transaction_identifier,
        }
    }
}

impl From<payments::ApplepayPaymentMethod> for ApplepayPaymentMethod {
    fn from(value: payments::ApplepayPaymentMethod) -> Self {
        Self {
            display_name: value.display_name,
            network: value.network,
            pm_type: value.pm_type,
        }
    }
}

impl From<payments::ApplepayPaymentData> for ApplepayPaymentData {
    fn from(value: payments::ApplepayPaymentData) -> Self {
        Self {
            data: value.data,
            signature: value.signature,
            header: value.header.into(),
            version: value.version,
        }
    }
}

impl From<payments::ApplepayHeader> for ApplepayHeader {
    fn from(value: payments::ApplepayHeader) -> Self {
        Self {
            public_key_hash: value.public_key_hash,
            ephemeral_public_key: value.ephemeral_public_key,
            transaction_id: value.transaction_id,
        }
    }
}

impl From<payments::PayPalWalletData> for PayPalWalletData {
    fn from(value: payments::PayPalWalletData) -> Self {
        Self { token: value.token }
    }
}

impl From<payments::GpayPaymentMethodInfo> for GpayPaymentMethodInfo {
    fn from(value: payments::GpayPaymentMethodInfo) -> Self {
        Self {
            card_network: value.card_network,
            card_details: value.card_details,
        }
    }
}

impl From<payments::GpayTokenizationData> for GpayTokenizationData {
    fn from(value: payments::GpayTokenizationData) -> Self {
        Self {
            token_type: value.token_type,
            token: value.token,
        }
    }
}

impl From<WalletData> for payments::WalletData {
    fn from(value: WalletData) -> Self {
        match value {
            WalletData::GpayWallet(data) => Self::GooglePay(data.into()),
            WalletData::ApplePayWallet(data) => Self::ApplePay(data.into()),
            WalletData::PayPalWallet(data) => Self::PaypalSdk(data.into()),
            WalletData::Paypal(_) => Self::PaypalRedirect(payments::PaypalRedirection {}),
        }
    }
}
