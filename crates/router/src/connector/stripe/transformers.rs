use std::ops::Deref;

use api_models::{self, enums as api_enums, payments};
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    pii::{self, Email},
};
use data_models::mandates::AcceptanceType;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, ExposeOptionInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    collect_missing_value_keys,
    connector::utils::{self as connector_util, ApplePay, PaymentsPreProcessingData, RouterData},
    core::errors,
    services,
    types::{
        self, api,
        storage::enums,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::{self, OptionExt},
};

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
pub struct PaymentIntentRequest {
    pub amount: i64, //amount in cents, hence passed as integer
    pub currency: String,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    #[serde(flatten)]
    pub meta_data: StripeMetadata,
    pub return_url: String,
    pub confirm: bool,
    pub mandate: Option<Secret<String>>,
    pub payment_method: Option<String>,
    pub customer: Option<Secret<String>>,
    #[serde(flatten)]
    pub setup_mandate_details: Option<StripeMandateRequest>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub shipping: StripeShippingAddress,
    #[serde(flatten)]
    pub billing: StripeBillingAddress,
    #[serde(flatten)]
    pub payment_data: Option<StripePaymentMethodData>,
    pub capture_method: StripeCaptureMethod,
    pub payment_method_options: Option<StripePaymentMethodOptions>, // For mandate txns using network_txns_id, needs to be validated
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    #[serde(rename = "payment_method_types[0]")]
    pub payment_method_types: Option<StripePaymentMethodType>,
}

