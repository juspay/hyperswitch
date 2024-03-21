use cards::CardNumber;
use common_utils::{
    crypto,
    pii::{self, Email},
};
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{enums as api_enums, payments};

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub enum PayoutRequest {
    PayoutActionRequest(PayoutActionRequest),
    PayoutCreateRequest(PayoutCreateRequest),
    PayoutRetrieveRequest(PayoutRetrieveRequest),
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PayoutCreateRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: Option<String>, // TODO: #1321 https://github.com/juspay/hyperswitch/issues/1321

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, value_type = String, example = "merchant_1668273825")]
    pub merchant_id: Option<String>,

    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 1000)]
    #[serde(default, deserialize_with = "payments::amount::deserialize_option")]
    pub amount: Option<payments::Amount>,

    /// The currency of the payout request can be specified here
    #[schema(value_type = Currency, example = "USD")]
    pub currency: Option<api_enums::Currency>,

    /// Specifies routing algorithm for selecting a connector
    #[schema(value_type = Option<RoutingAlgorithm>, example = json!({
        "type": "single",
        "data": "adyen"
    }))]
    pub routing: Option<serde_json::Value>,

    /// This allows the merchant to manually select a connector with which the payout can go through
    #[schema(value_type = Option<Vec<PayoutConnectors>>, max_length = 255, example = json!(["wise", "adyen"]))]
    pub connector: Option<Vec<api_enums::PayoutConnectors>>,

    /// The boolean value to create payout with connector
    #[schema(value_type = bool, example = true, default = false)]
    pub confirm: Option<bool>,

    /// The payout_type of the payout request can be specified here
    #[schema(value_type = PayoutType, example = "card")]
    pub payout_type: Option<api_enums::PayoutType>,

    /// The payout method information required for carrying out a payout
    #[schema(value_type = Option<PayoutMethodData>)]
    pub payout_method_data: Option<PayoutMethodData>,

    /// The billing address for the payout
    #[schema(value_type = Option<Object>, example = json!(r#"{
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "CA",
            "zip": "94122",
            "country": "US",
            "first_name": "John",
            "last_name": "Doe"
        },
        "phone": { "number": "8056594427", "country_code": "+1" }
    }"#))]
    pub billing: Option<payments::Address>,

    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(value_type = String, max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<String>,

    /// Set to true to confirm the payout without review, no further action required
    #[schema(value_type = bool, example = true, default = false)]
    pub auto_fulfill: Option<bool>,

    /// description: The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Email>,

    /// description: The customer's name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test")]
    pub name: Option<Secret<String>>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "3141592653")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+1")]
    pub phone_country_code: Option<String>,

    /// It's a token used for client side verification.
    #[schema(value_type = String, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    pub return_url: Option<String>,

    /// Business country of the merchant for this payout
    #[schema(example = "US", value_type = CountryAlpha2)]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payout
    #[schema(example = "food", value_type = Option<String>)]
    pub business_label: Option<String>,

    /// A description of the payout
    #[schema(example = "It's my first payout request", value_type = String)]
    pub description: Option<String>,

    /// Type of entity to whom the payout is being carried out to
    #[schema(value_type = PayoutEntityType, example = "Individual")]
    pub entity_type: Option<api_enums::PayoutEntityType>,

    /// Specifies whether or not the payout request is recurring
    #[schema(value_type = Option<bool>, default = false)]
    pub recurring: Option<bool>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Provide a reference to a stored payment method
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payout_token: Option<String>,

    /// The business profile to use for this payment, if not passed the default business profile
    /// associated with the merchant account will be used.
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayoutMethodData {
    Card(Card),
    Bank(Bank),
    Wallet(Wallet),
}

