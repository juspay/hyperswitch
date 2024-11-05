use std::collections::{HashMap, HashSet};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use std::str::FromStr;

use cards::CardNumber;
use common_utils::{
    consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH,
    crypto::OptionalEncryptableName,
    id_type, link_utils, pii,
    types::{MinorUnit, Percentage, Surcharge},
};
use masking::PeekInterface;
use serde::de;
use utoipa::{schema, ToSchema};

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::customers;
#[cfg(feature = "payouts")]
use crate::payouts;
use crate::{
    admin, enums as api_enums,
    payments::{self, Amount, BankCodeResponse},
};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodCreate {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: Option<api_enums::PaymentMethod>,

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
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<id_type::CustomerId>,

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

    /// For Client based calls, SDK will use the client_secret
    /// in order to call /payment_methods
    /// Client secret will be generated whenever a new
    /// payment method is created
    pub client_secret: Option<String>,

    /// Payment method data to be passed in case of client
    /// based flow
    pub payment_method_data: Option<PaymentMethodCreateData>,

    /// The billing details of the payment method
    #[schema(value_type = Option<Address>)]
    pub billing: Option<payments::Address>,

    #[serde(skip_deserializing)]
    /// The connector mandate details of the payment method, this is added only for cards migration
    /// api and is skipped during deserialization of the payment method create request as this
    /// it should not be passed in the request
    pub connector_mandate_details: Option<PaymentsMandateReference>,

    #[serde(skip_deserializing)]
    /// The transaction id of a CIT (customer initiated transaction) associated with the payment method,
    /// this is added only for cards migration api and is skipped during deserialization of the
    /// payment method create request as it should not be passed in the request
    pub network_transaction_id: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodCreate {
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = PaymentMethodType,example = "credit")]
    pub payment_method_type: api_enums::PaymentMethodType,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The unique identifier of the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,

    /// Payment method data to be passed
    pub payment_method_data: PaymentMethodCreateData,

    /// The billing details of the payment method
    #[schema(value_type = Option<Address>)]
    pub billing: Option<payments::Address>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodIntentCreate {
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The billing details of the payment method
    #[schema(value_type = Option<Address>)]
    pub billing: Option<payments::Address>,

    /// The unique identifier of the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodIntentConfirm {
    /// For SDK based calls, client_secret would be required
    pub client_secret: String,

    /// The unique identifier of the customer.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<id_type::CustomerId>,

    /// Payment method data to be passed
    pub payment_method_data: PaymentMethodCreateData,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = PaymentMethodType,example = "credit")]
    pub payment_method_type: api_enums::PaymentMethodType,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl PaymentMethodIntentConfirm {
    pub fn validate_payment_method_data_against_payment_method(
        payment_method: api_enums::PaymentMethod,
        payment_method_data: PaymentMethodCreateData,
    ) -> bool {
        match payment_method {
            api_enums::PaymentMethod::Card => {
                matches!(payment_method_data, PaymentMethodCreateData::Card(_))
            }
            _ => false,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodIntentConfirmInternal {
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub id: String,
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = PaymentMethodType,example = "credit")]
    pub payment_method_type: api_enums::PaymentMethodType,

    /// For SDK based calls, client_secret would be required
    pub client_secret: String,

    /// The unique identifier of the customer.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<id_type::CustomerId>,

    /// Payment method data to be passed
    pub payment_method_data: PaymentMethodCreateData,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<PaymentMethodIntentConfirmInternal> for PaymentMethodIntentConfirm {
    fn from(item: PaymentMethodIntentConfirmInternal) -> Self {
        Self {
            client_secret: item.client_secret,
            payment_method: item.payment_method,
            payment_method_type: item.payment_method_type,
            customer_id: item.customer_id,
            payment_method_data: item.payment_method_data.clone(),
        }
    }
}
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
/// This struct is only used by and internal api to migrate payment method
pub struct PaymentMethodMigrate {
    /// Merchant id
    pub merchant_id: id_type::MerchantId,

    /// The type of payment method use for the payment.
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// This is a sub-category of payment method.
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// The name of the bank/ provider issuing the payment method to the end user
    pub payment_method_issuer: Option<String>,

    /// A standard code representing the issuer of payment method
    pub payment_method_issuer_code: Option<api_enums::PaymentMethodIssuerCode>,

    /// Card Details
    pub card: Option<MigrateCardDetail>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    pub metadata: Option<pii::SecretSerdeValue>,

    /// The unique identifier of the customer.
    pub customer_id: Option<id_type::CustomerId>,

    /// The card network
    pub card_network: Option<String>,

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    pub bank_transfer: Option<payouts::Bank>,

    /// Payment method details from locker
    #[cfg(feature = "payouts")]
    pub wallet: Option<payouts::Wallet>,

    /// Payment method data to be passed in case of client
    /// based flow
    pub payment_method_data: Option<PaymentMethodCreateData>,

    /// The billing details of the payment method
    pub billing: Option<payments::Address>,

    /// The connector mandate details of the payment method
    pub connector_mandate_details: Option<PaymentsMandateReference>,

    // The CIT (customer initiated transaction) transaction id associated with the payment method
    pub network_transaction_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReference(
    pub HashMap<id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>,
);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl PaymentMethodCreate {
    pub fn get_payment_method_create_from_payment_method_migrate(
        card_number: CardNumber,
        payment_method_migrate: &PaymentMethodMigrate,
    ) -> Self {
        let card_details =
            payment_method_migrate
                .card
                .as_ref()
                .map(|payment_method_migrate_card| CardDetail {
                    card_number,
                    card_exp_month: payment_method_migrate_card.card_exp_month.clone(),
                    card_exp_year: payment_method_migrate_card.card_exp_year.clone(),
                    card_holder_name: payment_method_migrate_card.card_holder_name.clone(),
                    nick_name: payment_method_migrate_card.nick_name.clone(),
                    card_issuing_country: payment_method_migrate_card.card_issuing_country.clone(),
                    card_network: payment_method_migrate_card.card_network.clone(),
                    card_issuer: payment_method_migrate_card.card_issuer.clone(),
                    card_type: payment_method_migrate_card.card_type.clone(),
                });

        Self {
            customer_id: payment_method_migrate.customer_id.clone(),
            payment_method: payment_method_migrate.payment_method,
            payment_method_type: payment_method_migrate.payment_method_type,
            payment_method_issuer: payment_method_migrate.payment_method_issuer.clone(),
            payment_method_issuer_code: payment_method_migrate.payment_method_issuer_code,
            metadata: payment_method_migrate.metadata.clone(),
            payment_method_data: payment_method_migrate.payment_method_data.clone(),
            connector_mandate_details: payment_method_migrate.connector_mandate_details.clone(),
            client_secret: None,
            billing: payment_method_migrate.billing.clone(),
            card: card_details,
            card_network: payment_method_migrate.card_network.clone(),
            #[cfg(feature = "payouts")]
            bank_transfer: payment_method_migrate.bank_transfer.clone(),
            #[cfg(feature = "payouts")]
            wallet: payment_method_migrate.wallet.clone(),
            network_transaction_id: payment_method_migrate.network_transaction_id.clone(),
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl PaymentMethodCreate {
    pub fn validate_payment_method_data_against_payment_method(
        payment_method: api_enums::PaymentMethod,
        payment_method_data: PaymentMethodCreateData,
    ) -> bool {
        match payment_method {
            api_enums::PaymentMethod::Card => {
                matches!(payment_method_data, PaymentMethodCreateData::Card(_))
            }
            _ => false,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodUpdate {
    /// Card Details
    #[schema(example = json!({
    "card_number": "4111111145551142",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "John Doe"}))]
    pub card: Option<CardDetailUpdate>,

    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    #[schema(max_length = 30, min_length = 30, example = "secret_k2uj3he2893eiu2d")]
    pub client_secret: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodUpdate {
    /// payment method data to be passed
    pub payment_method_data: PaymentMethodUpdateData,

    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    #[schema(max_length = 30, min_length = 30, example = "secret_k2uj3he2893eiu2d")]
    pub client_secret: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodUpdateData {
    Card(CardDetailUpdate),
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodCreateData {
    Card(CardDetail),
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodCreateData {
    Card(CardDetail),
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(
    Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema, strum::EnumString, strum::Display,
)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Credit,
    Debit,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
    pub card_issuing_country: Option<api_enums::CountryAlpha2>,

    /// Card's Network
    #[schema(value_type = Option<CardNetwork>)]
    pub card_network: Option<api_enums::CardNetwork>,

    /// Issuer Bank for Card
    pub card_issuer: Option<String>,

    /// Card Type
    pub card_type: Option<CardType>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrateCardDetail {
    /// Card Number
    #[schema(value_type = String,example = "4111111145551142")]
    pub card_number: masking::Secret<String>,

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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardDetailUpdate {
    /// Card Expiry Month
    #[schema(value_type = String,example = "10")]
    pub card_exp_month: Option<masking::Secret<String>>,

    /// Card Expiry Year
    #[schema(value_type = String,example = "25")]
    pub card_exp_year: Option<masking::Secret<String>>,

    /// Card Holder Name
    #[schema(value_type = String,example = "John Doe")]
    pub card_holder_name: Option<masking::Secret<String>>,

    /// Card Holder's Nick Name
    #[schema(value_type = Option<String>,example = "John Doe")]
    pub nick_name: Option<masking::Secret<String>>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl CardDetailUpdate {
    pub fn apply(&self, card_data_from_locker: Card) -> CardDetail {
        CardDetail {
            card_number: card_data_from_locker.card_number,
            card_exp_month: self
                .card_exp_month
                .clone()
                .unwrap_or(card_data_from_locker.card_exp_month),
            card_exp_year: self
                .card_exp_year
                .clone()
                .unwrap_or(card_data_from_locker.card_exp_year),
            card_holder_name: self
                .card_holder_name
                .clone()
                .or(card_data_from_locker.name_on_card),
            nick_name: self
                .nick_name
                .clone()
                .or(card_data_from_locker.nick_name.map(masking::Secret::new)),
            card_issuing_country: None,
            card_network: None,
            card_issuer: None,
            card_type: None,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardDetailUpdate {
    /// Card Holder Name
    #[schema(value_type = String,example = "John Doe")]
    pub card_holder_name: Option<masking::Secret<String>>,

    /// Card Holder's Nick Name
    #[schema(value_type = Option<String>,example = "John Doe")]
    pub nick_name: Option<masking::Secret<String>>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl CardDetailUpdate {
    pub fn apply(&self, card_data_from_locker: Card) -> CardDetail {
        CardDetail {
            card_number: card_data_from_locker.card_number,
            card_exp_month: card_data_from_locker.card_exp_month,
            card_exp_year: card_data_from_locker.card_exp_year,
            card_holder_name: self
                .card_holder_name
                .clone()
                .or(card_data_from_locker.name_on_card),
            nick_name: self
                .nick_name
                .clone()
                .or(card_data_from_locker.nick_name.map(masking::Secret::new)),
            card_issuing_country: None,
            card_network: None,
            card_issuer: None,
            card_type: None,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLocker),
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentMethodResponse {
    /// Unique identifier for a merchant
    #[schema(example = "merchant_1671528864", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// The unique identifier of the customer.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<id_type::CustomerId>,

    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub payment_method_id: String,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method: Option<api_enums::PaymentMethod>,

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

    /// For Client based calls
    pub client_secret: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema, Clone)]
pub struct PaymentMethodResponse {
    /// Unique identifier for a merchant
    #[schema(example = "merchant_1671528864")]
    pub merchant_id: id_type::MerchantId,

    /// The unique identifier of the customer.
    #[schema(value_type = Option<String>, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,

    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub payment_method_id: String,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod, example = "card")]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>, example = "credit")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// Indicates whether the payment method is eligible for recurring payments
    #[schema(example = true)]
    pub recurring_enabled: bool,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = Option<PrimitiveDateTime>, example = "2023-01-18T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,

    #[schema(value_type = Option<PrimitiveDateTime>, example = "2024-02-24T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<time::PrimitiveDateTime>,

    /// For Client based calls
    pub client_secret: Option<String>,

    pub payment_method_data: Option<PaymentMethodResponseData>,
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
    pub mca_id: id_type::MerchantConnectorAccountId,
    pub access_token: BankAccountAccessCreds,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BankAccountAccessCreds {
    AccessToken(masking::Secret<String>),
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Card {
    pub card_number: CardNumber,
    pub name_on_card: Option<masking::Secret<String>>,
    pub card_exp_month: masking::Secret<String>,
    pub card_exp_year: masking::Secret<String>,
    pub card_brand: Option<String>,
    pub card_isin: Option<String>,
    pub nick_name: Option<String>,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct CardDetailFromLocker {
    pub issuer_country: Option<api_enums::CountryAlpha2>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    #[schema(value_type=Option<String>)]
    pub card_number: Option<CardNumber>,

    #[schema(value_type=Option<String>)]
    pub expiry_month: Option<masking::Secret<String>>,

    #[schema(value_type=Option<String>)]
    pub expiry_year: Option<masking::Secret<String>>,

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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
                .map(|card_number| card_number.get_extended_card_bin()),
            card_exp_month: item.expiry_month,
            card_exp_year: item.expiry_year,
            card_holder_name: item.card_holder_name,
            payment_checks: None,
            authentication_data: None,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<CardDetailFromLocker> for payments::AdditionalCardInfo {
    fn from(item: CardDetailFromLocker) -> Self {
        Self {
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type,
            card_issuing_country: item.issuer_country.map(|country| country.to_string()),
            bank_code: None,
            last4: item.last4_digits,
            card_isin: item.card_isin,
            card_extended_bin: item
                .card_number
                .map(|card_number| card_number.get_extended_card_bin()),
            card_exp_month: item.expiry_month,
            card_exp_year: item.expiry_year,
            card_holder_name: item.card_holder_name,
            payment_checks: None,
            authentication_data: None,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<CardDetailsPaymentMethod> for CardDetailFromLocker {
    fn from(item: CardDetailsPaymentMethod) -> Self {
        Self {
            issuer_country: item
                .issuer_country
                .as_ref()
                .map(|c| api_enums::CountryAlpha2::from_str(c))
                .transpose()
                .ok()
                .flatten(),
            last4_digits: item.last4_digits,
            card_number: None,
            expiry_month: item.expiry_month,
            expiry_year: item.expiry_year,
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<CardDetail> for CardDetailFromLocker {
    fn from(item: CardDetail) -> Self {
        Self {
            issuer_country: item.card_issuing_country,
            last4_digits: Some(item.card_number.get_last4()),
            card_number: Some(item.card_number),
            expiry_month: Some(item.card_exp_month),
            expiry_year: Some(item.card_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
            card_fingerprint: None,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<CardDetail> for CardDetailsPaymentMethod {
    fn from(item: CardDetail) -> Self {
        Self {
            issuer_country: item.card_issuing_country.map(|c| c.to_string()),
            last4_digits: Some(item.card_number.get_last4()),
            expiry_month: Some(item.card_exp_month),
            expiry_year: Some(item.card_exp_year),
            card_holder_name: item.card_holder_name,
            nick_name: item.nick_name,
            card_isin: None,
            card_issuer: item.card_issuer,
            card_network: item.card_network,
            card_type: item.card_type.map(|card| card.to_string()),
            saved_to_locker: true,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<CardDetailFromLocker> for CardDetailsPaymentMethod {
    fn from(item: CardDetailFromLocker) -> Self {
        Self {
            issuer_country: item.issuer_country.map(|country| country.to_string()),
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
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum SurchargeResponse {
    /// Fixed Surcharge value
    Fixed(MinorUnit),
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
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, ToSchema)]
pub struct RequiredFieldInfo {
    /// Required field for a payment_method through a payment_method_type
    pub required_field: String,

    /// Display name of the required field in the front-end
    pub display_name: String,

    /// Possible field type of required field
    #[schema(value_type = FieldType)]
    pub field_type: api_enums::FieldType,

    #[schema(value_type = Option<String>)]
    pub value: Option<masking::Secret<String>>,
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
    pub minimum_amount: Option<MinorUnit>,

    /// Maximum amount supported by the processor. To be represented in the lowest denomination of
    /// the target currency (For example, for USD it should be in cents)
    #[schema(example = 1313)]
    pub maximum_amount: Option<MinorUnit>,

    /// Boolean to enable recurring payments / mandates. Default is true.
    #[schema(default = true, example = false)]
    pub recurring_enabled: bool,

    /// Boolean to enable installment / EMI / BNPL payments. Default is true.
    #[schema(default = true, example = false)]
    pub installment_payment_enabled: bool,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
    pub amount: Option<MinorUnit>,

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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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

    /// Filter by amount
    #[schema(example = 60)]
    pub amount: Option<MinorUnit>,

    /// The three-letter ISO currency code
    #[schema(value_type = Option<Vec<Currency>>,example = json!(["USD", "EUR"]))]
    pub accepted_currencies: Option<Vec<api_enums::Currency>>,

    /// Indicates whether the payment method is eligible for recurring payments
    #[schema(example = true)]
    pub recurring_enabled: Option<bool>,

    /// Indicates whether the payment method is eligible for card netwotks
    #[schema(value_type = Option<Vec<CardNetwork>>, example = json!(["visa", "mastercard"]))]
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,

    /// Indicates the limit of last used payment methods
    #[schema(example = 1)]
    pub limit: Option<i64>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
                        "amount" => {
                            set_or_reject_duplicate(
                                &mut output.amount,
                                "amount",
                                map.next_value()?,
                            )?;
                        }
                        "accepted_currencies" => match output.accepted_currencies.as_mut() {
                            Some(inner) => inner.push(map.next_value()?),
                            None => {
                                output.accepted_currencies = Some(vec![map.next_value()?]);
                            }
                        },
                        "recurring_enabled" => {
                            set_or_reject_duplicate(
                                &mut output.recurring_enabled,
                                "recurring_enabled",
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

#[derive(Debug, Clone, serde::Serialize, Default, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentMethodListRequestV2 {
    #[schema(example = 1)]
    pub limit: Option<i64>,
    pub customer_id: Option<id_type::CustomerId>,
    pub mandate_id: Option<String>,
    pub request_external_three_ds_authentication: Option<bool>,
    pub currency: Option<api_enums::Currency>,
    pub business_country: Option<api_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub profile_id: Option<String>,
    pub amount: Option<Amount>,
    pub recurring_enabled: Option<bool>,
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
    pub installment_payment_enabled: Option<bool>,
}

impl<'de> serde::Deserialize<'de> for PaymentMethodListRequestV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> de::Visitor<'de> for FieldVisitor {
            type Value = PaymentMethodListRequestV2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("Failed while deserializing as map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut output = PaymentMethodListRequestV2::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        "customer_id" => {
                            set_or_reject_duplicate(
                                &mut output.customer_id,
                                "customer_id",
                                map.next_value()?,
                            )?;
                        },
                        "mandate_id" => {
                            set_or_reject_duplicate(
                                &mut output.mandate_id,
                                "mandate_id",
                                map.next_value()?,
                            )?;
                        },
                        "request_external_three_ds_authentication" => {
                            set_or_reject_duplicate(
                                &mut output.request_external_three_ds_authentication,
                                "request_external_three_ds_authentication",
                                map.next_value()?,
                            )?;
                        },
                        "currency" => {
                            set_or_reject_duplicate(
                                &mut output.currency,
                                "currency",
                                map.next_value()?,
                            )?;
                        },
                        "business_country" => {
                            set_or_reject_duplicate(
                                &mut output.business_country,
                                "business_country",
                                map.next_value()?,
                            )?;
                        },
                        "business_label" => {
                            set_or_reject_duplicate(
                                &mut output.business_label,
                                "business_label",
                                map.next_value()?,
                            )?;
                        },
                        "profile_id" => {
                            set_or_reject_duplicate(
                                &mut output.profile_id,
                                "profile_id",
                                map.next_value()?,
                            )?;
                        },
                        "limit" => {
                            set_or_reject_duplicate(&mut output.limit, "limit", map.next_value()?)?;
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
                        "card_network" => match output.card_networks.as_mut() {
                            Some(inner) => inner.push(map.next_value()?),
                            None => output.card_networks = Some(vec![map.next_value()?]),
                        },
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

    /// flag to indicate whether to perform external 3ds authentication
    #[schema(example = true)]
    pub request_external_three_ds_authentication: bool,

    /// flag that indicates whether to collect shipping details from wallets or from the customer
    pub collect_shipping_details_from_wallets: Option<bool>,

    /// flag that indicates whether to collect billing details from wallets or from the customer
    pub collect_billing_details_from_wallets: Option<bool>,

    /// flag that indicates whether to calculate tax on the order amount
    pub is_tax_calculation_enabled: bool,
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

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethodsListResponse {
    /// List of payment methods for customer
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
    /// Returns whether a customer id is not tied to a payment intent (only when the request is made against a client secret)
    pub is_guest_customer: Option<bool>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethodsListResponse {
    /// List of payment methods for customer
    pub customer_payment_methods: Vec<CustomerPaymentMethod>,
    /// Returns whether a customer id is not tied to a payment intent (only when the request is made against a client secret)
    pub is_guest_customer: Option<bool>,
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentMethodDeleteResponse {
    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub payment_method_id: String,

    /// Whether payment method was deleted or not
    #[schema(example = true)]
    pub deleted: bool,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentMethodDeleteResponse {
    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub payment_method_id: String,
}
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CustomerDefaultPaymentMethodResponse {
    /// The unique identifier of the Payment method
    #[schema(example = "card_rGK4Vi5iSW70MY7J2mIg")]
    pub default_payment_method_id: Option<String>,
    /// The unique identifier of the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,
    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,
    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>,example = "credit")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethod {
    /// Token for payment method in temporary card locker which gets refreshed often
    #[schema(example = "7ebf443f-a050-4067-84e5-e6f6d4800aef")]
    pub payment_token: Option<String>,
    /// The unique identifier of the customer.
    #[schema(example = "pm_iouuy468iyuowqs")]
    pub payment_method_id: String,

    /// The unique identifier of the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,

    /// The type of payment method use for the payment.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method: api_enums::PaymentMethod,

    /// This is a sub-category of payment method.
    #[schema(value_type = Option<PaymentMethodType>,example = "credit_card")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// Indicates whether the payment method is eligible for recurring payments
    #[schema(example = true)]
    pub recurring_enabled: bool,

    /// PaymentMethod Data from locker
    pub payment_method_data: Option<PaymentMethodListData>,

    /// Masked bank details from PM auth services
    #[schema(example = json!({"mask": "0000"}))]
    pub bank: Option<MaskedBankDetails>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,

    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[schema(value_type = Option<PrimitiveDateTime>,example = "2023-01-18T11:04:09.922Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,

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
    pub is_default: bool,

    /// The billing details of the payment method
    #[schema(value_type = Option<Address>)]
    pub billing: Option<payments::Address>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodListData {
    Card(CardDetailFromLocker),
    #[cfg(feature = "payouts")]
    Bank(payouts::Bank),
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CustomerPaymentMethod {
    /// Token for payment method in temporary card locker which gets refreshed often
    #[schema(example = "7ebf443f-a050-4067-84e5-e6f6d4800aef")]
    pub payment_token: String,
    /// The unique identifier of the customer.
    #[schema(example = "pm_iouuy468iyuowqs")]
    pub payment_method_id: String,

    /// The unique identifier of the customer.
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,

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

    /// The billing details of the payment method
    #[schema(value_type = Option<Address>)]
    pub billing: Option<payments::Address>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodCollectLinkRequest {
    /// The unique identifier for the collect link.
    #[schema(value_type = Option<String>, example = "pm_collect_link_2bdacf398vwzq5n422S1")]
    pub pm_collect_link_id: Option<String>,

    /// The unique identifier of the customer.
    #[schema(value_type = String, example = "cus_92dnwed8s32bV9D8Snbiasd8v")]
    pub customer_id: id_type::CustomerId,

    #[serde(flatten)]
    #[schema(value_type = Option<GenericLinkUiConfig>)]
    pub ui_config: Option<link_utils::GenericLinkUiConfig>,

    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    #[schema(value_type = Option<u32>, example = 900)]
    pub session_expiry: Option<u32>,

    /// Redirect to this URL post completion
    #[schema(value_type = Option<String>, example = "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status")]
    pub return_url: Option<String>,

    /// List of payment methods shown on collect UI
    #[schema(value_type = Option<Vec<EnabledPaymentMethod>>, example = r#"[{"payment_method": "bank_transfer", "payment_method_types": ["ach", "bacs"]}]"#)]
    pub enabled_payment_methods: Option<Vec<link_utils::EnabledPaymentMethod>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodCollectLinkResponse {
    /// The unique identifier for the collect link.
    #[schema(value_type = String, example = "pm_collect_link_2bdacf398vwzq5n422S1")]
    pub pm_collect_link_id: String,

    /// The unique identifier of the customer.
    #[schema(value_type = String, example = "cus_92dnwed8s32bV9D8Snbiasd8v")]
    pub customer_id: id_type::CustomerId,

    /// Time when this link will be expired in ISO8601 format
    #[schema(value_type = PrimitiveDateTime, example = "2025-01-18T11:04:09.922Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: time::PrimitiveDateTime,

    /// URL to the form's link generated for collecting payment method details.
    #[schema(value_type = String, example = "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1")]
    pub link: masking::Secret<url::Url>,

    /// Redirect to this URL post completion
    #[schema(value_type = Option<String>, example = "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status")]
    pub return_url: Option<String>,

    /// Collect link config used
    #[serde(flatten)]
    #[schema(value_type = GenericLinkUiConfig)]
    pub ui_config: link_utils::GenericLinkUiConfig,

    /// List of payment methods shown on collect UI
    #[schema(value_type = Option<Vec<EnabledPaymentMethod>>, example = r#"[{"payment_method": "bank_transfer", "payment_method_types": ["ach", "bacs"]}]"#)]
    pub enabled_payment_methods: Option<Vec<link_utils::EnabledPaymentMethod>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodCollectLinkRenderRequest {
    /// Unique identifier for a merchant.
    #[schema(example = "merchant_1671528864", value_type = String)]
    pub merchant_id: id_type::MerchantId,

    /// The unique identifier for the collect link.
    #[schema(value_type = String, example = "pm_collect_link_2bdacf398vwzq5n422S1")]
    pub pm_collect_link_id: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentMethodCollectLinkDetails {
    pub publishable_key: masking::Secret<String>,
    pub client_secret: masking::Secret<String>,
    pub pm_collect_link_id: String,
    pub customer_id: id_type::CustomerId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub session_expiry: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    #[serde(flatten)]
    pub ui_config: link_utils::GenericLinkUiConfigFormData,
    pub enabled_payment_methods: Option<Vec<link_utils::EnabledPaymentMethod>>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentMethodCollectLinkStatusDetails {
    pub pm_collect_link_id: String,
    pub customer_id: id_type::CustomerId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub session_expiry: time::PrimitiveDateTime,
    pub return_url: Option<url::Url>,
    pub status: link_utils::PaymentMethodCollectStatus,
    #[serde(flatten)]
    pub ui_config: link_utils::GenericLinkUiConfigFormData,
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
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: id_type::CustomerId,
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
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue1 {
    pub data: payments::WalletData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedWalletValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue1 {
    pub data: payments::BankTransferData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankTransferValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue1 {
    pub data: payments::BankRedirectData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenizedBankRedirectValue2 {
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PaymentMethodRecord {
    pub customer_id: id_type::CustomerId,
    pub name: Option<masking::Secret<String>>,
    pub email: Option<pii::Email>,
    pub phone: Option<masking::Secret<String>>,
    pub phone_country_code: Option<String>,
    pub merchant_id: id_type::MerchantId,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub nick_name: masking::Secret<String>,
    pub payment_instrument_id: masking::Secret<String>,
    pub card_number_masked: masking::Secret<String>,
    pub card_expiry_month: masking::Secret<String>,
    pub card_expiry_year: masking::Secret<String>,
    pub card_scheme: Option<String>,
    pub original_transaction_id: String,
    pub billing_address_zip: masking::Secret<String>,
    pub billing_address_state: masking::Secret<String>,
    pub billing_address_first_name: masking::Secret<String>,
    pub billing_address_last_name: masking::Secret<String>,
    pub billing_address_city: String,
    pub billing_address_country: Option<api_enums::CountryAlpha2>,
    pub billing_address_line1: masking::Secret<String>,
    pub billing_address_line2: Option<masking::Secret<String>>,
    pub billing_address_line3: Option<masking::Secret<String>>,
    pub raw_card_number: Option<masking::Secret<String>>,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
    pub original_transaction_amount: Option<i64>,
    pub original_transaction_currency: Option<common_enums::Currency>,
    pub line_number: Option<i64>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct PaymentMethodMigrationResponse {
    pub line_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<api_enums::PaymentMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub customer_id: Option<id_type::CustomerId>,
    pub migration_status: MigrationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_error: Option<String>,
    pub card_number_masked: Option<masking::Secret<String>>,
}

#[derive(Debug, Default, serde::Serialize)]
pub enum MigrationStatus {
    Success,
    #[default]
    Failed,
}

type PaymentMethodMigrationResponseType =
    (Result<PaymentMethodResponse, String>, PaymentMethodRecord);
#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
impl From<PaymentMethodMigrationResponseType> for PaymentMethodMigrationResponse {
    fn from((response, record): PaymentMethodMigrationResponseType) -> Self {
        match response {
            Ok(res) => Self {
                payment_method_id: Some(res.payment_method_id),
                payment_method: res.payment_method,
                payment_method_type: res.payment_method_type,
                customer_id: res.customer_id,
                migration_status: MigrationStatus::Success,
                migration_error: None,
                card_number_masked: Some(record.card_number_masked),
                line_number: record.line_number,
            },
            Err(e) => Self {
                customer_id: Some(record.customer_id),
                migration_status: MigrationStatus::Failed,
                migration_error: Some(e),
                card_number_masked: Some(record.card_number_masked),
                line_number: record.line_number,
                ..Self::default()
            },
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl From<PaymentMethodMigrationResponseType> for PaymentMethodMigrationResponse {
    fn from((response, record): PaymentMethodMigrationResponseType) -> Self {
        match response {
            Ok(res) => Self {
                payment_method_id: Some(res.payment_method_id),
                payment_method: res.payment_method,
                payment_method_type: res.payment_method_type,
                customer_id: Some(res.customer_id),
                migration_status: MigrationStatus::Success,
                migration_error: None,
                card_number_masked: Some(record.card_number_masked),
                line_number: record.line_number,
            },
            Err(e) => Self {
                customer_id: Some(record.customer_id),
                migration_status: MigrationStatus::Failed,
                migration_error: Some(e),
                card_number_masked: Some(record.card_number_masked),
                line_number: record.line_number,
                ..Self::default()
            },
        }
    }
}

impl From<PaymentMethodRecord> for PaymentMethodMigrate {
    fn from(record: PaymentMethodRecord) -> Self {
        let mut mandate_reference = HashMap::new();
        mandate_reference.insert(
            record.merchant_connector_id,
            PaymentsMandateReferenceRecord {
                connector_mandate_id: record.payment_instrument_id.peek().to_string(),
                payment_method_type: record.payment_method_type,
                original_payment_authorized_amount: record.original_transaction_amount,
                original_payment_authorized_currency: record.original_transaction_currency,
            },
        );
        Self {
            merchant_id: record.merchant_id,
            customer_id: Some(record.customer_id),
            card: Some(MigrateCardDetail {
                card_number: record.raw_card_number.unwrap_or(record.card_number_masked),
                card_exp_month: record.card_expiry_month,
                card_exp_year: record.card_expiry_year,
                card_holder_name: record.name,
                card_network: None,
                card_type: None,
                card_issuer: None,
                card_issuing_country: None,
                nick_name: Some(record.nick_name),
            }),
            payment_method: record.payment_method,
            payment_method_type: record.payment_method_type,
            payment_method_issuer: None,
            billing: Some(payments::Address {
                address: Some(payments::AddressDetails {
                    city: Some(record.billing_address_city),
                    country: record.billing_address_country,
                    line1: Some(record.billing_address_line1),
                    line2: record.billing_address_line2,
                    state: Some(record.billing_address_state),
                    line3: record.billing_address_line3,
                    zip: Some(record.billing_address_zip),
                    first_name: Some(record.billing_address_first_name),
                    last_name: Some(record.billing_address_last_name),
                }),
                phone: Some(payments::PhoneDetails {
                    number: record.phone,
                    country_code: record.phone_country_code,
                }),
                email: record.email,
            }),
            connector_mandate_details: Some(PaymentsMandateReference(mandate_reference)),
            metadata: None,
            payment_method_issuer_code: None,
            card_network: None,
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            #[cfg(feature = "payouts")]
            wallet: None,
            payment_method_data: None,
            network_transaction_id: record.original_transaction_id.into(),
        }
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl From<PaymentMethodRecord> for customers::CustomerRequest {
    fn from(record: PaymentMethodRecord) -> Self {
        Self {
            customer_id: Some(record.customer_id),
            merchant_id: record.merchant_id,
            name: record.name,
            email: record.email,
            phone: record.phone,
            description: None,
            phone_country_code: record.phone_country_code,
            address: Some(payments::AddressDetails {
                city: Some(record.billing_address_city),
                country: record.billing_address_country,
                line1: Some(record.billing_address_line1),
                line2: record.billing_address_line2,
                state: Some(record.billing_address_state),
                line3: record.billing_address_line3,
                zip: Some(record.billing_address_zip),
                first_name: Some(record.billing_address_first_name),
                last_name: Some(record.billing_address_last_name),
            }),
            metadata: None,
        }
    }
}

// #[cfg(feature = "v2")]
// impl From<PaymentMethodRecord> for customers::CustomerRequest {
//     fn from(record: PaymentMethodRecord) -> Self {
//         Self {
//             merchant_reference_id: Some(record.customer_id),
//             name: record.name.unwrap(),
//             email: record.email.unwrap(),
//             phone: record.phone,
//             description: None,
//             phone_country_code: record.phone_country_code,
//             default_billing_address: Some(payments::AddressDetails {
//                 city: Some(record.billing_address_city),
//                 country: record.billing_address_country,
//                 line1: Some(record.billing_address_line1),
//                 line2: record.billing_address_line2,
//                 state: Some(record.billing_address_state),
//                 line3: record.billing_address_line3,
//                 zip: Some(record.billing_address_zip),
//                 first_name: Some(record.billing_address_first_name),
//                 last_name: Some(record.billing_address_last_name),
//             }),
//             default_shipping_address: None,
//             metadata: None,
//         }
//     }
// }
