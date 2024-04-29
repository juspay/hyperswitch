use std::{collections::HashMap, ops::Deref};

use api_models::{self, enums as api_enums};
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode},
    pii::{self, Email},
    request::RequestContent,
};
use data_models::mandates::AcceptanceType;
use error_stack::ResultExt;
use masking::{ExposeInterface, ExposeOptionInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    collect_missing_value_keys,
    connector::utils::{
        self as connector_util, ApplePay, ApplePayDecrypt, BankRedirectBillingData,
        PaymentsPreProcessingData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{
        self, api, domain,
        storage::enums,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    unimplemented_payment_method,
    utils::OptionExt,
};

pub mod auth_headers {
    pub const STRIPE_API_VERSION: &str = "stripe-version";
    pub const STRIPE_VERSION: &str = "2022-11-15";
}

pub struct StripeAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for StripeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StripeCaptureMethod {
    Manual,
    #[default]
    Automatic,
}

impl From<Option<enums::CaptureMethod>> for StripeCaptureMethod {
    fn from(item: Option<enums::CaptureMethod>) -> Self {
        match item {
            Some(p) => match p {
                enums::CaptureMethod::ManualMultiple => Self::Manual,
                enums::CaptureMethod::Manual => Self::Manual,
                enums::CaptureMethod::Automatic => Self::Automatic,
                enums::CaptureMethod::Scheduled => Self::Manual,
            },
            None => Self::Automatic,
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth3ds {
    #[default]
    Automatic,
    Any,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(
    rename_all = "snake_case",
    tag = "mandate_data[customer_acceptance][type]"
)]
pub enum StripeMandateType {
    Online {
        #[serde(rename = "mandate_data[customer_acceptance][online][ip_address]")]
        ip_address: Secret<String, pii::IpAddress>,
        #[serde(rename = "mandate_data[customer_acceptance][online][user_agent]")]
        user_agent: String,
    },
    Offline,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeMandateRequest {
    #[serde(flatten)]
    mandate_type: StripeMandateType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpandableObjects {
    LatestCharge,
    Customer,
    LatestAttempt,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBrowserInformation {
    #[serde(rename = "payment_method_data[ip]")]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
    #[serde(rename = "payment_method_data[user_agent]")]
    pub user_agent: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PaymentIntentRequest {
    pub amount: i64, //amount in cents, hence passed as integer
    pub currency: String,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    #[serde(flatten)]
    pub meta_data: HashMap<String, String>,
    pub return_url: String,
    pub confirm: bool,
    pub payment_method: Option<String>,
    pub customer: Option<Secret<String>>,
    #[serde(flatten)]
    pub setup_mandate_details: Option<StripeMandateRequest>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub shipping: Option<StripeShippingAddress>,
    #[serde(flatten)]
    pub billing: StripeBillingAddress,
    #[serde(flatten)]
    pub payment_data: Option<StripePaymentMethodData>,
    pub capture_method: StripeCaptureMethod,
    #[serde(flatten)]
    pub payment_method_options: Option<StripePaymentMethodOptions>, // For mandate txns using network_txns_id, needs to be validated
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    #[serde(rename = "payment_method_types[0]")]
    pub payment_method_types: Option<StripePaymentMethodType>,
    #[serde(rename = "expand[0]")]
    pub expand: Option<ExpandableObjects>,
    #[serde(flatten)]
    pub browser_info: Option<StripeBrowserInformation>,
}

// Field rename is required only in case of serialization as it is passed in the request to the connector.
// Deserialization is happening only in case of webhooks, where fields name should be used as defined in the struct.
// Whenever adding new fields, Please ensure it doesn't break the webhook flow
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StripeMetadata {
    // merchant_reference_id
    #[serde(rename(serialize = "metadata[order_id]"))]
    pub order_id: Option<String>,
    // to check whether the order_id is refund_id or payment_id
    // before deployment, order id is set to payment_id in refunds but now it is set as refund_id
    // it is set as string instead of bool because stripe pass it as string even if we set it as bool
    #[serde(rename(serialize = "metadata[is_refund_id_as_reference]"))]
    pub is_refund_id_as_reference: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SetupIntentRequest {
    pub confirm: bool,
    pub usage: Option<enums::FutureUsage>,
    pub customer: Option<Secret<String>>,
    pub off_session: Option<bool>,
    pub return_url: Option<String>,
    #[serde(flatten)]
    pub payment_data: StripePaymentMethodData,
    pub payment_method_options: Option<StripePaymentMethodOptions>, // For mandate txns using network_txns_id, needs to be validated
    #[serde(flatten)]
    pub meta_data: Option<HashMap<String, String>>,
    #[serde(rename = "payment_method_types[0]")]
    pub payment_method_types: Option<StripePaymentMethodType>,
    #[serde(rename = "expand[0]")]
    pub expand: Option<ExpandableObjects>,
    #[serde(flatten)]
    pub browser_info: Option<StripeBrowserInformation>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeCardData {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[card][number]")]
    pub payment_method_data_card_number: cards::CardNumber,
    #[serde(rename = "payment_method_data[card][exp_month]")]
    pub payment_method_data_card_exp_month: Secret<String>,
    #[serde(rename = "payment_method_data[card][exp_year]")]
    pub payment_method_data_card_exp_year: Secret<String>,
    #[serde(rename = "payment_method_data[card][cvc]")]
    pub payment_method_data_card_cvc: Option<Secret<String>>,
    #[serde(rename = "payment_method_options[card][request_three_d_secure]")]
    pub payment_method_auth_type: Option<Auth3ds>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripePayLaterData {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TokenRequest {
    #[serde(flatten)]
    pub token_data: StripePaymentMethodData,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeTokenResponse {
    pub id: Secret<String>,
    pub object: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CustomerRequest {
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<Secret<String>>,
    pub source: Option<Secret<String>>,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeCustomerResponse {
    pub id: String,
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<Secret<String>>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ChargesRequest {
    pub amount: String,
    pub currency: String,
    pub customer: Secret<String>,
    pub source: Secret<String>,
    #[serde(flatten)]
    pub meta_data: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChargesResponse {
    pub id: String,
    pub amount: u64,
    pub amount_captured: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub source: StripeSourceResponse,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeBankName {
    Eps {
        #[serde(rename = "payment_method_data[eps][bank]")]
        bank_name: Option<StripeBankNames>,
    },
    Ideal {
        #[serde(rename = "payment_method_data[ideal][bank]")]
        ideal_bank_name: Option<StripeBankNames>,
    },
    Przelewy24 {
        #[serde(rename = "payment_method_data[p24][bank]")]
        bank_name: Option<StripeBankNames>,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BankSpecificData {
    Sofort {
        #[serde(rename = "payment_method_options[sofort][preferred_language]")]
        preferred_language: String,
        #[serde(rename = "payment_method_data[sofort][country]")]
        country: api_enums::CountryAlpha2,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeBankRedirectData {
    StripeGiropay(Box<StripeGiropay>),
    StripeIdeal(Box<StripeIdeal>),
    StripeSofort(Box<StripeSofort>),
    StripeBancontactCard(Box<StripeBancontactCard>),
    StripePrezelewy24(Box<StripePrezelewy24>),
    StripeEps(Box<StripeEps>),
    StripeBlik(Box<StripeBlik>),
    StripeOnlineBankingFpx(Box<StripeOnlineBankingFpx>),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeGiropay {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeIdeal {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[ideal][bank]")]
    ideal_bank_name: Option<StripeBankNames>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeSofort {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_options[sofort][preferred_language]")]
    preferred_language: Option<String>,
    #[serde(rename = "payment_method_data[sofort][country]")]
    country: api_enums::CountryAlpha2,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBancontactCard {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripePrezelewy24 {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[p24][bank]")]
    bank_name: Option<StripeBankNames>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeEps {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[eps][bank]")]
    bank_name: Option<StripeBankNames>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBlik {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_options[blik][code]")]
    pub code: Secret<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeOnlineBankingFpx {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AchTransferData {
    #[serde(rename = "owner[email]")]
    pub email: Email,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MultibancoTransferData {
    #[serde(rename = "owner[email]")]
    pub email: Email,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BacsBankTransferData {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_options[customer_balance][bank_transfer][type]")]
    pub bank_transfer_type: BankTransferType,
    #[serde(rename = "payment_method_options[customer_balance][funding_type]")]
    pub balance_funding_type: BankTransferType,
    #[serde(rename = "payment_method_types[0]")]
    pub payment_method_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SepaBankTransferData {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_options[customer_balance][bank_transfer][type]")]
    pub bank_transfer_type: BankTransferType,
    #[serde(rename = "payment_method_options[customer_balance][funding_type]")]
    pub balance_funding_type: BankTransferType,
    #[serde(rename = "payment_method_types[0]")]
    pub payment_method_type: StripePaymentMethodType,
    #[serde(
        rename = "payment_method_options[customer_balance][bank_transfer][eu_bank_transfer][country]"
    )]
    pub country: api_models::enums::CountryAlpha2,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeCreditTransferSourceRequest {
    AchBankTansfer(AchCreditTransferSourceRequest),
    MultibancoBankTansfer(MultibancoCreditTransferSourceRequest),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AchCreditTransferSourceRequest {
    #[serde(rename = "type")]
    pub transfer_type: StripeCreditTransferTypes,
    #[serde(flatten)]
    pub payment_method_data: AchTransferData,
    pub currency: enums::Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MultibancoCreditTransferSourceRequest {
    #[serde(rename = "type")]
    pub transfer_type: StripeCreditTransferTypes,
    #[serde(flatten)]
    pub payment_method_data: MultibancoTransferData,
    pub currency: enums::Currency,
    pub amount: Option<i64>,
    #[serde(rename = "redirect[return_url]")]
    pub return_url: Option<String>,
}

// Remove untagged when Deserialize is added
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripePaymentMethodData {
    Card(StripeCardData),
    PayLater(StripePayLaterData),
    Wallet(StripeWallet),
    BankRedirect(StripeBankRedirectData),
    BankDebit(StripeBankDebitData),
    BankTransfer(StripeBankTransferData),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "payment_method_data[type]")]
pub enum BankDebitData {
    #[serde(rename = "us_bank_account")]
    Ach {
        #[serde(rename = "payment_method_data[us_bank_account][account_holder_type]")]
        account_holder_type: String,
        #[serde(rename = "payment_method_data[us_bank_account][account_number]")]
        account_number: Secret<String>,
        #[serde(rename = "payment_method_data[us_bank_account][routing_number]")]
        routing_number: Secret<String>,
    },
    #[serde(rename = "sepa_debit")]
    Sepa {
        #[serde(rename = "payment_method_data[sepa_debit][iban]")]
        iban: Secret<String>,
    },
    #[serde(rename = "au_becs_debit")]
    Becs {
        #[serde(rename = "payment_method_data[au_becs_debit][account_number]")]
        account_number: Secret<String>,
        #[serde(rename = "payment_method_data[au_becs_debit][bsb_number]")]
        bsb_number: Secret<String>,
    },
    #[serde(rename = "bacs_debit")]
    Bacs {
        #[serde(rename = "payment_method_data[bacs_debit][account_number]")]
        account_number: Secret<String>,
        #[serde(rename = "payment_method_data[bacs_debit][sort_code]")]
        sort_code: Secret<String>,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeBankDebitData {
    #[serde(flatten)]
    pub bank_specific_data: BankDebitData,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BankTransferData {
    pub email: Email,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeBankTransferData {
    AchBankTransfer(Box<AchTransferData>),
    SepaBankTransfer(Box<SepaBankTransferData>),
    BacsBankTransfers(Box<BacsBankTransferData>),
    MultibancoBankTransfers(Box<MultibancoTransferData>),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripeWallet {
    ApplepayToken(StripeApplePay),
    GooglepayToken(GooglePayToken),
    ApplepayPayment(ApplepayPayment),
    WechatpayPayment(WechatpayPayment),
    AlipayPayment(AlipayPayment),
    Cashapp(CashappPayment),
    ApplePayPredecryptToken(Box<StripeApplePayPredecrypt>),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeApplePayPredecrypt {
    #[serde(rename = "card[number]")]
    number: Secret<String>,
    #[serde(rename = "card[exp_year]")]
    exp_year: Secret<String>,
    #[serde(rename = "card[exp_month]")]
    exp_month: Secret<String>,
    #[serde(rename = "card[cryptogram]")]
    cryptogram: Secret<String>,
    #[serde(rename = "card[eci]")]
    eci: Option<String>,
    #[serde(rename = "card[tokenization_method]")]
    tokenization_method: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeApplePay {
    pub pk_token: Secret<String>,
    pub pk_token_instrument_name: String,
    pub pk_token_payment_network: String,
    pub pk_token_transaction_id: Secret<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GooglePayToken {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[card][token]")]
    pub token: Secret<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ApplepayPayment {
    #[serde(rename = "payment_method_data[card][token]")]
    pub token: Secret<String>,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_types: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AlipayPayment {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CashappPayment {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct WechatpayPayment {
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_options[wechat_pay][client]")]
    pub client: WechatClient,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WechatClient {
    Web,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GooglepayPayment {
    #[serde(rename = "payment_method_data[card][token]")]
    pub token: Secret<String>,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_types: StripePaymentMethodType,
}

// All supported payment_method_types in stripe
// This enum goes in payment_method_types[] field in stripe request body
// https://stripe.com/docs/api/payment_intents/create#create_payment_intent-payment_method_types
#[derive(Eq, PartialEq, Serialize, Clone, Debug, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodType {
    Affirm,
    AfterpayClearpay,
    Alipay,
    #[serde(rename = "au_becs_debit")]
    Becs,
    #[serde(rename = "bacs_debit")]
    Bacs,
    Bancontact,
    Blik,
    Card,
    CustomerBalance,
    Eps,
    Giropay,
    Ideal,
    Klarna,
    #[serde(rename = "p24")]
    Przelewy24,
    #[serde(rename = "sepa_debit")]
    Sepa,
    Sofort,
    #[serde(rename = "us_bank_account")]
    Ach,
    #[serde(rename = "wechat_pay")]
    Wechatpay,
    #[serde(rename = "cashapp")]
    Cashapp,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum StripeCreditTransferTypes {
    AchCreditTransfer,
    Multibanco,
    Blik,
}

impl TryFrom<enums::PaymentMethodType> for StripePaymentMethodType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: enums::PaymentMethodType) -> Result<Self, Self::Error> {
        match value {
            enums::PaymentMethodType::Credit => Ok(Self::Card),
            enums::PaymentMethodType::Debit => Ok(Self::Card),
            enums::PaymentMethodType::Klarna => Ok(Self::Klarna),
            enums::PaymentMethodType::Affirm => Ok(Self::Affirm),
            enums::PaymentMethodType::AfterpayClearpay => Ok(Self::AfterpayClearpay),
            enums::PaymentMethodType::Eps => Ok(Self::Eps),
            enums::PaymentMethodType::Giropay => Ok(Self::Giropay),
            enums::PaymentMethodType::Ideal => Ok(Self::Ideal),
            enums::PaymentMethodType::Sofort => Ok(Self::Sofort),
            enums::PaymentMethodType::ApplePay => Ok(Self::Card),
            enums::PaymentMethodType::Ach => Ok(Self::Ach),
            enums::PaymentMethodType::Sepa => Ok(Self::Sepa),
            enums::PaymentMethodType::Becs => Ok(Self::Becs),
            enums::PaymentMethodType::Bacs => Ok(Self::Bacs),
            enums::PaymentMethodType::BancontactCard => Ok(Self::Bancontact),
            enums::PaymentMethodType::WeChatPay => Ok(Self::Wechatpay),
            enums::PaymentMethodType::Blik => Ok(Self::Blik),
            enums::PaymentMethodType::AliPay => Ok(Self::Alipay),
            enums::PaymentMethodType::Przelewy24 => Ok(Self::Przelewy24),
            // Stripe expects PMT as Card for Recurring Mandates Payments
            enums::PaymentMethodType::GooglePay => Ok(Self::Card),
            enums::PaymentMethodType::Boleto
            | enums::PaymentMethodType::CardRedirect
            | enums::PaymentMethodType::CryptoCurrency
            | enums::PaymentMethodType::Multibanco
            | enums::PaymentMethodType::OnlineBankingFpx
            | enums::PaymentMethodType::Paypal
            | enums::PaymentMethodType::Pix
            | enums::PaymentMethodType::UpiCollect
            | enums::PaymentMethodType::Cashapp
            | enums::PaymentMethodType::Oxxo => Err(errors::ConnectorError::NotImplemented(
                connector_util::get_unimplemented_payment_method_error_message("stripe"),
            )
            .into()),
            enums::PaymentMethodType::AliPayHk
            | enums::PaymentMethodType::Atome
            | enums::PaymentMethodType::Bizum
            | enums::PaymentMethodType::Alma
            | enums::PaymentMethodType::ClassicReward
            | enums::PaymentMethodType::Dana
            | enums::PaymentMethodType::Efecty
            | enums::PaymentMethodType::Evoucher
            | enums::PaymentMethodType::GoPay
            | enums::PaymentMethodType::Gcash
            | enums::PaymentMethodType::Interac
            | enums::PaymentMethodType::KakaoPay
            | enums::PaymentMethodType::MbWay
            | enums::PaymentMethodType::MobilePay
            | enums::PaymentMethodType::Momo
            | enums::PaymentMethodType::MomoAtm
            | enums::PaymentMethodType::OnlineBankingThailand
            | enums::PaymentMethodType::OnlineBankingCzechRepublic
            | enums::PaymentMethodType::OnlineBankingFinland
            | enums::PaymentMethodType::OnlineBankingPoland
            | enums::PaymentMethodType::OnlineBankingSlovakia
            | enums::PaymentMethodType::OpenBankingUk
            | enums::PaymentMethodType::PagoEfectivo
            | enums::PaymentMethodType::PayBright
            | enums::PaymentMethodType::Pse
            | enums::PaymentMethodType::RedCompra
            | enums::PaymentMethodType::RedPagos
            | enums::PaymentMethodType::SamsungPay
            | enums::PaymentMethodType::Swish
            | enums::PaymentMethodType::TouchNGo
            | enums::PaymentMethodType::Trustly
            | enums::PaymentMethodType::Twint
            | enums::PaymentMethodType::Vipps
            | enums::PaymentMethodType::Alfamart
            | enums::PaymentMethodType::BcaBankTransfer
            | enums::PaymentMethodType::BniVa
            | enums::PaymentMethodType::CimbVa
            | enums::PaymentMethodType::BriVa
            | enums::PaymentMethodType::DanamonVa
            | enums::PaymentMethodType::Indomaret
            | enums::PaymentMethodType::MandiriVa
            | enums::PaymentMethodType::PermataBankTransfer
            | enums::PaymentMethodType::PaySafeCard
            | enums::PaymentMethodType::Givex
            | enums::PaymentMethodType::Benefit
            | enums::PaymentMethodType::Knet
            | enums::PaymentMethodType::SevenEleven
            | enums::PaymentMethodType::Lawson
            | enums::PaymentMethodType::MiniStop
            | enums::PaymentMethodType::FamilyMart
            | enums::PaymentMethodType::Seicomart
            | enums::PaymentMethodType::PayEasy
            | enums::PaymentMethodType::LocalBankTransfer
            | enums::PaymentMethodType::Walley => Err(errors::ConnectorError::NotImplemented(
                connector_util::get_unimplemented_payment_method_error_message("stripe"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BankTransferType {
    GbBankTransfer,
    EuBankTransfer,
    #[serde(rename = "bank_transfer")]
    BankTransfers,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripeBankNames {
    AbnAmro,
    ArzteUndApothekerBank,
    AsnBank,
    AustrianAnadiBankAg,
    BankAustria,
    BankhausCarlSpangler,
    BankhausSchelhammerUndSchatteraAg,
    BawagPskAg,
    BksBankAg,
    BrullKallmusBankAg,
    BtvVierLanderBank,
    Bunq,
    CapitalBankGraweGruppeAg,
    CitiHandlowy,
    Dolomitenbank,
    EasybankAg,
    ErsteBankUndSparkassen,
    Handelsbanken,
    HypoAlpeadriabankInternationalAg,
    HypoNoeLbFurNiederosterreichUWien,
    HypoOberosterreichSalzburgSteiermark,
    HypoTirolBankAg,
    HypoVorarlbergBankAg,
    HypoBankBurgenlandAktiengesellschaft,
    Ing,
    Knab,
    MarchfelderBank,
    OberbankAg,
    RaiffeisenBankengruppeOsterreich,
    SchoellerbankAg,
    SpardaBankWien,
    VolksbankGruppe,
    VolkskreditbankAg,
    VrBankBraunau,
    Moneyou,
    Rabobank,
    Regiobank,
    Revolut,
    SnsBank,
    TriodosBank,
    VanLanschot,
    PlusBank,
    EtransferPocztowy24,
    BankiSpbdzielcze,
    BankNowyBfgSa,
    GetinBank,
    Blik,
    NoblePay,
    #[serde(rename = "ideabank")]
    IdeaBank,
    #[serde(rename = "envelobank")]
    EnveloBank,
    NestPrzelew,
    MbankMtransfer,
    Inteligo,
    PbacZIpko,
    BnpParibas,
    BankPekaoSa,
    VolkswagenBank,
    AliorBank,
    Boz,
}

// This is used only for Disputes
impl From<WebhookEventStatus> for api_models::webhooks::IncomingWebhookEvent {
    fn from(value: WebhookEventStatus) -> Self {
        match value {
            WebhookEventStatus::WarningNeedsResponse => Self::DisputeOpened,
            WebhookEventStatus::WarningClosed => Self::DisputeCancelled,
            WebhookEventStatus::WarningUnderReview => Self::DisputeChallenged,
            WebhookEventStatus::Won => Self::DisputeWon,
            WebhookEventStatus::Lost => Self::DisputeLost,
            WebhookEventStatus::NeedsResponse
            | WebhookEventStatus::UnderReview
            | WebhookEventStatus::ChargeRefunded
            | WebhookEventStatus::Succeeded
            | WebhookEventStatus::RequiresPaymentMethod
            | WebhookEventStatus::RequiresConfirmation
            | WebhookEventStatus::RequiresAction
            | WebhookEventStatus::Processing
            | WebhookEventStatus::RequiresCapture
            | WebhookEventStatus::Canceled
            | WebhookEventStatus::Chargeable
            | WebhookEventStatus::Failed
            | WebhookEventStatus::Unknown => Self::EventNotSupported,
        }
    }
}

impl TryFrom<&common_enums::enums::BankNames> for StripeBankNames {
    type Error = errors::ConnectorError;
    fn try_from(bank: &common_enums::enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            common_enums::enums::BankNames::AbnAmro => Self::AbnAmro,
            common_enums::enums::BankNames::ArzteUndApothekerBank => Self::ArzteUndApothekerBank,
            common_enums::enums::BankNames::AsnBank => Self::AsnBank,
            common_enums::enums::BankNames::AustrianAnadiBankAg => Self::AustrianAnadiBankAg,
            common_enums::enums::BankNames::BankAustria => Self::BankAustria,
            common_enums::enums::BankNames::BankhausCarlSpangler => Self::BankhausCarlSpangler,
            common_enums::enums::BankNames::BankhausSchelhammerUndSchatteraAg => {
                Self::BankhausSchelhammerUndSchatteraAg
            }
            common_enums::enums::BankNames::BawagPskAg => Self::BawagPskAg,
            common_enums::enums::BankNames::BksBankAg => Self::BksBankAg,
            common_enums::enums::BankNames::BrullKallmusBankAg => Self::BrullKallmusBankAg,
            common_enums::enums::BankNames::BtvVierLanderBank => Self::BtvVierLanderBank,
            common_enums::enums::BankNames::Bunq => Self::Bunq,
            common_enums::enums::BankNames::CapitalBankGraweGruppeAg => {
                Self::CapitalBankGraweGruppeAg
            }
            common_enums::enums::BankNames::Citi => Self::CitiHandlowy,
            common_enums::enums::BankNames::Dolomitenbank => Self::Dolomitenbank,
            common_enums::enums::BankNames::EasybankAg => Self::EasybankAg,
            common_enums::enums::BankNames::ErsteBankUndSparkassen => Self::ErsteBankUndSparkassen,
            common_enums::enums::BankNames::Handelsbanken => Self::Handelsbanken,
            common_enums::enums::BankNames::HypoAlpeadriabankInternationalAg => {
                Self::HypoAlpeadriabankInternationalAg
            }

            common_enums::enums::BankNames::HypoNoeLbFurNiederosterreichUWien => {
                Self::HypoNoeLbFurNiederosterreichUWien
            }
            common_enums::enums::BankNames::HypoOberosterreichSalzburgSteiermark => {
                Self::HypoOberosterreichSalzburgSteiermark
            }
            common_enums::enums::BankNames::HypoTirolBankAg => Self::HypoTirolBankAg,
            common_enums::enums::BankNames::HypoVorarlbergBankAg => Self::HypoVorarlbergBankAg,
            common_enums::enums::BankNames::HypoBankBurgenlandAktiengesellschaft => {
                Self::HypoBankBurgenlandAktiengesellschaft
            }
            common_enums::enums::BankNames::Ing => Self::Ing,
            common_enums::enums::BankNames::Knab => Self::Knab,
            common_enums::enums::BankNames::MarchfelderBank => Self::MarchfelderBank,
            common_enums::enums::BankNames::OberbankAg => Self::OberbankAg,
            common_enums::enums::BankNames::RaiffeisenBankengruppeOsterreich => {
                Self::RaiffeisenBankengruppeOsterreich
            }
            common_enums::enums::BankNames::Rabobank => Self::Rabobank,
            common_enums::enums::BankNames::Regiobank => Self::Regiobank,
            common_enums::enums::BankNames::Revolut => Self::Revolut,
            common_enums::enums::BankNames::SnsBank => Self::SnsBank,
            common_enums::enums::BankNames::TriodosBank => Self::TriodosBank,
            common_enums::enums::BankNames::VanLanschot => Self::VanLanschot,
            common_enums::enums::BankNames::Moneyou => Self::Moneyou,
            common_enums::enums::BankNames::SchoellerbankAg => Self::SchoellerbankAg,
            common_enums::enums::BankNames::SpardaBankWien => Self::SpardaBankWien,
            common_enums::enums::BankNames::VolksbankGruppe => Self::VolksbankGruppe,
            common_enums::enums::BankNames::VolkskreditbankAg => Self::VolkskreditbankAg,
            common_enums::enums::BankNames::VrBankBraunau => Self::VrBankBraunau,
            common_enums::enums::BankNames::PlusBank => Self::PlusBank,
            common_enums::enums::BankNames::EtransferPocztowy24 => Self::EtransferPocztowy24,
            common_enums::enums::BankNames::BankiSpbdzielcze => Self::BankiSpbdzielcze,
            common_enums::enums::BankNames::BankNowyBfgSa => Self::BankNowyBfgSa,
            common_enums::enums::BankNames::GetinBank => Self::GetinBank,
            common_enums::enums::BankNames::Blik => Self::Blik,
            common_enums::enums::BankNames::NoblePay => Self::NoblePay,
            common_enums::enums::BankNames::IdeaBank => Self::IdeaBank,
            common_enums::enums::BankNames::EnveloBank => Self::EnveloBank,
            common_enums::enums::BankNames::NestPrzelew => Self::NestPrzelew,
            common_enums::enums::BankNames::MbankMtransfer => Self::MbankMtransfer,
            common_enums::enums::BankNames::Inteligo => Self::Inteligo,
            common_enums::enums::BankNames::PbacZIpko => Self::PbacZIpko,
            common_enums::enums::BankNames::BnpParibas => Self::BnpParibas,
            common_enums::enums::BankNames::BankPekaoSa => Self::BankPekaoSa,
            common_enums::enums::BankNames::VolkswagenBank => Self::VolkswagenBank,
            common_enums::enums::BankNames::AliorBank => Self::AliorBank,
            common_enums::enums::BankNames::Boz => Self::Boz,

            _ => Err(errors::ConnectorError::NotImplemented(
                connector_util::get_unimplemented_payment_method_error_message("stripe"),
            ))?,
        })
    }
}

fn validate_shipping_address_against_payment_method(
    shipping_address: &Option<StripeShippingAddress>,
    payment_method: Option<&StripePaymentMethodType>,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    match payment_method {
        Some(StripePaymentMethodType::AfterpayClearpay) => match shipping_address {
            Some(address) => {
                let missing_fields = collect_missing_value_keys!(
                    ("shipping.address.line1", address.line1),
                    ("shipping.address.country", address.country),
                    ("shipping.address.zip", address.zip)
                );

                if !missing_fields.is_empty() {
                    return Err(errors::ConnectorError::MissingRequiredFields {
                        field_names: missing_fields,
                    }
                    .into());
                }
                Ok(())
            }
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "shipping.address",
            }
            .into()),
        },
        _ => Ok(()),
    }
}

impl TryFrom<&domain::payments::PayLaterData> for StripePaymentMethodType {
    type Error = errors::ConnectorError;
    fn try_from(pay_later_data: &domain::payments::PayLaterData) -> Result<Self, Self::Error> {
        match pay_later_data {
            domain::payments::PayLaterData::KlarnaRedirect { .. } => Ok(Self::Klarna),
            domain::payments::PayLaterData::AffirmRedirect {} => Ok(Self::Affirm),
            domain::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                Ok(Self::AfterpayClearpay)
            }

            domain::PayLaterData::KlarnaSdk { .. }
            | domain::PayLaterData::PayBrightRedirect {}
            | domain::PayLaterData::WalleyRedirect {}
            | domain::PayLaterData::AlmaRedirect {}
            | domain::PayLaterData::AtomeRedirect {} => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
        }
    }
}

impl TryFrom<&domain::BankRedirectData> for StripePaymentMethodType {
    type Error = errors::ConnectorError;
    fn try_from(bank_redirect_data: &domain::BankRedirectData) -> Result<Self, Self::Error> {
        match bank_redirect_data {
            domain::BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            domain::BankRedirectData::Ideal { .. } => Ok(Self::Ideal),
            domain::BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            domain::BankRedirectData::BancontactCard { .. } => Ok(Self::Bancontact),
            domain::BankRedirectData::Przelewy24 { .. } => Ok(Self::Przelewy24),
            domain::BankRedirectData::Eps { .. } => Ok(Self::Eps),
            domain::BankRedirectData::Blik { .. } => Ok(Self::Blik),
            domain::BankRedirectData::OnlineBankingFpx { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
            domain::BankRedirectData::Bizum {}
            | domain::BankRedirectData::Interac { .. }
            | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | domain::BankRedirectData::OnlineBankingFinland { .. }
            | domain::BankRedirectData::OnlineBankingPoland { .. }
            | domain::BankRedirectData::OnlineBankingSlovakia { .. }
            | domain::BankRedirectData::OnlineBankingThailand { .. }
            | domain::BankRedirectData::OpenBankingUk { .. }
            | domain::BankRedirectData::Trustly { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
        }
    }
}

impl ForeignTryFrom<&domain::WalletData> for Option<StripePaymentMethodType> {
    type Error = errors::ConnectorError;
    fn foreign_try_from(wallet_data: &domain::WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            domain::WalletData::AliPayRedirect(_) => Ok(Some(StripePaymentMethodType::Alipay)),
            domain::WalletData::ApplePay(_) => Ok(None),
            domain::WalletData::GooglePay(_) => Ok(Some(StripePaymentMethodType::Card)),
            domain::WalletData::WeChatPayQr(_) => Ok(Some(StripePaymentMethodType::Wechatpay)),
            domain::WalletData::CashappQr(_) => Ok(Some(StripePaymentMethodType::Cashapp)),
            domain::WalletData::MobilePayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
            domain::WalletData::PaypalRedirect(_)
            | domain::WalletData::AliPayQr(_)
            | domain::WalletData::AliPayHkRedirect(_)
            | domain::WalletData::MomoRedirect(_)
            | domain::WalletData::KakaoPayRedirect(_)
            | domain::WalletData::GoPayRedirect(_)
            | domain::WalletData::GcashRedirect(_)
            | domain::WalletData::ApplePayRedirect(_)
            | domain::WalletData::ApplePayThirdPartySdk(_)
            | domain::WalletData::DanaRedirect {}
            | domain::WalletData::GooglePayRedirect(_)
            | domain::WalletData::GooglePayThirdPartySdk(_)
            | domain::WalletData::MbWayRedirect(_)
            | domain::WalletData::PaypalSdk(_)
            | domain::WalletData::SamsungPay(_)
            | domain::WalletData::TwintRedirect {}
            | domain::WalletData::VippsRedirect {}
            | domain::WalletData::TouchNGoRedirect(_)
            | domain::WalletData::SwishQr(_)
            | domain::WalletData::WeChatPayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
        }
    }
}

impl From<&domain::BankDebitData> for StripePaymentMethodType {
    fn from(bank_debit_data: &domain::BankDebitData) -> Self {
        match bank_debit_data {
            domain::BankDebitData::AchBankDebit { .. } => Self::Ach,
            domain::BankDebitData::SepaBankDebit { .. } => Self::Sepa,
            domain::BankDebitData::BecsBankDebit { .. } => Self::Becs,
            domain::BankDebitData::BacsBankDebit { .. } => Self::Bacs,
        }
    }
}

impl TryFrom<(&domain::payments::PayLaterData, StripePaymentMethodType)> for StripeBillingAddress {
    type Error = errors::ConnectorError;

    fn try_from(
        (pay_later_data, pm_type): (&domain::payments::PayLaterData, StripePaymentMethodType),
    ) -> Result<Self, Self::Error> {
        match (pay_later_data, pm_type) {
            (
                domain::payments::PayLaterData::KlarnaRedirect {
                    billing_email,
                    billing_country,
                },
                StripePaymentMethodType::Klarna,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                country: Some(billing_country.to_owned()),
                ..Self::default()
            }),
            (
                domain::payments::PayLaterData::AffirmRedirect {},
                StripePaymentMethodType::Affirm,
            ) => Ok(Self::default()),
            (
                domain::payments::PayLaterData::AfterpayClearpayRedirect {
                    billing_email,
                    billing_name,
                },
                StripePaymentMethodType::AfterpayClearpay,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                name: Some(billing_name.to_owned()),
                ..Self::default()
            }),
            _ => Err(errors::ConnectorError::MismatchedPaymentData),
        }
    }
}

impl From<&domain::BankDebitBilling> for StripeBillingAddress {
    fn from(item: &domain::BankDebitBilling) -> Self {
        Self {
            email: Some(item.email.to_owned()),
            country: item
                .address
                .as_ref()
                .and_then(|address| address.country.to_owned()),
            name: Some(item.name.to_owned()),
            city: item
                .address
                .as_ref()
                .and_then(|address| address.city.to_owned()),
            address_line1: item
                .address
                .as_ref()
                .and_then(|address| address.line1.to_owned()),
            address_line2: item
                .address
                .as_ref()
                .and_then(|address| address.line2.to_owned()),
            zip_code: item
                .address
                .as_ref()
                .and_then(|address| address.zip.to_owned()),
            state: None,
            phone: None,
        }
    }
}

impl TryFrom<(&domain::BankRedirectData, Option<bool>)> for StripeBillingAddress {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (bank_redirection_data, is_customer_initiated_mandate_payment): (
            &domain::BankRedirectData,
            Option<bool>,
        ),
    ) -> Result<Self, Self::Error> {
        match bank_redirection_data {
            domain::BankRedirectData::Eps {
                billing_details, ..
            } => Ok({
                let billing_data = billing_details.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "billing_details",
                    },
                )?;
                Self {
                    name: Some(connector_util::BankRedirectBillingData::get_billing_name(
                        &billing_data,
                    )?),
                    ..Self::default()
                }
            }),
            domain::BankRedirectData::Giropay {
                billing_details, ..
            } => Ok(Self {
                name: Some(
                    billing_details
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "giropay.billing_details",
                        })?
                        .get_billing_name()?,
                ),
                ..Self::default()
            }),
            domain::BankRedirectData::Ideal {
                billing_details, ..
            } => Ok(get_stripe_sepa_dd_mandate_billing_details(
                billing_details,
                is_customer_initiated_mandate_payment,
            )?),
            domain::BankRedirectData::Przelewy24 {
                billing_details, ..
            } => Ok(Self {
                email: billing_details.email.clone(),
                ..Self::default()
            }),
            domain::BankRedirectData::BancontactCard {
                billing_details, ..
            } => {
                let billing_details = billing_details.as_ref().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "billing_details",
                    },
                )?;
                Ok(Self {
                    name: Some(
                        billing_details
                            .billing_name
                            .as_ref()
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "billing_details.billing_name",
                            })?
                            .to_owned(),
                    ),
                    email: Some(
                        billing_details
                            .email
                            .as_ref()
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "billing_details.email",
                            })?
                            .to_owned(),
                    ),
                    ..Self::default()
                })
            }
            domain::BankRedirectData::Sofort {
                billing_details, ..
            } => Ok(get_stripe_sepa_dd_mandate_billing_details(
                billing_details,
                is_customer_initiated_mandate_payment,
            )?),

            domain::BankRedirectData::Bizum {}
            | domain::BankRedirectData::Blik { .. }
            | domain::BankRedirectData::Interac { .. }
            | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | domain::BankRedirectData::OnlineBankingFinland { .. }
            | domain::BankRedirectData::OnlineBankingPoland { .. }
            | domain::BankRedirectData::OnlineBankingSlovakia { .. }
            | domain::BankRedirectData::Trustly { .. }
            | domain::BankRedirectData::OnlineBankingFpx { .. }
            | domain::BankRedirectData::OnlineBankingThailand { .. }
            | domain::BankRedirectData::OpenBankingUk { .. } => Ok(Self::default()),
        }
    }
}

fn get_bank_debit_data(
    bank_debit_data: &domain::BankDebitData,
) -> (StripePaymentMethodType, BankDebitData, StripeBillingAddress) {
    match bank_debit_data {
        domain::BankDebitData::AchBankDebit {
            billing_details,
            account_number,
            routing_number,
            ..
        } => {
            let ach_data = BankDebitData::Ach {
                account_holder_type: "individual".to_string(),
                account_number: account_number.to_owned(),
                routing_number: routing_number.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Ach, ach_data, billing_data)
        }
        domain::BankDebitData::SepaBankDebit {
            billing_details,
            iban,
            ..
        } => {
            let sepa_data = BankDebitData::Sepa {
                iban: iban.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Sepa, sepa_data, billing_data)
        }
        domain::BankDebitData::BecsBankDebit {
            billing_details,
            account_number,
            bsb_number,
            ..
        } => {
            let becs_data = BankDebitData::Becs {
                account_number: account_number.to_owned(),
                bsb_number: bsb_number.to_owned(),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Becs, becs_data, billing_data)
        }
        domain::BankDebitData::BacsBankDebit {
            billing_details,
            account_number,
            sort_code,
            ..
        } => {
            let bacs_data = BankDebitData::Bacs {
                account_number: account_number.to_owned(),
                sort_code: Secret::new(sort_code.clone().expose().replace('-', "")),
            };

            let billing_data = StripeBillingAddress::from(billing_details);
            (StripePaymentMethodType::Bacs, bacs_data, billing_data)
        }
    }
}

fn create_stripe_payment_method(
    payment_method_data: &domain::PaymentMethodData,
    auth_type: enums::AuthenticationType,
    payment_method_token: Option<types::PaymentMethodToken>,
    is_customer_initiated_mandate_payment: Option<bool>,
    billing_address: StripeBillingAddress,
) -> Result<
    (
        StripePaymentMethodData,
        Option<StripePaymentMethodType>,
        StripeBillingAddress,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match payment_method_data {
        domain::PaymentMethodData::Card(card_details) => {
            let payment_method_auth_type = match auth_type {
                enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
            };
            Ok((
                StripePaymentMethodData::try_from((card_details, payment_method_auth_type))?,
                Some(StripePaymentMethodType::Card),
                billing_address,
            ))
        }
        domain::PaymentMethodData::PayLater(pay_later_data) => {
            let stripe_pm_type = StripePaymentMethodType::try_from(pay_later_data)?;
            let billing_address = StripeBillingAddress::try_from((pay_later_data, stripe_pm_type))?;
            Ok((
                StripePaymentMethodData::PayLater(StripePayLaterData {
                    payment_method_data_type: stripe_pm_type,
                }),
                Some(stripe_pm_type),
                billing_address,
            ))
        }
        domain::PaymentMethodData::BankRedirect(bank_redirect_data) => {
            let billing_address = StripeBillingAddress::try_from((
                bank_redirect_data,
                is_customer_initiated_mandate_payment,
            ))?;
            let pm_type = StripePaymentMethodType::try_from(bank_redirect_data)?;
            let bank_redirect_data = StripePaymentMethodData::try_from(bank_redirect_data)?;

            Ok((bank_redirect_data, Some(pm_type), billing_address))
        }
        domain::PaymentMethodData::Wallet(wallet_data) => {
            let pm_type = ForeignTryFrom::foreign_try_from(wallet_data)?;
            let wallet_specific_data =
                StripePaymentMethodData::try_from((wallet_data, payment_method_token))?;
            Ok((
                wallet_specific_data,
                pm_type,
                StripeBillingAddress::default(),
            ))
        }
        domain::PaymentMethodData::BankDebit(bank_debit_data) => {
            let (pm_type, bank_debit_data, billing_address) = get_bank_debit_data(bank_debit_data);

            let pm_data = StripePaymentMethodData::BankDebit(StripeBankDebitData {
                bank_specific_data: bank_debit_data,
            });

            Ok((pm_data, Some(pm_type), billing_address))
        }
        domain::PaymentMethodData::BankTransfer(bank_transfer_data) => {
            match bank_transfer_data.deref() {
                domain::BankTransferData::AchBankTransfer { billing_details } => Ok((
                    StripePaymentMethodData::BankTransfer(StripeBankTransferData::AchBankTransfer(
                        Box::new(AchTransferData {
                            email: billing_details.email.to_owned(),
                        }),
                    )),
                    None,
                    StripeBillingAddress::default(),
                )),
                domain::BankTransferData::MultibancoBankTransfer { billing_details } => Ok((
                    StripePaymentMethodData::BankTransfer(
                        StripeBankTransferData::MultibancoBankTransfers(Box::new(
                            MultibancoTransferData {
                                email: billing_details.email.to_owned(),
                            },
                        )),
                    ),
                    None,
                    StripeBillingAddress::default(),
                )),
                domain::BankTransferData::SepaBankTransfer {
                    billing_details,
                    country,
                } => {
                    let billing_details = StripeBillingAddress {
                        email: Some(billing_details.email.clone()),
                        name: Some(billing_details.name.clone()),
                        ..Default::default()
                    };
                    Ok((
                        StripePaymentMethodData::BankTransfer(
                            StripeBankTransferData::SepaBankTransfer(Box::new(
                                SepaBankTransferData {
                                    payment_method_data_type:
                                        StripePaymentMethodType::CustomerBalance,
                                    bank_transfer_type: BankTransferType::EuBankTransfer,
                                    balance_funding_type: BankTransferType::BankTransfers,
                                    payment_method_type: StripePaymentMethodType::CustomerBalance,
                                    country: country.to_owned(),
                                },
                            )),
                        ),
                        Some(StripePaymentMethodType::CustomerBalance),
                        billing_details,
                    ))
                }
                domain::BankTransferData::BacsBankTransfer { billing_details } => {
                    let billing_details = StripeBillingAddress {
                        email: Some(billing_details.email.clone()),
                        name: Some(billing_details.name.clone()),
                        ..Default::default()
                    };
                    Ok((
                        StripePaymentMethodData::BankTransfer(
                            StripeBankTransferData::BacsBankTransfers(Box::new(
                                BacsBankTransferData {
                                    payment_method_data_type:
                                        StripePaymentMethodType::CustomerBalance,
                                    bank_transfer_type: BankTransferType::GbBankTransfer,
                                    balance_funding_type: BankTransferType::BankTransfers,
                                    payment_method_type: StripePaymentMethodType::CustomerBalance,
                                },
                            )),
                        ),
                        Some(StripePaymentMethodType::CustomerBalance),
                        billing_details,
                    ))
                }
                domain::BankTransferData::Pix {} => Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into()),
                domain::BankTransferData::Pse {}
                | domain::BankTransferData::LocalBankTransfer { .. }
                | domain::BankTransferData::PermataBankTransfer { .. }
                | domain::BankTransferData::BcaBankTransfer { .. }
                | domain::BankTransferData::BniVaBankTransfer { .. }
                | domain::BankTransferData::BriVaBankTransfer { .. }
                | domain::BankTransferData::CimbVaBankTransfer { .. }
                | domain::BankTransferData::DanamonVaBankTransfer { .. }
                | domain::BankTransferData::MandiriVaBankTransfer { .. } => {
                    Err(errors::ConnectorError::NotImplemented(
                        connector_util::get_unimplemented_payment_method_error_message("stripe"),
                    )
                    .into())
                }
            }
        }
        domain::PaymentMethodData::Crypto(_) => Err(errors::ConnectorError::NotImplemented(
            connector_util::get_unimplemented_payment_method_error_message("stripe"),
        )
        .into()),

        domain::PaymentMethodData::GiftCard(giftcard_data) => match giftcard_data.deref() {
            domain::GiftCardData::Givex(_) | domain::GiftCardData::PaySafeCard {} => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
        },
        domain::PaymentMethodData::CardRedirect(cardredirect_data) => match cardredirect_data {
            domain::CardRedirectData::Knet {}
            | domain::CardRedirectData::Benefit {}
            | domain::CardRedirectData::MomoAtm {}
            | domain::CardRedirectData::CardRedirect {} => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
        },
        domain::PaymentMethodData::Reward => Err(errors::ConnectorError::NotImplemented(
            connector_util::get_unimplemented_payment_method_error_message("stripe"),
        )
        .into()),

        domain::PaymentMethodData::Voucher(voucher_data) => match voucher_data {
            domain::VoucherData::Boleto(_) | domain::VoucherData::Oxxo => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
            domain::VoucherData::Alfamart(_)
            | domain::VoucherData::Efecty
            | domain::VoucherData::PagoEfectivo
            | domain::VoucherData::RedCompra
            | domain::VoucherData::RedPagos
            | domain::VoucherData::Indomaret(_)
            | domain::VoucherData::SevenEleven(_)
            | domain::VoucherData::Lawson(_)
            | domain::VoucherData::MiniStop(_)
            | domain::VoucherData::FamilyMart(_)
            | domain::VoucherData::Seicomart(_)
            | domain::VoucherData::PayEasy(_) => Err(errors::ConnectorError::NotImplemented(
                connector_util::get_unimplemented_payment_method_error_message("stripe"),
            )
            .into()),
        },

        domain::PaymentMethodData::Upi(_)
        | domain::PaymentMethodData::MandatePayment
        | domain::PaymentMethodData::CardToken(_) => Err(errors::ConnectorError::NotImplemented(
            connector_util::get_unimplemented_payment_method_error_message("stripe"),
        )
        .into()),
    }
}

impl TryFrom<(&domain::Card, Auth3ds)> for StripePaymentMethodData {
    type Error = errors::ConnectorError;
    fn try_from(
        (card, payment_method_auth_type): (&domain::Card, Auth3ds),
    ) -> Result<Self, Self::Error> {
        Ok(Self::Card(StripeCardData {
            payment_method_data_type: StripePaymentMethodType::Card,
            payment_method_data_card_number: card.card_number.clone(),
            payment_method_data_card_exp_month: card.card_exp_month.clone(),
            payment_method_data_card_exp_year: card.card_exp_year.clone(),
            payment_method_data_card_cvc: Some(card.card_cvc.clone()),
            payment_method_auth_type: Some(payment_method_auth_type),
        }))
    }
}

impl TryFrom<(&domain::WalletData, Option<types::PaymentMethodToken>)> for StripePaymentMethodData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (wallet_data, payment_method_token): (
            &domain::WalletData,
            Option<types::PaymentMethodToken>,
        ),
    ) -> Result<Self, Self::Error> {
        match wallet_data {
            domain::WalletData::ApplePay(applepay_data) => {
                let mut apple_pay_decrypt_data =
                    if let Some(types::PaymentMethodToken::ApplePayDecrypt(decrypt_data)) =
                        payment_method_token
                    {
                        let expiry_year_4_digit = decrypt_data.get_four_digit_expiry_year()?;
                        let exp_month = decrypt_data.get_expiry_month()?;

                        Some(Self::Wallet(StripeWallet::ApplePayPredecryptToken(
                            Box::new(StripeApplePayPredecrypt {
                                number: decrypt_data.clone().application_primary_account_number,
                                exp_year: expiry_year_4_digit,
                                exp_month,
                                eci: decrypt_data.payment_data.eci_indicator,
                                cryptogram: decrypt_data.payment_data.online_payment_cryptogram,
                                tokenization_method: "apple_pay".to_string(),
                            }),
                        )))
                    } else {
                        None
                    };

                if apple_pay_decrypt_data.is_none() {
                    apple_pay_decrypt_data =
                        Some(Self::Wallet(StripeWallet::ApplepayToken(StripeApplePay {
                            pk_token: applepay_data.get_applepay_decoded_payment_data()?,
                            pk_token_instrument_name: applepay_data
                                .payment_method
                                .pm_type
                                .to_owned(),
                            pk_token_payment_network: applepay_data
                                .payment_method
                                .network
                                .to_owned(),
                            pk_token_transaction_id: Secret::new(
                                applepay_data.transaction_identifier.to_owned(),
                            ),
                        })));
                };
                let pmd = apple_pay_decrypt_data
                    .ok_or(errors::ConnectorError::MissingApplePayTokenData)?;
                Ok(pmd)
            }
            domain::WalletData::WeChatPayQr(_) => Ok(Self::Wallet(StripeWallet::WechatpayPayment(
                WechatpayPayment {
                    client: WechatClient::Web,
                    payment_method_data_type: StripePaymentMethodType::Wechatpay,
                },
            ))),
            domain::WalletData::AliPayRedirect(_) => {
                Ok(Self::Wallet(StripeWallet::AlipayPayment(AlipayPayment {
                    payment_method_data_type: StripePaymentMethodType::Alipay,
                })))
            }
            domain::WalletData::CashappQr(_) => {
                Ok(Self::Wallet(StripeWallet::Cashapp(CashappPayment {
                    payment_method_data_type: StripePaymentMethodType::Cashapp,
                })))
            }
            domain::WalletData::GooglePay(gpay_data) => Ok(Self::try_from(gpay_data)?),
            domain::WalletData::PaypalRedirect(_) | domain::WalletData::MobilePayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
            domain::WalletData::AliPayQr(_)
            | domain::WalletData::AliPayHkRedirect(_)
            | domain::WalletData::MomoRedirect(_)
            | domain::WalletData::KakaoPayRedirect(_)
            | domain::WalletData::GoPayRedirect(_)
            | domain::WalletData::GcashRedirect(_)
            | domain::WalletData::ApplePayRedirect(_)
            | domain::WalletData::ApplePayThirdPartySdk(_)
            | domain::WalletData::DanaRedirect {}
            | domain::WalletData::GooglePayRedirect(_)
            | domain::WalletData::GooglePayThirdPartySdk(_)
            | domain::WalletData::MbWayRedirect(_)
            | domain::WalletData::PaypalSdk(_)
            | domain::WalletData::SamsungPay(_)
            | domain::WalletData::TwintRedirect {}
            | domain::WalletData::VippsRedirect {}
            | domain::WalletData::TouchNGoRedirect(_)
            | domain::WalletData::SwishQr(_)
            | domain::WalletData::WeChatPayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::BankRedirectData> for StripePaymentMethodData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank_redirect_data: &domain::BankRedirectData) -> Result<Self, Self::Error> {
        let payment_method_data_type = StripePaymentMethodType::try_from(bank_redirect_data)?;
        match bank_redirect_data {
            domain::BankRedirectData::BancontactCard { .. } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeBancontactCard(Box::new(StripeBancontactCard {
                    payment_method_data_type,
                })),
            )),
            domain::BankRedirectData::Blik { blik_code } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeBlik(Box::new(StripeBlik {
                    payment_method_data_type,
                    code: Secret::new(blik_code.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "blik_code",
                        },
                    )?),
                })),
            )),
            domain::BankRedirectData::Eps { bank_name, .. } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeEps(Box::new(StripeEps {
                    payment_method_data_type,
                    bank_name: bank_name
                        .map(|bank_name| StripeBankNames::try_from(&bank_name))
                        .transpose()?,
                })),
            )),
            domain::BankRedirectData::Giropay { .. } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeGiropay(Box::new(StripeGiropay {
                    payment_method_data_type,
                })),
            )),
            domain::BankRedirectData::Ideal { bank_name, .. } => {
                let bank_name = bank_name
                    .map(|bank_name| StripeBankNames::try_from(&bank_name))
                    .transpose()?;
                Ok(Self::BankRedirect(StripeBankRedirectData::StripeIdeal(
                    Box::new(StripeIdeal {
                        payment_method_data_type,
                        ideal_bank_name: bank_name,
                    }),
                )))
            }
            domain::BankRedirectData::Przelewy24 { bank_name, .. } => {
                let bank_name = bank_name
                    .map(|bank_name| StripeBankNames::try_from(&bank_name))
                    .transpose()?;
                Ok(Self::BankRedirect(
                    StripeBankRedirectData::StripePrezelewy24(Box::new(StripePrezelewy24 {
                        payment_method_data_type,
                        bank_name,
                    })),
                ))
            }
            domain::BankRedirectData::Sofort {
                country,
                preferred_language,
                ..
            } => Ok(Self::BankRedirect(StripeBankRedirectData::StripeSofort(
                Box::new(StripeSofort {
                    payment_method_data_type,
                    country: country.ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "sofort.country",
                    })?,
                    preferred_language: preferred_language.clone(),
                }),
            ))),
            domain::BankRedirectData::OnlineBankingFpx { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
            domain::BankRedirectData::Bizum {}
            | domain::BankRedirectData::Interac { .. }
            | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | domain::BankRedirectData::OnlineBankingFinland { .. }
            | domain::BankRedirectData::OnlineBankingPoland { .. }
            | domain::BankRedirectData::OnlineBankingSlovakia { .. }
            | domain::BankRedirectData::OnlineBankingThailand { .. }
            | domain::BankRedirectData::OpenBankingUk { .. }
            | domain::BankRedirectData::Trustly { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::GooglePayWalletData> for StripePaymentMethodData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(gpay_data: &domain::GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self::Wallet(StripeWallet::GooglepayToken(GooglePayToken {
            token: Secret::new(
                gpay_data
                    .tokenization_data
                    .token
                    .as_bytes()
                    .parse_struct::<StripeGpayToken>("StripeGpayToken")
                    .change_context(errors::ConnectorError::InvalidWalletToken {
                        wallet_name: "Google Pay".to_string(),
                    })?
                    .id,
            ),
            payment_type: StripePaymentMethodType::Card,
        })))
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let order_id = item.connector_request_reference_id.clone();

        let shipping_address = match item.get_optional_shipping() {
            Some(shipping_details) => {
                let shipping_address = shipping_details.address.as_ref();
                Some(StripeShippingAddress {
                    city: shipping_address.and_then(|a| a.city.clone()),
                    country: shipping_address.and_then(|a| a.country),
                    line1: shipping_address.and_then(|a| a.line1.clone()),
                    line2: shipping_address.and_then(|a| a.line2.clone()),
                    zip: shipping_address.and_then(|a| a.zip.clone()),
                    state: shipping_address.and_then(|a| a.state.clone()),
                    name: shipping_address
                        .and_then(|a| {
                            a.first_name.as_ref().map(|first_name| {
                                format!(
                                    "{} {}",
                                    first_name.clone().expose(),
                                    a.last_name.clone().expose_option().unwrap_or_default()
                                )
                                .into()
                            })
                        })
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "shipping_address.first_name",
                        })?,
                    phone: shipping_details.phone.as_ref().map(|p| {
                        format!(
                            "{}{}",
                            p.country_code.clone().unwrap_or_default(),
                            p.number.clone().expose_option().unwrap_or_default()
                        )
                        .into()
                    }),
                })
            }
            None => None,
        };

        let billing_address = match item.get_optional_billing() {
            Some(billing_details) => {
                let billing_address = billing_details.address.as_ref();
                StripeBillingAddress {
                    city: billing_address.and_then(|a| a.city.clone()),
                    country: billing_address.and_then(|a| a.country),
                    address_line1: billing_address.and_then(|a| a.line1.clone()),
                    address_line2: billing_address.and_then(|a| a.line2.clone()),
                    zip_code: billing_address.and_then(|a| a.zip.clone()),
                    state: billing_address.and_then(|a| a.state.clone()),
                    name: billing_address.and_then(|a| {
                        a.first_name.as_ref().map(|first_name| {
                            format!(
                                "{} {}",
                                first_name.clone().expose(),
                                a.last_name.clone().expose_option().unwrap_or_default()
                            )
                            .into()
                        })
                    }),
                    email: billing_details.email.clone(),
                    phone: billing_details.phone.as_ref().map(|p| {
                        format!(
                            "{}{}",
                            p.country_code.clone().unwrap_or_default(),
                            p.number.clone().expose_option().unwrap_or_default()
                        )
                        .into()
                    }),
                }
            }
            None => StripeBillingAddress::default(),
        };
        let mut payment_method_options = None;

        let (mut payment_data, payment_method, billing_address, payment_method_types) = {
            match item
                .request
                .mandate_id
                .clone()
                .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
            {
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                    connector_mandate_ids,
                )) => (
                    None,
                    connector_mandate_ids.connector_mandate_id,
                    StripeBillingAddress::default(),
                    get_payment_method_type_for_saved_payment_method_payment(item)?,
                ),
                Some(api_models::payments::MandateReferenceId::NetworkMandateId(
                    network_transaction_id,
                )) => {
                    payment_method_options = Some(StripePaymentMethodOptions::Card {
                        mandate_options: None,
                        network_transaction_id: None,
                        mit_exemption: Some(MitExemption {
                            network_transaction_id: Secret::new(network_transaction_id),
                        }),
                    });

                    let payment_data = match item.request.payment_method_data {
                        domain::payments::PaymentMethodData::Card(ref card) => {
                            StripePaymentMethodData::Card(StripeCardData {
                                payment_method_data_type: StripePaymentMethodType::Card,
                                payment_method_data_card_number: card.card_number.clone(),
                                payment_method_data_card_exp_month: card.card_exp_month.clone(),
                                payment_method_data_card_exp_year: card.card_exp_year.clone(),
                                payment_method_data_card_cvc: None,
                                payment_method_auth_type: None,
                            })
                        }
                        domain::payments::PaymentMethodData::CardRedirect(_)
                        | domain::payments::PaymentMethodData::Wallet(_)
                        | domain::payments::PaymentMethodData::PayLater(_)
                        | domain::payments::PaymentMethodData::BankRedirect(_)
                        | domain::payments::PaymentMethodData::BankDebit(_)
                        | domain::payments::PaymentMethodData::BankTransfer(_)
                        | domain::payments::PaymentMethodData::Crypto(_)
                        | domain::payments::PaymentMethodData::MandatePayment
                        | domain::payments::PaymentMethodData::Reward
                        | domain::payments::PaymentMethodData::Upi(_)
                        | domain::payments::PaymentMethodData::Voucher(_)
                        | domain::payments::PaymentMethodData::GiftCard(_)
                        | domain::payments::PaymentMethodData::CardToken(_) => {
                            Err(errors::ConnectorError::NotSupported {
                                message: "Network tokenization for payment method".to_string(),
                                connector: "Stripe",
                            })?
                        }
                    };

                    (
                        Some(payment_data),
                        None,
                        StripeBillingAddress::default(),
                        None,
                    )
                }
                _ => {
                    let (payment_method_data, payment_method_type, billing_address) =
                        create_stripe_payment_method(
                            &item.request.payment_method_data,
                            item.auth_type,
                            item.payment_method_token.clone(),
                            Some(connector_util::PaymentsAuthorizeRequestData::is_customer_initiated_mandate_payment(
                                &item.request,
                            )),
                            billing_address
                        )?;

                    validate_shipping_address_against_payment_method(
                        &shipping_address,
                        payment_method_type.as_ref(),
                    )?;

                    (
                        Some(payment_method_data),
                        None,
                        billing_address,
                        payment_method_type,
                    )
                }
            }
        };

        payment_data = match item.request.payment_method_data {
            domain::PaymentMethodData::Wallet(domain::WalletData::ApplePay(_)) => {
                let payment_method_token = item
                    .payment_method_token
                    .to_owned()
                    .get_required_value("payment_token")
                    .change_context(errors::ConnectorError::InvalidWalletToken {
                        wallet_name: "Apple Pay".to_string(),
                    })?;

                let payment_method_token = match payment_method_token {
                    types::PaymentMethodToken::Token(payment_method_token) => payment_method_token,
                    types::PaymentMethodToken::ApplePayDecrypt(_) => Err(
                        unimplemented_payment_method!("Apple Pay", "Simplified", "Stripe"),
                    )?,
                };
                Some(StripePaymentMethodData::Wallet(
                    StripeWallet::ApplepayPayment(ApplepayPayment {
                        token: Secret::new(payment_method_token),
                        payment_method_types: StripePaymentMethodType::Card,
                    }),
                ))
            }
            _ => payment_data,
        };

        let setup_mandate_details = item
            .request
            .setup_mandate_details
            .as_ref()
            .and_then(|mandate_details| {
                mandate_details
                    .customer_acceptance
                    .as_ref()
                    .map(|customer_acceptance| {
                        Ok::<_, error_stack::Report<errors::ConnectorError>>(
                            match customer_acceptance.acceptance_type {
                                AcceptanceType::Online => {
                                    let online_mandate = customer_acceptance
                                        .online
                                        .clone()
                                        .get_required_value("online")
                                        .change_context(
                                            errors::ConnectorError::MissingRequiredField {
                                                field_name: "online",
                                            },
                                        )?;
                                    StripeMandateRequest {
                                        mandate_type: StripeMandateType::Online {
                                            ip_address: online_mandate
                                                .ip_address
                                                .get_required_value("ip_address")
                                                .change_context(
                                                    errors::ConnectorError::MissingRequiredField {
                                                        field_name: "ip_address",
                                                    },
                                                )?,
                                            user_agent: online_mandate.user_agent,
                                        },
                                    }
                                }
                                AcceptanceType::Offline => StripeMandateRequest {
                                    mandate_type: StripeMandateType::Offline,
                                },
                            },
                        )
                    })
            })
            .transpose()?
            .or_else(|| {
                //stripe requires us to send mandate_data while making recurring payment through saved bank debit
                if payment_method.is_some() {
                    //check if payment is done through saved payment method
                    match &payment_method_types {
                        //check if payment method is bank debit
                        Some(
                            StripePaymentMethodType::Ach
                            | StripePaymentMethodType::Sepa
                            | StripePaymentMethodType::Becs
                            | StripePaymentMethodType::Bacs,
                        ) => Some(StripeMandateRequest {
                            mandate_type: StripeMandateType::Offline,
                        }),
                        _ => None,
                    }
                } else {
                    None
                }
            });

        let meta_data = get_transaction_metadata(item.request.metadata.clone(), order_id);

        // We pass browser_info only when payment_data exists.
        // Hence, we're pass Null during recurring payments as payment_method_data[type] is not passed
        let browser_info = if payment_data.is_some() {
            item.request
                .browser_info
                .clone()
                .map(StripeBrowserInformation::from)
        } else {
            None
        };

        Ok(Self {
            amount: item.request.amount, //hopefully we don't loose some cents here
            currency: item.request.currency.to_string(), //we need to copy the value and not transfer ownership
            statement_descriptor_suffix: item.request.statement_descriptor_suffix.clone(),
            statement_descriptor: item.request.statement_descriptor.clone(),
            meta_data,
            return_url: item
                .request
                .router_return_url
                .clone()
                .unwrap_or_else(|| "https://juspay.in/".to_string()),
            confirm: true, // Stripe requires confirm to be true if return URL is present
            description: item.description.clone(),
            shipping: shipping_address,
            billing: billing_address,
            capture_method: StripeCaptureMethod::from(item.request.capture_method),
            payment_data,
            payment_method_options,
            payment_method,
            customer: item.connector_customer.to_owned().map(Secret::new),
            setup_mandate_details,
            off_session: item.request.off_session,
            setup_future_usage: item.request.setup_future_usage,
            payment_method_types,
            expand: Some(ExpandableObjects::LatestCharge),
            browser_info,
        })
    }
}

fn get_payment_method_type_for_saved_payment_method_payment(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<StripePaymentMethodType>, error_stack::Report<errors::ConnectorError>> {
    if item.payment_method == api_enums::PaymentMethod::Card {
        Ok(Some(StripePaymentMethodType::Card)) //stripe takes ["Card"] as default
    } else {
        let stripe_payment_method_type = match item.recurring_mandate_payment_data.clone() {
            Some(recurring_payment_method_data) => {
                match recurring_payment_method_data.payment_method_type {
                    Some(payment_method_type) => {
                        StripePaymentMethodType::try_from(payment_method_type)
                    }
                    None => Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_method_type",
                    }
                    .into()),
                }
            }
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "recurring_mandate_payment_data",
            }
            .into()),
        }?;
        match stripe_payment_method_type {
            //Stripe converts Ideal, Bancontact & Sofort Bank redirect methods to Sepa direct debit and attaches to the customer for future usage
            StripePaymentMethodType::Ideal
            | StripePaymentMethodType::Bancontact
            | StripePaymentMethodType::Sofort => Ok(Some(StripePaymentMethodType::Sepa)),
            _ => Ok(Some(stripe_payment_method_type)),
        }
    }
}

impl From<types::BrowserInformation> for StripeBrowserInformation {
    fn from(item: types::BrowserInformation) -> Self {
        Self {
            ip_address: item.ip_address.map(|ip| Secret::new(ip.to_string())),
            user_agent: item.user_agent,
        }
    }
}

impl TryFrom<&types::SetupMandateRouterData> for SetupIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        //Only cards supported for mandates
        let pm_type = StripePaymentMethodType::Card;
        let payment_data = StripePaymentMethodData::try_from((
            item.request.payment_method_data.clone(),
            item.auth_type,
            pm_type,
        ))?;

        let meta_data = Some(get_transaction_metadata(
            item.request.metadata.clone(),
            item.connector_request_reference_id.clone(),
        ));

        let browser_info = item
            .request
            .browser_info
            .clone()
            .map(StripeBrowserInformation::from);

        Ok(Self {
            confirm: true,
            payment_data,
            return_url: item.request.router_return_url.clone(),
            off_session: item.request.off_session,
            usage: item.request.setup_future_usage,
            payment_method_options: None,
            customer: item.connector_customer.to_owned().map(Secret::new),
            meta_data,
            payment_method_types: Some(pm_type),
            expand: Some(ExpandableObjects::LatestAttempt),
            browser_info,
        })
    }
}

impl TryFrom<&types::TokenizationRouterData> for TokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        let payment_data = create_stripe_payment_method(
            &item.request.payment_method_data,
            item.auth_type,
            item.payment_method_token.clone(),
            None,
            StripeBillingAddress::default(),
        )?;
        Ok(Self {
            token_data: payment_data.0,
        })
    }
}

impl TryFrom<&types::ConnectorCustomerRouterData> for CustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            description: item.request.description.to_owned(),
            email: item.request.email.to_owned(),
            phone: item.request.phone.to_owned(),
            name: item.request.name.to_owned(),
            source: item.request.preprocessing_id.to_owned().map(Secret::new),
        })
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    #[serde(rename = "requires_action")]
    RequiresCustomerAction,
    #[serde(rename = "requires_payment_method")]
    RequiresPaymentMethod,
    RequiresConfirmation,
    Canceled,
    RequiresCapture,
    Chargeable,
    Consumed,
    Pending,
}

impl From<StripePaymentStatus> for enums::AttemptStatus {
    fn from(item: StripePaymentStatus) -> Self {
        match item {
            StripePaymentStatus::Succeeded => Self::Charged,
            StripePaymentStatus::Failed => Self::Failure,
            StripePaymentStatus::Processing => Self::Authorizing,
            StripePaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
            // Make the payment attempt status as failed
            StripePaymentStatus::RequiresPaymentMethod => Self::Failure,
            StripePaymentStatus::RequiresConfirmation => Self::ConfirmationAwaited,
            StripePaymentStatus::Canceled => Self::Voided,
            StripePaymentStatus::RequiresCapture => Self::Authorized,
            StripePaymentStatus::Chargeable => Self::Authorizing,
            StripePaymentStatus::Consumed => Self::Authorizing,
            StripePaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentIntentResponse {
    pub id: String,
    pub object: String,
    pub amount: i64,
    pub amount_received: Option<i64>,
    pub amount_capturable: Option<i64>,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: Option<Secret<String>>,
    pub created: i32,
    pub customer: Option<Secret<String>>,
    pub payment_method: Option<Secret<String>>,
    pub description: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub last_payment_error: Option<ErrorDetails>,
    pub latest_attempt: Option<LatestAttempt>, //need a merchant to test this
    pub latest_charge: Option<StripeChargeEnum>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeSourceResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ach_credit_transfer: Option<AchCreditTransferResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multibanco: Option<MultibancoCreditTansferResponse>,
    pub receiver: AchReceiverDetails,
    pub status: StripePaymentStatus,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct AchCreditTransferResponse {
    pub account_number: Secret<String>,
    pub bank_name: Secret<String>,
    pub routing_number: Secret<String>,
    pub swift_code: Secret<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct MultibancoCreditTansferResponse {
    pub reference: Secret<String>,
    pub entity: Secret<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct AchReceiverDetails {
    pub amount_received: i64,
    pub amount_charged: i64,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct SepaAndBacsBankTransferInstructions {
    pub bacs_bank_instructions: Option<BacsFinancialDetails>,
    pub sepa_bank_instructions: Option<SepaFinancialDetails>,
    pub receiver: SepaAndBacsReceiver,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
pub struct QrCodeNextInstructions {
    pub image_data_url: Url,
    pub display_to_timestamp: Option<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct SepaAndBacsReceiver {
    pub amount_received: i64,
    pub amount_remaining: i64,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct PaymentSyncResponse {
    #[serde(flatten)]
    pub intent_fields: PaymentIntentResponse,
    pub last_payment_error: Option<ErrorDetails>,
}

impl Deref for PaymentSyncResponse {
    type Target = PaymentIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.intent_fields
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PaymentIntentSyncResponse {
    #[serde(flatten)]
    payment_intent_fields: PaymentIntentResponse,
    pub latest_charge: Option<StripeChargeEnum>,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum StripeChargeEnum {
    ChargeId(String),
    ChargeObject(StripeCharge),
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StripeCharge {
    pub id: String,
    pub payment_method_details: Option<StripePaymentMethodDetailsResponse>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StripeBankRedirectDetails {
    #[serde(rename = "generated_sepa_debit")]
    attached_payment_method: Option<Secret<String>>,
}

impl Deref for PaymentIntentSyncResponse {
    type Target = PaymentIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.payment_intent_fields
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StripeAdditionalCardDetails {
    checks: Option<Value>,
    three_d_secure: Option<Value>,
    network_transaction_id: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StripePaymentMethodDetailsResponse {
    //only ideal, sofort and bancontact is supported by stripe for recurring payment in bank redirect
    Ideal {
        ideal: StripeBankRedirectDetails,
    },
    Sofort {
        sofort: StripeBankRedirectDetails,
    },
    Bancontact {
        bancontact: StripeBankRedirectDetails,
    },

    //other payment method types supported by stripe. To avoid deserialization error.
    Blik,
    Eps,
    Fpx,
    Giropay,
    #[serde(rename = "p24")]
    Przelewy24,
    Card {
        card: StripeAdditionalCardDetails,
    },
    Klarna,
    Affirm,
    AfterpayClearpay,
    ApplePay,
    #[serde(rename = "us_bank_account")]
    Ach,
    #[serde(rename = "sepa_debit")]
    Sepa,
    #[serde(rename = "au_becs_debit")]
    Becs,
    #[serde(rename = "bacs_debit")]
    Bacs,
    #[serde(rename = "wechat_pay")]
    Wechatpay,
    Alipay,
    CustomerBalance,
}

pub struct AdditionalPaymentMethodDetails {
    pub payment_checks: Option<Value>,
    pub authentication_details: Option<Value>,
}

impl From<AdditionalPaymentMethodDetails> for types::AdditionalPaymentMethodConnectorResponse {
    fn from(item: AdditionalPaymentMethodDetails) -> Self {
        Self::Card {
            authentication_data: item.authentication_details,
            payment_checks: item.payment_checks,
        }
    }
}

impl StripePaymentMethodDetailsResponse {
    pub fn get_additional_payment_method_data(&self) -> Option<AdditionalPaymentMethodDetails> {
        match self {
            Self::Card { card } => Some(AdditionalPaymentMethodDetails {
                payment_checks: card.checks.clone(),
                authentication_details: card.three_d_secure.clone(),
            }),
            Self::Ideal { .. }
            | Self::Sofort { .. }
            | Self::Bancontact { .. }
            | Self::Blik
            | Self::Eps
            | Self::Fpx
            | Self::Giropay
            | Self::Przelewy24
            | Self::Klarna
            | Self::Affirm
            | Self::AfterpayClearpay
            | Self::ApplePay
            | Self::Ach
            | Self::Sepa
            | Self::Becs
            | Self::Bacs
            | Self::Wechatpay
            | Self::Alipay
            | Self::CustomerBalance => None,
        }
    }
}

#[derive(Deserialize)]
pub struct SetupIntentSyncResponse {
    #[serde(flatten)]
    setup_intent_fields: SetupIntentResponse,
}

impl Deref for SetupIntentSyncResponse {
    type Target = SetupIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.setup_intent_fields
    }
}

impl From<SetupIntentSyncResponse> for PaymentIntentSyncResponse {
    fn from(value: SetupIntentSyncResponse) -> Self {
        Self {
            payment_intent_fields: value.setup_intent_fields.into(),
            latest_charge: None,
        }
    }
}

impl From<SetupIntentResponse> for PaymentIntentResponse {
    fn from(value: SetupIntentResponse) -> Self {
        Self {
            id: value.id,
            object: value.object,
            status: value.status,
            client_secret: Some(value.client_secret),
            customer: value.customer,
            description: None,
            statement_descriptor: value.statement_descriptor,
            statement_descriptor_suffix: value.statement_descriptor_suffix,
            metadata: value.metadata,
            next_action: value.next_action,
            payment_method_options: value.payment_method_options,
            last_payment_error: value.last_setup_error,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct SetupIntentResponse {
    pub id: String,
    pub object: String,
    pub status: StripePaymentStatus, // Change to SetupStatus
    pub client_secret: Secret<String>,
    pub customer: Option<Secret<String>>,
    pub payment_method: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub latest_attempt: Option<LatestAttempt>,
    pub last_setup_error: Option<ErrorDetails>,
}

fn extract_payment_method_connector_response_from_latest_charge(
    stripe_charge_enum: &StripeChargeEnum,
) -> Option<types::ConnectorResponseData> {
    if let StripeChargeEnum::ChargeObject(charge_object) = stripe_charge_enum {
        charge_object
            .payment_method_details
            .as_ref()
            .and_then(StripePaymentMethodDetailsResponse::get_additional_payment_method_data)
    } else {
        None
    }
    .map(types::AdditionalPaymentMethodConnectorResponse::from)
    .map(types::ConnectorResponseData::with_additional_payment_method_data)
}

fn extract_payment_method_connector_response_from_latest_attempt(
    stripe_latest_attempt: &LatestAttempt,
) -> Option<types::ConnectorResponseData> {
    if let LatestAttempt::PaymentIntentAttempt(intent_attempt) = stripe_latest_attempt {
        intent_attempt
            .payment_method_details
            .as_ref()
            .and_then(StripePaymentMethodDetailsResponse::get_additional_payment_method_data)
    } else {
        None
    }
    .map(types::AdditionalPaymentMethodConnectorResponse::from)
    .map(types::ConnectorResponseData::with_additional_payment_method_data)
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirect_data = item.response.next_action.clone();
        let redirection_data = redirect_data
            .and_then(|redirection_data| redirection_data.get_url())
            .map(|redirection_url| {
                services::RedirectForm::from((redirection_url, services::Method::Get))
            });

        let mandate_reference = item.response.payment_method.map(|payment_method_id| {
            // Implemented Save and re-use payment information for recurring charges
            // For more info: https://docs.stripe.com/recurring-payments#accept-recurring-payments
            // For backward compatibility payment_method_id & connector_mandate_id is being populated with the same value
            let connector_mandate_id = Some(payment_method_id.clone().expose());
            let payment_method_id = Some(payment_method_id.expose());
            types::MandateReference {
                connector_mandate_id,
                payment_method_id,
            }
        });

        //Note: we might have to call retrieve_setup_intent to get the network_transaction_id in case its not sent in PaymentIntentResponse
        // Or we identify the mandate txns before hand and always call SetupIntent in case of mandate payment call
        let network_txn_id = Option::foreign_from(item.response.latest_attempt);

        let connector_metadata =
            get_connector_metadata(item.response.next_action.as_ref(), item.response.amount)?;

        let status = enums::AttemptStatus::from(item.response.status);

        let response = if connector_util::is_payment_failure(status) {
            types::PaymentsResponseData::try_from((
                &item.response.last_payment_error,
                item.http_code,
                item.response.id.clone(),
            ))
        } else {
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata,
                network_txn_id,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
            })
        };

        let connector_response_data = item
            .response
            .latest_charge
            .as_ref()
            .and_then(extract_payment_method_connector_response_from_latest_charge);

        Ok(Self {
            status,
            // client_secret: Some(item.response.client_secret.clone().as_str()),
            // description: item.response.description.map(|x| x.as_str()),
            // statement_descriptor_suffix: item.response.statement_descriptor_suffix.map(|x| x.as_str()),
            // three_ds_form,
            response,
            amount_captured: item.response.amount_received,
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

pub fn get_connector_metadata(
    next_action: Option<&StripeNextActionResponse>,
    amount: i64,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    let next_action_response = next_action
        .and_then(|next_action_response| match next_action_response {
            StripeNextActionResponse::DisplayBankTransferInstructions(response) => {
                let bank_instructions = response.financial_addresses.first();
                let (sepa_bank_instructions, bacs_bank_instructions) =
                    bank_instructions.map_or((None, None), |financial_address| {
                        (
                            financial_address.iban.to_owned(),
                            financial_address.sort_code.to_owned(),
                        )
                    });

                let bank_transfer_instructions = SepaAndBacsBankTransferInstructions {
                    sepa_bank_instructions,
                    bacs_bank_instructions,
                    receiver: SepaAndBacsReceiver {
                        amount_received: amount - response.amount_remaining,
                        amount_remaining: response.amount_remaining,
                    },
                };

                Some(bank_transfer_instructions.encode_to_value())
            }
            StripeNextActionResponse::WechatPayDisplayQrCode(response) => {
                let wechat_pay_instructions = QrCodeNextInstructions {
                    image_data_url: response.image_data_url.to_owned(),
                    display_to_timestamp: None,
                };

                Some(wechat_pay_instructions.encode_to_value())
            }
            StripeNextActionResponse::CashappHandleRedirectOrDisplayQrCode(response) => {
                let cashapp_qr_instructions: QrCodeNextInstructions = QrCodeNextInstructions {
                    image_data_url: response.qr_code.image_url_png.to_owned(),
                    display_to_timestamp: response.qr_code.expires_at.to_owned(),
                };
                Some(cashapp_qr_instructions.encode_to_value())
            }
            _ => None,
        })
        .transpose()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
    Ok(next_action_response)
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaymentIntentSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PaymentIntentSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let redirect_data = item.response.next_action.clone();
        let redirection_data = redirect_data
            .and_then(|redirection_data| redirection_data.get_url())
            .map(|redirection_url| {
                services::RedirectForm::from((redirection_url, services::Method::Get))
            });

        let mandate_reference = item
            .response
            .payment_method
            .clone()
            .map(|payment_method_id| {
                // Implemented Save and re-use payment information for recurring charges
                // For more info: https://docs.stripe.com/recurring-payments#accept-recurring-payments
                // For backward compatibility payment_method_id & connector_mandate_id is being populated with the same value
                let connector_mandate_id = Some(payment_method_id.clone().expose());
                let payment_method_id = match item.response.latest_charge.clone() {
                    Some(StripeChargeEnum::ChargeObject(charge)) => {
                        match charge.payment_method_details {
                            Some(StripePaymentMethodDetailsResponse::Bancontact { bancontact }) => {
                                bancontact
                                    .attached_payment_method
                                    .map(|attached_payment_method| attached_payment_method.expose())
                                    .unwrap_or(payment_method_id.expose())
                            }
                            Some(StripePaymentMethodDetailsResponse::Ideal { ideal }) => ideal
                                .attached_payment_method
                                .map(|attached_payment_method| attached_payment_method.expose())
                                .unwrap_or(payment_method_id.expose()),
                            Some(StripePaymentMethodDetailsResponse::Sofort { sofort }) => sofort
                                .attached_payment_method
                                .map(|attached_payment_method| attached_payment_method.expose())
                                .unwrap_or(payment_method_id.expose()),
                            Some(StripePaymentMethodDetailsResponse::Blik)
                            | Some(StripePaymentMethodDetailsResponse::Eps)
                            | Some(StripePaymentMethodDetailsResponse::Fpx)
                            | Some(StripePaymentMethodDetailsResponse::Giropay)
                            | Some(StripePaymentMethodDetailsResponse::Przelewy24)
                            | Some(StripePaymentMethodDetailsResponse::Card { .. })
                            | Some(StripePaymentMethodDetailsResponse::Klarna)
                            | Some(StripePaymentMethodDetailsResponse::Affirm)
                            | Some(StripePaymentMethodDetailsResponse::AfterpayClearpay)
                            | Some(StripePaymentMethodDetailsResponse::ApplePay)
                            | Some(StripePaymentMethodDetailsResponse::Ach)
                            | Some(StripePaymentMethodDetailsResponse::Sepa)
                            | Some(StripePaymentMethodDetailsResponse::Becs)
                            | Some(StripePaymentMethodDetailsResponse::Bacs)
                            | Some(StripePaymentMethodDetailsResponse::Wechatpay)
                            | Some(StripePaymentMethodDetailsResponse::Alipay)
                            | Some(StripePaymentMethodDetailsResponse::CustomerBalance)
                            | None => payment_method_id.expose(),
                        }
                    }
                    Some(StripeChargeEnum::ChargeId(_)) | None => payment_method_id.expose(),
                };
                types::MandateReference {
                    connector_mandate_id,
                    payment_method_id: Some(payment_method_id),
                }
            });

        let connector_metadata =
            get_connector_metadata(item.response.next_action.as_ref(), item.response.amount)?;

        let status = enums::AttemptStatus::from(item.response.status.to_owned());

        let connector_response_data = item
            .response
            .latest_charge
            .as_ref()
            .and_then(extract_payment_method_connector_response_from_latest_charge);

        let response = if connector_util::is_payment_failure(status) {
            types::PaymentsResponseData::try_from((
                &item.response.payment_intent_fields.last_payment_error,
                item.http_code,
                item.response.id.clone(),
            ))
        } else {
            let network_transaction_id = match item.response.latest_charge.clone() {
                Some(StripeChargeEnum::ChargeObject(charge_object)) => charge_object
                    .payment_method_details
                    .and_then(|payment_method_details| match payment_method_details {
                        StripePaymentMethodDetailsResponse::Card { card } => {
                            card.network_transaction_id
                        }
                        _ => None,
                    }),
                _ => None,
            };
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata,
                network_txn_id: network_transaction_id,
                connector_response_reference_id: Some(item.response.id.clone()),
                incremental_authorization_allowed: None,
            })
        };

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.to_owned()),
            response,
            amount_captured: item.response.amount_received,
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, SetupIntentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, SetupIntentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirect_data = item.response.next_action.clone();
        let redirection_data = redirect_data
            .and_then(|redirection_data| redirection_data.get_url())
            .map(|redirection_url| {
                services::RedirectForm::from((redirection_url, services::Method::Get))
            });

        let mandate_reference = item.response.payment_method.map(|payment_method_id| {
            // Implemented Save and re-use payment information for recurring charges
            // For more info: https://docs.stripe.com/recurring-payments#accept-recurring-payments
            // For backward compatibility payment_method_id & connector_mandate_id is being populated with the same value
            let connector_mandate_id = Some(payment_method_id.clone());
            let payment_method_id = Some(payment_method_id);
            types::MandateReference {
                connector_mandate_id,
                payment_method_id,
            }
        });
        let status = enums::AttemptStatus::from(item.response.status);
        let connector_response_data = item
            .response
            .latest_attempt
            .as_ref()
            .and_then(extract_payment_method_connector_response_from_latest_attempt);

        let response = if connector_util::is_payment_failure(status) {
            types::PaymentsResponseData::try_from((
                &item.response.last_setup_error,
                item.http_code,
                item.response.id.clone(),
            ))
        } else {
            let network_transaction_id = match item.response.latest_attempt {
                Some(LatestAttempt::PaymentIntentAttempt(attempt)) => attempt
                    .payment_method_details
                    .and_then(|payment_method_details| match payment_method_details {
                        StripePaymentMethodDetailsResponse::Card { card } => {
                            card.network_transaction_id
                        }
                        _ => None,
                    }),
                _ => None,
            };

            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
                network_txn_id: network_transaction_id,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
            })
        };

        Ok(Self {
            status,
            response,
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

impl ForeignFrom<Option<LatestAttempt>> for Option<String> {
    fn foreign_from(latest_attempt: Option<LatestAttempt>) -> Self {
        match latest_attempt {
            Some(LatestAttempt::PaymentIntentAttempt(attempt)) => attempt
                .payment_method_options
                .and_then(|payment_method_options| match payment_method_options {
                    StripePaymentMethodOptions::Card {
                        network_transaction_id,
                        ..
                    } => network_transaction_id.map(|network_id| network_id.expose()),
                    _ => None,
                }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", remote = "Self")]
pub enum StripeNextActionResponse {
    CashappHandleRedirectOrDisplayQrCode(StripeCashappQrResponse),
    RedirectToUrl(StripeRedirectToUrlResponse),
    AlipayHandleRedirect(StripeRedirectToUrlResponse),
    VerifyWithMicrodeposits(StripeVerifyWithMicroDepositsResponse),
    WechatPayDisplayQrCode(WechatPayRedirectToQr),
    DisplayBankTransferInstructions(StripeBankTransferDetails),
    NoNextActionBody,
}

impl StripeNextActionResponse {
    fn get_url(&self) -> Option<Url> {
        match self {
            Self::RedirectToUrl(redirect_to_url) | Self::AlipayHandleRedirect(redirect_to_url) => {
                Some(redirect_to_url.url.to_owned())
            }
            Self::WechatPayDisplayQrCode(_) => None,
            Self::VerifyWithMicrodeposits(verify_with_microdeposits) => {
                Some(verify_with_microdeposits.hosted_verification_url.to_owned())
            }
            Self::CashappHandleRedirectOrDisplayQrCode(_) => None,
            Self::DisplayBankTransferInstructions(_) => None,
            Self::NoNextActionBody => None,
        }
    }
}

// This impl is required because Stripe's response is of the below format, which is externally
// tagged, but also with an extra 'type' field specifying the enum variant name:
// "next_action": {
//   "redirect_to_url": { "return_url": "...", "url": "..." },
//   "type": "redirect_to_url"
// },
// Reference: https://github.com/serde-rs/serde/issues/1343#issuecomment-409698470
impl<'de> Deserialize<'de> for StripeNextActionResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(rename = "type")]
            _ignore: String,
            #[serde(flatten, with = "StripeNextActionResponse")]
            inner: StripeNextActionResponse,
        }

        // There is some exception in the stripe next action, it usually sends :
        // "next_action": {
        //   "redirect_to_url": { "return_url": "...", "url": "..." },
        //   "type": "redirect_to_url"
        // },
        // But there is a case where it only sends the type and not other field named as it's type
        let stripe_next_action_response =
            Wrapper::deserialize(deserializer).map_or(Self::NoNextActionBody, |w| w.inner);

        Ok(stripe_next_action_response)
    }
}

impl Serialize for StripeNextActionResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Self::CashappHandleRedirectOrDisplayQrCode(ref i) => {
                serde::Serialize::serialize(i, serializer)
            }
            Self::RedirectToUrl(ref i) => serde::Serialize::serialize(i, serializer),
            Self::AlipayHandleRedirect(ref i) => serde::Serialize::serialize(i, serializer),
            Self::VerifyWithMicrodeposits(ref i) => serde::Serialize::serialize(i, serializer),
            Self::WechatPayDisplayQrCode(ref i) => serde::Serialize::serialize(i, serializer),
            Self::DisplayBankTransferInstructions(ref i) => {
                serde::Serialize::serialize(i, serializer)
            }
            Self::NoNextActionBody => serde::Serialize::serialize("NoNextActionBody", serializer),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeRedirectToUrlResponse {
    return_url: String,
    url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct WechatPayRedirectToQr {
    // This data contains url, it should be converted to QR code.
    // Note: The url in this data is not redirection url
    data: Url,
    // This is the image source, this image_data_url can directly be used by sdk to show the QR code
    image_data_url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeVerifyWithMicroDepositsResponse {
    hosted_verification_url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeBankTransferDetails {
    pub amount_remaining: i64,
    pub currency: String,
    pub financial_addresses: Vec<StripeFinancialInformation>,
    pub hosted_instructions_url: Option<String>,
    pub reference: Option<String>,
    #[serde(rename = "type")]
    pub bank_transfer_type: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeCashappQrResponse {
    pub mobile_auth_url: Url,
    pub qr_code: QrCodeResponse,
    pub hosted_instructions_url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct QrCodeResponse {
    pub expires_at: Option<i64>,
    pub image_url_png: Url,
    pub image_url_svg: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeFinancialInformation {
    pub iban: Option<SepaFinancialDetails>,
    pub sort_code: Option<BacsFinancialDetails>,
    pub supported_networks: Vec<String>,
    #[serde(rename = "type")]
    pub financial_info_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SepaFinancialDetails {
    pub account_holder_name: Secret<String>,
    pub bic: Secret<String>,
    pub country: Secret<String>,
    pub iban: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BacsFinancialDetails {
    pub account_holder_name: Secret<String>,
    pub account_number: Secret<String>,
    pub sort_code: Secret<String>,
}

// REFUND :
// Type definition for Stripe RefundRequest

#[derive(Debug, Serialize)]
pub struct RefundRequest {
    pub amount: Option<i64>, //amount in cents, hence passed as integer
    pub payment_intent: String,
    #[serde(flatten)]
    pub meta_data: StripeMetadata,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = item.request.refund_amount;
        let payment_intent = item.request.connector_transaction_id.clone();
        Ok(Self {
            amount: Some(amount),
            payment_intent,
            meta_data: StripeMetadata {
                order_id: Some(item.request.refund_id.clone()),
                is_refund_id_as_reference: Some("true".to_string()),
            },
        })
    }
}

// Type definition for Stripe Refund Response

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresAction,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => Self::Success,
            self::RefundStatus::Failed => Self::Failure,
            self::RefundStatus::Pending => Self::Pending,
            self::RefundStatus::RequiresAction => Self::ManualReview,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub object: String,
    pub amount: i64,
    pub currency: String,
    pub metadata: StripeMetadata,
    pub payment_intent: String,
    pub status: RefundStatus,
    pub failure_reason: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        let response = if connector_util::is_refund_failure(refund_status) {
            Err(types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: item
                    .response
                    .failure_reason
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.failure_reason,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id),
            })
        } else {
            Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        let response = if connector_util::is_refund_failure(refund_status) {
            Err(types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: item
                    .response
                    .failure_reason
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.failure_reason,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id),
            })
        } else {
            Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
    pub decline_code: Option<String>,
    pub payment_intent: Option<PaymentIntentErrorResponse>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentIntentErrorResponse {
    pub id: String,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeShippingAddress {
    #[serde(rename = "shipping[address][city]")]
    pub city: Option<String>,
    #[serde(rename = "shipping[address][country]")]
    pub country: Option<api_enums::CountryAlpha2>,
    #[serde(rename = "shipping[address][line1]")]
    pub line1: Option<Secret<String>>,
    #[serde(rename = "shipping[address][line2]")]
    pub line2: Option<Secret<String>>,
    #[serde(rename = "shipping[address][postal_code]")]
    pub zip: Option<Secret<String>>,
    #[serde(rename = "shipping[address][state]")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "shipping[name]")]
    pub name: Secret<String>,
    #[serde(rename = "shipping[phone]")]
    pub phone: Option<Secret<String>>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeBillingAddress {
    #[serde(rename = "payment_method_data[billing_details][email]")]
    pub email: Option<Email>,
    #[serde(rename = "payment_method_data[billing_details][address][country]")]
    pub country: Option<api_enums::CountryAlpha2>,
    #[serde(rename = "payment_method_data[billing_details][name]")]
    pub name: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][city]")]
    pub city: Option<String>,
    #[serde(rename = "payment_method_data[billing_details][address][line1]")]
    pub address_line1: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][line2]")]
    pub address_line2: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][postal_code]")]
    pub zip_code: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][address][state]")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "payment_method_data[billing_details][phone]")]
    pub phone: Option<Secret<String>>,
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct StripeRedirectResponse {
    pub payment_intent: Option<String>,
    pub payment_intent_client_secret: Option<Secret<String>>,
    pub source_redirect_slug: Option<String>,
    pub redirect_status: Option<StripePaymentStatus>,
    pub source_type: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct CancelRequest {
    cancellation_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for CancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            cancellation_reason: item.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum StripePaymentMethodOptions {
    Card {
        mandate_options: Option<StripeMandateOptions>,
        #[serde(rename = "payment_method_options[card][network_transaction_id]")]
        network_transaction_id: Option<Secret<String>>,
        #[serde(flatten)]
        mit_exemption: Option<MitExemption>, // To be used for MIT mandate txns
    },
    Klarna {},
    Affirm {},
    AfterpayClearpay {},
    Eps {},
    Giropay {},
    Ideal {},
    Sofort {},
    #[serde(rename = "us_bank_account")]
    Ach {},
    #[serde(rename = "sepa_debit")]
    Sepa {},
    #[serde(rename = "au_becs_debit")]
    Becs {},
    #[serde(rename = "bacs_debit")]
    Bacs {},
    Bancontact {},
    WechatPay {},
    Alipay {},
    #[serde(rename = "p24")]
    Przelewy24 {},
    CustomerBalance {},
    Multibanco {},
    Blik {},
    Cashapp {},
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MitExemption {
    #[serde(rename = "payment_method_options[card][mit_exemption][network_transaction_id]")]
    pub network_transaction_id: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LatestAttempt {
    PaymentIntentAttempt(LatestPaymentAttempt),
    SetupAttempt(String),
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct LatestPaymentAttempt {
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub payment_method_details: Option<StripePaymentMethodDetailsResponse>,
}

// #[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
// pub struct Card
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct StripeMandateOptions {
    reference: Secret<String>, // Extendable, But only important field to be captured
}
/// Represents the capture request body for stripe connector.
#[derive(Debug, Serialize, Clone, Copy)]
pub struct CaptureRequest {
    /// If amount_to_capture is None stripe captures the amount in the payment intent.
    amount_to_capture: Option<i64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for CaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_to_capture: Some(item.request.amount_to_capture),
        })
    }
}

impl TryFrom<&types::PaymentsPreProcessingRouterData> for StripeCreditTransferSourceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let currency = item.request.get_currency()?;

        match &item.request.payment_method_data {
            Some(domain::PaymentMethodData::BankTransfer(bank_transfer_data)) => {
                match **bank_transfer_data {
                    domain::BankTransferData::MultibancoBankTransfer { .. } => Ok(
                        Self::MultibancoBankTansfer(MultibancoCreditTransferSourceRequest {
                            transfer_type: StripeCreditTransferTypes::Multibanco,
                            currency,
                            payment_method_data: MultibancoTransferData {
                                email: item.request.get_email()?,
                            },
                            amount: Some(item.request.get_amount()?),
                            return_url: Some(item.get_return_url()?),
                        }),
                    ),
                    domain::BankTransferData::AchBankTransfer { .. } => {
                        Ok(Self::AchBankTansfer(AchCreditTransferSourceRequest {
                            transfer_type: StripeCreditTransferTypes::AchCreditTransfer,
                            payment_method_data: AchTransferData {
                                email: item.request.get_email()?,
                            },
                            currency,
                        }))
                    }
                    domain::BankTransferData::SepaBankTransfer { .. }
                    | domain::BankTransferData::BacsBankTransfer { .. }
                    | domain::BankTransferData::PermataBankTransfer { .. }
                    | domain::BankTransferData::BcaBankTransfer { .. }
                    | domain::BankTransferData::BniVaBankTransfer { .. }
                    | domain::BankTransferData::BriVaBankTransfer { .. }
                    | domain::BankTransferData::CimbVaBankTransfer { .. }
                    | domain::BankTransferData::DanamonVaBankTransfer { .. }
                    | domain::BankTransferData::MandiriVaBankTransfer { .. }
                    | domain::BankTransferData::LocalBankTransfer { .. }
                    | domain::BankTransferData::Pix { .. }
                    | domain::BankTransferData::Pse { .. } => {
                        Err(errors::ConnectorError::NotImplemented(
                            connector_util::get_unimplemented_payment_method_error_message(
                                "stripe",
                            ),
                        )
                        .into())
                    }
                }
            }
            Some(domain::PaymentMethodData::Card(..))
            | Some(domain::PaymentMethodData::Wallet(..))
            | Some(domain::PaymentMethodData::BankDebit(..))
            | Some(domain::PaymentMethodData::BankRedirect(..))
            | Some(domain::PaymentMethodData::PayLater(..))
            | Some(domain::PaymentMethodData::Crypto(..))
            | Some(domain::PaymentMethodData::Reward)
            | Some(domain::PaymentMethodData::MandatePayment)
            | Some(domain::PaymentMethodData::Upi(..))
            | Some(domain::PaymentMethodData::GiftCard(..))
            | Some(domain::PaymentMethodData::CardRedirect(..))
            | Some(domain::PaymentMethodData::Voucher(..))
            | Some(domain::PaymentMethodData::CardToken(..))
            | None => Err(errors::ConnectorError::NotImplemented(
                connector_util::get_unimplemented_payment_method_error_message("stripe"),
            )
            .into()),
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StripeSourceResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StripeSourceResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_source_response = item.response.to_owned();
        let connector_metadata = connector_source_response
            .encode_to_value()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        // We get pending as the status from stripe, but hyperswitch should give it as requires_customer_action as
        // customer has to make payment to the virtual account number given in the source response
        let status = match connector_source_response.status.clone().into() {
            diesel_models::enums::AttemptStatus::Pending => {
                diesel_models::enums::AttemptStatus::AuthenticationPending
            }
            _ => connector_source_response.status.into(),
        };
        Ok(Self {
            response: Ok(types::PaymentsResponseData::PreProcessingResponse {
                pre_processing_id: types::PreprocessingResponseId::PreProcessingId(
                    item.response.id,
                ),
                connector_metadata: Some(connector_metadata),
                session_token: None,
                connector_response_reference_id: None,
            }),
            status,
            ..item.data
        })
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for ChargesRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(value: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        {
            let order_id = value.connector_request_reference_id.clone();
            let meta_data = Some(get_transaction_metadata(
                value.request.metadata.clone(),
                order_id,
            ));
            Ok(Self {
                amount: value.request.amount.to_string(),
                currency: value.request.currency.to_string(),
                customer: Secret::new(value.get_connector_customer_id()?),
                source: Secret::new(value.get_preprocessing_id()?),
                meta_data,
            })
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, ChargesResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ChargesResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_source_response = item.response.to_owned();
        let connector_metadata = connector_source_response
            .source
            .encode_to_value()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        let status = enums::AttemptStatus::from(item.response.status);
        let response = if connector_util::is_payment_failure(status) {
            Err(types::ErrorResponse {
                code: item
                    .response
                    .failure_code
                    .unwrap_or_else(|| crate::consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .failure_message
                    .clone()
                    .unwrap_or_else(|| crate::consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.failure_message,
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id),
            })
        } else {
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(connector_metadata),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StripeTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StripeTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StripeCustomerResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StripeCustomerResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id,
            }),
            ..item.data
        })
    }
}

// #[cfg(test)]
// mod test_stripe_transformers {
//     use super::*;

//     #[test]
//     fn verify_transform_from_router_to_stripe_req() {
//         let router_req = PaymentsRequest {
//             amount: 100.0,
//             currency: "USD".to_string(),
//             ..Default::default()
//         };

//         let stripe_req = PaymentIntentRequest::from(router_req);

//         //metadata is generated everytime. So use the transformed struct to copy uuid

//         let stripe_req_expected = PaymentIntentRequest {
//             amount: 10000,
//             currency: "USD".to_string(),
//             statement_descriptor_suffix: None,
//             metadata_order_id: "Auto generate Order ID".to_string(),
//             metadata_txn_id: "Fetch from Merchant Account_Auto generate Order ID_1".to_string(),
//             metadata_txn_uuid: stripe_req.metadata_txn_uuid.clone(),
//             return_url: "Fetch Url from Merchant Account".to_string(),
//             confirm: false,
//             payment_method_types: "card".to_string(),
//             payment_method_data_type: "card".to_string(),
//             payment_method_data_card_number: None,
//             payment_method_data_card_exp_month: None,
//             payment_method_data_card_exp_year: None,
//             payment_method_data_card_cvc: None,
//             description: None,
//         };
//         assert_eq!(stripe_req_expected, stripe_req);
//     }
// }

#[derive(Debug, Deserialize)]
pub struct WebhookEventDataResource {
    pub object: Value,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventObjectResource {
    pub data: WebhookEventDataResource,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
    #[serde(rename = "data")]
    pub event_data: WebhookEventData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventTypeBody {
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
    #[serde(rename = "data")]
    pub event_data: WebhookStatusData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventData {
    #[serde(rename = "object")]
    pub event_object: WebhookEventObjectData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookStatusData {
    #[serde(rename = "object")]
    pub event_object: WebhookStatusObjectData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookStatusObjectData {
    pub status: Option<WebhookEventStatus>,
    pub payment_method_details: Option<WebhookPaymentMethodDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookPaymentMethodType {
    AchCreditTransfer,
    MultibancoBankTransfers,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPaymentMethodDetails {
    #[serde(rename = "type")]
    pub payment_method: WebhookPaymentMethodType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventObjectData {
    pub id: String,
    pub object: WebhookEventObjectType,
    pub amount: Option<i32>,
    pub currency: String,
    pub payment_intent: Option<String>,
    pub client_secret: Option<Secret<String>>,
    pub reason: Option<String>,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub created: PrimitiveDateTime,
    pub evidence_details: Option<EvidenceDetails>,
    pub status: Option<WebhookEventStatus>,
    pub metadata: Option<StripeMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventObjectType {
    PaymentIntent,
    Dispute,
    Charge,
    Source,
    Refund,
}

#[derive(Debug, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "payment_intent.payment_failed")]
    PaymentIntentFailed,
    #[serde(rename = "payment_intent.succeeded")]
    PaymentIntentSucceed,
    #[serde(rename = "charge.dispute.created")]
    DisputeCreated,
    #[serde(rename = "charge.dispute.closed")]
    DisputeClosed,
    #[serde(rename = "charge.dispute.updated")]
    DisputeUpdated,
    #[serde(rename = "charge.dispute.funds_reinstated")]
    ChargeDisputeFundsReinstated,
    #[serde(rename = "charge.dispute.funds_withdrawn")]
    ChargeDisputeFundsWithdrawn,
    #[serde(rename = "charge.expired")]
    ChargeExpired,
    #[serde(rename = "charge.failed")]
    ChargeFailed,
    #[serde(rename = "charge.pending")]
    ChargePending,
    #[serde(rename = "charge.captured")]
    ChargeCaptured,
    #[serde(rename = "charge.refund.updated")]
    ChargeRefundUpdated,
    #[serde(rename = "charge.succeeded")]
    ChargeSucceeded,
    #[serde(rename = "charge.updated")]
    ChargeUpdated,
    #[serde(rename = "charge.refunded")]
    ChargeRefunded,
    #[serde(rename = "payment_intent.canceled")]
    PaymentIntentCanceled,
    #[serde(rename = "payment_intent.created")]
    PaymentIntentCreated,
    #[serde(rename = "payment_intent.processing")]
    PaymentIntentProcessing,
    #[serde(rename = "payment_intent.requires_action")]
    PaymentIntentRequiresAction,
    #[serde(rename = "payment_intent.amount_capturable_updated")]
    PaymentIntentAmountCapturableUpdated,
    #[serde(rename = "source.chargeable")]
    SourceChargeable,
    #[serde(rename = "source.transaction.created")]
    SourceTransactionCreated,
    #[serde(rename = "payment_intent.partially_funded")]
    PaymentIntentPartiallyFunded,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, strum::Display, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventStatus {
    WarningNeedsResponse,
    WarningClosed,
    WarningUnderReview,
    Won,
    Lost,
    NeedsResponse,
    UnderReview,
    ChargeRefunded,
    Succeeded,
    RequiresPaymentMethod,
    RequiresConfirmation,
    RequiresAction,
    Processing,
    RequiresCapture,
    Canceled,
    Chargeable,
    Failed,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceDetails {
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub due_by: PrimitiveDateTime,
}

impl
    TryFrom<(
        domain::PaymentMethodData,
        enums::AuthenticationType,
        StripePaymentMethodType,
    )> for StripePaymentMethodData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (pm_data, auth_type, pm_type): (
            domain::PaymentMethodData,
            enums::AuthenticationType,
            StripePaymentMethodType,
        ),
    ) -> Result<Self, Self::Error> {
        match pm_data {
            domain::PaymentMethodData::Card(ref ccard) => {
                let payment_method_auth_type = match auth_type {
                    enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                    enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
                };
                Ok(Self::try_from((ccard, payment_method_auth_type))?)
            }
            domain::PaymentMethodData::PayLater(_) => Ok(Self::PayLater(StripePayLaterData {
                payment_method_data_type: pm_type,
            })),
            domain::PaymentMethodData::BankRedirect(ref bank_redirect_data) => {
                Ok(Self::try_from(bank_redirect_data)?)
            }
            domain::PaymentMethodData::Wallet(ref wallet_data) => {
                Ok(Self::try_from((wallet_data, None))?)
            }
            domain::PaymentMethodData::BankDebit(bank_debit_data) => {
                let (_pm_type, bank_data, _) = get_bank_debit_data(&bank_debit_data);

                Ok(Self::BankDebit(StripeBankDebitData {
                    bank_specific_data: bank_data,
                }))
            }
            domain::PaymentMethodData::BankTransfer(bank_transfer_data) => match bank_transfer_data
                .deref()
            {
                domain::BankTransferData::AchBankTransfer { billing_details } => {
                    Ok(Self::BankTransfer(StripeBankTransferData::AchBankTransfer(
                        Box::new(AchTransferData {
                            email: billing_details.email.to_owned(),
                        }),
                    )))
                }
                domain::BankTransferData::MultibancoBankTransfer { billing_details } => Ok(
                    Self::BankTransfer(StripeBankTransferData::MultibancoBankTransfers(Box::new(
                        MultibancoTransferData {
                            email: billing_details.email.to_owned(),
                        },
                    ))),
                ),
                domain::BankTransferData::SepaBankTransfer { country, .. } => {
                    Ok(Self::BankTransfer(
                        StripeBankTransferData::SepaBankTransfer(Box::new(SepaBankTransferData {
                            payment_method_data_type: StripePaymentMethodType::CustomerBalance,
                            bank_transfer_type: BankTransferType::EuBankTransfer,
                            balance_funding_type: BankTransferType::BankTransfers,
                            payment_method_type: StripePaymentMethodType::CustomerBalance,
                            country: country.to_owned(),
                        })),
                    ))
                }
                domain::BankTransferData::BacsBankTransfer { .. } => Ok(Self::BankTransfer(
                    StripeBankTransferData::BacsBankTransfers(Box::new(BacsBankTransferData {
                        payment_method_data_type: StripePaymentMethodType::CustomerBalance,
                        bank_transfer_type: BankTransferType::GbBankTransfer,
                        balance_funding_type: BankTransferType::BankTransfers,
                        payment_method_type: StripePaymentMethodType::CustomerBalance,
                    })),
                )),
                domain::BankTransferData::Pix {}
                | domain::BankTransferData::Pse {}
                | domain::BankTransferData::PermataBankTransfer { .. }
                | domain::BankTransferData::BcaBankTransfer { .. }
                | domain::BankTransferData::BniVaBankTransfer { .. }
                | domain::BankTransferData::BriVaBankTransfer { .. }
                | domain::BankTransferData::CimbVaBankTransfer { .. }
                | domain::BankTransferData::DanamonVaBankTransfer { .. }
                | domain::BankTransferData::LocalBankTransfer { .. }
                | domain::BankTransferData::MandiriVaBankTransfer { .. } => {
                    Err(errors::ConnectorError::NotImplemented(
                        connector_util::get_unimplemented_payment_method_error_message("stripe"),
                    )
                    .into())
                }
            },
            domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))?
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeGpayToken {
    pub id: String,
}

pub fn get_bank_transfer_request_data(
    req: &types::PaymentsAuthorizeRouterData,
    bank_transfer_data: &domain::BankTransferData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    match bank_transfer_data {
        domain::BankTransferData::AchBankTransfer { .. }
        | domain::BankTransferData::MultibancoBankTransfer { .. } => {
            let req = ChargesRequest::try_from(req)?;
            Ok(RequestContent::FormUrlEncoded(Box::new(req)))
        }
        _ => {
            let req = PaymentIntentRequest::try_from(req)?;
            Ok(RequestContent::FormUrlEncoded(Box::new(req)))
        }
    }
}

pub fn construct_file_upload_request(
    file_upload_router_data: types::UploadFileRouterData,
) -> CustomResult<reqwest::multipart::Form, errors::ConnectorError> {
    let request = file_upload_router_data.request;
    let mut multipart = reqwest::multipart::Form::new();
    multipart = multipart.text("purpose", "dispute_evidence");
    let file_data = reqwest::multipart::Part::bytes(request.file)
        .file_name(request.file_key)
        .mime_str(request.file_type.as_ref())
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
    multipart = multipart.part("file", file_data);
    Ok(multipart)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileUploadResponse {
    #[serde(rename = "id")]
    pub file_id: String,
}

#[derive(Debug, Serialize)]
pub struct Evidence {
    #[serde(rename = "evidence[access_activity_log]")]
    pub access_activity_log: Option<String>,
    #[serde(rename = "evidence[billing_address]")]
    pub billing_address: Option<Secret<String>>,
    #[serde(rename = "evidence[cancellation_policy]")]
    pub cancellation_policy: Option<String>,
    #[serde(rename = "evidence[cancellation_policy_disclosure]")]
    pub cancellation_policy_disclosure: Option<String>,
    #[serde(rename = "evidence[cancellation_rebuttal]")]
    pub cancellation_rebuttal: Option<String>,
    #[serde(rename = "evidence[customer_communication]")]
    pub customer_communication: Option<String>,
    #[serde(rename = "evidence[customer_email_address]")]
    pub customer_email_address: Option<Secret<String, pii::EmailStrategy>>,
    #[serde(rename = "evidence[customer_name]")]
    pub customer_name: Option<Secret<String>>,
    #[serde(rename = "evidence[customer_purchase_ip]")]
    pub customer_purchase_ip: Option<Secret<String, pii::IpAddress>>,
    #[serde(rename = "evidence[customer_signature]")]
    pub customer_signature: Option<Secret<String>>,
    #[serde(rename = "evidence[product_description]")]
    pub product_description: Option<String>,
    #[serde(rename = "evidence[receipt]")]
    pub receipt: Option<Secret<String>>,
    #[serde(rename = "evidence[refund_policy]")]
    pub refund_policy: Option<String>,
    #[serde(rename = "evidence[refund_policy_disclosure]")]
    pub refund_policy_disclosure: Option<String>,
    #[serde(rename = "evidence[refund_refusal_explanation]")]
    pub refund_refusal_explanation: Option<String>,
    #[serde(rename = "evidence[service_date]")]
    pub service_date: Option<String>,
    #[serde(rename = "evidence[service_documentation]")]
    pub service_documentation: Option<String>,
    #[serde(rename = "evidence[shipping_address]")]
    pub shipping_address: Option<Secret<String>>,
    #[serde(rename = "evidence[shipping_carrier]")]
    pub shipping_carrier: Option<String>,
    #[serde(rename = "evidence[shipping_date]")]
    pub shipping_date: Option<String>,
    #[serde(rename = "evidence[shipping_documentation]")]
    pub shipping_documentation: Option<Secret<String>>,
    #[serde(rename = "evidence[shipping_tracking_number]")]
    pub shipping_tracking_number: Option<Secret<String>>,
    #[serde(rename = "evidence[uncategorized_file]")]
    pub uncategorized_file: Option<String>,
    #[serde(rename = "evidence[uncategorized_text]")]
    pub uncategorized_text: Option<String>,
    pub submit: bool,
}

// Mandates for bank redirects - ideal and sofort happens through sepa direct debit in stripe
fn get_stripe_sepa_dd_mandate_billing_details(
    billing_details: &Option<domain::BankRedirectBilling>,
    is_customer_initiated_mandate_payment: Option<bool>,
) -> Result<StripeBillingAddress, errors::ConnectorError> {
    let billing_name = billing_details
        .clone()
        .and_then(|billing_data| billing_data.billing_name.clone());

    let billing_email = billing_details
        .clone()
        .and_then(|billing_data| billing_data.email.clone());
    match is_customer_initiated_mandate_payment {
        Some(true) => Ok(StripeBillingAddress {
            name: Some(
                billing_name.ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "billing_name",
                })?,
            ),

            email: Some(
                billing_email.ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "billing_email",
                })?,
            ),
            ..StripeBillingAddress::default()
        }),
        Some(false) | None => Ok(StripeBillingAddress {
            name: billing_name,
            email: billing_email,
            ..StripeBillingAddress::default()
        }),
    }
}

impl TryFrom<&types::SubmitEvidenceRouterData> for Evidence {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SubmitEvidenceRouterData) -> Result<Self, Self::Error> {
        let submit_evidence_request_data = item.request.clone();
        Ok(Self {
            access_activity_log: submit_evidence_request_data.access_activity_log,
            billing_address: submit_evidence_request_data
                .billing_address
                .map(Secret::new),
            cancellation_policy: submit_evidence_request_data.cancellation_policy_provider_file_id,
            cancellation_policy_disclosure: submit_evidence_request_data
                .cancellation_policy_disclosure,
            cancellation_rebuttal: submit_evidence_request_data.cancellation_rebuttal,
            customer_communication: submit_evidence_request_data
                .customer_communication_provider_file_id,
            customer_email_address: submit_evidence_request_data
                .customer_email_address
                .map(Secret::new),
            customer_name: submit_evidence_request_data.customer_name.map(Secret::new),
            customer_purchase_ip: submit_evidence_request_data
                .customer_purchase_ip
                .map(Secret::new),
            customer_signature: submit_evidence_request_data
                .customer_signature_provider_file_id
                .map(Secret::new),
            product_description: submit_evidence_request_data.product_description,
            receipt: submit_evidence_request_data
                .receipt_provider_file_id
                .map(Secret::new),
            refund_policy: submit_evidence_request_data.refund_policy_provider_file_id,
            refund_policy_disclosure: submit_evidence_request_data.refund_policy_disclosure,
            refund_refusal_explanation: submit_evidence_request_data.refund_refusal_explanation,
            service_date: submit_evidence_request_data.service_date,
            service_documentation: submit_evidence_request_data
                .service_documentation_provider_file_id,
            shipping_address: submit_evidence_request_data
                .shipping_address
                .map(Secret::new),
            shipping_carrier: submit_evidence_request_data.shipping_carrier,
            shipping_date: submit_evidence_request_data.shipping_date,
            shipping_documentation: submit_evidence_request_data
                .shipping_documentation_provider_file_id
                .map(Secret::new),
            shipping_tracking_number: submit_evidence_request_data
                .shipping_tracking_number
                .map(Secret::new),
            uncategorized_file: submit_evidence_request_data.uncategorized_file_provider_file_id,
            uncategorized_text: submit_evidence_request_data.uncategorized_text,
            submit: true,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DisputeObj {
    #[serde(rename = "id")]
    pub dispute_id: String,
    pub status: String,
}

fn get_transaction_metadata(
    merchant_metadata: Option<Secret<Value>>,
    order_id: String,
) -> HashMap<String, String> {
    let mut meta_data = HashMap::from([("metadata[order_id]".to_string(), order_id)]);
    let mut request_hash_map = HashMap::new();

    if let Some(metadata) = merchant_metadata {
        let hashmap: HashMap<String, Value> =
            serde_json::from_str(&metadata.peek().to_string()).unwrap_or(HashMap::new());

        for (key, value) in hashmap {
            request_hash_map.insert(format!("metadata[{}]", key), value.to_string());
        }

        meta_data.extend(request_hash_map)
    };
    meta_data
}

impl TryFrom<(&Option<ErrorDetails>, u16, String)> for types::PaymentsResponseData {
    type Error = types::ErrorResponse;
    fn try_from(
        (response, http_code, response_id): (&Option<ErrorDetails>, u16, String),
    ) -> Result<Self, Self::Error> {
        let (code, error_message) = match response {
            Some(error_details) => (
                error_details
                    .code
                    .to_owned()
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                error_details
                    .message
                    .to_owned()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            ),
            None => (
                consts::NO_ERROR_CODE.to_string(),
                consts::NO_ERROR_MESSAGE.to_string(),
            ),
        };

        Err(types::ErrorResponse {
            code,
            message: error_message.clone(),
            reason: response.clone().and_then(|res| {
                res.decline_code
                    .clone()
                    .map(|decline_code| {
                        format!(
                            "message - {}, decline_code - {}",
                            error_message, decline_code
                        )
                    })
                    .or(Some(error_message.clone()))
            }),
            status_code: http_code,
            attempt_status: None,
            connector_transaction_id: Some(response_id),
        })
    }
}

#[cfg(test)]
mod test_validate_shipping_address_against_payment_method {
    #![allow(clippy::unwrap_used)]
    use api_models::enums::CountryAlpha2;
    use masking::Secret;

    use crate::{
        connector::stripe::transformers::{
            validate_shipping_address_against_payment_method, StripePaymentMethodType,
            StripeShippingAddress,
        },
        core::errors,
    };

    #[test]
    fn should_return_ok() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            "name".to_string(),
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &Some(stripe_shipping_address),
            Some(payment_method),
        );

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_return_err_for_empty_line1() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            "name".to_string(),
            None,
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &Some(stripe_shipping_address),
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(*missing_fields.first().unwrap(), "shipping.address.line1");
    }

    #[test]
    fn should_return_err_for_empty_country() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            "name".to_string(),
            Some("line1".to_string()),
            None,
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &Some(stripe_shipping_address),
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(*missing_fields.first().unwrap(), "shipping.address.country");
    }

    #[test]
    fn should_return_err_for_empty_zip() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            "name".to_string(),
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            None,
        );
        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &Some(stripe_shipping_address),
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(*missing_fields.first().unwrap(), "shipping.address.zip");
    }

    #[test]
    fn should_return_error_when_missing_multiple_fields() {
        // Arrange
        let expected_missing_field_names: Vec<&'static str> =
            vec!["shipping.address.zip", "shipping.address.country"];
        let stripe_shipping_address = create_stripe_shipping_address(
            "name".to_string(),
            Some("line1".to_string()),
            None,
            None,
        );
        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &Some(stripe_shipping_address),
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        for field in missing_fields {
            assert!(expected_missing_field_names.contains(&field));
        }
    }

    fn get_missing_fields(connector_error: &errors::ConnectorError) -> Vec<&'static str> {
        if let errors::ConnectorError::MissingRequiredFields { field_names } = connector_error {
            return field_names.to_vec();
        }

        vec![]
    }

    fn create_stripe_shipping_address(
        name: String,
        line1: Option<String>,
        country: Option<CountryAlpha2>,
        zip: Option<String>,
    ) -> StripeShippingAddress {
        StripeShippingAddress {
            name: Secret::new(name),
            line1: line1.map(Secret::new),
            country,
            zip: zip.map(Secret::new),
            city: Some(String::from("city")),
            line2: Some(Secret::new(String::from("line2"))),
            state: Some(Secret::new(String::from("state"))),
            phone: Some(Secret::new(String::from("pbone number"))),
        }
    }
}