impl Default for PayoutMethodData {
    fn default() -> Self {
        Self::Card(Card::default())
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Card {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    pub card_number: CardNumber,

    /// The card's expiry month
    #[schema(value_type = String)]
    pub expiry_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String)]
    pub expiry_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Doe")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
pub enum Bank {
    Ach(AchBankTransfer),
    Bacs(BacsBankTransfer),
    Sepa(SepaBankTransfer),
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AchBankTransfer {
    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<api_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,

    /// Bank account number is an unique identifier assigned by a bank to a customer.
    #[schema(value_type = String, example = "000123456")]
    pub bank_account_number: Secret<String>,

    /// [9 digits] Routing number - used in USA for identifying a specific bank.
    #[schema(value_type = String, example = "110000000")]
    pub bank_routing_number: Secret<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BacsBankTransfer {
    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<api_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,

    /// Bank account number is an unique identifier assigned by a bank to a customer.
    #[schema(value_type = String, example = "000123456")]
    pub bank_account_number: Secret<String>,

    /// [6 digits] Sort Code - used in UK and Ireland for identifying a bank and it's branches.
    #[schema(value_type = String, example = "98-76-54")]
    pub bank_sort_code: Secret<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
// The SEPA (Single Euro Payments Area) is a pan-European network that allows you to send and receive payments in euros between two cross-border bank accounts in the eurozone.
pub struct SepaBankTransfer {
    /// Bank name
    #[schema(value_type = Option<String>, example = "Deutsche Bank")]
    pub bank_name: Option<String>,

    /// Bank country code
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub bank_country_code: Option<api_enums::CountryAlpha2>,

    /// Bank city
    #[schema(value_type = Option<String>, example = "California")]
    pub bank_city: Option<String>,

    /// International Bank Account Number (iban) - used in many countries for identifying a bank along with it's customer.
    #[schema(value_type = String, example = "DE89370400440532013000")]
    pub iban: Secret<String>,

    /// [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it's branches
    #[schema(value_type = String, example = "HSBCGB2LXXX")]
    pub bic: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Wallet {
    Paypal(Paypal),
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Paypal {
    /// Email linked with paypal account
    #[schema(value_type = String, example = "john.doe@example.com")]
    pub email: Option<Email>,
}

#[derive(Debug, Default, ToSchema, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayoutCreateResponse {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = String,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: String, // TODO: Update this to PayoutIdType similar to PaymentIdType

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, value_type = String, example = "merchant_1668273825")]
    pub merchant_id: String,

    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = i64, example = 1000)]
    pub amount: i64,

    /// Recipient's currency for the payout request
    #[schema(value_type = Currency, example = "USD")]
    pub currency: api_enums::Currency,

    /// The connector used for the payout
    #[schema(example = "wise")]
    pub connector: Option<String>,

    /// The payout method that is to be used
    #[schema(value_type = PayoutType, example = "bank")]
    pub payout_type: api_enums::PayoutType,

    /// The billing address for the payout
    #[schema(value_type = Option<Object>, example = json!(r#"{
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "CA",
            "zip": "94122",
            "country": "US",
            "first_name": "John",
            "last_name": "Doe"
        },
        "phone": { "number": "8056594427", "country_code": "+1" }
    }"#))]
    pub billing: Option<payments::Address>,

    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(value_type = String, max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: String,

    /// Set to true to confirm the payout without review, no further action required
    #[schema(value_type = bool, example = true, default = false)]
    pub auto_fulfill: bool,

    /// description: The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: crypto::OptionalEncryptableEmail,

    /// description: The customer's name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test")]
    pub name: crypto::OptionalEncryptableName,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "3141592653")]
    pub phone: crypto::OptionalEncryptablePhone,

    /// The country code for the customer phone number
    #[schema(max_length = 255, example = "+1")]
    pub phone_country_code: Option<String>,

    /// It's a token used for client side verification.
    #[schema(value_type = String, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = String, example = "https://hyperswitch.io")]
    pub return_url: Option<String>,

    /// Business country of the merchant for this payout
    #[schema(example = "US", value_type = CountryAlpha2)]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payout
    #[schema(example = "food", value_type = Option<String>)]
    pub business_label: Option<String>,

    /// A description of the payout
    #[schema(example = "It's my first payout request", value_type = String)]
    pub description: Option<String>,

    /// Type of entity to whom the payout is being carried out to
    #[schema(value_type = PayoutEntityType, example = "Individual")]
    pub entity_type: api_enums::PayoutEntityType,

    /// Specifies whether or not the payout request is recurring
    #[schema(value_type = Option<bool>, default = false)]
    pub recurring: bool,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// Current status of the Payout
    #[schema(value_type = PayoutStatus, example = Pending)]
    pub status: api_enums::PayoutStatus,

    /// If there was an error while calling the connector the error message is received here
    #[schema(value_type = String, example = "Failed while verifying the card")]
    pub error_message: Option<String>,

    /// If there was an error while calling the connectors the code is received here
    #[schema(value_type = String, example = "E0001")]
    pub error_code: Option<String>,

    /// The business profile that is associated with this payment
    pub profile_id: String,
}

#[derive(Default, Debug, Clone, Deserialize, ToSchema)]
pub struct PayoutRetrieveBody {
    pub force_sync: Option<bool>,
}

#[derive(Default, Debug, Serialize, ToSchema, Clone, Deserialize)]
pub struct PayoutRetrieveRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = String,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: String,

    /// `force_sync` with the connector to get payout details
    /// (defaults to false)
    #[schema(value_type = Option<bool>, default = false, example = true)]
    pub force_sync: Option<bool>,
}

#[derive(Default, Debug, Serialize, ToSchema, Clone, Deserialize)]
pub struct PayoutActionRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = String,
        min_length = 30,
        max_length = 30,
        example = "payout_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payout_id: String,
}
