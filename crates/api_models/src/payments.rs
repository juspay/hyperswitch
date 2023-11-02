use std::num::NonZeroI64;

use cards::CardNumber;
use common_utils::{
    crypto,
    ext_traits::Encode,
    pii::{self, Email},
};
use masking::Secret;
use router_derive::Setter;
use time::PrimitiveDateTime;
use url::Url;
use utoipa::ToSchema;

use crate::{
    admin, disputes,
    enums::{self as api_enums},
    ephemeral_key::EphemeralKeyCreateResponse,
    refunds,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentOp {
    Create,
    Update,
    Confirm,
}

use crate::enums;
#[derive(serde::Deserialize)]
pub struct BankData {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub code_information: Vec<BankCodeInformation>,
}

#[derive(serde::Deserialize)]
pub struct BankCodeInformation {
    pub bank_name: api_enums::BankNames,
    pub connector_codes: Vec<ConnectorCode>,
}

#[derive(serde::Deserialize)]
pub struct ConnectorCode {
    pub connector: api_enums::Connector,
    pub code: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq)]
pub struct BankCodeResponse {
    pub bank_name: Vec<api_enums::BankNames>,
    pub eligible_connectors: Vec<String>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct CustomerDetails {
    /// The identifier for the customer.
    pub id: String,

    /// The customer's name
    #[schema(max_length = 255, value_type = Option<String>, example = "John Doe")]
    pub name: Option<Secret<String>>,

    /// The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Email>,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 10, example = "3141592653")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer's phone number
    #[schema(max_length = 2, example = "+1")]
    pub phone_country_code: Option<String>,
}

#[derive(
    Default,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    ToSchema,
    router_derive::PolymorphicSchema,
)]
#[generate_schemas(PaymentsCreateRequest)]
#[serde(deny_unknown_fields)]
pub struct PaymentsRequest {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    #[schema(
        value_type = Option<String>,
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    #[serde(default, deserialize_with = "payment_id_type::deserialize_option")]
    pub payment_id: Option<PaymentIdType>,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825")]
    pub merchant_id: Option<String>,

    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(value_type = Option<u64>, example = 6540)]
    #[serde(default, deserialize_with = "amount::deserialize_option")]
    #[mandatory_in(PaymentsCreateRequest)]
    // Makes the field mandatory in PaymentsCreateRequest
    pub amount: Option<Amount>,

    #[schema(value_type = Option<RoutingAlgorithm>, example = json!({
        "type": "single",
        "data": "stripe"
    }))]
    pub routing: Option<serde_json::Value>,

    /// This allows the merchant to manually select a connector with which the payment can go through
    #[schema(value_type = Option<Vec<Connector>>, max_length = 255, example = json!(["stripe", "adyen"]))]
    pub connector: Option<Vec<api_enums::Connector>>,

    /// The currency of the payment request can be specified here
    #[schema(value_type = Option<Currency>, example = "USD")]
    #[mandatory_in(PaymentsCreateRequest)]
    pub currency: Option<api_enums::Currency>,

    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    /// The Amount to be captured/ debited from the users payment method. It shall be in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    /// If not provided, the default amount_to_capture will be the payment amount.
    #[schema(example = 6540)]
    pub amount_to_capture: Option<i64>,

    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,

    /// Whether to confirm the payment (if applicable)
    #[schema(default = false, example = true)]
    pub confirm: Option<bool>,

    /// The details of a customer for this payment
    /// This will create the customer if `customer.id` does not exist
    /// If customer id already exists, it will update the details of the customer
    pub customer: Option<CustomerDetails>,

    /// The identifier for the customer object.
    /// This field will be deprecated soon, use the customer object instead
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<String>,

    /// The customer's email address
    /// This field will be deprecated soon, use the customer object instead
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: Option<Email>,

    /// description: The customer's name
    /// This field will be deprecated soon, use the customer object instead
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test")]
    pub name: Option<Secret<String>>,

    /// The customer's phone number
    /// This field will be deprecated soon, use the customer object instead
    #[schema(value_type = Option<String>, max_length = 255, example = "3141592653")]
    pub phone: Option<Secret<String>>,

    /// The country code for the customer phone number
    /// This field will be deprecated soon, use the customer object instead
    #[schema(max_length = 255, example = "+1")]
    pub phone_country_code: Option<String>,

    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with `confirm: true`.
    #[schema(example = true)]
    pub off_session: Option<bool>,

    /// A description of the payment
    #[schema(example = "It's my first payment request")]
    pub description: Option<String>,

    /// The URL to redirect after the completion of the operation
    #[schema(value_type = Option<String>, example = "https://hyperswitch.io")]
    pub return_url: Option<Url>,
    /// Indicates that you intend to make future payments with this Payment’s payment method. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// The transaction authentication can be set to undergo payer authentication.
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// The payment method information provided for making a payment
    #[schema(example = "bank_transfer")]
    pub payment_method_data: Option<PaymentMethodData>,

    /// The payment method that is to be used
    #[schema(value_type = Option<PaymentMethod>, example = "card")]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// Provide a reference to a stored payment method
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// This is used when payment is to be confirmed and the card is not saved
    #[schema(value_type = Option<String>)]
    pub card_cvc: Option<Secret<String>>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// The billing address for the payment
    pub billing: Option<Address>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 255, example = "Hyperswitch Router")]
    pub statement_descriptor_name: Option<String>,

    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    #[schema(max_length = 255, example = "Payment for shoes purchase")]
    pub statement_descriptor_suffix: Option<String>,

    /// Information about the product , quantity and amount for connectors. (e.g. Klarna)
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "gillete creme",
        "quantity": 15,
        "amount" : 900
        "product_img_link" : "https://dummy-img-link.com"
    }]"#)]
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,

    /// It's a token used for client side verification.
    #[schema(example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<String>,

    /// Provide mandate information for creating a mandate
    pub mandate_data: Option<MandateData>,

    /// A unique identifier to link the payment to a mandate, can be use instead of payment_method_data
    #[schema(max_length = 255, example = "mandate_iwer89rnjef349dni3")]
    pub mandate_id: Option<String>,

    /// Additional details required by 3DS 2.0
    #[schema(value_type = Option<Object>, example = r#"{
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
        "accept_header": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
        "language": "nl-NL",
        "color_depth": 24,
        "screen_height": 723,
        "screen_width": 1536,
        "time_zone": 0,
        "java_enabled": true,
        "java_script_enabled":true
    }"#)]
    pub browser_info: Option<serde_json::Value>,

    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<api_enums::PaymentExperience>,

    /// Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "google_pay")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// Business country of the merchant for this payment
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// Business label of the merchant for this payment
    #[schema(example = "food")]
    pub business_label: Option<String>,

    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,

    /// Allowed Payment Method Types for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<Vec<api_enums::PaymentMethodType>>,

    /// Business sub label for the payment
    pub business_sub_label: Option<String>,

    /// Denotes the retry action
    #[schema(value_type = Option<RetryAction>)]
    pub retry_action: Option<api_enums::RetryAction>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// additional data related to some connectors
    pub connector_metadata: Option<ConnectorMetadata>,

    /// additional data that might be required by hyperswitch
    pub feature_metadata: Option<FeatureMetadata>,
    /// payment link object required for generating the payment_link
    pub payment_link_object: Option<PaymentLinkObject>,

    /// The business profile to use for this payment, if not passed the default business profile
    /// associated with the merchant account will be used.
    pub profile_id: Option<String>,

    /// surcharge_details for this payment
    #[schema(value_type = Option<RequestSurchargeDetails>)]
    pub surcharge_details: Option<RequestSurchargeDetails>,

    /// The type of the payment that differentiates between normal and various types of mandate payments
    #[schema(value_type = Option<PaymentType>)]
    pub payment_type: Option<api_enums::PaymentType>,
}

#[derive(
    Default, Debug, Clone, serde::Serialize, serde::Deserialize, Copy, ToSchema, PartialEq,
)]
pub struct RequestSurchargeDetails {
    pub surcharge_amount: i64,
    pub tax_amount: Option<i64>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct HeaderPayload {
    pub payment_confirm_source: Option<api_enums::PaymentSource>,
    pub x_hs_latency: Option<bool>,
}

#[derive(
    Default, Debug, serde::Serialize, Clone, PartialEq, ToSchema, router_derive::PolymorphicSchema,
)]
pub struct PaymentAttemptResponse {
    /// Unique identifier for the attempt
    pub attempt_id: String,
    /// The status of the attempt
    #[schema(value_type = AttemptStatus, example = "charged")]
    pub status: enums::AttemptStatus,
    /// The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    pub amount: i64,
    /// The currency of the amount of the payment attempt
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<enums::Currency>,
    /// The connector used for the payment
    pub connector: Option<String>,
    /// If there was an error while calling the connector the error message is received here
    pub error_message: Option<String>,
    /// The payment method that is to be used
    #[schema(value_type = Option<PaymentMethod>, example = "bank_transfer")]
    pub payment_method: Option<enums::PaymentMethod>,
    /// A unique identifier for a payment provided by the connector
    pub connector_transaction_id: Option<String>,
    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "scheduled")]
    pub capture_method: Option<enums::CaptureMethod>,
    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<enums::AuthenticationType>,
    /// If the payment was cancelled the reason provided here
    pub cancellation_reason: Option<String>,
    /// A unique identifier to link the payment to a mandate, can be use instead of payment_method_data
    pub mandate_id: Option<String>,
    /// If there was an error while calling the connectors the code is received here
    pub error_code: Option<String>,
    /// Provide a reference to a stored payment method
    pub payment_token: Option<String>,
    /// additional data related to some connectors
    pub connector_metadata: Option<serde_json::Value>,
    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<enums::PaymentExperience>,
    /// Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "google_pay")]
    pub payment_method_type: Option<enums::PaymentMethodType>,
    /// reference to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub reference_id: Option<String>,
}

