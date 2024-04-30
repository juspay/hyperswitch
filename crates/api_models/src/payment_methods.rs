use std::collections::{HashMap, HashSet};

use cards::CardNumber;
use common_utils::{
    consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH,
    crypto::OptionalEncryptableName,
    pii,
    types::{Percentage, Surcharge},
};
use serde::de;
use utoipa::{schema, ToSchema};

#[cfg(feature = "payouts")]
use crate::payouts;
use crate::{
    admin, enums as api_enums,
    payments::{self, BankCodeResponse},
};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodCreate {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
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

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<Bank>)]
    pub bank_transfer: Option<payouts::Bank>,

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<Wallet>)]
    pub wallet: Option<payouts::Wallet>,
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

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<Bank>)]
    pub bank_transfer: Option<payouts::Bank>,

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<Wallet>)]
    pub wallet: Option<payouts::Wallet>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardDetail {
    /// Card Number
    #[schema(value_type = String,example = "4111111145551142")]
    pub card_number: CardNumber,

    /// Card Expiry Month
    #[schema(value_type = String,example = "10")]
    pub card_exp_month: masking::Secret<String>,

    /// Card Expiry Year
    #[schema(value_type = String,example = "25")]
    pub card_exp_year: masking::Secret<String>,

    /// Card Holder Name
    #[schema(value_type = String,example = "John Doe")]
    pub card_holder_name: Option<masking::Secret<String>>,

    /// Card Holder's Nick Name
    #[schema(value_type = Option<String>,example = "John Doe")]
    pub nick_name: Option<masking::Secret<String>>,

    /// Card Issuing Country
    pub card_issuing_country: Option<String>,

    /// Card's Network
    #[schema(value_type = Option<CardNetwork>)]
    pub card_network: Option<api_enums::CardNetwork>,

    /// Issuer Bank for Card
    pub card_issuer: Option<String>,

    /// Card Type
    pub card_type: Option<String>,
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
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub payment_method_id: String,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>, example = "credit")]
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
    #[schema(value_type = Option<Vec<PaymentExperience>>, example = json!(["redirect_to_url"]))]
    pub payment_experience: Option<Vec<api_enums::PaymentExperience>>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = Option<PrimitiveDateTime>, example = "2023-01-18T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<Bank>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_transfer: Option<payouts::Bank>,

    #[schema(value_type = Option<PrimitiveDateTime>, example = "2024-02-24T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<time::PrimitiveDateTime>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PaymentMethodsData {
    Card(CardDetailsPaymentMethod),
    BankDetails(PaymentMethodDataBankCreds),
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<String>,
    pub expiry_month: Option<masking::Secret<String>>,
    pub expiry_year: Option<masking::Secret<String>>,
    pub nick_name: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodDataBankCreds {
    pub mask: String,
    pub hash: String,
    pub account_type: Option<String>,
    pub account_name: Option<String>,
    pub payment_method_type: api_enums::PaymentMethodType,
    pub connector_details: Vec<BankAccountConnectorDetails>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BankAccountTokenData {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub payment_method: api_enums::PaymentMethod,
    pub connector_details: BankAccountConnectorDetails,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BankAccountConnectorDetails {
    pub connector: String,
    pub account_id: masking::Secret<String>,
    pub mca_id: String,
    pub access_token: BankAccountAccessCreds,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BankAccountAccessCreds {
    AccessToken(masking::Secret<String>),
}
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct CardDetailFromLocker {
    pub scheme: Option<String>,
    pub issuer_country: Option<String>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    #[schema(value_type=Option<String>)]
    pub card_number: Option<CardNumber>,

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

    #[schema(value_type=Option<String>)]
    pub nick_name: Option<masking::Secret<String>>,

    #[schema(value_type = Option<CardNetwork>)]
    pub card_network: Option<api_enums::CardNetwork>,

    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_type: Option<String>,
    pub saved_to_locker: bool,
}

fn saved_in_locker_default() -> bool {
    true
}

impl From<CardDetailFromLocker> for payments::AdditionalCardInfo {
    fn from(item: CardDetailFromLocker) -> Self {
        Self {
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            card_issuing_country: item.issuer_country,
            bank_code: None,
            last4: item.last4_digits,
            card_isin: item.card_isin,
            card_extended_bin: item
                .card_number
                .map(|card_number| card_number.get_card_extended_bin()),
            card_exp_month: item.expiry_month,
            card_exp_year: item.expiry_year,
            card_holder_name: item.card_holder_name,
            payment_checks: None,
            authentication_data: None,
        }
    }
}

impl From<CardDetailsPaymentMethod> for CardDetailFromLocker {
    fn from(item: CardDetailsPaymentMethod) -> Self {
        Self {
            scheme: None,
            issuer_country: item.issuer_country,
            last4_digits: item.last4_digits,
            card_number: None,
            expiry_month: item.expiry_month,
            expiry_year: item.expiry_year,
            card_token: None,
            card_holder_name: item.card_holder_name,
            card_fingerprint: None,
            nick_name: item.nick_name,
            card_isin: item.card_isin,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            saved_to_locker: item.saved_to_locker,
        }
    }
}

impl From<CardDetailFromLocker> for CardDetailsPaymentMethod {
    fn from(item: CardDetailFromLocker) -> Self {
        Self {
            issuer_country: item.issuer_country,
            last4_digits: item.last4_digits,
            expiry_month: item.expiry_month,
            expiry_year: item.expiry_year,
            nick_name: item.nick_name,
            card_holder_name: item.card_holder_name,
            card_isin: item.card_isin,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            saved_to_locker: item.saved_to_locker,
        }
    }
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

#[derive(Debug, Clone, serde::Serialize, ToSchema, PartialEq)]
pub struct CardNetworkTypes {
    /// The card network enabled
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    pub card_network: api_enums::CardNetwork,

    /// surcharge details for this card network
    pub surcharge_details: Option<SurchargeDetailsResponse>,

    /// The list of eligible connectors for a given card network
    #[schema(example = json!(["stripe", "adyen"]))]
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct BankDebitTypes {
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema, PartialEq)]
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

    /// The Bank debit payment method information, if applicable for a payment method type.
    pub bank_debits: Option<BankDebitTypes>,
    /// The Bank transfer payment method information, if applicable for a payment method type.
    pub bank_transfers: Option<BankTransferTypes>,

    /// Required fields for the payment_method_type.
    pub required_fields: Option<HashMap<String, RequiredFieldInfo>>,

    /// surcharge details for this payment method type if exists
    pub surcharge_details: Option<SurchargeDetailsResponse>,

    /// auth service connector label for this payment method type, if exists
    pub pm_auth_connector: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SurchargeDetailsResponse {
    /// surcharge value
    pub surcharge: SurchargeResponse,
    /// tax on surcharge value
    pub tax_on_surcharge: Option<SurchargePercentage>,
    /// surcharge amount for this payment
    pub display_surcharge_amount: f64,
    /// tax on surcharge amount for this payment
    pub display_tax_on_surcharge_amount: f64,
    /// sum of display_surcharge_amount and display_tax_on_surcharge_amount
    pub display_total_surcharge_amount: f64,
    /// sum of original amount,
    pub display_final_amount: f64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum SurchargeResponse {
    /// Fixed Surcharge value
    Fixed(i64),
    /// Surcharge percentage
    Rate(SurchargePercentage),
}

impl From<Surcharge> for SurchargeResponse {
    fn from(value: Surcharge) -> Self {
        match value {
            Surcharge::Fixed(amount) => Self::Fixed(amount),
            Surcharge::Rate(percentage) => Self::Rate(percentage.into()),
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct SurchargePercentage {
    percentage: f32,
}

impl From<Percentage<SURCHARGE_PERCENTAGE_PRECISION_LENGTH>> for SurchargePercentage {
    fn from(value: Percentage<SURCHARGE_PERCENTAGE_PRECISION_LENGTH>) -> Self {
        Self {
            percentage: value.get_percentage(),
        }
    }
}
/// Required fields info used while listing the payment_method_data
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, ToSchema, Hash)]
pub struct RequiredFieldInfo {
    /// Required field for a payment_method through a payment_method_type
    pub required_field: String,

    /// Display name of the required field in the front-end
    pub display_name: String,

    /// Possible field type of required field
    #[schema(value_type = FieldType)]
    pub field_type: api_enums::FieldType,

    pub value: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ResponsePaymentMethodsEnabled {
    /// The payment method enabled
    #[schema(value_type = PaymentMethod)]
    pub payment_method: api_enums::PaymentMethod,

    /// The list of payment method types enabled for a connector account
    pub payment_method_types: Vec<ResponsePaymentMethodTypes>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct BankTransferTypes {
    /// The list of eligible connectors for a given payment experience
    #[schema(example = json!(["stripe", "adyen"]))]
    pub eligible_connectors: Vec<String>,
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
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_type: api_enums::PaymentMethodType,
    #[schema(value_type = Option<PaymentExperience>)]
    pub payment_experience: Option<api_enums::PaymentExperience>,
    #[schema(value_type = Option<Vec<CardNetwork>>)]
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(example = json!(
        {
            "type": "specific_accepted",
            "list": ["USD", "INR"]
        }
    ), value_type = Option<AcceptedCurrencies>)]
    pub accepted_currencies: Option<admin::AcceptedCurrencies>,

    ///  List of Countries accepted or has the processing capabilities of the processor
    #[schema(example = json!(
        {
            "type": "specific_accepted",
            "list": ["UK", "AU"]
        }
    ), value_type = Option<AcceptedCountries>)]
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
#[derive(Debug, Clone, serde::Serialize, Default, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodListRequest {
    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    #[schema(max_length = 30, min_length = 30, example = "secret_k2uj3he2893eiu2d")]
    pub client_secret: Option<String>,

    /// The two-letter ISO currency code
    #[schema(value_type = Option<Vec<CountryAlpha2>>, example = json!(["US", "UK", "IN"]))]
    pub accepted_countries: Option<Vec<api_enums::CountryAlpha2>>,

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

    /// Indicates the limit of last used payment methods
    #[schema(example = 1)]
    pub limit: Option<i64>,
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
                        "limit" => {
                            set_or_reject_duplicate(&mut output.limit, "limit", map.next_value()?)?;
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
pub struct PaymentMethodListResponse {
    /// Redirect URL of the merchant
    #[schema(example = "https://www.google.com")]
    pub redirect_url: Option<String>,

    /// currency of the Payment to be done
    #[schema(example = "USD", value_type = Currency)]
    pub currency: Option<api_enums::Currency>,

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
    /// Value indicating if the current payment is a mandate payment
    #[schema(value_type = MandateType)]
    pub mandate_payment: Option<payments::MandateType>,

    #[schema(value_type = Option<String>)]
    pub merchant_name: OptionalEncryptableName,

    /// flag to indicate if surcharge and tax breakup screen should be shown or not
    #[schema(value_type = bool)]
    pub show_surcharge_breakup_screen: bool,

    #[schema(value_type = Option<PaymentType>)]
    pub payment_type: Option<api_enums::PaymentType>,
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
    /// Returns whether a customer id is not tied to a payment intent (only when the request is made against a client secret)
    pub is_guest_customer: Option<bool>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentMethodDeleteResponse {
    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub payment_method_id: String,

    /// Whether payment method was deleted or not
    #[schema(example = true)]
    pub deleted: bool,
}
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CustomerDefaultPaymentMethodResponse {
    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub default_payment_method_id: Option<String>,
    /// The unique identifier of the customer.
    #[schema(example = "cus_meowerunwiuwiwqw")]
    pub customer_id: String,
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,
    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>,example = "credit")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethod {
    /// Token for payment method in temporary card locker which gets refreshed often
    #[schema(example = "7ebf443f-a050-4067-84e5-e6f6d4800aef")]
    pub payment_token: String,
    /// The unique identifier of the customer.
    #[schema(example = "pm_iouuy468iyuowqs")]
    pub payment_method_id: String,

    /// The unique identifier of the customer.
    #[schema(example = "cus_meowerunwiuwiwqw")]
    pub customer_id: String,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
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

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    #[schema(value_type = Option<Bank>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_transfer: Option<payouts::Bank>,

    /// Masked bank details from PM auth services
    #[schema(example = json!({"mask": "0000"}))]
    pub bank: Option<MaskedBankDetails>,

    /// Surcharge details for this saved card
    pub surcharge_details: Option<SurchargeDetailsResponse>,

    /// Whether this payment method requires CVV to be collected
    #[schema(example = true)]
    pub requires_cvv: bool,

    ///  A timestamp (ISO 8601 code) that determines when the payment method was last used
    #[schema(value_type = Option<PrimitiveDateTime>,example = "2024-02-24T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<time::PrimitiveDateTime>,
    /// Indicates if the payment method has been set to default or not
    #[schema(example = true)]
    pub default_payment_method_set: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodCollectLinkRequest {
    /// The unique identifier for the collect link.
    #[schema(value_type = Option<String>, example = "pm_collect_link_2bdacf398vwzq5n422S1")]
    pub pm_collect_link_id: Option<String>,

    /// The unique identifier of the customer.
    #[schema(example = "cus_92dnwed8s32bV9D8Snbiasd8v")]
    pub customer_id: String,

    #[serde(flatten)]
    pub config: Option<admin::CollectLinkConfigRequest>,

    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    #[schema(example = 900)]
    pub session_expiry: Option<u32>,

    /// Redirect to this URL post completion
    pub return_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodCollectLinkResponse {
    /// The unique identifier for the collect link.
    #[schema(value_type = String, example = "pm_collect_link_2bdacf398vwzq5n422S1")]
    pub pm_collect_link_id: String,

    /// The unique identifier of the customer.
    #[schema(value_type = String, example = "cus_92dnwed8s32bV9D8Snbiasd8v")]
    pub customer_id: String,

    /// Time when this link will be expired in ISO8601 format
    #[schema(value_type = PrimitiveDateTime, example = "2025-01-18T11:04:09.922Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: time::PrimitiveDateTime,

    /// URL to the form's link generated for collecting payment method details.
    #[schema(
        value_type = String, example = "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1"
    )]
    pub link: masking::Secret<String>,

    /// Redirect to this URL post completion
    #[schema(
    value_type = Option<String>, example = "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status"
)]
    pub return_url: Option<String>,

    /// Collect link config used
    #[serde(flatten)]
    pub config: api_enums::CollectLinkConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodCollectLinkRenderRequest {
    /// Unique identifier for a merchant.
    #[schema(example = "merchant_1671528864")]
    pub merchant_id: String,

    /// The unique identifier for the collect link.
    #[schema(value_type = String, example = "pm_collect_link_2bdacf398vwzq5n422S1")]
    pub pm_collect_link_id: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentMethodCollectLinkDetails {
    pub pub_key: masking::Secret<String>,
    pub client_secret: masking::Secret<String>,
    pub pm_collect_link_id: String,
    pub customer_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub session_expiry: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    #[serde(flatten)]
    pub config: api_enums::CollectLinkConfig,
    pub enabled_payment_methods: Vec<EnabledPaymentMethod>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EnabledPaymentMethod {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_types: Vec<api_enums::PaymentMethodType>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentMethodCollectLinkStatusDetails {
    pub pm_collect_link_id: String,
    pub customer_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub session_expiry: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub status: api_enums::PaymentMethodCollectStatus,
    #[serde(flatten)]
    pub config: api_enums::CollectLinkConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MaskedBankDetails {
    pub mask: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodId {
    pub payment_method_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct DefaultPaymentMethod {
    pub customer_id: String,
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
pub struct ListCountriesCurrenciesRequest {
    pub connector: api_enums::Connector,
    pub payment_method_type: api_enums::PaymentMethodType,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCountriesCurrenciesResponse {
    pub currencies: HashSet<api_enums::Currency>,
    pub countries: HashSet<CountryCodeWithName>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Eq, Hash, PartialEq)]
pub struct CountryCodeWithName {
    pub code: api_enums::CountryAlpha2,
    pub name: api_enums::Country,
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue1 {
    pub data: payments::BankTransferData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue2 {
    pub customer_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue1 {
    pub data: payments::BankRedirectData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue2 {
    pub customer_id: Option<String>,
}