// Field rename is required only in case of serialization as it is passed in the request to the connector.
// Deserialization is happening only in case of webhooks, where fields name should be used as defined in the struct.
// Whenever adding new fields, Please ensure it doesn't break the webhook flow
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StripeMetadata {
    // merchant_reference_id
    #[serde(rename(serialize = "metadata[order_id]"))]
    pub order_id: String,
    // to check whether the order_id is refund_id or payemnt_id
    // before deployment, order id is set to payemnt_id in refunds but now it is set as refund_id
    // it is set as string instead of bool because stripe pass it as string even if we set it as bool
    #[serde(rename(serialize = "metadata[is_refund_id_as_reference]"))]
    pub is_refund_id_as_reference: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SetupIntentRequest {
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
    pub confirm: bool,
    pub usage: Option<enums::FutureUsage>,
    pub customer: Option<Secret<String>>,
    pub off_session: Option<bool>,
    pub return_url: Option<String>,
    #[serde(flatten)]
    pub payment_data: StripePaymentMethodData,
    pub payment_method_options: Option<StripePaymentMethodOptions>, // For mandate txns using network_txns_id, needs to be validated
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
    pub payment_method_data_card_cvc: Secret<String>,
    #[serde(rename = "payment_method_options[card][request_three_d_secure]")]
    pub payment_method_auth_type: Auth3ds,
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

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct StripeTokenResponse {
    pub id: String,
    pub object: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CustomerRequest {
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<Secret<String>>,
    pub source: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
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
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct ChargesResponse {
    pub id: String,
    pub amount: u64,
    pub amount_captured: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub source: StripeSourceResponse,
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
    preferred_language: String,
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
    pub code: String,
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
    eci: Option<Secret<String>>,
    #[serde(rename = "card[tokenization_method]")]
    tokenization_method: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeApplePay {
    pub pk_token: Secret<String>,
    pub pk_token_instrument_name: String,
    pub pk_token_payment_network: String,
    pub pk_token_transaction_id: String,
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
    pub token: String,
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
            enums::PaymentMethodType::Boleto
            | enums::PaymentMethodType::CryptoCurrency
            | enums::PaymentMethodType::GooglePay
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
            | enums::PaymentMethodType::Walley => Err(errors::ConnectorError::NotSupported {
                message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "stripe",
            }
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

impl TryFrom<&api_models::enums::BankNames> for StripeBankNames {
    type Error = errors::ConnectorError;
    fn try_from(bank: &api_models::enums::BankNames) -> Result<Self, Self::Error> {
        Ok(match bank {
            api_models::enums::BankNames::AbnAmro => Self::AbnAmro,
            api_models::enums::BankNames::ArzteUndApothekerBank => Self::ArzteUndApothekerBank,
            api_models::enums::BankNames::AsnBank => Self::AsnBank,
            api_models::enums::BankNames::AustrianAnadiBankAg => Self::AustrianAnadiBankAg,
            api_models::enums::BankNames::BankAustria => Self::BankAustria,
            api_models::enums::BankNames::BankhausCarlSpangler => Self::BankhausCarlSpangler,
            api_models::enums::BankNames::BankhausSchelhammerUndSchatteraAg => {
                Self::BankhausSchelhammerUndSchatteraAg
            }
            api_models::enums::BankNames::BawagPskAg => Self::BawagPskAg,
            api_models::enums::BankNames::BksBankAg => Self::BksBankAg,
            api_models::enums::BankNames::BrullKallmusBankAg => Self::BrullKallmusBankAg,
            api_models::enums::BankNames::BtvVierLanderBank => Self::BtvVierLanderBank,
            api_models::enums::BankNames::Bunq => Self::Bunq,
            api_models::enums::BankNames::CapitalBankGraweGruppeAg => {
                Self::CapitalBankGraweGruppeAg
            }
            api_models::enums::BankNames::Citi => Self::CitiHandlowy,
            api_models::enums::BankNames::Dolomitenbank => Self::Dolomitenbank,
            api_models::enums::BankNames::EasybankAg => Self::EasybankAg,
            api_models::enums::BankNames::ErsteBankUndSparkassen => Self::ErsteBankUndSparkassen,
            api_models::enums::BankNames::Handelsbanken => Self::Handelsbanken,
            api_models::enums::BankNames::HypoAlpeadriabankInternationalAg => {
                Self::HypoAlpeadriabankInternationalAg
            }

            api_models::enums::BankNames::HypoNoeLbFurNiederosterreichUWien => {
                Self::HypoNoeLbFurNiederosterreichUWien
            }
            api_models::enums::BankNames::HypoOberosterreichSalzburgSteiermark => {
                Self::HypoOberosterreichSalzburgSteiermark
            }
            api_models::enums::BankNames::HypoTirolBankAg => Self::HypoTirolBankAg,
            api_models::enums::BankNames::HypoVorarlbergBankAg => Self::HypoVorarlbergBankAg,
            api_models::enums::BankNames::HypoBankBurgenlandAktiengesellschaft => {
                Self::HypoBankBurgenlandAktiengesellschaft
            }
            api_models::enums::BankNames::Ing => Self::Ing,
            api_models::enums::BankNames::Knab => Self::Knab,
            api_models::enums::BankNames::MarchfelderBank => Self::MarchfelderBank,
            api_models::enums::BankNames::OberbankAg => Self::OberbankAg,
            api_models::enums::BankNames::RaiffeisenBankengruppeOsterreich => {
                Self::RaiffeisenBankengruppeOsterreich
            }
            api_models::enums::BankNames::Rabobank => Self::Rabobank,
            api_models::enums::BankNames::Regiobank => Self::Regiobank,
            api_models::enums::BankNames::Revolut => Self::Revolut,
            api_models::enums::BankNames::SnsBank => Self::SnsBank,
            api_models::enums::BankNames::TriodosBank => Self::TriodosBank,
            api_models::enums::BankNames::VanLanschot => Self::VanLanschot,
            api_models::enums::BankNames::Moneyou => Self::Moneyou,
            api_models::enums::BankNames::SchoellerbankAg => Self::SchoellerbankAg,
            api_models::enums::BankNames::SpardaBankWien => Self::SpardaBankWien,
            api_models::enums::BankNames::VolksbankGruppe => Self::VolksbankGruppe,
            api_models::enums::BankNames::VolkskreditbankAg => Self::VolkskreditbankAg,
            api_models::enums::BankNames::VrBankBraunau => Self::VrBankBraunau,
            api_models::enums::BankNames::PlusBank => Self::PlusBank,
            api_models::enums::BankNames::EtransferPocztowy24 => Self::EtransferPocztowy24,
            api_models::enums::BankNames::BankiSpbdzielcze => Self::BankiSpbdzielcze,
            api_models::enums::BankNames::BankNowyBfgSa => Self::BankNowyBfgSa,
            api_models::enums::BankNames::GetinBank => Self::GetinBank,
            api_models::enums::BankNames::Blik => Self::Blik,
            api_models::enums::BankNames::NoblePay => Self::NoblePay,
            api_models::enums::BankNames::IdeaBank => Self::IdeaBank,
            api_models::enums::BankNames::EnveloBank => Self::EnveloBank,
            api_models::enums::BankNames::NestPrzelew => Self::NestPrzelew,
            api_models::enums::BankNames::MbankMtransfer => Self::MbankMtransfer,
            api_models::enums::BankNames::Inteligo => Self::Inteligo,
            api_models::enums::BankNames::PbacZIpko => Self::PbacZIpko,
            api_models::enums::BankNames::BnpParibas => Self::BnpParibas,
            api_models::enums::BankNames::BankPekaoSa => Self::BankPekaoSa,
            api_models::enums::BankNames::VolkswagenBank => Self::VolkswagenBank,
            api_models::enums::BankNames::AliorBank => Self::AliorBank,
            api_models::enums::BankNames::Boz => Self::Boz,

            _ => Err(errors::ConnectorError::NotSupported {
                message: api_enums::PaymentMethod::BankRedirect.to_string(),
                connector: "Stripe",
            })?,
        })
    }
}

fn validate_shipping_address_against_payment_method(
    shipping_address: &StripeShippingAddress,
    payment_method: Option<&StripePaymentMethodType>,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    if let Some(StripePaymentMethodType::AfterpayClearpay) = payment_method {
        let missing_fields = collect_missing_value_keys!(
            ("shipping.address.first_name", shipping_address.name),
            ("shipping.address.line1", shipping_address.line1),
            ("shipping.address.country", shipping_address.country),
            ("shipping.address.zip", shipping_address.zip)
        );

        if !missing_fields.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredFields {
                field_names: missing_fields,
            })
            .into_report();
        }
    }

    Ok(())
}

impl TryFrom<&api_models::payments::PayLaterData> for StripePaymentMethodType {
    type Error = errors::ConnectorError;
    fn try_from(pay_later_data: &api_models::payments::PayLaterData) -> Result<Self, Self::Error> {
        match pay_later_data {
            api_models::payments::PayLaterData::KlarnaRedirect { .. } => Ok(Self::Klarna),
            api_models::payments::PayLaterData::AffirmRedirect {} => Ok(Self::Affirm),
            api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                Ok(Self::AfterpayClearpay)
            }

            payments::PayLaterData::KlarnaSdk { .. }
            | payments::PayLaterData::PayBrightRedirect {}
            | payments::PayLaterData::WalleyRedirect {}
            | payments::PayLaterData::AlmaRedirect {}
            | payments::PayLaterData::AtomeRedirect {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "stripe",
                })
            }
        }
    }
}

impl TryFrom<&payments::BankRedirectData> for StripePaymentMethodType {
    type Error = errors::ConnectorError;
    fn try_from(bank_redirect_data: &payments::BankRedirectData) -> Result<Self, Self::Error> {
        match bank_redirect_data {
            payments::BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            payments::BankRedirectData::Ideal { .. } => Ok(Self::Ideal),
            payments::BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            payments::BankRedirectData::BancontactCard { .. } => Ok(Self::Bancontact),
            payments::BankRedirectData::Przelewy24 { .. } => Ok(Self::Przelewy24),
            payments::BankRedirectData::Eps { .. } => Ok(Self::Eps),
            payments::BankRedirectData::Blik { .. } => Ok(Self::Blik),
            payments::BankRedirectData::OnlineBankingFpx { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
            payments::BankRedirectData::Bizum {}
            | payments::BankRedirectData::Interac { .. }
            | payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | payments::BankRedirectData::OnlineBankingFinland { .. }
            | payments::BankRedirectData::OnlineBankingPoland { .. }
            | payments::BankRedirectData::OnlineBankingSlovakia { .. }
            | payments::BankRedirectData::OnlineBankingThailand { .. }
            | payments::BankRedirectData::OpenBankingUk { .. }
            | payments::BankRedirectData::Trustly { .. } => {
                Err(errors::ConnectorError::NotSupported {
                    message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "stripe",
                })
            }
        }
    }
}

impl ForeignTryFrom<&payments::WalletData> for Option<StripePaymentMethodType> {
    type Error = errors::ConnectorError;
    fn foreign_try_from(wallet_data: &payments::WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            payments::WalletData::AliPayRedirect(_) => Ok(Some(StripePaymentMethodType::Alipay)),
            payments::WalletData::ApplePay(_) => Ok(None),
            payments::WalletData::GooglePay(_) => Ok(Some(StripePaymentMethodType::Card)),
            payments::WalletData::WeChatPayQr(_) => Ok(Some(StripePaymentMethodType::Wechatpay)),
            payments::WalletData::CashappQr(_) => Ok(Some(StripePaymentMethodType::Cashapp)),
            payments::WalletData::MobilePayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                ))
            }
            payments::WalletData::PaypalRedirect(_)
            | payments::WalletData::AliPayQr(_)
            | payments::WalletData::AliPayHkRedirect(_)
            | payments::WalletData::MomoRedirect(_)
            | payments::WalletData::KakaoPayRedirect(_)
            | payments::WalletData::GoPayRedirect(_)
            | payments::WalletData::GcashRedirect(_)
            | payments::WalletData::ApplePayRedirect(_)
            | payments::WalletData::ApplePayThirdPartySdk(_)
            | payments::WalletData::DanaRedirect {}
            | payments::WalletData::GooglePayRedirect(_)
            | payments::WalletData::GooglePayThirdPartySdk(_)
            | payments::WalletData::MbWayRedirect(_)
            | payments::WalletData::PaypalSdk(_)
            | payments::WalletData::SamsungPay(_)
            | payments::WalletData::TwintRedirect {}
            | payments::WalletData::VippsRedirect {}
            | payments::WalletData::TouchNGoRedirect(_)
            | payments::WalletData::SwishQr(_)
            | payments::WalletData::WeChatPayRedirect(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "stripe",
                })
            }
        }
    }
}

impl From<&payments::BankDebitData> for StripePaymentMethodType {
    fn from(bank_debit_data: &payments::BankDebitData) -> Self {
        match bank_debit_data {
            payments::BankDebitData::AchBankDebit { .. } => Self::Ach,
            payments::BankDebitData::SepaBankDebit { .. } => Self::Sepa,
            payments::BankDebitData::BecsBankDebit { .. } => Self::Becs,
            payments::BankDebitData::BacsBankDebit { .. } => Self::Bacs,
        }
    }
}