#[derive(
    Default, Debug, serde::Serialize, Clone, PartialEq, ToSchema, router_derive::PolymorphicSchema,
)]
pub struct CaptureResponse {
    /// unique identifier for the capture
    pub capture_id: String,
    /// The status of the capture
    #[schema(value_type = CaptureStatus, example = "charged")]
    pub status: enums::CaptureStatus,
    /// The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    pub amount: i64,
    /// The currency of the amount of the capture
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<enums::Currency>,
    /// The connector used for the payment
    pub connector: String,
    /// unique identifier for the parent attempt on which this capture is made
    pub authorized_attempt_id: String,
    /// A unique identifier for a capture provided by the connector
    pub connector_capture_id: Option<String>,
    /// sequence number of this capture
    pub capture_sequence: i16,
    /// If there was an error while calling the connector the error message is received here
    pub error_message: Option<String>,
    /// If there was an error while calling the connectors the code is received here
    pub error_code: Option<String>,
    /// If there was an error while calling the connectors the reason is received here
    pub error_reason: Option<String>,
    /// reference to the capture at connector side
    pub reference_id: Option<String>,
}

impl PaymentsRequest {
    pub fn get_feature_metadata_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<serde_json::Value>,
        common_utils::errors::ParsingError,
    > {
        self.feature_metadata
            .as_ref()
            .map(Encode::<FeatureMetadata>::encode_to_value)
            .transpose()
    }

    pub fn get_connector_metadata_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<serde_json::Value>,
        common_utils::errors::ParsingError,
    > {
        self.connector_metadata
            .as_ref()
            .map(Encode::<ConnectorMetadata>::encode_to_value)
            .transpose()
    }

    pub fn get_allowed_payment_method_types_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<serde_json::Value>,
        common_utils::errors::ParsingError,
    > {
        self.allowed_payment_method_types
            .as_ref()
            .map(Encode::<Vec<api_enums::PaymentMethodType>>::encode_to_value)
            .transpose()
    }

    pub fn get_order_details_as_value(
        &self,
    ) -> common_utils::errors::CustomResult<
        Option<Vec<pii::SecretSerdeValue>>,
        common_utils::errors::ParsingError,
    > {
        self.order_details
            .as_ref()
            .map(|od| {
                od.iter()
                    .map(|order| {
                        Encode::<OrderDetailsWithAmount>::encode_to_value(order)
                            .map(masking::Secret::new)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    Value(NonZeroI64),
    #[default]
    Zero,
}

impl From<Amount> for i64 {
    fn from(amount: Amount) -> Self {
        match amount {
            Amount::Value(val) => val.get(),
            Amount::Zero => 0,
        }
    }
}

impl From<i64> for Amount {
    fn from(val: i64) -> Self {
        NonZeroI64::new(val).map_or(Self::Zero, Amount::Value)
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PaymentsRedirectRequest {
    pub payment_id: String,
    pub merchant_id: String,
    pub connector: String,
    pub param: String,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    // The merchant_id is generated through api key
    // and is later passed in the struct
    pub merchant_id: Option<String>,
    pub customer_id: Option<String>,
    pub email: Option<Email>,
    pub name: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_data: Option<PaymentMethodData>,
    pub payment_token: Option<String>,
    pub mandate_data: Option<MandateData>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateTransactionType {
    NewMandateTransaction,
    RecurringMandateTransaction,
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct MandateIds {
    pub mandate_id: String,
    pub mandate_reference_id: Option<MandateReferenceId>,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum MandateReferenceId {
    ConnectorMandateId(ConnectorMandateReferenceId), // mandate_id send by connector
    NetworkMandateId(String), // network_txns_id send by Issuer to connector, Used for PG agnostic mandate txns
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ConnectorMandateReferenceId {
    pub connector_mandate_id: Option<String>,
    pub payment_method_id: Option<String>,
}

impl MandateIds {
    pub fn new(mandate_id: String) -> Self {
        Self {
            mandate_id,
            mandate_reference_id: None,
        }
    }
}

// The fields on this struct are optional, as we want to allow the merchant to provide partial
// information about creating mandates
#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MandateData {
    /// A concent from the customer to store the payment method
    pub customer_acceptance: Option<CustomerAcceptance>,
    /// A way to select the type of mandate used
    pub mandate_type: Option<MandateType>,
}

#[derive(Clone, Eq, PartialEq, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SingleUseMandate {
    pub amount: i64,
    pub currency: api_enums::Currency,
}

#[derive(Clone, Eq, PartialEq, Debug, Default, ToSchema, serde::Serialize, serde::Deserialize)]
pub struct MandateAmountData {
    /// The maximum amount to be debited for the mandate transaction
    #[schema(example = 6540)]
    pub amount: i64,
    /// The currency for the transaction
    #[schema(value_type = Currency, example = "USD")]
    pub currency: api_enums::Currency,
    /// Specifying start date of the mandate
    #[schema(example = "2022-09-10T00:00:00Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_date: Option<PrimitiveDateTime>,
    /// Specifying end date of the mandate
    #[schema(example = "2023-09-10T23:59:59Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_date: Option<PrimitiveDateTime>,
    /// Additional details required by mandate
    #[schema(value_type = Option<Object>, example = r#"{
        "frequency": "DAILY"
    }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MandateType {
    /// If the mandate should only be valid for 1 off-session use
    SingleUse(MandateAmountData),
    /// If the mandate should be valid for multiple debits
    MultiUse(Option<MandateAmountData>),
}

impl Default for MandateType {
    fn default() -> Self {
        Self::MultiUse(None)
    }
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomerAcceptance {
    /// Type of acceptance provided by the
    #[schema(example = "online")]
    pub acceptance_type: AcceptanceType,
    /// Specifying when the customer acceptance was provided
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub accepted_at: Option<PrimitiveDateTime>,
    /// Information required for online mandate generation
    pub online: Option<OnlineMandate>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AcceptanceType {
    Online,
    #[default]
    Offline,
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OnlineMandate {
    /// Ip address of the customer machine from which the mandate was created
    #[schema(value_type = String, example = "123.32.25.123")]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
    /// The user-agent of the customer's browser
    pub user_agent: String,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct Card {
    /// The card number
    #[schema(value_type = String, example = "4242424242424242")]
    pub card_number: CardNumber,

    /// The card's expiry month
    #[schema(value_type = String, example = "24")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String, example = "24")]
    pub card_exp_year: Secret<String>,

    /// The card holder's name
    #[schema(value_type = String, example = "John Test")]
    pub card_holder_name: Secret<String>,

    /// The CVC number for the card
    #[schema(value_type = String, example = "242")]
    pub card_cvc: Secret<String>,

    /// The name of the issuer of card
    #[schema(example = "chase")]
    pub card_issuer: Option<String>,

    /// The card network for the card
    #[schema(value_type = Option<CardNetwork>, example = "Visa")]
    pub card_network: Option<api_enums::CardNetwork>,

    #[schema(example = "CREDIT")]
    pub card_type: Option<String>,

    #[schema(example = "INDIA")]
    pub card_issuing_country: Option<String>,

    #[schema(example = "JP_AMEX")]
    pub bank_code: Option<String>,
    /// The card holder's nick name
    #[schema(value_type = Option<String>, example = "John Test")]
    pub nick_name: Option<Secret<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CardRedirectData {
    Knet {},
    Benefit {},
    MomoAtm {},
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayLaterData {
    /// For KlarnaRedirect as PayLater Option
    KlarnaRedirect {
        /// The billing email
        #[schema(value_type = String)]
        billing_email: Email,
        // The billing country code
        #[schema(value_type = CountryAlpha2, example = "US")]
        billing_country: api_enums::CountryAlpha2,
    },
    /// For Klarna Sdk as PayLater Option
    KlarnaSdk {
        /// The token for the sdk workflow
        token: String,
    },
    /// For Affirm redirect as PayLater Option
    AffirmRedirect {},
    /// For AfterpayClearpay redirect as PayLater Option
    AfterpayClearpayRedirect {
        /// The billing email
        #[schema(value_type = String)]
        billing_email: Email,
        /// The billing name
        #[schema(value_type = String)]
        billing_name: Secret<String>,
    },
    /// For PayBright Redirect as PayLater Option
    PayBrightRedirect {},
    /// For WalleyRedirect as PayLater Option
    WalleyRedirect {},
    /// For Alma Redirection as PayLater Option
    AlmaRedirect {},
    AtomeRedirect {},
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitData {
    /// Payment Method data for Ach bank debit
    AchBankDebit {
        /// Billing details for bank debit
        billing_details: BankDebitBilling,
        /// Account number for ach bank debit payment
        #[schema(value_type = String, example = "000123456789")]
        account_number: Secret<String>,
        /// Routing number for ach bank debit payment
        #[schema(value_type = String, example = "110000000")]
        routing_number: Secret<String>,

        #[schema(value_type = String, example = "John Test")]
        card_holder_name: Option<Secret<String>>,

        #[schema(value_type = String, example = "John Doe")]
        bank_account_holder_name: Option<Secret<String>>,

        #[schema(value_type = String, example = "ACH")]
        bank_name: Option<enums::BankNames>,

        #[schema(value_type = String, example = "Checking")]
        bank_type: Option<enums::BankType>,

        #[schema(value_type = String, example = "Personal")]
        bank_holder_type: Option<enums::BankHolderType>,
    },
    SepaBankDebit {
        /// Billing details for bank debit
        billing_details: BankDebitBilling,
        /// International bank account number (iban) for SEPA
        #[schema(value_type = String, example = "DE89370400440532013000")]
        iban: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    BecsBankDebit {
        /// Billing details for bank debit
        billing_details: BankDebitBilling,
        /// Account number for Becs payment method
        #[schema(value_type = String, example = "000123456")]
        account_number: Secret<String>,
        /// Bank-State-Branch (bsb) number
        #[schema(value_type = String, example = "000000")]
        bsb_number: Secret<String>,
        /// Owner name for bank debit
        #[schema(value_type = Option<String>, example = "A. Schneider")]
        bank_account_holder_name: Option<Secret<String>>,
    },
    BacsBankDebit {
        /// Billing details for bank debit
        billing_details: BankDebitBilling,
        /// Account number for Bacs payment method
        #[schema(value_type = String, example = "00012345")]
        account_number: Secret<String>,
        /// Sort code for Bacs payment method
        #[schema(value_type = String, example = "108800")]
        sort_code: Secret<String>,
        /// holder name for bank debit
        #[schema(value_type = String, example = "A. Schneider")]
        bank_account_holder_name: Option<Secret<String>>,
    },
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodData {
    Card(Card),
    CardRedirect(CardRedirectData),
    Wallet(WalletData),
    PayLater(PayLaterData),
    BankRedirect(BankRedirectData),
    BankDebit(BankDebitData),
    BankTransfer(Box<BankTransferData>),
    Crypto(CryptoData),
    MandatePayment,
    Reward,
    Upi(UpiData),
    Voucher(VoucherData),
    GiftCard(Box<GiftCardData>),
}

pub trait GetPaymentMethodType {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType;
}

impl GetPaymentMethodType for CardRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Knet {} => api_enums::PaymentMethodType::Knet,
            Self::Benefit {} => api_enums::PaymentMethodType::Benefit,
            Self::MomoAtm {} => api_enums::PaymentMethodType::MomoAtm,
        }
    }
}

impl GetPaymentMethodType for WalletData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AliPayQr(_) | Self::AliPayRedirect(_) => api_enums::PaymentMethodType::AliPay,
            Self::AliPayHkRedirect(_) => api_enums::PaymentMethodType::AliPayHk,
            Self::MomoRedirect(_) => api_enums::PaymentMethodType::Momo,
            Self::KakaoPayRedirect(_) => api_enums::PaymentMethodType::KakaoPay,
            Self::GoPayRedirect(_) => api_enums::PaymentMethodType::GoPay,
            Self::GcashRedirect(_) => api_enums::PaymentMethodType::Gcash,
            Self::ApplePay(_) | Self::ApplePayRedirect(_) | Self::ApplePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::ApplePay
            }
            Self::DanaRedirect {} => api_enums::PaymentMethodType::Dana,
            Self::GooglePay(_) | Self::GooglePayRedirect(_) | Self::GooglePayThirdPartySdk(_) => {
                api_enums::PaymentMethodType::GooglePay
            }
            Self::MbWayRedirect(_) => api_enums::PaymentMethodType::MbWay,
            Self::MobilePayRedirect(_) => api_enums::PaymentMethodType::MobilePay,
            Self::PaypalRedirect(_) | Self::PaypalSdk(_) => api_enums::PaymentMethodType::Paypal,
            Self::SamsungPay(_) => api_enums::PaymentMethodType::SamsungPay,
            Self::TwintRedirect {} => api_enums::PaymentMethodType::Twint,
            Self::VippsRedirect {} => api_enums::PaymentMethodType::Vipps,
            Self::TouchNGoRedirect(_) => api_enums::PaymentMethodType::TouchNGo,
            Self::WeChatPayRedirect(_) | Self::WeChatPayQr(_) => {
                api_enums::PaymentMethodType::WeChatPay
            }
            Self::CashappQr(_) => api_enums::PaymentMethodType::Cashapp,
            Self::SwishQr(_) => api_enums::PaymentMethodType::Swish,
        }
    }
}

impl GetPaymentMethodType for PayLaterData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::KlarnaRedirect { .. } => api_enums::PaymentMethodType::Klarna,
            Self::KlarnaSdk { .. } => api_enums::PaymentMethodType::Klarna,
            Self::AffirmRedirect {} => api_enums::PaymentMethodType::Affirm,
            Self::AfterpayClearpayRedirect { .. } => api_enums::PaymentMethodType::AfterpayClearpay,
            Self::PayBrightRedirect {} => api_enums::PaymentMethodType::PayBright,
            Self::WalleyRedirect {} => api_enums::PaymentMethodType::Walley,
            Self::AlmaRedirect {} => api_enums::PaymentMethodType::Alma,
            Self::AtomeRedirect {} => api_enums::PaymentMethodType::Atome,
        }
    }
}

impl GetPaymentMethodType for BankRedirectData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::BancontactCard { .. } => api_enums::PaymentMethodType::BancontactCard,
            Self::Bizum {} => api_enums::PaymentMethodType::Bizum,
            Self::Blik { .. } => api_enums::PaymentMethodType::Blik,
            Self::Eps { .. } => api_enums::PaymentMethodType::Eps,
            Self::Giropay { .. } => api_enums::PaymentMethodType::Giropay,
            Self::Ideal { .. } => api_enums::PaymentMethodType::Ideal,
            Self::Interac { .. } => api_enums::PaymentMethodType::Interac,
            Self::OnlineBankingCzechRepublic { .. } => {
                api_enums::PaymentMethodType::OnlineBankingCzechRepublic
            }
            Self::OnlineBankingFinland { .. } => api_enums::PaymentMethodType::OnlineBankingFinland,
            Self::OnlineBankingPoland { .. } => api_enums::PaymentMethodType::OnlineBankingPoland,
            Self::OnlineBankingSlovakia { .. } => {
                api_enums::PaymentMethodType::OnlineBankingSlovakia
            }
            Self::OpenBankingUk { .. } => api_enums::PaymentMethodType::OpenBankingUk,
            Self::Przelewy24 { .. } => api_enums::PaymentMethodType::Przelewy24,
            Self::Sofort { .. } => api_enums::PaymentMethodType::Sofort,
            Self::Trustly { .. } => api_enums::PaymentMethodType::Trustly,
            Self::OnlineBankingFpx { .. } => api_enums::PaymentMethodType::OnlineBankingFpx,
            Self::OnlineBankingThailand { .. } => {
                api_enums::PaymentMethodType::OnlineBankingThailand
            }
        }
    }
}

impl GetPaymentMethodType for BankDebitData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankDebit { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankDebit { .. } => api_enums::PaymentMethodType::Sepa,
            Self::BecsBankDebit { .. } => api_enums::PaymentMethodType::Becs,
            Self::BacsBankDebit { .. } => api_enums::PaymentMethodType::Bacs,
        }
    }
}

impl GetPaymentMethodType for BankTransferData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::AchBankTransfer { .. } => api_enums::PaymentMethodType::Ach,
            Self::SepaBankTransfer { .. } => api_enums::PaymentMethodType::Sepa,
            Self::BacsBankTransfer { .. } => api_enums::PaymentMethodType::Bacs,
            Self::MultibancoBankTransfer { .. } => api_enums::PaymentMethodType::Multibanco,
            Self::PermataBankTransfer { .. } => api_enums::PaymentMethodType::PermataBankTransfer,
            Self::BcaBankTransfer { .. } => api_enums::PaymentMethodType::BcaBankTransfer,
            Self::BniVaBankTransfer { .. } => api_enums::PaymentMethodType::BniVa,
            Self::BriVaBankTransfer { .. } => api_enums::PaymentMethodType::BriVa,
            Self::CimbVaBankTransfer { .. } => api_enums::PaymentMethodType::CimbVa,
            Self::DanamonVaBankTransfer { .. } => api_enums::PaymentMethodType::DanamonVa,
            Self::MandiriVaBankTransfer { .. } => api_enums::PaymentMethodType::MandiriVa,
            Self::Pix {} => api_enums::PaymentMethodType::Pix,
            Self::Pse {} => api_enums::PaymentMethodType::Pse,
        }
    }
}