impl TryFrom<(&api_models::payments::PayLaterData, StripePaymentMethodType)>
    for StripeBillingAddress
{
    type Error = errors::ConnectorError;

    fn try_from(
        (pay_later_data, pm_type): (&api_models::payments::PayLaterData, StripePaymentMethodType),
    ) -> Result<Self, Self::Error> {
        match (pay_later_data, pm_type) {
            (
                payments::PayLaterData::KlarnaRedirect {
                    billing_email,
                    billing_country,
                },
                StripePaymentMethodType::Klarna,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                country: Some(billing_country.to_owned()),
                ..Self::default()
            }),
            (payments::PayLaterData::AffirmRedirect {}, StripePaymentMethodType::Affirm) => {
                Ok(Self::default())
            }
            (
                payments::PayLaterData::AfterpayClearpayRedirect {
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

impl From<&payments::BankDebitBilling> for StripeBillingAddress {
    fn from(item: &payments::BankDebitBilling) -> Self {
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
        }
    }
}

impl TryFrom<&payments::BankRedirectData> for StripeBillingAddress {
    type Error = errors::ConnectorError;

    fn try_from(bank_redirection_data: &payments::BankRedirectData) -> Result<Self, Self::Error> {
        match bank_redirection_data {
            payments::BankRedirectData::Eps {
                billing_details, ..
            } => Ok(Self {
                name: billing_details.billing_name.clone(),
                ..Self::default()
            }),
            payments::BankRedirectData::Giropay {
                billing_details, ..
            } => Ok(Self {
                name: billing_details.billing_name.clone(),
                ..Self::default()
            }),
            payments::BankRedirectData::Ideal {
                billing_details, ..
            } => Ok(Self {
                name: billing_details.billing_name.clone(),
                email: billing_details.email.clone(),
                ..Self::default()
            }),
            payments::BankRedirectData::Przelewy24 {
                billing_details, ..
            } => Ok(Self {
                email: billing_details.email.clone(),
                ..Self::default()
            }),
            payments::BankRedirectData::BancontactCard {
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
            payments::BankRedirectData::Sofort {
                billing_details, ..
            } => Ok(Self {
                name: billing_details.billing_name.clone(),
                email: billing_details.email.clone(),
                ..Self::default()
            }),
            payments::BankRedirectData::Bizum {}
            | payments::BankRedirectData::Blik { .. }
            | payments::BankRedirectData::Interac { .. }
            | payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | payments::BankRedirectData::OnlineBankingFinland { .. }
            | payments::BankRedirectData::OnlineBankingPoland { .. }
            | payments::BankRedirectData::OnlineBankingSlovakia { .. }
            | payments::BankRedirectData::Trustly { .. }
            | payments::BankRedirectData::OnlineBankingFpx { .. }
            | payments::BankRedirectData::OnlineBankingThailand { .. }
            | payments::BankRedirectData::OpenBankingUk { .. } => Ok(Self::default()),
        }
    }
}

fn get_bank_debit_data(
    bank_debit_data: &payments::BankDebitData,
) -> (StripePaymentMethodType, BankDebitData, StripeBillingAddress) {
    match bank_debit_data {
        payments::BankDebitData::AchBankDebit {
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
        payments::BankDebitData::SepaBankDebit {
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
        payments::BankDebitData::BecsBankDebit {
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
        payments::BankDebitData::BacsBankDebit {
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
    payment_method_data: &api_models::payments::PaymentMethodData,
    auth_type: enums::AuthenticationType,
    payment_method_token: Option<types::PaymentMethodToken>,
) -> Result<
    (
        StripePaymentMethodData,
        Option<StripePaymentMethodType>,
        StripeBillingAddress,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    match payment_method_data {
        payments::PaymentMethodData::Card(card_details) => {
            let payment_method_auth_type = match auth_type {
                enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
            };
            Ok((
                StripePaymentMethodData::try_from((card_details, payment_method_auth_type))?,
                Some(StripePaymentMethodType::Card),
                StripeBillingAddress::default(),
            ))
        }
        payments::PaymentMethodData::PayLater(pay_later_data) => {
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
        payments::PaymentMethodData::BankRedirect(bank_redirect_data) => {
            let billing_address = StripeBillingAddress::try_from(bank_redirect_data)?;
            let pm_type = StripePaymentMethodType::try_from(bank_redirect_data)?;
            let bank_redirect_data = StripePaymentMethodData::try_from(bank_redirect_data)?;

            Ok((bank_redirect_data, Some(pm_type), billing_address))
        }
        payments::PaymentMethodData::Wallet(wallet_data) => {
            let pm_type = ForeignTryFrom::foreign_try_from(wallet_data)?;
            let wallet_specific_data =
                StripePaymentMethodData::try_from((wallet_data, payment_method_token))?;
            Ok((
                wallet_specific_data,
                pm_type,
                StripeBillingAddress::default(),
            ))
        }
        payments::PaymentMethodData::BankDebit(bank_debit_data) => {
            let (pm_type, bank_debit_data, billing_address) = get_bank_debit_data(bank_debit_data);

            let pm_data = StripePaymentMethodData::BankDebit(StripeBankDebitData {
                bank_specific_data: bank_debit_data,
            });

            Ok((pm_data, Some(pm_type), billing_address))
        }
        payments::PaymentMethodData::BankTransfer(bank_transfer_data) => {
            match bank_transfer_data.deref() {
                payments::BankTransferData::AchBankTransfer { billing_details } => Ok((
                    StripePaymentMethodData::BankTransfer(StripeBankTransferData::AchBankTransfer(
                        Box::new(AchTransferData {
                            email: billing_details.email.to_owned(),
                        }),
                    )),
                    None,
                    StripeBillingAddress::default(),
                )),
                payments::BankTransferData::MultibancoBankTransfer { billing_details } => Ok((
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
                payments::BankTransferData::SepaBankTransfer {
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
                payments::BankTransferData::BacsBankTransfer { billing_details } => {
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
                payments::BankTransferData::Pix {} => Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into()),
                payments::BankTransferData::Pse {}
                | payments::BankTransferData::PermataBankTransfer { .. }
                | payments::BankTransferData::BcaBankTransfer { .. }
                | payments::BankTransferData::BniVaBankTransfer { .. }
                | payments::BankTransferData::BriVaBankTransfer { .. }
                | payments::BankTransferData::CimbVaBankTransfer { .. }
                | payments::BankTransferData::DanamonVaBankTransfer { .. }
                | payments::BankTransferData::MandiriVaBankTransfer { .. } => {
                    Err(errors::ConnectorError::NotSupported {
                        message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                        connector: "stripe",
                    }
                    .into())
                }
            }
        }
        payments::PaymentMethodData::Crypto(_) => Err(errors::ConnectorError::NotImplemented(
            connector_util::get_unimplemented_payment_method_error_message("stripe"),
        )
        .into()),

        payments::PaymentMethodData::GiftCard(giftcard_data) => match giftcard_data.deref() {
            payments::GiftCardData::Givex(_) | payments::GiftCardData::PaySafeCard {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "stripe",
                }
                .into())
            }
        },

        payments::PaymentMethodData::CardRedirect(cardredirect_data) => match cardredirect_data {
            payments::CardRedirectData::Knet {}
            | payments::CardRedirectData::Benefit {}
            | payments::CardRedirectData::MomoAtm {} => Err(errors::ConnectorError::NotSupported {
                message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "stripe",
            }
            .into()),
        },
        payments::PaymentMethodData::Reward => Err(errors::ConnectorError::NotImplemented(
            connector_util::get_unimplemented_payment_method_error_message("stripe"),
        )
        .into()),

        payments::PaymentMethodData::Voucher(voucher_data) => match voucher_data {
            payments::VoucherData::Boleto(_) | payments::VoucherData::Oxxo => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
            payments::VoucherData::Alfamart(_)
            | payments::VoucherData::Efecty
            | payments::VoucherData::PagoEfectivo
            | payments::VoucherData::RedCompra
            | payments::VoucherData::RedPagos
            | payments::VoucherData::Indomaret(_)
            | payments::VoucherData::SevenEleven(_)
            | payments::VoucherData::Lawson(_)
            | payments::VoucherData::MiniStop(_)
            | payments::VoucherData::FamilyMart(_)
            | payments::VoucherData::Seicomart(_)
            | payments::VoucherData::PayEasy(_) => Err(errors::ConnectorError::NotSupported {
                message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "stripe",
            }
            .into()),
        },

        payments::PaymentMethodData::Upi(_) | payments::PaymentMethodData::MandatePayment => {
            Err(errors::ConnectorError::NotSupported {
                message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "stripe",
            }
            .into())
        }
    }
}

impl TryFrom<(&payments::Card, Auth3ds)> for StripePaymentMethodData {
    type Error = errors::ConnectorError;
    fn try_from(
        (card, payment_method_auth_type): (&payments::Card, Auth3ds),
    ) -> Result<Self, Self::Error> {
        Ok(Self::Card(StripeCardData {
            payment_method_data_type: StripePaymentMethodType::Card,
            payment_method_data_card_number: card.card_number.clone(),
            payment_method_data_card_exp_month: card.card_exp_month.clone(),
            payment_method_data_card_exp_year: card.card_exp_year.clone(),
            payment_method_data_card_cvc: card.card_cvc.clone(),
            payment_method_auth_type,
        }))
    }
}

impl TryFrom<(&payments::WalletData, Option<types::PaymentMethodToken>)>
    for StripePaymentMethodData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (wallet_data, payment_method_token): (
            &payments::WalletData,
            Option<types::PaymentMethodToken>,
        ),
    ) -> Result<Self, Self::Error> {
        match wallet_data {
            payments::WalletData::ApplePay(applepay_data) => {
                let mut apple_pay_decrypt_data =
                    if let Some(types::PaymentMethodToken::ApplePayDecrypt(decrypt_data)) =
                        payment_method_token
                    {
                        let expiry_year_4_digit = Secret::new(format!(
                            "20{}",
                            decrypt_data
                                .clone()
                                .application_expiration_date
                                .peek()
                                .get(0..2)
                                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                        ));
                        let exp_month = Secret::new(
                            decrypt_data
                                .clone()
                                .application_expiration_date
                                .peek()
                                .get(2..4)
                                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                                .to_owned(),
                        );

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
                            pk_token: applepay_data
                                .get_applepay_decoded_payment_data()
                                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                            pk_token_instrument_name: applepay_data
                                .payment_method
                                .pm_type
                                .to_owned(),
                            pk_token_payment_network: applepay_data
                                .payment_method
                                .network
                                .to_owned(),
                            pk_token_transaction_id: applepay_data
                                .transaction_identifier
                                .to_owned(),
                        })));
                };
                let pmd = apple_pay_decrypt_data
                    .ok_or(errors::ConnectorError::MissingApplePayTokenData)?;
                Ok(pmd)
            }
            payments::WalletData::WeChatPayQr(_) => Ok(Self::Wallet(
                StripeWallet::WechatpayPayment(WechatpayPayment {
                    client: WechatClient::Web,
                    payment_method_data_type: StripePaymentMethodType::Wechatpay,
                }),
            )),
            payments::WalletData::AliPayRedirect(_) => {
                Ok(Self::Wallet(StripeWallet::AlipayPayment(AlipayPayment {
                    payment_method_data_type: StripePaymentMethodType::Alipay,
                })))
            }
            payments::WalletData::CashappQr(_) => {
                Ok(Self::Wallet(StripeWallet::Cashapp(CashappPayment {
                    payment_method_data_type: StripePaymentMethodType::Cashapp,
                })))
            }
            payments::WalletData::GooglePay(gpay_data) => Ok(Self::try_from(gpay_data)?),
            payments::WalletData::PaypalRedirect(_)
            | payments::WalletData::MobilePayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
            payments::WalletData::AliPayQr(_)
            | payments::WalletData::AliPayHkRedirect(_)
            | payments::WalletData::MomoRedirect(_)
            | payments::WalletData::KakaoPayRedirect(_)
            | payments::WalletData::GoPayRedirect(_)
            | payments::WalletData::GcashRedirect(_)
            | payments::WalletData::ApplePayRedirect(_)
            | payments::WalletData::ApplePayThirdPartySdk(_)
            | payments::WalletData::DanaRedirect {}
            | payments::WalletData::GooglePayRedirect(_)
            | payments::WalletData::GooglePayThirdPartySdk(_)
            | payments::WalletData::MbWayRedirect(_)
            | payments::WalletData::PaypalSdk(_)
            | payments::WalletData::SamsungPay(_)
            | payments::WalletData::TwintRedirect {}
            | payments::WalletData::VippsRedirect {}
            | payments::WalletData::TouchNGoRedirect(_)
            | payments::WalletData::SwishQr(_)
            | payments::WalletData::WeChatPayRedirect(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "stripe",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&payments::BankRedirectData> for StripePaymentMethodData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank_redirect_data: &payments::BankRedirectData) -> Result<Self, Self::Error> {
        let payment_method_data_type = StripePaymentMethodType::try_from(bank_redirect_data)?;
        match bank_redirect_data {
            payments::BankRedirectData::BancontactCard { .. } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeBancontactCard(Box::new(StripeBancontactCard {
                    payment_method_data_type,
                })),
            )),
            payments::BankRedirectData::Blik { blik_code } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeBlik(Box::new(StripeBlik {
                    payment_method_data_type,
                    code: blik_code.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "blik_code",
                        },
                    )?,
                })),
            )),
            payments::BankRedirectData::Eps { bank_name, .. } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeEps(Box::new(StripeEps {
                    payment_method_data_type,
                    bank_name: bank_name
                        .map(|bank_name| StripeBankNames::try_from(&bank_name))
                        .transpose()?,
                })),
            )),
            payments::BankRedirectData::Giropay { .. } => Ok(Self::BankRedirect(
                StripeBankRedirectData::StripeGiropay(Box::new(StripeGiropay {
                    payment_method_data_type,
                })),
            )),
            payments::BankRedirectData::Ideal { bank_name, .. } => {
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
            payments::BankRedirectData::Przelewy24 { bank_name, .. } => {
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
            payments::BankRedirectData::Sofort {
                country,
                preferred_language,
                ..
            } => Ok(Self::BankRedirect(StripeBankRedirectData::StripeSofort(
                Box::new(StripeSofort {
                    payment_method_data_type,
                    country: country.to_owned(),
                    preferred_language: preferred_language.to_owned(),
                }),
            ))),
            payments::BankRedirectData::OnlineBankingFpx { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_util::get_unimplemented_payment_method_error_message("stripe"),
                )
                .into())
            }
            payments::BankRedirectData::Bizum {}
            | payments::BankRedirectData::Interac { .. }
            | payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | payments::BankRedirectData::OnlineBankingFinland { .. }
            | payments::BankRedirectData::OnlineBankingPoland { .. }
            | payments::BankRedirectData::OnlineBankingSlovakia { .. }
            | payments::BankRedirectData::OnlineBankingThailand { .. }
            | payments::BankRedirectData::OpenBankingUk { .. }
            | payments::BankRedirectData::Trustly { .. } => {
                Err(errors::ConnectorError::NotSupported {
                    message: connector_util::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "stripe",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&payments::GooglePayWalletData> for StripePaymentMethodData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(gpay_data: &payments::GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self::Wallet(StripeWallet::GooglepayToken(GooglePayToken {
            token: Secret::new(
                gpay_data
                    .tokenization_data
                    .token
                    .as_bytes()
                    .parse_struct::<StripeGpayToken>("StripeGpayToken")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?
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

        let shipping_address = match item.address.shipping.clone() {
            Some(mut shipping) => StripeShippingAddress {
                city: shipping.address.as_mut().and_then(|a| a.city.take()),
                country: shipping.address.as_mut().and_then(|a| a.country.take()),
                line1: shipping
                    .address
                    .as_mut()
                    .and_then(|a: &mut payments::AddressDetails| a.line1.take()),
                line2: shipping.address.as_mut().and_then(|a| a.line2.take()),
                zip: shipping.address.as_mut().and_then(|a| a.zip.take()),
                state: shipping.address.as_mut().and_then(|a| a.state.take()),
                name: shipping.address.as_mut().and_then(|a| {
                    a.first_name.as_ref().map(|first_name| {
                        format!(
                            "{} {}",
                            first_name.clone().expose(),
                            a.last_name.clone().expose_option().unwrap_or_default()
                        )
                        .into()
                    })
                }),
                phone: shipping.phone.map(|p| {
                    format!(
                        "{}{}",
                        p.country_code.unwrap_or_default(),
                        p.number.expose_option().unwrap_or_default()
                    )
                    .into()
                }),
            },
            None => StripeShippingAddress::default(),
        };
        let mut payment_method_options = None;

        let (mut payment_data, payment_method, mandate, billing_address, payment_method_types) = {
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
                    connector_mandate_ids.payment_method_id,
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
                    (None, None, None, StripeBillingAddress::default(), None)
                }
                _ => {
                    let (payment_method_data, payment_method_type, billing_address) =
                        create_stripe_payment_method(
                            &item.request.payment_method_data,
                            item.auth_type,
                            item.payment_method_token.clone(),
                        )?;

                    validate_shipping_address_against_payment_method(
                        &shipping_address,
                        payment_method_type.as_ref(),
                    )?;

                    (
                        Some(payment_method_data),
                        None,
                        None,
                        billing_address,
                        payment_method_type,
                    )
                }
            }
        };

        payment_data = match item.request.payment_method_data {
            payments::PaymentMethodData::Wallet(payments::WalletData::ApplePay(_)) => {
                let payment_method_token = item
                    .payment_method_token
                    .to_owned()
                    .get_required_value("payment_token")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                let payment_method_token = match payment_method_token {
                    types::PaymentMethodToken::Token(payment_method_token) => payment_method_token,
                    types::PaymentMethodToken::ApplePayDecrypt(_) => {
                        Err(errors::ConnectorError::InvalidWalletToken)?
                    }
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
        Ok(Self {
            amount: item.request.amount, //hopefully we don't loose some cents here
            currency: item.request.currency.to_string(), //we need to copy the value and not transfer ownership
            statement_descriptor_suffix: item.request.statement_descriptor_suffix.clone(),
            statement_descriptor: item.request.statement_descriptor.clone(),
            meta_data: StripeMetadata {
                order_id,
                is_refund_id_as_reference: None,
            },
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
            mandate: mandate.map(Secret::new),
            payment_method_options,
            payment_method,
            customer: item.connector_customer.to_owned().map(Secret::new),
            setup_mandate_details,
            off_session: item.request.off_session,
            setup_future_usage: item.request.setup_future_usage,
            payment_method_types,
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

impl TryFrom<&types::SetupMandateRouterData> for SetupIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        let metadata_order_id = item.connector_request_reference_id.clone();
        let metadata_txn_id = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
        let metadata_txn_uuid = Uuid::new_v4().to_string();

        //Only cards supported for mandates
        let pm_type = StripePaymentMethodType::Card;
        let payment_data = StripePaymentMethodData::try_from((
            item.request.payment_method_data.clone(),
            item.auth_type,
            pm_type,
        ))?;

        Ok(Self {
            confirm: true,
            metadata_order_id,
            metadata_txn_id,
            metadata_txn_uuid,
            payment_data,
            return_url: item.request.router_return_url.clone(),
            off_session: item.request.off_session,
            usage: item.request.setup_future_usage,
            payment_method_options: None,
            customer: item.connector_customer.to_owned().map(Secret::new),
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
            name: item.request.name.to_owned().map(Secret::new),
            source: item.request.preprocessing_id.to_owned(),
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

#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
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
    pub customer: Option<String>,
    pub payment_method: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub last_payment_error: Option<ErrorDetails>,
    pub latest_attempt: Option<LatestAttempt>, //need a merchant to test this
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

#[derive(Serialize, Deserialize, Debug)]
pub struct LastPaymentError {
    code: String,
    message: String,
    decline_code: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PaymentIntentSyncResponse {
    #[serde(flatten)]
    payment_intent_fields: PaymentIntentResponse,
    pub last_payment_error: Option<LastPaymentError>,
    pub latest_charge: Option<StripeChargeEnum>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum StripeChargeEnum {
    ChargeId(String),
    ChargeObject(StripeCharge),
}

#[derive(Deserialize, Clone, Debug)]
pub struct StripeCharge {
    pub id: String,
    pub payment_method_details: Option<StripePaymentMethodDetailsResponse>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StripeBankRedirectDetails {
    #[serde(rename = "generated_sepa_debit")]
    attached_payment_method: Option<String>,
}

impl Deref for PaymentIntentSyncResponse {
    type Target = PaymentIntentResponse;

    fn deref(&self) -> &Self::Target {
        &self.payment_intent_fields
    }
}

#[derive(Deserialize, Clone, Debug)]
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
    Card,
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

#[derive(Deserialize)]
pub struct SetupIntentSyncResponse {
    #[serde(flatten)]
    setup_intent_fields: SetupIntentResponse,
    pub last_payment_error: Option<LastPaymentError>,
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
            last_payment_error: value.last_payment_error,
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
            last_payment_error: None,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct SetupIntentResponse {
    pub id: String,
    pub object: String,
    pub status: StripePaymentStatus, // Change to SetupStatus
    pub client_secret: Secret<String>,
    pub customer: Option<String>,
    pub payment_method: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
    pub latest_attempt: Option<LatestAttempt>,
}

impl ForeignFrom<(Option<StripePaymentMethodOptions>, String)> for types::MandateReference {
    fn foreign_from(
        (payment_method_options, payment_method_id): (Option<StripePaymentMethodOptions>, String),
    ) -> Self {
        Self {
            connector_mandate_id: payment_method_options.and_then(|options| match options {
                StripePaymentMethodOptions::Card {
                    mandate_options, ..
                } => mandate_options.map(|mandate_options| mandate_options.reference),
                StripePaymentMethodOptions::Klarna {}
                | StripePaymentMethodOptions::Affirm {}
                | StripePaymentMethodOptions::AfterpayClearpay {}
                | StripePaymentMethodOptions::Eps {}
                | StripePaymentMethodOptions::Giropay {}
                | StripePaymentMethodOptions::Ideal {}
                | StripePaymentMethodOptions::Sofort {}
                | StripePaymentMethodOptions::Ach {}
                | StripePaymentMethodOptions::Bacs {}
                | StripePaymentMethodOptions::Becs {}
                | StripePaymentMethodOptions::WechatPay {}
                | StripePaymentMethodOptions::Alipay {}
                | StripePaymentMethodOptions::Sepa {}
                | StripePaymentMethodOptions::Bancontact {}
                | StripePaymentMethodOptions::Przelewy24 {}
                | StripePaymentMethodOptions::CustomerBalance {}
                | StripePaymentMethodOptions::Blik {}
                | StripePaymentMethodOptions::Multibanco {}
                | StripePaymentMethodOptions::Cashapp {} => None,
            }),
            payment_method_id: Some(payment_method_id),
        }
    }
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

        let mandate_reference = item.response.payment_method.map(|pm| {
            types::MandateReference::foreign_from((item.response.payment_method_options, pm))
        });

        //Note: we might have to call retrieve_setup_intent to get the network_transaction_id in case its not sent in PaymentIntentResponse
        // Or we identify the mandate txns before hand and always call SetupIntent in case of mandate payment call
        let network_txn_id = Option::foreign_from(item.response.latest_attempt);

        let connector_metadata =
            get_connector_metadata(item.response.next_action.as_ref(), item.response.amount)?;

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            // client_secret: Some(item.response.client_secret.clone().as_str()),
            // description: item.response.description.map(|x| x.as_str()),
            // statement_descriptor_suffix: item.response.statement_descriptor_suffix.map(|x| x.as_str()),
            // three_ds_form,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata,
                network_txn_id,
                connector_response_reference_id: Some(item.response.id),
            }),
            amount_captured: item.response.amount_received,
            ..item.data
        })
    }
}

pub fn get_connector_metadata(
    next_action: Option<&StripeNextActionResponse>,
    amount: i64,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let next_action_response = next_action
        .and_then(|next_action_response| match next_action_response {
            StripeNextActionResponse::DisplayBankTransferInstructions(response) => {
                let bank_instructions = response.financial_addresses.get(0);
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

                Some(common_utils::ext_traits::Encode::<
                    SepaAndBacsBankTransferInstructions,
                >::encode_to_value(
                    &bank_transfer_instructions
                ))
            }
            StripeNextActionResponse::WechatPayDisplayQrCode(response) => {
                let wechat_pay_instructions = QrCodeNextInstructions {
                    image_data_url: response.image_data_url.to_owned(),
                    display_to_timestamp: None,
                };

                Some(
                    common_utils::ext_traits::Encode::<QrCodeNextInstructions>::encode_to_value(
                        &wechat_pay_instructions,
                    ),
                )
            }
            StripeNextActionResponse::CashappHandleRedirectOrDisplayQrCode(response) => {
                let cashapp_qr_instructions: QrCodeNextInstructions = QrCodeNextInstructions {
                    image_data_url: response.qr_code.image_url_png.to_owned(),
                    display_to_timestamp: response.qr_code.expires_at.to_owned(),
                };
                Some(
                    common_utils::ext_traits::Encode::<QrCodeNextInstructions>::encode_to_value(
                        &cashapp_qr_instructions,
                    ),
                )
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

        let mandate_reference = item.response.payment_method.clone().map(|pm| {
            types::MandateReference::foreign_from((
                item.response.payment_method_options.clone(),
                match item.response.latest_charge.clone() {
                    Some(StripeChargeEnum::ChargeObject(charge)) => {
                        match charge.payment_method_details {
                            Some(StripePaymentMethodDetailsResponse::Bancontact { bancontact }) => {
                                bancontact.attached_payment_method.unwrap_or(pm)
                            }
                            Some(StripePaymentMethodDetailsResponse::Ideal { ideal }) => {
                                ideal.attached_payment_method.unwrap_or(pm)
                            }
                            Some(StripePaymentMethodDetailsResponse::Sofort { sofort }) => {
                                sofort.attached_payment_method.unwrap_or(pm)
                            }
                            Some(StripePaymentMethodDetailsResponse::Blik)
                            | Some(StripePaymentMethodDetailsResponse::Eps)
                            | Some(StripePaymentMethodDetailsResponse::Fpx)
                            | Some(StripePaymentMethodDetailsResponse::Giropay)
                            | Some(StripePaymentMethodDetailsResponse::Przelewy24)
                            | Some(StripePaymentMethodDetailsResponse::Card)
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
                            | None => pm,
                        }
                    }
                    Some(StripeChargeEnum::ChargeId(_)) | None => pm,
                },
            ))
        });
        let error_res =
            item.response
                .last_payment_error
                .as_ref()
                .map(|error| types::ErrorResponse {
                    code: error.code.to_owned(),
                    message: error.code.to_owned(),
                    reason: error
                        .decline_code
                        .clone()
                        .map(|decline_code| {
                            format!(
                                "message - {}, decline_code - {}",
                                error.message, decline_code
                            )
                        })
                        .or(Some(error.message.clone())),
                    status_code: item.http_code,
                });

        let connector_metadata =
            get_connector_metadata(item.response.next_action.as_ref(), item.response.amount)?;

        let response = error_res.map_or(
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id.clone()),
            }),
            Err,
        );

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.to_owned()),
            response,
            amount_captured: item.response.amount_received,
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

        let mandate_reference = item.response.payment_method.map(|pm| {
            types::MandateReference::foreign_from((item.response.payment_method_options, pm))
        });

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
                network_txn_id: Option::foreign_from(item.response.latest_attempt),
                connector_response_reference_id: Some(item.response.id),
            }),
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct StripeVerifyWithMicroDepositsResponse {
    hosted_verification_url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeBankTransferDetails {
    pub amount_remaining: i64,
    pub currency: String,
    pub financial_addresses: Vec<StripeFinanicalInformation>,
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
pub struct StripeFinanicalInformation {
    pub iban: Option<SepaFinancialDetails>,
    pub sort_code: Option<BacsFinancialDetails>,
    pub supported_networks: Vec<String>,
    #[serde(rename = "type")]
    pub financial_info_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SepaFinancialDetails {
    pub account_holder_name: String,
    pub bic: String,
    pub country: String,
    pub iban: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BacsFinancialDetails {
    pub account_holder_name: String,
    pub account_number: String,
    pub sort_code: String,
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
                order_id: item.request.refund_id.clone(),
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
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
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
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
    pub decline_code: Option<String>,
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
    pub name: Option<Secret<String>>,
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
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct StripeRedirectResponse {
    pub payment_intent: Option<String>,
    pub payment_intent_client_secret: Option<String>,
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
pub enum StripePaymentMethodOptions {
    Card {
        mandate_options: Option<StripeMandateOptions>,
        network_transaction_id: Option<Secret<String>>,
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
    pub network_transaction_id: Secret<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum LatestAttempt {
    PaymentIntentAttempt(LatestPaymentAttempt),
    SetupAttempt(String),
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct LatestPaymentAttempt {
    pub payment_method_options: Option<StripePaymentMethodOptions>,
}
// #[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
// pub struct Card
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct StripeMandateOptions {
    reference: String, // Extendable, But only important field to be captured
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
            Some(payments::PaymentMethodData::BankTransfer(bank_transfer_data)) => {
                match **bank_transfer_data {
                    payments::BankTransferData::MultibancoBankTransfer { .. } => Ok(
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
                    payments::BankTransferData::AchBankTransfer { .. } => {
                        Ok(Self::AchBankTansfer(AchCreditTransferSourceRequest {
                            transfer_type: StripeCreditTransferTypes::AchCreditTransfer,
                            payment_method_data: AchTransferData {
                                email: item.request.get_email()?,
                            },
                            currency,
                        }))
                    }
                    payments::BankTransferData::SepaBankTransfer { .. }
                    | payments::BankTransferData::BacsBankTransfer { .. }
                    | payments::BankTransferData::PermataBankTransfer { .. }
                    | payments::BankTransferData::BcaBankTransfer { .. }
                    | payments::BankTransferData::BniVaBankTransfer { .. }
                    | payments::BankTransferData::BriVaBankTransfer { .. }
                    | payments::BankTransferData::CimbVaBankTransfer { .. }
                    | payments::BankTransferData::DanamonVaBankTransfer { .. }
                    | payments::BankTransferData::MandiriVaBankTransfer { .. }
                    | payments::BankTransferData::Pix { .. }
                    | payments::BankTransferData::Pse { .. } => {
                        Err(errors::ConnectorError::NotImplemented(
                            connector_util::get_unimplemented_payment_method_error_message(
                                "stripe",
                            ),
                        )
                        .into())
                    }
                }
            }
            Some(payments::PaymentMethodData::Card(..))
            | Some(payments::PaymentMethodData::Wallet(..))
            | Some(payments::PaymentMethodData::BankDebit(..))
            | Some(payments::PaymentMethodData::BankRedirect(..))
            | Some(payments::PaymentMethodData::PayLater(..))
            | Some(payments::PaymentMethodData::Crypto(..))
            | Some(payments::PaymentMethodData::Reward)
            | Some(payments::PaymentMethodData::MandatePayment)
            | Some(payments::PaymentMethodData::Upi(..))
            | Some(payments::PaymentMethodData::GiftCard(..))
            | Some(payments::PaymentMethodData::CardRedirect(..))
            | Some(payments::PaymentMethodData::Voucher(..))
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
        let connector_metadata =
            common_utils::ext_traits::Encode::<StripeSourceResponse>::encode_to_value(
                &connector_source_response,
            )
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
        Ok(Self {
            amount: value.request.amount.to_string(),
            currency: value.request.currency.to_string(),
            customer: Secret::new(value.get_connector_customer_id()?),
            source: Secret::new(value.get_preprocessing_id()?),
        })
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
        let connector_metadata =
            common_utils::ext_traits::Encode::<StripeSourceResponse>::encode_to_value(
                &connector_source_response.source,
            )
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(connector_metadata),
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
            }),
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
                token: item.response.id,
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
    pub object: serde_json::Value,
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

#[derive(Debug, Deserialize)]
pub struct WebhookEventObjectData {
    pub id: String,
    pub object: WebhookEventObjectType,
    pub amount: Option<i32>,
    pub currency: String,
    pub payment_intent: Option<String>,
    pub reason: Option<String>,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub created: PrimitiveDateTime,
    pub evidence_details: Option<EvidenceDetails>,
    pub status: Option<WebhookEventStatus>,
    pub metadata: Option<StripeMetadata>,
}

#[derive(Debug, Deserialize, strum::Display)]
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
    #[serde(rename = "amount_capturable_updated")]
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

#[derive(Debug, Serialize, strum::Display, Deserialize, PartialEq)]
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

#[derive(Debug, Deserialize, PartialEq)]
pub struct EvidenceDetails {
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub due_by: PrimitiveDateTime,
}

impl
    TryFrom<(
        api::PaymentMethodData,
        enums::AuthenticationType,
        StripePaymentMethodType,
    )> for StripePaymentMethodData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (pm_data, auth_type, pm_type): (
            api::PaymentMethodData,
            enums::AuthenticationType,
            StripePaymentMethodType,
        ),
    ) -> Result<Self, Self::Error> {
        match pm_data {
            api::PaymentMethodData::Card(ref ccard) => {
                let payment_method_auth_type = match auth_type {
                    enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                    enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
                };
                Ok(Self::try_from((ccard, payment_method_auth_type))?)
            }
            api::PaymentMethodData::PayLater(_) => Ok(Self::PayLater(StripePayLaterData {
                payment_method_data_type: pm_type,
            })),
            api::PaymentMethodData::BankRedirect(ref bank_redirect_data) => {
                Ok(Self::try_from(bank_redirect_data)?)
            }
            payments::PaymentMethodData::Wallet(ref wallet_data) => {
                Ok(Self::try_from((wallet_data, None))?)
            }
            api::PaymentMethodData::BankDebit(bank_debit_data) => {
                let (_pm_type, bank_data, _) = get_bank_debit_data(&bank_debit_data);

                Ok(Self::BankDebit(StripeBankDebitData {
                    bank_specific_data: bank_data,
                }))
            }
            api::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                match bank_transfer_data.deref() {
                    payments::BankTransferData::AchBankTransfer { billing_details } => {
                        Ok(Self::BankTransfer(StripeBankTransferData::AchBankTransfer(
                            Box::new(AchTransferData {
                                email: billing_details.email.to_owned(),
                            }),
                        )))
                    }
                    payments::BankTransferData::MultibancoBankTransfer { billing_details } => Ok(
                        Self::BankTransfer(StripeBankTransferData::MultibancoBankTransfers(
                            Box::new(MultibancoTransferData {
                                email: billing_details.email.to_owned(),
                            }),
                        )),
                    ),
                    payments::BankTransferData::SepaBankTransfer { country, .. } => Ok(
                        Self::BankTransfer(StripeBankTransferData::SepaBankTransfer(Box::new(
                            SepaBankTransferData {
                                payment_method_data_type: StripePaymentMethodType::CustomerBalance,
                                bank_transfer_type: BankTransferType::EuBankTransfer,
                                balance_funding_type: BankTransferType::BankTransfers,
                                payment_method_type: StripePaymentMethodType::CustomerBalance,
                                country: country.to_owned(),
                            },
                        ))),
                    ),
                    payments::BankTransferData::BacsBankTransfer { .. } => Ok(Self::BankTransfer(
                        StripeBankTransferData::BacsBankTransfers(Box::new(BacsBankTransferData {
                            payment_method_data_type: StripePaymentMethodType::CustomerBalance,
                            bank_transfer_type: BankTransferType::GbBankTransfer,
                            balance_funding_type: BankTransferType::BankTransfers,
                            payment_method_type: StripePaymentMethodType::CustomerBalance,
                        })),
                    )),
                    payments::BankTransferData::Pix {}
                    | payments::BankTransferData::Pse {}
                    | payments::BankTransferData::PermataBankTransfer { .. }
                    | payments::BankTransferData::BcaBankTransfer { .. }
                    | payments::BankTransferData::BniVaBankTransfer { .. }
                    | payments::BankTransferData::BriVaBankTransfer { .. }
                    | payments::BankTransferData::CimbVaBankTransfer { .. }
                    | payments::BankTransferData::DanamonVaBankTransfer { .. }
                    | payments::BankTransferData::MandiriVaBankTransfer { .. } => {
                        Err(errors::ConnectorError::NotImplemented(
                            connector_util::get_unimplemented_payment_method_error_message(
                                "stripe",
                            ),
                        )
                        .into())
                    }
                }
            }
            api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Voucher(_) => Err(errors::ConnectorError::NotSupported {
                message: format!("{pm_type:?}"),
                connector: "Stripe",
            })?,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeGpayToken {
    pub id: String,
}

pub fn get_bank_transfer_request_data(
    req: &types::PaymentsAuthorizeRouterData,
    bank_transfer_data: &api_models::payments::BankTransferData,
) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
    match bank_transfer_data {
        api_models::payments::BankTransferData::AchBankTransfer { .. }
        | api_models::payments::BankTransferData::MultibancoBankTransfer { .. } => {
            let req = ChargesRequest::try_from(req)?;
            let request = types::RequestBody::log_and_get_request_body(
                &req,
                utils::Encode::<ChargesRequest>::url_encode,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(request))
        }
        _ => {
            let req = PaymentIntentRequest::try_from(req)?;
            let request = types::RequestBody::log_and_get_request_body(
                &req,
                utils::Encode::<PaymentIntentRequest>::url_encode,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            Ok(Some(request))
        }
    }
}

pub fn get_bank_transfer_authorize_response(
    data: &types::PaymentsAuthorizeRouterData,
    res: types::Response,
    bank_transfer_data: &api_models::payments::BankTransferData,
) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
    match bank_transfer_data {
        api_models::payments::BankTransferData::AchBankTransfer { .. }
        | api_models::payments::BankTransferData::MultibancoBankTransfer { .. } => {
            let response: ChargesResponse = res
                .response
                .parse_struct("ChargesResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
        _ => {
            let response: PaymentIntentResponse = res
                .response
                .parse_struct("PaymentIntentResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
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

#[derive(Debug, Deserialize)]
pub struct FileUploadResponse {
    #[serde(rename = "id")]
    pub file_id: String,
}

#[derive(Debug, Serialize)]
pub struct Evidence {
    #[serde(rename = "evidence[access_activity_log]")]
    pub access_activity_log: Option<String>,
    #[serde(rename = "evidence[billing_address]")]
    pub billing_address: Option<String>,
    #[serde(rename = "evidence[cancellation_policy]")]
    pub cancellation_policy: Option<String>,
    #[serde(rename = "evidence[cancellation_policy_disclosure]")]
    pub cancellation_policy_disclosure: Option<String>,
    #[serde(rename = "evidence[cancellation_rebuttal]")]
    pub cancellation_rebuttal: Option<String>,
    #[serde(rename = "evidence[customer_communication]")]
    pub customer_communication: Option<String>,
    #[serde(rename = "evidence[customer_email_address]")]
    pub customer_email_address: Option<String>,
    #[serde(rename = "evidence[customer_name]")]
    pub customer_name: Option<String>,
    #[serde(rename = "evidence[customer_purchase_ip]")]
    pub customer_purchase_ip: Option<String>,
    #[serde(rename = "evidence[customer_signature]")]
    pub customer_signature: Option<String>,
    #[serde(rename = "evidence[product_description]")]
    pub product_description: Option<String>,
    #[serde(rename = "evidence[receipt]")]
    pub receipt: Option<String>,
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
    pub shipping_address: Option<String>,
    #[serde(rename = "evidence[shipping_carrier]")]
    pub shipping_carrier: Option<String>,
    #[serde(rename = "evidence[shipping_date]")]
    pub shipping_date: Option<String>,
    #[serde(rename = "evidence[shipping_documentation]")]
    pub shipping_documentation: Option<String>,
    #[serde(rename = "evidence[shipping_tracking_number]")]
    pub shipping_tracking_number: Option<String>,
    #[serde(rename = "evidence[uncategorized_file]")]
    pub uncategorized_file: Option<String>,
    #[serde(rename = "evidence[uncategorized_text]")]
    pub uncategorized_text: Option<String>,
    pub submit: bool,
}

impl TryFrom<&types::SubmitEvidenceRouterData> for Evidence {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SubmitEvidenceRouterData) -> Result<Self, Self::Error> {
        let submit_evidence_request_data = item.request.clone();
        Ok(Self {
            access_activity_log: submit_evidence_request_data.access_activity_log,
            billing_address: submit_evidence_request_data.billing_address,
            cancellation_policy: submit_evidence_request_data.cancellation_policy_provider_file_id,
            cancellation_policy_disclosure: submit_evidence_request_data
                .cancellation_policy_disclosure,
            cancellation_rebuttal: submit_evidence_request_data.cancellation_rebuttal,
            customer_communication: submit_evidence_request_data
                .customer_communication_provider_file_id,
            customer_email_address: submit_evidence_request_data.customer_email_address,
            customer_name: submit_evidence_request_data.customer_name,
            customer_purchase_ip: submit_evidence_request_data.customer_purchase_ip,
            customer_signature: submit_evidence_request_data.customer_signature_provider_file_id,
            product_description: submit_evidence_request_data.product_description,
            receipt: submit_evidence_request_data.receipt_provider_file_id,
            refund_policy: submit_evidence_request_data.refund_policy_provider_file_id,
            refund_policy_disclosure: submit_evidence_request_data.refund_policy_disclosure,
            refund_refusal_explanation: submit_evidence_request_data.refund_refusal_explanation,
            service_date: submit_evidence_request_data.service_date,
            service_documentation: submit_evidence_request_data
                .service_documentation_provider_file_id,
            shipping_address: submit_evidence_request_data.shipping_address,
            shipping_carrier: submit_evidence_request_data.shipping_carrier,
            shipping_date: submit_evidence_request_data.shipping_date,
            shipping_documentation: submit_evidence_request_data
                .shipping_documentation_provider_file_id,
            shipping_tracking_number: submit_evidence_request_data.shipping_tracking_number,
            uncategorized_file: submit_evidence_request_data.uncategorized_file_provider_file_id,
            uncategorized_text: submit_evidence_request_data.uncategorized_text,
            submit: true,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct DisputeObj {
    #[serde(rename = "id")]
    pub dispute_id: String,
    pub status: String,
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
            Some("name".to_string()),
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            Some(payment_method),
        );

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_return_err_for_empty_name() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            None,
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.first_name");
    }

    #[test]
    fn should_return_err_for_empty_line1() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            None,
            Some(CountryAlpha2::AD),
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.line1");
    }

    #[test]
    fn should_return_err_for_empty_country() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            None,
            Some("zip".to_string()),
        );

        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.country");
    }

    #[test]
    fn should_return_err_for_empty_zip() {
        // Arrange
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            Some(CountryAlpha2::AD),
            None,
        );
        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
            Some(payment_method),
        );

        // Assert
        assert!(result.is_err());
        let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
        assert_eq!(missing_fields.len(), 1);
        assert_eq!(missing_fields[0], "shipping.address.zip");
    }

    #[test]
    fn should_return_error_when_missing_multiple_fields() {
        // Arrange
        let expected_missing_field_names: Vec<&'static str> =
            vec!["shipping.address.zip", "shipping.address.country"];
        let stripe_shipping_address = create_stripe_shipping_address(
            Some("name".to_string()),
            Some("line1".to_string()),
            None,
            None,
        );
        let payment_method = &StripePaymentMethodType::AfterpayClearpay;

        //Act
        let result = validate_shipping_address_against_payment_method(
            &stripe_shipping_address,
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
        name: Option<String>,
        line1: Option<String>,
        country: Option<CountryAlpha2>,
        zip: Option<String>,
    ) -> StripeShippingAddress {
        StripeShippingAddress {
            name: name.map(Secret::new),
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