impl GetPaymentMethodType for CryptoData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        api_enums::PaymentMethodType::CryptoCurrency
    }
}

impl GetPaymentMethodType for UpiData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        api_enums::PaymentMethodType::UpiCollect
    }
}
impl GetPaymentMethodType for VoucherData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Boleto(_) => api_enums::PaymentMethodType::Boleto,
            Self::Efecty => api_enums::PaymentMethodType::Efecty,
            Self::PagoEfectivo => api_enums::PaymentMethodType::PagoEfectivo,
            Self::RedCompra => api_enums::PaymentMethodType::RedCompra,
            Self::RedPagos => api_enums::PaymentMethodType::RedPagos,
            Self::Alfamart(_) => api_enums::PaymentMethodType::Alfamart,
            Self::Indomaret(_) => api_enums::PaymentMethodType::Indomaret,
            Self::Oxxo => api_enums::PaymentMethodType::Oxxo,
            Self::SevenEleven(_) => api_enums::PaymentMethodType::SevenEleven,
            Self::Lawson(_) => api_enums::PaymentMethodType::Lawson,
            Self::MiniStop(_) => api_enums::PaymentMethodType::MiniStop,
            Self::FamilyMart(_) => api_enums::PaymentMethodType::FamilyMart,
            Self::Seicomart(_) => api_enums::PaymentMethodType::Seicomart,
            Self::PayEasy(_) => api_enums::PaymentMethodType::PayEasy,
        }
    }
}
impl GetPaymentMethodType for GiftCardData {
    fn get_payment_method_type(&self) -> api_enums::PaymentMethodType {
        match self {
            Self::Givex(_) => api_enums::PaymentMethodType::Givex,
            Self::PaySafeCard {} => api_enums::PaymentMethodType::PaySafeCard,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GiftCardData {
    Givex(GiftCardDetails),
    PaySafeCard {},
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GiftCardDetails {
    /// The gift card number
    #[schema(value_type = String)]
    pub number: Secret<String>,
    /// The card verification code.
    #[schema(value_type = String)]
    pub cvc: Secret<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AdditionalCardInfo {
    pub card_issuer: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code: Option<String>,
    pub last4: Option<String>,
    pub card_isin: Option<String>,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalPaymentData {
    Card(Box<AdditionalCardInfo>),
    BankRedirect {
        bank_name: Option<api_enums::BankNames>,
    },
    Wallet {},
    PayLater {},
    BankTransfer {},
    Crypto {},
    BankDebit {},
    MandatePayment {},
    Reward {},
    Upi {},
    GiftCard {},
    Voucher {},
    CardRedirect {},
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankRedirectData {
    BancontactCard {
        /// The card number
        #[schema(value_type = String, example = "4242424242424242")]
        card_number: Option<CardNumber>,
        /// The card's expiry month
        #[schema(value_type = String, example = "24")]
        card_exp_month: Option<Secret<String>>,

        /// The card's expiry year
        #[schema(value_type = String, example = "24")]
        card_exp_year: Option<Secret<String>>,

        /// The card holder's name
        #[schema(value_type = String, example = "John Test")]
        card_holder_name: Option<Secret<String>>,

        //Required by Stripes
        billing_details: Option<BankRedirectBilling>,
    },
    Bizum {},
    Blik {
        // Blik Code
        blik_code: Option<String>,
    },
    Eps {
        /// The billing details for bank redirection
        billing_details: BankRedirectBilling,

        /// The hyperswitch bank code for eps
        #[schema(value_type = BankNames, example = "triodos_bank")]
        bank_name: Option<api_enums::BankNames>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Giropay {
        /// The billing details for bank redirection
        billing_details: BankRedirectBilling,
        /// Bank account details for Giropay

        #[schema(value_type = Option<String>)]
        /// Bank account bic code
        bank_account_bic: Option<Secret<String>>,

        /// Bank account iban
        #[schema(value_type = Option<String>)]
        bank_account_iban: Option<Secret<String>>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Ideal {
        /// The billing details for bank redirection
        billing_details: BankRedirectBilling,

        /// The hyperswitch bank code for ideal
        #[schema(value_type = BankNames, example = "abn_amro")]
        bank_name: Option<api_enums::BankNames>,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: Option<api_enums::CountryAlpha2>,
    },
    Interac {
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: api_enums::CountryAlpha2,

        #[schema(value_type = String, example = "john.doe@example.com")]
        email: Email,
    },
    OnlineBankingCzechRepublic {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: api_enums::BankNames,
    },
    OnlineBankingFinland {
        // Shopper Email
        #[schema(value_type = Option<String>)]
        email: Option<Email>,
    },
    OnlineBankingPoland {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: api_enums::BankNames,
    },
    OnlineBankingSlovakia {
        // Issuer value corresponds to the bank
        #[schema(value_type = BankNames)]
        issuer: api_enums::BankNames,
    },
    OpenBankingUk {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: api_enums::BankNames,
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: api_enums::CountryAlpha2,
    },
    Przelewy24 {
        //Issuer banks
        #[schema(value_type = Option<BankNames>)]
        bank_name: Option<api_enums::BankNames>,

        // The billing details for bank redirect
        billing_details: BankRedirectBilling,
    },
    Sofort {
        /// The billing details for bank redirection
        billing_details: BankRedirectBilling,

        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: api_enums::CountryAlpha2,

        /// The preferred language
        #[schema(example = "en")]
        preferred_language: String,
    },
    Trustly {
        /// The country for bank payment
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: api_enums::CountryAlpha2,
    },
    OnlineBankingFpx {
        // Issuer banks
        #[schema(value_type = BankNames)]
        issuer: api_enums::BankNames,
    },
    OnlineBankingThailand {
        #[schema(value_type = BankNames)]
        issuer: api_enums::BankNames,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AlfamartVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name for Alfamart
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct IndomaretVoucherData {
    /// The billing first name for Alfamart
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name for Alfamart
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Alfamart
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct JCSVoucherData {
    /// The billing first name for Japanese convenience stores
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name Japanese convenience stores
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Japanese convenience stores
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
    /// The telephone number for Japanese convenience stores
    #[schema(value_type = String, example = "9999999999")]
    pub phone_number: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AchBillingDetails {
    /// The Email ID for ACH billing
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct DokuBillingDetails {
    /// The billing first name for Doku
    #[schema(value_type = String, example = "Jane")]
    pub first_name: Secret<String>,
    /// The billing second name for Doku
    #[schema(value_type = String, example = "Doe")]
    pub last_name: Option<Secret<String>>,
    /// The Email ID for Doku billing
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MultibancoBillingDetails {
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    #[schema(value_type = String, example = "example@me.com")]
    pub email: Email,
    /// The billing name for SEPA and BACS billing
    #[schema(value_type = String, example = "Jane Doe")]
    pub name: Secret<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CryptoData {
    pub pay_currency: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpiData {
    #[schema(value_type = Option<String>, example = "successtest@iata")]
    pub vpa_id: Option<Secret<String, pii::UpiVpaMaskingStrategy>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SofortBilling {
    /// The country associated with the billing
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub billing_country: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BankRedirectBilling {
    /// The name for which billing is issued
    #[schema(value_type = String, example = "John Doe")]
    pub billing_name: Option<Secret<String>>,
    /// The billing email for bank redirect
    #[schema(value_type = String, example = "example@example.com")]
    pub email: Option<Email>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferData {
    AchBankTransfer {
        /// The billing details for ACH Bank Transfer
        billing_details: AchBillingDetails,
    },
    SepaBankTransfer {
        /// The billing details for SEPA
        billing_details: SepaAndBacsBillingDetails,

        /// The two-letter ISO country code for SEPA and BACS
        #[schema(value_type = CountryAlpha2, example = "US")]
        country: api_enums::CountryAlpha2,
    },
    BacsBankTransfer {
        /// The billing details for SEPA
        billing_details: SepaAndBacsBillingDetails,
    },
    MultibancoBankTransfer {
        /// The billing details for Multibanco
        billing_details: MultibancoBillingDetails,
    },
    PermataBankTransfer {
        /// The billing details for Permata Bank Transfer
        billing_details: DokuBillingDetails,
    },
    BcaBankTransfer {
        /// The billing details for BCA Bank Transfer
        billing_details: DokuBillingDetails,
    },
    BniVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    BriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    CimbVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    DanamonVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    MandiriVaBankTransfer {
        /// The billing details for BniVa Bank Transfer
        billing_details: DokuBillingDetails,
    },
    Pix {},
    Pse {},
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, ToSchema, Eq, PartialEq)]
pub struct BankDebitBilling {
    /// The billing name for bank debits
    #[schema(value_type = String, example = "John Doe")]
    pub name: Secret<String>,
    /// The billing email for bank debits
    #[schema(value_type = String, example = "example@example.com")]
    pub email: Email,
    /// The billing address for bank debits
    pub address: Option<AddressDetails>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WalletData {
    /// The wallet data for Ali Pay QrCode
    AliPayQr(Box<AliPayQr>),
    /// The wallet data for Ali Pay redirect
    AliPayRedirect(AliPayRedirection),
    /// The wallet data for Ali Pay HK redirect
    AliPayHkRedirect(AliPayHkRedirection),
    /// The wallet data for Momo redirect
    MomoRedirect(MomoRedirection),
    /// The wallet data for KakaoPay redirect
    KakaoPayRedirect(KakaoPayRedirection),
    /// The wallet data for GoPay redirect
    GoPayRedirect(GoPayRedirection),
    /// The wallet data for Gcash redirect
    GcashRedirect(GcashRedirection),
    /// The wallet data for Apple pay
    ApplePay(ApplePayWalletData),
    /// Wallet data for apple pay redirect flow
    ApplePayRedirect(Box<ApplePayRedirectData>),
    /// Wallet data for apple pay third party sdk flow
    ApplePayThirdPartySdk(Box<ApplePayThirdPartySdkData>),
    /// Wallet data for DANA redirect flow
    DanaRedirect {},
    /// The wallet data for Google pay
    GooglePay(GooglePayWalletData),
    /// Wallet data for google pay redirect flow
    GooglePayRedirect(Box<GooglePayRedirectData>),
    /// Wallet data for Google pay third party sdk flow
    GooglePayThirdPartySdk(Box<GooglePayThirdPartySdkData>),
    MbWayRedirect(Box<MbWayRedirection>),
    /// The wallet data for MobilePay redirect
    MobilePayRedirect(Box<MobilePayRedirection>),
    /// This is for paypal redirection
    PaypalRedirect(PaypalRedirection),
    /// The wallet data for Paypal
    PaypalSdk(PayPalWalletData),
    /// The wallet data for Samsung Pay
    SamsungPay(Box<SamsungPayWalletData>),
    /// Wallet data for Twint Redirection
    TwintRedirect {},
    /// Wallet data for Vipps Redirection
    VippsRedirect {},
    /// The wallet data for Touch n Go Redirection
    TouchNGoRedirect(Box<TouchNGoRedirection>),
    /// The wallet data for WeChat Pay Redirection
    WeChatPayRedirect(Box<WeChatPayRedirection>),
    /// The wallet data for WeChat Pay Display QrCode
    WeChatPayQr(Box<WeChatPayQr>),
    /// The wallet data for Cashapp Qr
    CashappQr(Box<CashappQr>),
    // The wallet data for Swish
    SwishQr(SwishQrData),
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SamsungPayWalletData {
    /// The encrypted payment token from Samsung
    #[schema(value_type = String)]
    pub token: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayWalletData {
    /// The type of payment method
    #[serde(rename = "type")]
    pub pm_type: String,
    /// User-facing message to describe the payment method that funds this transaction.
    pub description: String,
    /// The information of the payment method
    pub info: GooglePayPaymentMethodInfo,
    /// The tokenization data of Google pay
    pub tokenization_data: GpayTokenizationData,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GooglePayRedirectData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GooglePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayThirdPartySdkData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPay {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct WeChatPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CashappQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaypalRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayQr {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct AliPayHkRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MomoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct KakaoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GoPayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GcashRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MobilePayRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    #[schema(value_type = String)]
    pub telephone_number: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct GooglePayPaymentMethodInfo {
    /// The name of the card network
    pub card_network: String,
    /// The details of the card
    pub card_details: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PayPalWalletData {
    /// Token generated for the Apple pay
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct TouchNGoRedirection {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SwishQrData {}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GpayTokenizationData {
    /// The type of the token
    #[serde(rename = "type")]
    pub token_type: String,
    /// Token generated for the wallet
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplePayWalletData {
    /// The payment data of Apple pay
    pub payment_data: String,
    /// The payment method of Apple pay
    pub payment_method: ApplepayPaymentMethod,
    /// The unique identifier for the transaction
    pub transaction_identifier: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ApplepayPaymentMethod {
    /// The name to be displayed on Apple Pay button
    pub display_name: String,
    /// The network of the Apple pay payment method
    pub network: String,
    /// The type of the payment method
    #[serde(rename = "type")]
    pub pm_type: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CardResponse {
    pub last4: Option<String>,
    pub card_type: Option<String>,
    pub card_network: Option<api_enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub card_issuing_country: Option<String>,
    pub card_isin: Option<String>,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RewardData {
    /// The merchant ID with which we have to call the connector
    pub merchant_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BoletoVoucherData {
    /// The shopper's social security number
    #[schema(value_type = Option<String>)]
    pub social_security_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoucherData {
    Boleto(Box<BoletoVoucherData>),
    Efecty,
    PagoEfectivo,
    RedCompra,
    RedPagos,
    Alfamart(Box<AlfamartVoucherData>),
    Indomaret(Box<IndomaretVoucherData>),
    Oxxo,
    SevenEleven(Box<JCSVoucherData>),
    Lawson(Box<JCSVoucherData>),
    MiniStop(Box<JCSVoucherData>),
    FamilyMart(Box<JCSVoucherData>),
    Seicomart(Box<JCSVoucherData>),
    PayEasy(Box<JCSVoucherData>),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodDataResponse {
    #[serde(rename = "card")]
    Card(CardResponse),
    BankTransfer,
    Wallet,
    PayLater,
    Paypal,
    BankRedirect,
    Crypto,
    BankDebit,
    MandatePayment,
    Reward,
    Upi,
    Voucher,
    GiftCard,
    CardRedirect,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema)]
pub enum PaymentIdType {
    /// The identifier for payment intent
    PaymentIntentId(String),
    /// The identifier for connector transaction
    ConnectorTransactionId(String),
    /// The identifier for payment attempt
    PaymentAttemptId(String),
    /// The identifier for preprocessing step
    PreprocessingId(String),
}

impl std::fmt::Display for PaymentIdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaymentIntentId(payment_id) => {
                write!(f, "payment_intent_id = \"{payment_id}\"")
            }
            Self::ConnectorTransactionId(connector_transaction_id) => write!(
                f,
                "connector_transaction_id = \"{connector_transaction_id}\""
            ),
            Self::PaymentAttemptId(payment_attempt_id) => {
                write!(f, "payment_attempt_id = \"{payment_attempt_id}\"")
            }
            Self::PreprocessingId(preprocessing_id) => {
                write!(f, "preprocessing_id = \"{preprocessing_id}\"")
            }
        }
    }
}

impl Default for PaymentIdType {
    fn default() -> Self {
        Self::PaymentIntentId(Default::default())
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, ToSchema, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    /// Provide the address details
    pub address: Option<AddressDetails>,

    pub phone: Option<PhoneDetails>,
}

// used by customers also, could be moved outside
#[derive(Clone, Default, Debug, Eq, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddressDetails {
    /// The address city
    #[schema(max_length = 50, example = "New York")]
    pub city: Option<String>,

    /// The two-letter ISO country code for the address
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub country: Option<api_enums::CountryAlpha2>,

    /// The first line of the address
    #[schema(value_type = Option<String>, max_length = 200, example = "123, King Street")]
    pub line1: Option<Secret<String>>,

    /// The second line of the address
    #[schema(value_type = Option<String>, max_length = 50, example = "Powelson Avenue")]
    pub line2: Option<Secret<String>>,

    /// The third line of the address
    #[schema(value_type = Option<String>, max_length = 50, example = "Bridgewater")]
    pub line3: Option<Secret<String>>,

    /// The zip/postal code for the address
    #[schema(value_type = Option<String>, max_length = 50, example = "08807")]
    pub zip: Option<Secret<String>>,

    /// The address state
    #[schema(value_type = Option<String>, example = "New York")]
    pub state: Option<Secret<String>>,

    /// The first name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "John")]
    pub first_name: Option<Secret<String>>,

    /// The last name for the address
    #[schema(value_type = Option<String>, max_length = 255, example = "Doe")]
    pub last_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, ToSchema, serde::Deserialize, serde::Serialize)]
pub struct PhoneDetails {
    /// The contact number
    #[schema(value_type = Option<String>, example = "9999999999")]
    pub number: Option<Secret<String>>,
    /// The country code attached to the number
    #[schema(example = "+1")]
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PaymentsCaptureRequest {
    /// The unique identifier for the payment
    #[serde(skip_deserializing)]
    pub payment_id: String,
    /// The unique identifier for the merchant
    pub merchant_id: Option<String>,
    /// The Amount to be captured/ debited from the user's payment method.
    pub amount_to_capture: Option<i64>,
    /// Decider to refund the uncaptured amount
    pub refund_uncaptured_amount: Option<bool>,
    /// Provides information about a card payment that customers see on their statements.
    pub statement_descriptor_suffix: Option<String>,
    /// Concatenated with the statement descriptor suffix that’s set on the account to form the complete statement descriptor.
    pub statement_descriptor_prefix: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct UrlDetails {
    pub url: String,
    pub method: String,
}
#[derive(Default, Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct AuthenticationForStartResponse {
    pub authentication: UrlDetails,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionType {
    RedirectToUrl,
    DisplayQrCode,
    InvokeSdkClient,
    TriggerApi,
    DisplayBankTransferInformation,
    DisplayWaitScreen,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NextActionData {
    /// Contains the url for redirection flow
    RedirectToUrl { redirect_to_url: String },
    /// Informs the next steps for bank transfer and also contains the charges details (ex: amount received, amount charged etc)
    DisplayBankTransferInformation {
        bank_transfer_steps_and_charges_details: BankTransferNextStepsData,
    },
    /// Contains third party sdk session token response
    ThirdPartySdkSessionToken { session_token: Option<SessionToken> },
    /// Contains url for Qr code image, this qr code has to be shown in sdk
    QrCodeInformation {
        #[schema(value_type = String)]
        image_data_url: Url,
        display_to_timestamp: Option<i64>,
    },
    /// Contains the download url and the reference number for transaction
    DisplayVoucherInformation {
        #[schema(value_type = String)]
        voucher_details: VoucherNextStepData,
    },
    /// Contains duration for displaying a wait screen, wait screen with timer is displayed by sdk
    WaitScreenInformation {
        display_from_timestamp: i128,
        display_to_timestamp: Option<i128>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct BankTransferNextStepsData {
    /// The instructions for performing a bank transfer
    #[serde(flatten)]
    pub bank_transfer_instructions: BankTransferInstructions,
    /// The details received by the receiver
    pub receiver: Option<ReceiverDetails>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct VoucherNextStepData {
    /// Voucher expiry date and time
    pub expires_at: Option<String>,
    /// Reference number required for the transaction
    pub reference: String,
    /// Url to download the payment instruction
    pub download_url: Option<Url>,
    /// Url to payment instruction page
    pub instructions_url: Option<Url>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct QrCodeNextStepsInstruction {
    pub image_data_url: Url,
    pub display_to_timestamp: Option<i64>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct WaitScreenInstructions {
    pub display_from_timestamp: i128,
    pub display_to_timestamp: Option<i128>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferInstructions {
    /// The instructions for Doku bank transactions
    DokuBankTransferInstructions(Box<DokuBankTransferInstructions>),
    /// The credit transfer for ACH transactions
    AchCreditTransfer(Box<AchTransfer>),
    /// The instructions for SEPA bank transactions
    SepaBankInstructions(Box<SepaBankTransferInstructions>),
    /// The instructions for BACS bank transactions
    BacsBankInstructions(Box<BacsBankTransferInstructions>),
    /// The instructions for Multibanco bank transactions
    Multibanco(Box<MultibancoTransferInstructions>),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct SepaBankTransferInstructions {
    #[schema(value_type = String, example = "Jane Doe")]
    pub account_holder_name: Secret<String>,
    #[schema(value_type = String, example = "1024419982")]
    pub bic: Secret<String>,
    pub country: String,
    #[schema(value_type = String, example = "123456789")]
    pub iban: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct BacsBankTransferInstructions {
    #[schema(value_type = String, example = "Jane Doe")]
    pub account_holder_name: Secret<String>,
    #[schema(value_type = String, example = "10244123908")]
    pub account_number: Secret<String>,
    #[schema(value_type = String, example = "012")]
    pub sort_code: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct MultibancoTransferInstructions {
    #[schema(value_type = String, example = "122385736258")]
    pub reference: Secret<String>,
    #[schema(value_type = String, example = "12345")]
    pub entity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct DokuBankTransferInstructions {
    #[schema(value_type = String, example = "2023-07-26T17:33:00-07-21")]
    pub expires_at: Option<String>,
    #[schema(value_type = String, example = "122385736258")]
    pub reference: Secret<String>,
    #[schema(value_type = String)]
    pub instructions_url: Option<Url>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AchTransfer {
    #[schema(value_type = String, example = "122385736258")]
    pub account_number: Secret<String>,
    pub bank_name: String,
    #[schema(value_type = String, example = "012")]
    pub routing_number: Secret<String>,
    #[schema(value_type = String, example = "234")]
    pub swift_code: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ReceiverDetails {
    /// The amount received by receiver
    amount_received: i64,
    /// The amount charged by ACH
    amount_charged: Option<i64>,
    /// The amount remaining to be sent via ACH
    amount_remaining: Option<i64>,
}

#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct PaymentsResponse {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    #[schema(
        min_length = 30,
        max_length = 30,
        example = "pay_mbabizu24mvu3mela5njyhpit4"
    )]
    pub payment_id: Option<String>,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(max_length = 255, example = "merchant_1668273825")]
    pub merchant_id: Option<String>,

    /// The status of the current payment that was made
    #[schema(value_type = IntentStatus, example = "failed", default = "requires_confirmation")]
    pub status: api_enums::IntentStatus,

    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    #[schema(example = 100)]
    pub amount: i64,

    /// The maximum amount that could be captured from the payment
    #[schema(minimum = 100, example = 6540)]
    pub amount_capturable: Option<i64>,

    /// The amount which is already captured from the payment
    #[schema(minimum = 100, example = 6540)]
    pub amount_received: Option<i64>,

    /// The connector used for the payment
    #[schema(example = "stripe")]
    pub connector: Option<String>,

    /// It's a token used for client side verification.
    #[schema(value_type = Option<String>, example = "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<Secret<String>>,

    /// Time when the payment was created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,

    /// The currency of the amount of the payment
    #[schema(value_type = Currency, example = "USD")]
    pub currency: String,

    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    #[schema(max_length = 255, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub customer_id: Option<String>,

    /// A description of the payment
    #[schema(example = "It's my first payment request")]
    pub description: Option<String>,

    /// List of refund that happened on this intent
    #[schema(value_type = Option<Vec<RefundResponse>>)]
    pub refunds: Option<Vec<refunds::RefundResponse>>,

    /// List of dispute that happened on this intent
    #[schema(value_type = Option<Vec<DisputeResponsePaymentsRetrieve>>)]
    pub disputes: Option<Vec<disputes::DisputeResponsePaymentsRetrieve>>,

    /// List of attempts that happened on this intent
    #[schema(value_type = Option<Vec<PaymentAttemptResponse>>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<Vec<PaymentAttemptResponse>>,

    /// List of captures done on latest attempt
    #[schema(value_type = Option<Vec<CaptureResponse>>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captures: Option<Vec<CaptureResponse>>,

    /// A unique identifier to link the payment to a mandate, can be use instead of payment_method_data
    #[schema(max_length = 255, example = "mandate_iwer89rnjef349dni3")]
    pub mandate_id: Option<String>,

    /// Provided mandate information for creating a mandate
    #[auth_based]
    pub mandate_data: Option<MandateData>,

    /// Indicates that you intend to make future payments with this Payment’s payment method. Providing this parameter will attach the payment method to the Customer, if present, after the Payment is confirmed and any required actions from the user are complete.
    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<api_enums::FutureUsage>,

    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with confirm=true.
    #[schema(example = true)]
    pub off_session: Option<bool>,

    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,

    /// This is the instruction for capture/ debit the money from the users' card. On the other hand authorization refers to blocking the amount on the users' payment method.
    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub capture_method: Option<api_enums::CaptureMethod>,

    /// The payment method that is to be used
    #[schema(value_type = PaymentMethodType, example = "bank_transfer")]
    #[auth_based]
    pub payment_method: Option<api_enums::PaymentMethod>,

    /// The payment method information provided for making a payment
    #[schema(value_type = Option<PaymentMethod>, example = "bank_transfer")]
    #[auth_based]
    pub payment_method_data: Option<PaymentMethodDataResponse>,

    /// Provide a reference to a stored payment method
    #[schema(example = "187282ab-40ef-47a9-9206-5099ba31e432")]
    pub payment_token: Option<String>,

    /// The shipping address for the payment
    pub shipping: Option<Address>,

    /// The billing address for the payment
    pub billing: Option<Address>,

    /// Information about the product , quantity and amount for connectors. (e.g. Klarna)
    #[schema(value_type = Option<Vec<OrderDetailsWithAmount>>, example = r#"[{
        "product_name": "gillete creme",
        "quantity": 15,
        "amount" : 900
    }]"#)]
    pub order_details: Option<Vec<pii::SecretSerdeValue>>,

    /// description: The customer's email address
    #[schema(max_length = 255, value_type = Option<String>, example = "johntest@test.com")]
    pub email: crypto::OptionalEncryptableEmail,

    /// description: The customer's name
    #[schema(value_type = Option<String>, max_length = 255, example = "John Test")]
    pub name: crypto::OptionalEncryptableName,

    /// The customer's phone number
    #[schema(value_type = Option<String>, max_length = 255, example = "3141592653")]
    pub phone: crypto::OptionalEncryptablePhone,

    /// The URL to redirect after the completion of the operation
    #[schema(example = "https://hyperswitch.io")]
    pub return_url: Option<String>,

    /// The transaction authentication can be set to undergo payer authentication. By default, the authentication will be marked as NO_THREE_DS
    #[schema(value_type = Option<AuthenticationType>, example = "no_three_ds", default = "three_ds")]
    pub authentication_type: Option<api_enums::AuthenticationType>,

    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    #[schema(max_length = 255, example = "Hyperswitch Router")]
    pub statement_descriptor_name: Option<String>,

    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 255 characters for the concatenated descriptor.
    #[schema(max_length = 255, example = "Payment for shoes purchase")]
    pub statement_descriptor_suffix: Option<String>,

    /// Additional information required for redirection
    pub next_action: Option<NextActionData>,

    /// If the payment was cancelled the reason provided here
    pub cancellation_reason: Option<String>,

    /// If there was an error while calling the connectors the code is received here
    #[schema(example = "E0001")]
    pub error_code: Option<String>,

    /// If there was an error while calling the connector the error message is received here
    #[schema(example = "Failed while verifying the card")]
    pub error_message: Option<String>,

    /// Payment Experience for the current payment
    #[schema(value_type = Option<PaymentExperience>, example = "redirect_to_url")]
    pub payment_experience: Option<api_enums::PaymentExperience>,

    /// Payment Method Type
    #[schema(value_type = Option<PaymentMethodType>, example = "gpay")]
    pub payment_method_type: Option<api_enums::PaymentMethodType>,

    /// The connector used for this payment along with the country and business details
    #[schema(example = "stripe_US_food")]
    pub connector_label: Option<String>,

    /// The business country of merchant for this payment
    #[schema(value_type = Option<CountryAlpha2>, example = "US")]
    pub business_country: Option<api_enums::CountryAlpha2>,

    /// The business label of merchant for this payment
    pub business_label: Option<String>,

    /// The business_sub_label for this payment
    pub business_sub_label: Option<String>,

    /// Allowed Payment Method Types for a given PaymentIntent
    #[schema(value_type = Option<Vec<PaymentMethodType>>)]
    pub allowed_payment_method_types: Option<serde_json::Value>,

    /// ephemeral_key for the customer_id mentioned
    pub ephemeral_key: Option<EphemeralKeyCreateResponse>,

    /// If true the payment can be retried with same or different payment method which means the confirm call can be made again.
    pub manual_retry_allowed: Option<bool>,

    /// A unique identifier for a payment provided by the connector
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub connector_transaction_id: Option<String>,

    /// Frm message contains information about the frm response
    pub frm_message: Option<FrmMessage>,

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<pii::SecretSerdeValue>,

    /// additional data related to some connectors
    #[schema(value_type = Option<ConnectorMetadata>)]
    pub connector_metadata: Option<serde_json::Value>, // This is Value because it is fetched from DB and before putting in DB the type is validated

    /// additional data that might be required by hyperswitch
    #[schema(value_type = Option<FeatureMetadata>)]
    pub feature_metadata: Option<serde_json::Value>, // This is Value because it is fetched from DB and before putting in DB the type is validated

    /// reference to the payment at connector side
    #[schema(value_type = Option<String>, example = "993672945374576J")]
    pub reference_id: Option<String>,

    pub payment_link: Option<PaymentLinkResponse>,
    /// The business profile that is associated with this payment
    pub profile_id: Option<String>,

    /// details of surcharge applied on this payment
    pub surcharge_details: Option<RequestSurchargeDetails>,

    /// total number of attempts associated with this payment
    pub attempt_count: i16,

    /// Denotes the action(approve or reject) taken by merchant in case of manual review. Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    pub merchant_decision: Option<String>,

    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    pub merchant_connector_id: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, ToSchema, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentListConstraints {
    /// The identifier for customer
    #[schema(example = "cus_meowuwunwiuwiwqw")]
    pub customer_id: Option<String>,

    /// A cursor for use in pagination, fetch the next list after some object
    #[schema(example = "pay_fafa124123")]
    pub starting_after: Option<String>,

    /// A cursor for use in pagination, fetch the previous list before some object
    #[schema(example = "pay_fafa124123")]
    pub ending_before: Option<String>,

    /// limit on the number of objects to return
    #[schema(default = 10, maximum = 100)]
    #[serde(default = "default_limit")]
    pub limit: u32,

    /// The time at which payment is created
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,

    /// Time less than the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lt"
    )]
    pub created_lt: Option<PrimitiveDateTime>,

    /// Time greater than the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.gt"
    )]
    pub created_gt: Option<PrimitiveDateTime>,

    /// Time less than or equals to the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(
        default,
        with = "common_utils::custom_serde::iso8601::option",
        rename = "created.lte"
    )]
    pub created_lte: Option<PrimitiveDateTime>,

    /// Time greater than or equals to the payment created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.gte")]
    pub created_gte: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct PaymentListResponse {
    /// The number of payments included in the list
    pub size: usize,
    // The list of payments response objects
    pub data: Vec<PaymentsResponse>,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentListResponseV2 {
    /// The number of payments included in the list for given constraints
    pub count: usize,
    /// The total number of available payments for given constraints
    pub total_count: i64,
    /// The list of payments response objects
    pub data: Vec<PaymentsResponse>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentListFilterConstraints {
    /// The identifier for payment
    pub payment_id: Option<String>,
    /// The identifier for business profile
    pub profile_id: Option<String>,
    /// The identifier for customer
    pub customer_id: Option<String>,
    /// The limit on the number of objects. The default limit is 10 and max limit is 20
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// The starting point within a list of objects
    pub offset: Option<u32>,
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
    /// The list of connectors to filter payments list
    pub connector: Option<Vec<api_enums::Connector>>,
    /// The list of currencies to filter payments list
    pub currency: Option<Vec<enums::Currency>>,
    /// The list of payment status to filter payments list
    pub status: Option<Vec<enums::IntentStatus>>,
    /// The list of payment methods to filter payments list
    pub payment_method: Option<Vec<enums::PaymentMethod>>,
    /// The list of payment method types to filter payments list
    pub payment_method_type: Option<Vec<enums::PaymentMethodType>>,
    /// The list of authentication types to filter payments list
    pub authentication_type: Option<Vec<enums::AuthenticationType>>,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentListFilters {
    /// The list of available connector filters
    pub connector: Vec<String>,
    /// The list of available currency filters
    pub currency: Vec<enums::Currency>,
    /// The list of available payment status filters
    pub status: Vec<enums::IntentStatus>,
    /// The list of available payment method filters
    pub payment_method: Vec<enums::PaymentMethod>,
    /// The list of available payment method types
    pub payment_method_type: Vec<enums::PaymentMethodType>,
    /// The list of available authentication types
    pub authentication_type: Vec<enums::AuthenticationType>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct TimeRange {
    /// The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub start_time: PrimitiveDateTime,
    /// The end time to filter payments list or to get list of filters. If not passed the default time is now
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_time: Option<PrimitiveDateTime>,
}

#[derive(Setter, Clone, Default, Debug, PartialEq, serde::Serialize)]
pub struct VerifyResponse {
    pub verify_id: Option<String>,
    pub merchant_id: Option<String>,
    // pub status: enums::VerifyStatus,
    pub client_secret: Option<Secret<String>>,
    pub customer_id: Option<String>,
    pub email: crypto::OptionalEncryptableEmail,
    pub name: crypto::OptionalEncryptableName,
    pub phone: crypto::OptionalEncryptablePhone,
    pub mandate_id: Option<String>,
    #[auth_based]
    pub payment_method: Option<api_enums::PaymentMethod>,
    #[auth_based]
    pub payment_method_data: Option<PaymentMethodDataResponse>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

fn default_limit() -> u32 {
    10
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentsRedirectionResponse {
    pub redirect_url: String,
}

pub struct MandateValidationFields {
    pub mandate_id: Option<String>,
    pub confirm: Option<bool>,
    pub customer_id: Option<String>,
    pub mandate_data: Option<MandateData>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub off_session: Option<bool>,
}

impl From<&PaymentsRequest> for MandateValidationFields {
    fn from(req: &PaymentsRequest) -> Self {
        Self {
            mandate_id: req.mandate_id.clone(),
            confirm: req.confirm,
            customer_id: req
                .customer
                .as_ref()
                .map(|customer_details| &customer_details.id)
                .or(req.customer_id.as_ref())
                .map(ToOwned::to_owned),
            mandate_data: req.mandate_data.clone(),
            setup_future_usage: req.setup_future_usage,
            off_session: req.off_session,
        }
    }
}

impl From<&VerifyRequest> for MandateValidationFields {
    fn from(req: &VerifyRequest) -> Self {
        Self {
            mandate_id: None,
            confirm: Some(true),
            customer_id: req.customer_id.clone(),
            mandate_data: req.mandate_data.clone(),
            off_session: req.off_session,
            setup_future_usage: req.setup_future_usage,
        }
    }
}

impl From<PaymentsSessionRequest> for PaymentsSessionResponse {
    fn from(item: PaymentsSessionRequest) -> Self {
        let client_secret: Secret<String, pii::ClientSecret> = Secret::new(item.client_secret);
        Self {
            session_token: vec![],
            payment_id: item.payment_id,
            client_secret,
        }
    }
}

impl From<PaymentsStartRequest> for PaymentsRequest {
    fn from(item: PaymentsStartRequest) -> Self {
        Self {
            payment_id: Some(PaymentIdType::PaymentIntentId(item.payment_id)),
            merchant_id: Some(item.merchant_id),
            ..Default::default()
        }
    }
}

impl From<AdditionalCardInfo> for CardResponse {
    fn from(card: AdditionalCardInfo) -> Self {
        Self {
            last4: card.last4,
            card_type: card.card_type,
            card_network: card.card_network,
            card_issuer: card.card_issuer,
            card_issuing_country: card.card_issuing_country,
            card_isin: card.card_isin,
            card_exp_month: card.card_exp_month,
            card_exp_year: card.card_exp_year,
            card_holder_name: card.card_holder_name,
        }
    }
}

impl From<AdditionalPaymentData> for PaymentMethodDataResponse {
    fn from(payment_method_data: AdditionalPaymentData) -> Self {
        match payment_method_data {
            AdditionalPaymentData::Card(card) => Self::Card(CardResponse::from(*card)),
            AdditionalPaymentData::PayLater {} => Self::PayLater,
            AdditionalPaymentData::Wallet {} => Self::Wallet,
            AdditionalPaymentData::BankRedirect { .. } => Self::BankRedirect,
            AdditionalPaymentData::Crypto {} => Self::Crypto,
            AdditionalPaymentData::BankDebit {} => Self::BankDebit,
            AdditionalPaymentData::MandatePayment {} => Self::MandatePayment,
            AdditionalPaymentData::Reward {} => Self::Reward,
            AdditionalPaymentData::Upi {} => Self::Upi,
            AdditionalPaymentData::BankTransfer {} => Self::BankTransfer,
            AdditionalPaymentData::Voucher {} => Self::Voucher,
            AdditionalPaymentData::GiftCard {} => Self::GiftCard,
            AdditionalPaymentData::CardRedirect {} => Self::CardRedirect,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PgRedirectResponse {
    pub payment_id: String,
    pub status: api_enums::IntentStatus,
    pub gateway_id: String,
    pub customer_id: Option<String>,
    pub amount: Option<i64>,
}

#[derive(Debug, serde::Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct RedirectionResponse {
    pub return_url: String,
    pub params: Vec<(String, String)>,
    pub return_url_with_query_params: String,
    pub http_method: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PaymentsResponseForm {
    pub transaction_id: String,
    // pub transaction_reference_id: String,
    pub merchant_id: String,
    pub order_id: String,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsRetrieveRequest {
    /// The type of ID (ex: payment intent id, payment attempt id or connector txn id)
    pub resource_id: PaymentIdType,
    /// The identifier for the Merchant Account.
    pub merchant_id: Option<String>,
    /// Decider to enable or disable the connector call for retrieve request
    pub force_sync: bool,
    /// The parameters passed to a retrieve request
    pub param: Option<String>,
    /// The name of the connector
    pub connector: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<String>,
    /// If enabled provides list of captures linked to latest attempt
    pub expand_captures: Option<bool>,
    /// If enabled provides list of attempts linked to payment intent
    pub expand_attempts: Option<bool>,
}

#[derive(Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    #[schema(max_length = 255, example = "shirt")]
    pub product_name: String,
    /// The quantity of the product to be purchased
    #[schema(example = 1)]
    pub quantity: u16,
    /// the amount per quantity of product
    pub amount: i64,
    /// The image URL of the product
    pub product_img_link: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct OrderDetails {
    /// Name of the product that is being purchased
    #[schema(max_length = 255, example = "shirt")]
    pub product_name: String,
    /// The quantity of the product to be purchased
    #[schema(example = 1)]
    pub quantity: u16,
    /// The image URL of the product
    pub product_img_link: Option<String>,
}

#[derive(Default, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct RedirectResponse {
    #[schema(value_type = Option<String>)]
    pub param: Option<Secret<String>>,
    #[schema(value_type = Option<Object>)]
    pub json_payload: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsSessionRequest {
    /// The identifier for the payment
    pub payment_id: String,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: String,
    /// The list of the supported wallets
    #[schema(value_type = Vec<PaymentMethodType>)]
    pub wallets: Vec<api_enums::PaymentMethodType>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = Option<MerchantConnectorDetailsWrap>)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayAllowedMethodsParameters {
    /// The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc)
    pub allowed_auth_methods: Vec<String>,
    /// The list of allowed card networks (ex: AMEX,JCB etc)
    pub allowed_card_networks: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayTokenParameters {
    /// The name of the connector
    pub gateway: String,
    /// The merchant ID registered in the connector associated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_merchant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stripe:version")]
    pub stripe_version: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "stripe:publishableKey"
    )]
    pub stripe_publishable_key: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayTokenizationSpecification {
    /// The token specification type(ex: PAYMENT_GATEWAY)
    #[serde(rename = "type")]
    pub token_specification_type: String,
    /// The parameters for the token specification Google Pay
    pub parameters: GpayTokenParameters,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayAllowedPaymentMethods {
    /// The type of payment method
    #[serde(rename = "type")]
    pub payment_method_type: String,
    /// The parameters Google Pay requires
    pub parameters: GpayAllowedMethodsParameters,
    /// The tokenization specification for Google Pay
    pub tokenization_specification: GpayTokenizationSpecification,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayTransactionInfo {
    /// The country code
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub country_code: api_enums::CountryAlpha2,
    /// The currency code
    #[schema(value_type = Currency, example = "USD")]
    pub currency_code: api_enums::Currency,
    /// The total price status (ex: 'FINAL')
    pub total_price_status: String,
    /// The total price
    pub total_price: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct GpayMerchantInfo {
    /// The merchant Identifier that needs to be passed while invoking Gpay SDK
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
    /// The name of the merchant that needs to be displayed on Gpay PopUp
    pub merchant_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayMetaData {
    pub merchant_info: GpayMerchantInfo,
    pub allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpaySessionTokenData {
    #[serde(rename = "google_pay")]
    pub data: GpayMetaData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepaySessionRequest {
    pub merchant_identifier: String,
    pub display_name: String,
    pub initiative: String,
    pub initiative_context: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConnectorMetadata {
    pub apple_pay: Option<ApplepayConnectorMetadataRequest>,
    pub airwallex: Option<AirwallexData>,
    pub noon: Option<NoonData>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AirwallexData {
    /// payload required by airwallex
    payload: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct NoonData {
    /// Information about the order category that merchant wants to specify at connector level. (e.g. In Noon Payments it can take values like "pay", "food", or any other custom string set by the merchant in Noon's Dashboard)
    pub order_category: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ApplepayConnectorMetadataRequest {
    pub session_token_data: Option<SessionTokenInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplepaySessionTokenData {
    pub apple_pay: ApplePayMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplepayCombinedSessionTokenData {
    pub apple_pay_combined: ApplePayCombinedMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplepaySessionTokenMetadata {
    ApplePayCombined(ApplePayCombinedMetadata),
    ApplePay(ApplePayMetadata),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplePayMetadata {
    pub payment_request_data: PaymentRequestMetadata,
    pub session_token_data: SessionTokenInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplePayCombinedMetadata {
    Simplified {
        payment_request_data: PaymentRequestMetadata,
        session_token_data: SessionTokenForSimplifiedApplePay,
    },
    Manual {
        payment_request_data: PaymentRequestMetadata,
        session_token_data: SessionTokenInfo,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentRequestMetadata {
    pub supported_networks: Vec<String>,
    pub merchant_capabilities: Vec<String>,
    pub label: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct SessionTokenInfo {
    pub certificate: String,
    pub certificate_keys: String,
    pub merchant_identifier: String,
    pub display_name: String,
    pub initiative: String,
    pub initiative_context: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct SessionTokenForSimplifiedApplePay {
    pub initiative_context: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(tag = "wallet_name")]
#[serde(rename_all = "snake_case")]
pub enum SessionToken {
    /// The session response structure for Google Pay
    GooglePay(Box<GpaySessionTokenResponse>),
    /// The session response structure for Klarna
    Klarna(Box<KlarnaSessionTokenResponse>),
    /// The session response structure for PayPal
    Paypal(Box<PaypalSessionTokenResponse>),
    /// The session response structure for Apple Pay
    ApplePay(Box<ApplepaySessionTokenResponse>),
    /// Whenever there is no session token response or an error in session response
    NoSessionTokenReceived,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(untagged)]
pub enum GpaySessionTokenResponse {
    /// Google pay response involving third party sdk
    ThirdPartyResponse(GooglePayThirdPartySdk),
    /// Google pay session response for non third party sdk
    GooglePaySession(GooglePaySessionResponse),
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct GooglePayThirdPartySdk {
    /// Identifier for the delayed session response
    pub delayed_session_token: bool,
    /// The name of the connector
    pub connector: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct GooglePaySessionResponse {
    /// The merchant info
    pub merchant_info: GpayMerchantInfo,
    /// List of the allowed payment meythods
    pub allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
    /// The transaction info Google Pay requires
    pub transaction_info: GpayTransactionInfo,
    /// Identifier for the delayed session response
    pub delayed_session_token: bool,
    /// The name of the connector
    pub connector: String,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
    /// Secrets for sdk display and payment
    pub secrets: Option<SecretInfoToInitiateSdk>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct KlarnaSessionTokenResponse {
    /// The session token for Klarna
    pub session_token: String,
    /// The identifier for the session
    pub session_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct PaypalSessionTokenResponse {
    /// The session token for PayPal
    pub session_token: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct ApplepaySessionTokenResponse {
    /// Session object for Apple Pay
    pub session_token_data: ApplePaySessionResponse,
    /// Payment request object for Apple Pay
    pub payment_request_data: Option<ApplePayPaymentRequest>,
    /// The session token is w.r.t this connector
    pub connector: String,
    /// Identifier for the delayed session response
    pub delayed_session_token: bool,
    /// The next action for the sdk (ex: calling confirm or sync call)
    pub sdk_next_action: SdkNextAction,
    /// The connector transaction id
    pub connector_reference_id: Option<String>,
    /// The public key id is to invoke third party sdk
    pub connector_sdk_public_key: Option<String>,
    /// The connector merchant id
    pub connector_merchant_id: Option<String>,
}

#[derive(Debug, Eq, PartialEq, serde::Serialize, Clone, ToSchema)]
pub struct SdkNextAction {
    /// The type of next action
    pub next_action: NextActionCall,
}

#[derive(Debug, Eq, PartialEq, serde::Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionCall {
    /// The next action call is confirm
    Confirm,
    /// The next action call is sync
    Sync,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
#[serde(untagged)]
pub enum ApplePaySessionResponse {
    ///  We get this session response, when third party sdk is involved
    ThirdPartySdk(ThirdPartySdkSessionResponse),
    ///  We get this session response, when there is no involvement of third party sdk
    /// This is the common response most of the times
    NoThirdPartySdk(NoThirdPartySdkSessionResponse),
    /// This is for the empty session response
    NoSessionResponse,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct NoThirdPartySdkSessionResponse {
    /// Timestamp at which session is requested
    pub epoch_timestamp: u64,
    /// Timestamp at which session expires
    pub expires_at: u64,
    /// The identifier for the merchant session
    pub merchant_session_identifier: String,
    /// Apple pay generated unique ID (UUID) value
    pub nonce: String,
    /// The identifier for the merchant
    pub merchant_identifier: String,
    /// The domain name of the merchant which is registered in Apple Pay
    pub domain_name: String,
    /// The name to be displayed on Apple Pay button
    pub display_name: String,
    /// A string which represents the properties of a payment
    pub signature: String,
    /// The identifier for the operational analytics
    pub operational_analytics_identifier: String,
    /// The number of retries to get the session response
    pub retries: u8,
    /// The identifier for the connector transaction
    pub psp_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema)]
pub struct ThirdPartySdkSessionResponse {
    pub secrets: SecretInfoToInitiateSdk,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct SecretInfoToInitiateSdk {
    // Authorization secrets used by client to initiate sdk
    #[schema(value_type = String)]
    pub display: Secret<String>,
    // Authorization secrets used by client for payment
    #[schema(value_type = String)]
    pub payment: Secret<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct ApplePayPaymentRequest {
    /// The code for country
    #[schema(value_type = CountryAlpha2, example = "US")]
    pub country_code: api_enums::CountryAlpha2,
    /// The code for currency
    #[schema(value_type = Currency, example = "USD")]
    pub currency_code: api_enums::Currency,
    /// Represents the total for the payment.
    pub total: AmountInfo,
    /// The list of merchant capabilities(ex: whether capable of 3ds or no-3ds)
    pub merchant_capabilities: Option<Vec<String>>,
    /// The list of supported networks
    pub supported_networks: Option<Vec<String>>,
    pub merchant_identifier: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, ToSchema, serde::Deserialize)]
pub struct AmountInfo {
    /// The label must be the name of the merchant.
    pub label: String,
    /// A value that indicates whether the line item(Ex: total, tax, discount, or grand total) is final or pending.
    #[serde(rename = "type")]
    pub total_type: Option<String>,
    /// The total amount for the payment
    pub amount: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayErrorResponse {
    pub status_code: String,
    pub status_message: String,
}

#[derive(Default, Debug, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsSessionResponse {
    /// The identifier for the payment
    pub payment_id: String,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    #[schema(value_type = String)]
    pub client_secret: Secret<String, pii::ClientSecret>,
    /// The list of session token object
    pub session_token: Vec<SessionToken>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentRetrieveBody {
    /// The identifier for the Merchant Account.
    pub merchant_id: Option<String>,
    /// Decider to enable or disable the connector call for retrieve request
    pub force_sync: Option<bool>,
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<String>,
    /// If enabled provides list of captures linked to latest attempt
    pub expand_captures: Option<bool>,
    /// If enabled provides list of attempts linked to payment intent
    pub expand_attempts: Option<bool>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentRetrieveBodyWithCredentials {
    /// The identifier for payment.
    pub payment_id: String,
    /// The identifier for the Merchant Account.
    pub merchant_id: Option<String>,
    /// Decider to enable or disable the connector call for retrieve request
    pub force_sync: Option<bool>,
    /// Merchant connector details used to make payments.
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsCancelRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: String,
    /// The reason for the payment cancel
    pub cancellation_reason: Option<String>,
    /// Merchant connector details used to make payments.
    #[schema(value_type = MerchantConnectorDetailsWrap)]
    pub merchant_connector_details: Option<admin::MerchantConnectorDetailsWrap>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsApproveRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: String,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, ToSchema)]
pub struct PaymentsRejectRequest {
    /// The identifier for the payment
    #[serde(skip)]
    pub payment_id: String,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, ToSchema, Clone)]
pub struct PaymentsStartRequest {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    pub payment_id: String,
    /// The identifier for the Merchant Account.
    pub merchant_id: String,
    /// The identifier for the payment transaction
    pub attempt_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    #[schema(value_type = Option<RedirectResponse>)]
    pub redirect_response: Option<RedirectResponse>,
}

///frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its None
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct FrmMessage {
    pub frm_name: String,
    pub frm_transaction_id: Option<String>,
    pub frm_transaction_type: Option<String>,
    pub frm_status: Option<String>,
    pub frm_score: Option<i32>,
    pub frm_reason: Option<serde_json::Value>,
    pub frm_error: Option<String>,
}

mod payment_id_type {
    use std::fmt;

    use serde::{
        de::{self, Visitor},
        Deserializer,
    };

    use super::PaymentIdType;

    struct PaymentIdVisitor;
    struct OptionalPaymentIdVisitor;

    impl<'de> Visitor<'de> for PaymentIdVisitor {
        type Value = PaymentIdType;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(PaymentIdType::PaymentIntentId(value.to_string()))
        }
    }

    impl<'de> Visitor<'de> for OptionalPaymentIdVisitor {
        type Value = Option<PaymentIdType>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("payment id")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(PaymentIdVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'a, D>(deserializer: D) -> Result<PaymentIdType, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(PaymentIdVisitor)
    }

    pub(crate) fn deserialize_option<'a, D>(
        deserializer: D,
    ) -> Result<Option<PaymentIdType>, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_option(OptionalPaymentIdVisitor)
    }
}

pub mod amount {
    use serde::de;

    use super::Amount;
    struct AmountVisitor;
    struct OptionalAmountVisitor;

    // This is defined to provide guarded deserialization of amount
    // which itself handles zero and non-zero values internally
    impl<'de> de::Visitor<'de> for AmountVisitor {
        type Value = Amount;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "amount as integer")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let v = i64::try_from(v).map_err(|_| {
                E::custom(format!(
                    "invalid value `{v}`, expected an integer between 0 and {}",
                    i64::MAX
                ))
            })?;
            self.visit_i64(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_negative() {
                return Err(E::custom(format!(
                    "invalid value `{v}`, expected a positive integer"
                )));
            }
            Ok(Amount::from(v))
        }
    }

    impl<'de> de::Visitor<'de> for OptionalAmountVisitor {
        type Value = Option<Amount>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "option of amount (as integer)")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_i64(AmountVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(AmountVisitor)
    }
    pub(crate) fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionalAmountVisitor)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_mandate_type() {
        let mandate_type = MandateType::default();
        assert_eq!(
            serde_json::to_string(&mandate_type).unwrap(),
            r#"{"multi_use":null}"#
        )
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkObject {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub link_expiry: Option<PrimitiveDateTime>,
    pub merchant_custom_domain_name: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, Clone, ToSchema, serde::Serialize)]
pub struct RetrievePaymentLinkRequest {
    pub client_secret: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, ToSchema)]
pub struct PaymentLinkResponse {
    pub link: String,
    pub payment_link_id: String,
}

#[derive(Clone, Debug, serde::Serialize, ToSchema)]
pub struct RetrievePaymentLinkResponse {
    pub payment_link_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub link_to_pay: String,
    pub amount: i64,
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<api_enums::Currency>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub link_expiry: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, serde::Deserialize, ToSchema, serde::Serialize)]
pub struct PaymentLinkInitiateRequest {
    pub merchant_id: String,
    pub payment_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentLinkDetails {
    pub amount: i64,
    pub currency: api_enums::Currency,
    pub pub_key: String,
    pub client_secret: String,
    pub payment_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub merchant_logo: String,
    pub return_url: String,
    pub merchant_name: crypto::OptionalEncryptableName,
    pub order_details: Vec<pii::SecretSerdeValue>,
    pub max_items_visible_after_collapse: i8,
}
