#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use std::str::FromStr;

use api_models::subscription as api;
use common_enums::{connector_enums, enums};
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    id_type::{CustomerId, InvoiceId, SubscriptionId},
    pii::{self, Email},
    types::MinorUnit,
};
use error_stack::ResultExt;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{subscriptions::SubscriptionAutoCollection, ResponseId},
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            self, GetSubscriptionEstimateResponse, GetSubscriptionItemPricesResponse,
            GetSubscriptionItemsResponse, SubscriptionCancelResponse, SubscriptionCreateResponse,
            SubscriptionInvoiceData, SubscriptionLineItem, SubscriptionPauseResponse,
            SubscriptionResumeResponse, SubscriptionStatus,
        },
        ConnectorCustomerResponseData, PaymentsResponseData, RefundsResponseData,
    },
    types::{
        GetSubscriptionEstimateRouterData, InvoiceRecordBackRouterData,
        PaymentsAuthorizeRouterData, RefundsRouterData, SubscriptionCancelRouterData,
        SubscriptionPauseRouterData, SubscriptionResumeRouterData,
    },
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    convert_connector_response_to_domain_response,
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData, RouterData as OtherRouterData},
};

// SubscriptionCreate structures
#[derive(Debug, Serialize)]
pub struct ChargebeeSubscriptionCreateRequest {
    #[serde(rename = "id")]
    pub subscription_id: SubscriptionId,
    #[serde(rename = "subscription_items[item_price_id][0]")]
    pub item_price_id: String,
    #[serde(rename = "subscription_items[quantity][0]")]
    pub quantity: Option<u32>,
    #[serde(rename = "billing_address[line1]")]
    pub billing_address_line1: Option<Secret<String>>,
    #[serde(rename = "billing_address[city]")]
    pub billing_address_city: Option<String>,
    #[serde(rename = "billing_address[state]")]
    pub billing_address_state: Option<Secret<String>>,
    #[serde(rename = "billing_address[zip]")]
    pub billing_address_zip: Option<Secret<String>>,
    #[serde(rename = "billing_address[country]")]
    pub billing_address_country: Option<common_enums::CountryAlpha2>,
    pub auto_collection: ChargebeeAutoCollection,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeAutoCollection {
    On,
    Off,
}

impl From<SubscriptionAutoCollection> for ChargebeeAutoCollection {
    fn from(auto_collection: SubscriptionAutoCollection) -> Self {
        match auto_collection {
            SubscriptionAutoCollection::On => Self::On,
            SubscriptionAutoCollection::Off => Self::Off,
        }
    }
}

impl TryFrom<&ChargebeeRouterData<&hyperswitch_domain_models::types::SubscriptionCreateRouterData>>
    for ChargebeeSubscriptionCreateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ChargebeeRouterData<&hyperswitch_domain_models::types::SubscriptionCreateRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = &item.router_data.request;

        let first_item =
            req.subscription_items
                .first()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "subscription_items",
                })?;

        Ok(Self {
            subscription_id: req.subscription_id.clone(),
            item_price_id: first_item.item_price_id.clone(),
            quantity: first_item.quantity,
            billing_address_line1: item.router_data.get_optional_billing_line1(),
            billing_address_city: item.router_data.get_optional_billing_city(),
            billing_address_state: item.router_data.get_optional_billing_state(),
            billing_address_zip: item.router_data.get_optional_billing_zip(),
            billing_address_country: item.router_data.get_optional_billing_country(),
            auto_collection: req.auto_collection.clone().into(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeSubscriptionCreateResponse {
    pub subscription: ChargebeeSubscriptionDetails,
    pub invoice: Option<ChargebeeInvoiceData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeSubscriptionDetails {
    pub id: SubscriptionId,
    pub status: ChargebeeSubscriptionStatus,
    pub customer_id: CustomerId,
    pub currency_code: enums::Currency,
    pub total_dues: Option<MinorUnit>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub next_billing_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub pause_date: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    cancelled_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeSubscriptionStatus {
    Future,
    #[serde(rename = "in_trial")]
    InTrial,
    Active,
    #[serde(rename = "non_renewing")]
    NonRenewing,
    Paused,
    Cancelled,
    Transferred,
}

impl From<ChargebeeSubscriptionStatus> for SubscriptionStatus {
    fn from(status: ChargebeeSubscriptionStatus) -> Self {
        match status {
            ChargebeeSubscriptionStatus::Future => Self::Pending,
            ChargebeeSubscriptionStatus::InTrial => Self::Trial,
            ChargebeeSubscriptionStatus::Active => Self::Active,
            ChargebeeSubscriptionStatus::NonRenewing => Self::Onetime,
            ChargebeeSubscriptionStatus::Paused => Self::Paused,
            ChargebeeSubscriptionStatus::Cancelled => Self::Cancelled,
            ChargebeeSubscriptionStatus::Transferred => Self::Cancelled,
        }
    }
}

convert_connector_response_to_domain_response!(
    ChargebeeSubscriptionCreateResponse,
    SubscriptionCreateResponse,
    |item: ResponseRouterData<_, ChargebeeSubscriptionCreateResponse, _, _>| {
        let subscription = &item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionCreateResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                customer_id: subscription.customer_id.clone(),
                currency_code: subscription.currency_code,
                total_amount: subscription.total_dues.unwrap_or(MinorUnit::new(0)),
                next_billing_at: subscription.next_billing_at,
                created_at: subscription.created_at,
                invoice_details: item.response.invoice.map(SubscriptionInvoiceData::from),
            }),
            ..item.data
        })
    }
);

//TODO: Fill the struct with respective fields
pub struct ChargebeeRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for ChargebeeRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ChargebeePaymentsRequest {
    amount: MinorUnit,
    card: ChargebeeCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ChargebeeCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&ChargebeeRouterData<&PaymentsAuthorizeRouterData>> for ChargebeePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ChargebeeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = ChargebeeCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct ChargebeeAuthType {
    pub(super) full_access_key_v1: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChargebeeMetadata {
    pub(super) site: Secret<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for ChargebeeMetadata {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

impl TryFrom<&ConnectorAuthType> for ChargebeeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                full_access_key_v1: api_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChargebeePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ChargebeePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: ChargebeePaymentStatus) -> Self {
        match item {
            ChargebeePaymentStatus::Succeeded => Self::Charged,
            ChargebeePaymentStatus::Failed => Self::Failure,
            ChargebeePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChargebeePaymentsResponse {
    status: ChargebeePaymentStatus,
    id: String,
}

convert_connector_response_to_domain_response!(
    ChargebeePaymentsResponse,
    PaymentsResponseData,
    |item: ResponseRouterData<_, ChargebeePaymentsResponse, _, _>| {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
);

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ChargebeeRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&ChargebeeRouterData<&RefundsRouterData<F>>> for ChargebeeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ChargebeeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChargebeeErrorResponse {
    pub api_error_code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeWebhookBody {
    pub content: ChargebeeWebhookContent,
    pub event_type: ChargebeeEventType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeInvoiceBody {
    pub content: ChargebeeInvoiceContent,
    pub event_type: ChargebeeEventType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeInvoiceContent {
    pub invoice: ChargebeeInvoiceData,
    pub subscription: Option<ChargebeeSubscriptionData>,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct ChargebeeWebhookContent {
    pub transaction: ChargebeeTransactionData,
    pub invoice: ChargebeeInvoiceData,
    pub customer: Option<ChargebeeCustomer>,
    pub subscription: Option<ChargebeeSubscriptionData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeSubscriptionData {
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub current_term_start: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub next_billing_at: Option<PrimitiveDateTime>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeEventType {
    PaymentSucceeded,
    PaymentFailed,
    InvoiceDeleted,
    InvoiceGenerated,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChargebeeInvoiceData {
    // invoice id
    pub id: InvoiceId,
    pub total: MinorUnit,
    pub currency_code: enums::Currency,
    pub status: Option<ChargebeeInvoiceStatus>,
    pub billing_address: Option<ChargebeeInvoiceBillingAddress>,
    pub linked_payments: Option<Vec<ChargebeeInvoicePayments>>,
    pub customer_id: CustomerId,
    pub subscription_id: SubscriptionId,
    pub first_invoice: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChargebeeInvoicePayments {
    pub txn_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeTransactionData {
    // Chargebee's own transaction id (`txn_…`). Optional so that a payload without it keeps
    // deserializing as before rather than being rejected outright.
    id: Option<String>,
    id_at_gateway: Option<String>,
    status: ChargebeeTranasactionStatus,
    error_code: Option<String>,
    error_text: Option<String>,
    gateway_account_id: String,
    currency_code: enums::Currency,
    amount: MinorUnit,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    date: Option<PrimitiveDateTime>,
    payment_method: ChargebeeTransactionPaymentMethod,
    // Documented by Chargebee as optional, and its contents are a gateway and payment method
    // specific blob rather than a fixed schema, so it is kept as a raw string and parsed
    // leniently into `ChargebeePaymentMethodDetails`.
    payment_method_details: Option<String>,
}

// Chargebee's `transaction.payment_method` is not card-only. This models the Chargebee payment
// methods that have a Hyperswitch equivalent; the remainder Chargebee documents (generic,
// electronic_payment_standard, kbc_payment_button, naver_pay, grab_pay, pay_co, payme, paypay,
// paynow, tamara, qpay) have no `PaymentMethodType` to map onto, so they fall through to the
// `Other` catch-all along with anything Chargebee adds later. `Other` still deserializes, and is
// then rejected with an explicit unsupported error rather than a body decoding failure.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeTransactionPaymentMethod {
    Card,
    #[serde(rename = "unionpay")]
    UnionPay,
    SouthKoreanCards,
    PaypalExpressCheckout,
    AmazonPayments,
    ApplePay,
    GooglePay,
    #[serde(rename = "wechat_pay")]
    WeChatPay,
    #[serde(rename = "alipay")]
    AliPay,
    #[serde(rename = "alipay_hk")]
    AliPayHk,
    Venmo,
    KakaoPay,
    RevolutPay,
    CashAppPay,
    Twint,
    GoPay,
    Gcash,
    Dana,
    TouchNGo,
    Swish,
    Ideal,
    Sofort,
    Bancontact,
    PayconiqByBancontact,
    Giropay,
    Dotpay,
    OnlineBankingPoland,
    Trustly,
    Bizum,
    NetbankingEmandates,
    PayByBank,
    Upi,
    DirectDebit,
    PayTo,
    FasterPayments,
    SepaInstantTransfer,
    AutomatedBankTransfer,
    Pix,
    Promptpay,
    Klarna,
    KlarnaPayNow,
    AfterPay,
    Stablecoin,
    #[serde(other)]
    Other,
}

impl ChargebeeTransactionPaymentMethod {
    /// Sub type to fall back on when the transaction carries no card details.
    fn payment_method_sub_type(self) -> Option<enums::PaymentMethodType> {
        match self {
            Self::PaypalExpressCheckout => Some(enums::PaymentMethodType::Paypal),
            Self::AmazonPayments => Some(enums::PaymentMethodType::AmazonPay),
            Self::ApplePay => Some(enums::PaymentMethodType::ApplePay),
            Self::GooglePay => Some(enums::PaymentMethodType::GooglePay),
            Self::WeChatPay => Some(enums::PaymentMethodType::WeChatPay),
            Self::AliPay => Some(enums::PaymentMethodType::AliPay),
            Self::AliPayHk => Some(enums::PaymentMethodType::AliPayHk),
            Self::Venmo => Some(enums::PaymentMethodType::Venmo),
            Self::KakaoPay => Some(enums::PaymentMethodType::KakaoPay),
            Self::RevolutPay => Some(enums::PaymentMethodType::RevolutPay),
            Self::CashAppPay => Some(enums::PaymentMethodType::Cashapp),
            Self::Twint => Some(enums::PaymentMethodType::Twint),
            Self::GoPay => Some(enums::PaymentMethodType::GoPay),
            Self::Gcash => Some(enums::PaymentMethodType::Gcash),
            Self::Dana => Some(enums::PaymentMethodType::Dana),
            Self::TouchNGo => Some(enums::PaymentMethodType::TouchNGo),
            Self::Swish => Some(enums::PaymentMethodType::Swish),
            Self::Ideal => Some(enums::PaymentMethodType::Ideal),
            Self::Sofort => Some(enums::PaymentMethodType::Sofort),
            // Payconiq is offered through Bancontact, which is the closest sub type available.
            Self::Bancontact | Self::PayconiqByBancontact => {
                Some(enums::PaymentMethodType::BancontactCard)
            }
            Self::Giropay => Some(enums::PaymentMethodType::Giropay),
            Self::Dotpay | Self::OnlineBankingPoland => {
                Some(enums::PaymentMethodType::OnlineBankingPoland)
            }
            Self::Trustly => Some(enums::PaymentMethodType::Trustly),
            Self::Bizum => Some(enums::PaymentMethodType::Bizum),
            Self::NetbankingEmandates => Some(enums::PaymentMethodType::LocalBankRedirect),
            Self::PayByBank => Some(enums::PaymentMethodType::OpenBanking),
            Self::Upi => Some(enums::PaymentMethodType::UpiCollect),
            // Chargebee does not say which scheme a `direct_debit` mandate belongs to (ACH, Bacs,
            // SEPA and BECS all report as `direct_debit`), so this takes the most common one.
            Self::DirectDebit => Some(enums::PaymentMethodType::Sepa),
            Self::PayTo => Some(enums::PaymentMethodType::Becs),
            Self::FasterPayments => Some(enums::PaymentMethodType::Bacs),
            Self::SepaInstantTransfer => Some(enums::PaymentMethodType::InstantBankTransfer),
            Self::AutomatedBankTransfer => Some(enums::PaymentMethodType::LocalBankTransfer),
            Self::Pix => Some(enums::PaymentMethodType::Pix),
            Self::Promptpay => Some(enums::PaymentMethodType::PromptPay),
            Self::Klarna | Self::KlarnaPayNow => Some(enums::PaymentMethodType::Klarna),
            Self::AfterPay => Some(enums::PaymentMethodType::AfterpayClearpay),
            Self::Stablecoin => Some(enums::PaymentMethodType::CryptoCurrency),
            // These arrive with card details, so the card funding type determines the sub type.
            Self::Card | Self::UnionPay | Self::SouthKoreanCards => None,
            Self::Other => None,
        }
    }

    /// Chargebee keys `payment_method_details` by the payment method, so the payload shape
    /// follows `transaction.payment_method`. Parsing dispatches on that rather than probing the
    /// blob for a `card` key, which keeps a malformed card payload reporting as a decoding
    /// failure instead of being silently mistaken for a non-card transaction.
    fn parse_payment_method_details(
        self,
        raw_details: &str,
    ) -> Result<ChargebeePaymentMethodDetails, error_stack::Report<errors::ConnectorError>> {
        match self {
            Self::Card | Self::UnionPay | Self::SouthKoreanCards => {
                let details: ChargebeeCardPaymentMethodDetails = serde_json::from_str(raw_details)
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
                Ok(ChargebeePaymentMethodDetails::Card(details.card))
            }
            // Non-card methods carry gateway specific fields that revenue recovery does not
            // consume; their sub type comes from the payment method itself. Add a typed variant
            // here once a real payload for the method is available.
            _ => Ok(ChargebeePaymentMethodDetails::NonCard),
        }
    }
}

/// Parsed form of `transaction.payment_method_details`, selected by the transaction's
/// `payment_method`.
#[derive(Debug)]
pub enum ChargebeePaymentMethodDetails {
    Card(ChargebeeCardDetails),
    NonCard,
}

#[derive(Deserialize, Debug)]
struct ChargebeeCardPaymentMethodDetails {
    card: ChargebeeCardDetails,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeCardDetails {
    funding_type: ChargebeeFundingType,
    brand: ChargebeeCardBrand,
    iin: String,
}

// Chargebee sends card brand values in lowercase snake_case (e.g. `visa`, `mastercard`,
// `american_express`), which don't match `common_enums::CardNetwork`'s serde representation.
// We deserialize into this connector-local enum, which mirrors the networks defined in
// `CardNetwork`, and map it over via `From`.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeCardBrand {
    Visa,
    Mastercard,
    AmericanExpress,
    Jcb,
    DinersClub,
    Discover,
    CartesBancaires,
    UnionPay,
    Interac,
    #[serde(rename = "rupay")]
    RuPay,
    Maestro,
    Star,
    Pulse,
    Accel,
    Nyce,
    // Chargebee also reports `other` and `not_applicable`, and neither maps onto a `CardNetwork`.
    #[serde(other)]
    Other,
}

impl From<ChargebeeCardBrand> for Option<common_enums::CardNetwork> {
    fn from(brand: ChargebeeCardBrand) -> Self {
        use common_enums::CardNetwork;
        Some(match brand {
            ChargebeeCardBrand::Visa => CardNetwork::Visa,
            ChargebeeCardBrand::Mastercard => CardNetwork::Mastercard,
            ChargebeeCardBrand::AmericanExpress => CardNetwork::AmericanExpress,
            ChargebeeCardBrand::Jcb => CardNetwork::JCB,
            ChargebeeCardBrand::DinersClub => CardNetwork::DinersClub,
            ChargebeeCardBrand::Discover => CardNetwork::Discover,
            ChargebeeCardBrand::CartesBancaires => CardNetwork::CartesBancaires,
            ChargebeeCardBrand::UnionPay => CardNetwork::UnionPay,
            ChargebeeCardBrand::Interac => CardNetwork::Interac,
            ChargebeeCardBrand::RuPay => CardNetwork::RuPay,
            ChargebeeCardBrand::Maestro => CardNetwork::Maestro,
            ChargebeeCardBrand::Star => CardNetwork::Star,
            ChargebeeCardBrand::Pulse => CardNetwork::Pulse,
            ChargebeeCardBrand::Accel => CardNetwork::Accel,
            ChargebeeCardBrand::Nyce => CardNetwork::Nyce,
            ChargebeeCardBrand::Other => return None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeFundingType {
    Credit,
    Debit,
    Prepaid,
    NotKnown,
    NotApplicable,
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeTranasactionStatus {
    // Waiting for response from the payment gateway.
    InProgress,
    // The transaction is successful.
    Success,
    // Transaction failed.
    Failure,
    // No response received while trying to charge the card.
    Timeout,
    // Indicates that a successful payment transaction has failed now due to a late failure notification from the payment gateway,
    // typically caused by issues like insufficient funds or a closed bank account.
    LateFailure,
    // Connection with Gateway got terminated abruptly. So, status of this transaction needs to be resolved manually
    NeedsAttention,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeCustomer {
    pub payment_method: ChargebeePaymentMethod,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChargebeeInvoiceBillingAddress {
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub country: Option<enums::CountryAlpha2>,
    pub zip: Option<Secret<String>>,
    pub city: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeePaymentMethod {
    pub reference_id: String,
    pub gateway: ChargebeeGateway,
}

// Chargebee gateways that also exist as Hyperswitch connectors, with the Hyperswitch
// `common_enums::connector_enums::Connector` variant each one corresponds to. Chargebee supports
// roughly sixty gateways in total; the rest have no Hyperswitch equivalent and fall through to
// `Other`. Since `reference_id` is parsed by shape rather than by gateway, an unlisted gateway
// still resolves correctly instead of failing the webhook decode.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeGateway {
    Adyen,                 // Adyen
    Stripe,                // Stripe
    Braintree,             // Braintree
    AuthorizeNet,          // Authorizedotnet
    Paypal,                // Paypal (PayPal Commerce)
    PaypalPro,             // Paypal
    PaypalExpressCheckout, // Paypal
    PaypalPayflowPro,      // Paypal
    AmazonPayments,        // Amazonpay
    Worldpay,              // Worldpay
    Vantiv,                // Worldpayvantiv
    Beanstream,            // Bambora (Chargebee still names it Beanstream)
    Elavon,                // Elavon
    Nmi,                   // Nmi
    Gocardless,            // Gocardless
    Moneris,               // Moneris
    MonerisUs,             // Moneris
    Bluesnap,              // Bluesnap
    Cybersource,           // Cybersource
    CheckoutCom,           // Checkout
    IngenicoDirect,        // Worldline (Worldline Online Payments)
    Mollie,                // Mollie
    Razorpay,              // Razorpay
    GlobalPayments,        // Globalpay
    BankOfAmerica,         // Bankofamerica
    Ebanx,                 // Ebanx
    Dlocal,                // Dlocal
    Nuvei,                 // Nuvei
    Paystack,              // Paystack
    JpMorgan,              // Jpmorgan
    DeutscheBank,          // Deutschebank
    #[serde(other)]
    Other,
}

impl ChargebeeWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("ChargebeeWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

impl ChargebeeInvoiceBody {
    pub fn get_invoice_webhook_data_from_body(
        body: &[u8],
    ) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("ChargebeeInvoiceBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}
// Structure to extract MIT payment data from invoice_generated webhook
#[derive(Debug, Clone)]
pub struct ChargebeeMitPaymentData {
    pub invoice_id: InvoiceId,
    pub amount_due: MinorUnit,
    pub currency_code: enums::Currency,
    pub status: Option<ChargebeeInvoiceStatus>,
    pub customer_id: CustomerId,
    pub subscription_id: SubscriptionId,
    pub first_invoice: bool,
}

impl TryFrom<ChargebeeInvoiceBody> for ChargebeeMitPaymentData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(webhook_body: ChargebeeInvoiceBody) -> Result<Self, Self::Error> {
        let invoice = webhook_body.content.invoice;

        Ok(Self {
            invoice_id: invoice.id,
            amount_due: invoice.total,
            currency_code: invoice.currency_code,
            status: invoice.status,
            customer_id: invoice.customer_id,
            subscription_id: invoice.subscription_id,
            first_invoice: invoice.first_invoice.unwrap_or(false),
        })
    }
}
pub struct ChargebeeMandateDetails {
    pub customer_id: String,
    pub mandate_id: String,
}

impl ChargebeeCustomer {
    // Reference: https://apidocs.chargebee.com/docs/api/customers?prod_cat_ver=2#customer_payment_method_reference_id
    //
    // The layout of `reference_id` follows the payment method rather than the gateway. Card
    // vaults send a composite `<connector_customer_id>/<token>` (Stripe, Braintree), while
    // PayPal and Amazon billing agreements and GoCardless mandates arrive as a single bare
    // identifier. Parsing on that shape keeps every gateway working without having to encode
    // a per-gateway format we cannot verify.
    pub fn find_connector_ids(&self) -> Result<ChargebeeMandateDetails, errors::ConnectorError> {
        let reference_id = self.payment_method.reference_id.as_str();
        let mut parts = reference_id.split('/');
        let customer_id = parts.next().unwrap_or(reference_id);
        // A bare identifier has no trailing segment; it doubles as the customer identifier so
        // token storage still has a stable key to group retries under.
        let mandate_id = parts.next_back().unwrap_or(customer_id);
        Ok(ChargebeeMandateDetails {
            customer_id: customer_id.to_string(),
            mandate_id: mandate_id.to_string(),
        })
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<ChargebeeWebhookBody> for revenue_recovery::RevenueRecoveryAttemptData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ChargebeeWebhookBody) -> Result<Self, Self::Error> {
        let amount = item.content.transaction.amount;
        let currency = item.content.transaction.currency_code.to_owned();
        let merchant_reference_id = common_utils::id_type::PaymentReferenceId::from_str(
            item.content.invoice.id.get_string_repr(),
        )
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        // Chargebee only sets `id_at_gateway` when the gateway returned a reference of its own;
        // external and offline transactions have none. This id is what recovery matches an
        // incoming webhook against an already recorded attempt on
        // (`find_attempt_in_attempts_list_using_connector_transaction_id`), so leaving it unset
        // makes every replay record a duplicate attempt. Chargebee's own transaction id is stable
        // per transaction, so it stands in as that key. It never leaks back to Chargebee as an
        // `id_at_gateway`: record back only runs for internally triggered retries, which carry a
        // real processor id.
        let connector_transaction_id = item
            .content
            .transaction
            .id_at_gateway
            .or(item.content.transaction.id)
            .map(common_utils::types::ConnectorTransactionId::TxnId);
        let error_code = item.content.transaction.error_code.clone();
        let error_message = item.content.transaction.error_text.clone();
        let connector_mandate_details = item
            .content
            .customer
            .as_ref()
            .map(|customer| customer.find_connector_ids())
            .transpose()?
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_mandate_details",
            })?;
        let connector_account_reference_id = item.content.transaction.gateway_account_id.clone();
        let transaction_created_at = item.content.transaction.date;
        let status = enums::AttemptStatus::from(item.content.transaction.status);
        let chargebee_payment_method = item.content.transaction.payment_method;
        let payment_method_type = enums::PaymentMethod::try_from(chargebee_payment_method)?;
        let payment_method_details = item
            .content
            .transaction
            .payment_method_details
            .as_deref()
            .map(|raw_details| chargebee_payment_method.parse_payment_method_details(raw_details))
            .transpose()?;
        // Card transactions take their sub type from the card funding type; everything else
        // falls back to the sub type implied by the Chargebee payment method itself.
        let (payment_method_sub_type, card_info) = match payment_method_details {
            Some(ChargebeePaymentMethodDetails::Card(card)) => (
                enums::PaymentMethodType::from(card.funding_type),
                api_models::payments::AdditionalCardInfo {
                    card_network: card.brand.into(),
                    card_isin: Some(card.iin),
                    ..Default::default()
                },
            ),
            Some(ChargebeePaymentMethodDetails::NonCard) | None => (
                chargebee_payment_method.payment_method_sub_type().ok_or(
                    errors::ConnectorError::NotSupported {
                        message: "payment method in revenue recovery webhook".to_string(),
                        connector: "chargebee",
                    },
                )?,
                api_models::payments::AdditionalCardInfo::default(),
            ),
        };
        // Chargebee retry count will always be less than u16 always. Chargebee can have maximum 12 retry attempts
        #[allow(clippy::as_conversions)]
        let retry_count = item
            .content
            .invoice
            .linked_payments
            .map(|linked_payments| linked_payments.len() as u16);
        let invoice_next_billing_time = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.next_billing_at);
        let invoice_billing_started_at_time = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.current_term_start);
        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            connector_transaction_id,
            error_code,
            error_message,
            processor_payment_method_token: connector_mandate_details.mandate_id,
            connector_customer_id: connector_mandate_details.customer_id,
            connector_account_reference_id,
            transaction_created_at,
            status,
            payment_method_type,
            payment_method_sub_type,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            retry_count,
            invoice_next_billing_time,
            invoice_billing_started_at_time,
            // This field is none because it is specific to stripebilling.
            charge_id: None,
            card_info,
        })
    }
}

impl From<ChargebeeTranasactionStatus> for enums::AttemptStatus {
    fn from(status: ChargebeeTranasactionStatus) -> Self {
        match status {
            ChargebeeTranasactionStatus::InProgress
            | ChargebeeTranasactionStatus::NeedsAttention => Self::Pending,
            ChargebeeTranasactionStatus::Success => Self::Charged,
            ChargebeeTranasactionStatus::Failure
            | ChargebeeTranasactionStatus::Timeout
            | ChargebeeTranasactionStatus::LateFailure => Self::Failure,
        }
    }
}

impl TryFrom<ChargebeeTransactionPaymentMethod> for enums::PaymentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(payment_method: ChargebeeTransactionPaymentMethod) -> Result<Self, Self::Error> {
        match payment_method {
            ChargebeeTransactionPaymentMethod::Card
            | ChargebeeTransactionPaymentMethod::UnionPay
            | ChargebeeTransactionPaymentMethod::SouthKoreanCards => Ok(Self::Card),
            ChargebeeTransactionPaymentMethod::PaypalExpressCheckout
            | ChargebeeTransactionPaymentMethod::AmazonPayments
            | ChargebeeTransactionPaymentMethod::ApplePay
            | ChargebeeTransactionPaymentMethod::GooglePay
            | ChargebeeTransactionPaymentMethod::WeChatPay
            | ChargebeeTransactionPaymentMethod::AliPay
            | ChargebeeTransactionPaymentMethod::AliPayHk
            | ChargebeeTransactionPaymentMethod::Venmo
            | ChargebeeTransactionPaymentMethod::KakaoPay
            | ChargebeeTransactionPaymentMethod::RevolutPay
            | ChargebeeTransactionPaymentMethod::CashAppPay
            | ChargebeeTransactionPaymentMethod::Twint
            | ChargebeeTransactionPaymentMethod::GoPay
            | ChargebeeTransactionPaymentMethod::Gcash
            | ChargebeeTransactionPaymentMethod::Dana
            | ChargebeeTransactionPaymentMethod::TouchNGo
            | ChargebeeTransactionPaymentMethod::Swish => Ok(Self::Wallet),
            ChargebeeTransactionPaymentMethod::DirectDebit
            | ChargebeeTransactionPaymentMethod::PayTo => Ok(Self::BankDebit),
            ChargebeeTransactionPaymentMethod::SepaInstantTransfer
            | ChargebeeTransactionPaymentMethod::AutomatedBankTransfer
            | ChargebeeTransactionPaymentMethod::FasterPayments
            | ChargebeeTransactionPaymentMethod::Pix => Ok(Self::BankTransfer),
            ChargebeeTransactionPaymentMethod::Ideal
            | ChargebeeTransactionPaymentMethod::Sofort
            | ChargebeeTransactionPaymentMethod::Bancontact
            | ChargebeeTransactionPaymentMethod::PayconiqByBancontact
            | ChargebeeTransactionPaymentMethod::Giropay
            | ChargebeeTransactionPaymentMethod::Dotpay
            | ChargebeeTransactionPaymentMethod::OnlineBankingPoland
            | ChargebeeTransactionPaymentMethod::Trustly
            | ChargebeeTransactionPaymentMethod::Bizum
            | ChargebeeTransactionPaymentMethod::NetbankingEmandates => Ok(Self::BankRedirect),
            ChargebeeTransactionPaymentMethod::PayByBank => Ok(Self::OpenBanking),
            ChargebeeTransactionPaymentMethod::Upi => Ok(Self::Upi),
            ChargebeeTransactionPaymentMethod::Promptpay => Ok(Self::RealTimePayment),
            ChargebeeTransactionPaymentMethod::Stablecoin => Ok(Self::Crypto),
            ChargebeeTransactionPaymentMethod::Klarna
            | ChargebeeTransactionPaymentMethod::KlarnaPayNow
            | ChargebeeTransactionPaymentMethod::AfterPay => Ok(Self::PayLater),
            ChargebeeTransactionPaymentMethod::Other => Err(errors::ConnectorError::NotSupported {
                message: "payment method in revenue recovery webhook".to_string(),
                connector: "chargebee",
            }
            .into()),
        }
    }
}

impl From<ChargebeeFundingType> for enums::PaymentMethodType {
    fn from(funding_type: ChargebeeFundingType) -> Self {
        match funding_type {
            ChargebeeFundingType::Credit => Self::Credit,
            ChargebeeFundingType::Debit => Self::Debit,
            // Chargebee reports `prepaid`, `not_known` and `not_applicable` for cards it cannot
            // classify. These are still card payments, so they fall back to the generic card sub
            // type rather than guessing credit or debit.
            ChargebeeFundingType::Prepaid
            | ChargebeeFundingType::NotKnown
            | ChargebeeFundingType::NotApplicable
            | ChargebeeFundingType::Other => {
                #[cfg(feature = "v2")]
                {
                    Self::Card
                }
                // V1 has no generic card subtype. Preserve the historical card fallback without
                // rejecting the webhook; v2 can represent this accurately as `Card`.
                #[cfg(feature = "v1")]
                {
                    Self::Credit
                }
            }
        }
    }
}
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl From<ChargebeeEventType> for api_models::webhooks::IncomingWebhookEvent {
    fn from(event: ChargebeeEventType) -> Self {
        match event {
            ChargebeeEventType::PaymentSucceeded => Self::RecoveryPaymentSuccess,
            ChargebeeEventType::PaymentFailed => Self::RecoveryPaymentFailure,
            ChargebeeEventType::InvoiceDeleted => Self::RecoveryInvoiceCancel,
            ChargebeeEventType::InvoiceGenerated => Self::InvoiceGenerated,
        }
    }
}

#[cfg(feature = "v1")]
impl From<ChargebeeEventType> for api_models::webhooks::IncomingWebhookEvent {
    fn from(event: ChargebeeEventType) -> Self {
        match event {
            ChargebeeEventType::PaymentSucceeded => Self::PaymentIntentSuccess,
            ChargebeeEventType::PaymentFailed => Self::PaymentIntentFailure,
            ChargebeeEventType::InvoiceDeleted => Self::EventNotSupported,
            ChargebeeEventType::InvoiceGenerated => Self::InvoiceGenerated,
        }
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<ChargebeeInvoiceBody> for revenue_recovery::RevenueRecoveryInvoiceData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ChargebeeInvoiceBody) -> Result<Self, Self::Error> {
        let merchant_reference_id = common_utils::id_type::PaymentReferenceId::from_str(
            item.content.invoice.id.get_string_repr(),
        )
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        // The retry count will never exceed u16 limit in a billing connector. It can have maximum of 12 in case of charge bee so its ok to suppress this
        #[allow(clippy::as_conversions)]
        let retry_count = item
            .content
            .invoice
            .linked_payments
            .as_ref()
            .map(|linked_payments| linked_payments.len() as u16);
        let invoice_next_billing_time = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.next_billing_at);
        let billing_started_at = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.current_term_start);
        Ok(Self {
            amount: item.content.invoice.total,
            currency: item.content.invoice.currency_code,
            merchant_reference_id,
            billing_address: Some(api_models::payments::Address::from(item.content.invoice)),
            retry_count,
            next_billing_at: invoice_next_billing_time,
            billing_started_at,
            metadata: None,
            // TODO! This field should be handled for billing connnector integrations
            enable_partial_authorization: None,
        })
    }
}

impl From<ChargebeeInvoiceData> for api_models::payments::Address {
    fn from(item: ChargebeeInvoiceData) -> Self {
        Self {
            address: item
                .billing_address
                .map(api_models::payments::AddressDetails::from),
            phone: None,
            email: None,
        }
    }
}

impl From<ChargebeeInvoiceBillingAddress> for api_models::payments::AddressDetails {
    fn from(item: ChargebeeInvoiceBillingAddress) -> Self {
        Self {
            city: item.city,
            country: item.country,
            state: item.state,
            zip: item.zip,
            line1: item.line1,
            line2: item.line2,
            line3: item.line3,
            first_name: None,
            last_name: None,
            origin_zip: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChargebeeRecordPaymentRequest {
    #[serde(rename = "transaction[amount]")]
    pub amount: MinorUnit,
    #[serde(rename = "transaction[payment_method]")]
    pub payment_method: ChargebeeRecordPaymentMethod,
    #[serde(rename = "transaction[id_at_gateway]")]
    pub connector_payment_id: Option<String>,
    #[serde(rename = "transaction[status]")]
    pub status: ChargebeeRecordStatus,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeRecordPaymentMethod {
    Other,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeRecordStatus {
    Success,
    Failure,
}

impl TryFrom<&ChargebeeRouterData<&InvoiceRecordBackRouterData>> for ChargebeeRecordPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ChargebeeRouterData<&InvoiceRecordBackRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = &item.router_data.request;
        Ok(Self {
            amount: req.amount,
            payment_method: ChargebeeRecordPaymentMethod::Other,
            connector_payment_id: req
                .connector_transaction_id
                .as_ref()
                .map(|connector_payment_id| connector_payment_id.get_id().to_string()),
            status: ChargebeeRecordStatus::try_from(req.attempt_status)?,
        })
    }
}

impl TryFrom<enums::AttemptStatus> for ChargebeeRecordStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(status: enums::AttemptStatus) -> Result<Self, Self::Error> {
        match status {
            enums::AttemptStatus::Charged
            | enums::AttemptStatus::PartialCharged
            | enums::AttemptStatus::PartialChargedAndChargeable => Ok(Self::Success),
            enums::AttemptStatus::Failure
            | enums::AttemptStatus::CaptureFailed
            | enums::AttemptStatus::RouterDeclined => Ok(Self::Failure),
            enums::AttemptStatus::AuthenticationFailed
            | enums::AttemptStatus::Started
            | enums::AttemptStatus::AuthenticationPending
            | enums::AttemptStatus::AuthenticationSuccessful
            | enums::AttemptStatus::Authorized
            | enums::AttemptStatus::PartiallyAuthorized
            | enums::AttemptStatus::AuthorizationFailed
            | enums::AttemptStatus::Authorizing
            | enums::AttemptStatus::CodInitiated
            | enums::AttemptStatus::Voided
            | enums::AttemptStatus::VoidedPostCharge
            | enums::AttemptStatus::VoidInitiated
            | enums::AttemptStatus::CaptureInitiated
            | enums::AttemptStatus::VoidFailed
            | enums::AttemptStatus::AutoRefunded
            | enums::AttemptStatus::Unresolved
            | enums::AttemptStatus::Pending
            | enums::AttemptStatus::PaymentMethodAwaited
            | enums::AttemptStatus::ConfirmationAwaited
            | enums::AttemptStatus::DeviceDataCollectionPending
            | enums::AttemptStatus::IntegrityFailure
            | enums::AttemptStatus::Expired
            | enums::AttemptStatus::CaptureReview => Err(errors::ConnectorError::NotSupported {
                message: "Record back flow is only supported for terminal status".to_string(),
                connector: "chargebee",
            }
            .into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeRecordbackResponse {
    pub invoice: ChargebeeRecordbackInvoice,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeRecordbackInvoice {
    pub id: common_utils::id_type::PaymentReferenceId,
}

convert_connector_response_to_domain_response!(
    ChargebeeRecordbackResponse,
    InvoiceRecordBackResponse,
    |item: ResponseRouterData<_, ChargebeeRecordbackResponse, _, _>| {
        let merchant_reference_id = item.response.invoice.id;
        Ok(Self {
            response: Ok(InvoiceRecordBackResponse {
                merchant_reference_id,
            }),
            ..item.data
        })
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeListPlansResponse {
    pub list: Vec<ChargebeeItemList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeItemList {
    pub item: ChargebeeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub plan_type: String,
    pub is_giftable: bool,
    pub enabled_for_checkout: bool,
    pub enabled_in_portal: bool,
    pub metered: bool,
    pub deleted: bool,
    pub description: Option<String>,
}

convert_connector_response_to_domain_response!(
    SubscriptionEstimateResponse,
    GetSubscriptionEstimateResponse,
    |item: ResponseRouterData<_, SubscriptionEstimateResponse, _, _>| {
        let estimate = item.response.estimate;
        Ok(Self {
            response: Ok(GetSubscriptionEstimateResponse {
                sub_total: estimate.invoice_estimate.sub_total,
                total: estimate.invoice_estimate.total,
                amount_paid: Some(estimate.invoice_estimate.amount_paid),
                amount_due: Some(estimate.invoice_estimate.amount_due),
                currency: estimate.subscription_estimate.currency_code,
                next_billing_at: estimate.subscription_estimate.next_billing_at,
                credits_applied: Some(estimate.invoice_estimate.credits_applied),
                customer_id: Some(estimate.invoice_estimate.customer_id),
                line_items: estimate
                    .invoice_estimate
                    .line_items
                    .into_iter()
                    .map(|line_item| SubscriptionLineItem {
                        item_id: line_item.entity_id,
                        item_type: line_item.entity_type,
                        description: line_item.description,
                        amount: line_item.amount,
                        currency: estimate.invoice_estimate.currency_code,
                        unit_amount: Some(line_item.unit_amount),
                        quantity: line_item.quantity,
                        pricing_model: Some(line_item.pricing_model),
                    })
                    .collect(),
            }),
            ..item.data
        })
    }
);

convert_connector_response_to_domain_response!(
    ChargebeeListPlansResponse,
    GetSubscriptionItemsResponse,
    |item: ResponseRouterData<_, ChargebeeListPlansResponse, _, _>| {
        let plans = item
            .response
            .list
            .into_iter()
            .map(|plan| subscriptions::SubscriptionItems {
                subscription_provider_item_id: plan.item.id,
                name: plan.item.name,
                description: plan.item.description,
            })
            .collect();
        Ok(Self {
            response: Ok(GetSubscriptionItemsResponse { list: plans }),
            ..item.data
        })
    }
);

#[derive(Debug, Serialize)]
pub struct ChargebeeCustomerCreateRequest {
    #[serde(rename = "id")]
    pub customer_id: CustomerId,
    #[serde(rename = "first_name")]
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
    #[serde(rename = "billing_address[first_name]")]
    pub billing_address_first_name: Option<Secret<String>>,
    #[serde(rename = "billing_address[last_name]")]
    pub billing_address_last_name: Option<Secret<String>>,
    #[serde(rename = "billing_address[line1]")]
    pub billing_address_line1: Option<Secret<String>>,
    #[serde(rename = "billing_address[city]")]
    pub billing_address_city: Option<String>,
    #[serde(rename = "billing_address[state]")]
    pub billing_address_state: Option<Secret<String>>,
    #[serde(rename = "billing_address[zip]")]
    pub billing_address_zip: Option<Secret<String>>,
    #[serde(rename = "billing_address[country]")]
    pub billing_address_country: Option<String>,
}

impl TryFrom<&ChargebeeRouterData<&hyperswitch_domain_models::types::ConnectorCustomerRouterData>>
    for ChargebeeCustomerCreateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &ChargebeeRouterData<&hyperswitch_domain_models::types::ConnectorCustomerRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = &item.router_data.request;

        Ok(Self {
            customer_id: req
                .customer_id
                .as_ref()
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                })?
                .clone(),
            name: req.name.clone(),
            email: req.email.clone(),
            billing_address_first_name: req
                .billing_address
                .as_ref()
                .and_then(|address| address.first_name.clone()),
            billing_address_last_name: req
                .billing_address
                .as_ref()
                .and_then(|address| address.last_name.clone()),
            billing_address_line1: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.line1.clone()),
            billing_address_city: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.city.clone()),
            billing_address_country: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.country.map(|country| country.to_string())),
            billing_address_state: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.state.clone()),
            billing_address_zip: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.zip.clone()),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeCustomerCreateResponse {
    pub customer: ChargebeeCustomerDetails,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeCustomerDetails {
    pub id: String,
    #[serde(rename = "first_name")]
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
    pub billing_address: Option<api_models::payments::AddressDetails>,
}

convert_connector_response_to_domain_response!(
    ChargebeeCustomerCreateResponse,
    PaymentsResponseData,
    |item: ResponseRouterData<_, ChargebeeCustomerCreateResponse, _, _>| {
        let customer_response = &item.response.customer;

        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new(
                    customer_response.id.clone(),
                    customer_response
                        .name
                        .as_ref()
                        .map(|name| name.clone().expose()),
                    customer_response
                        .email
                        .as_ref()
                        .map(|email| email.clone().expose().expose()),
                    customer_response.billing_address.clone(),
                ),
            )),
            ..item.data
        })
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeSubscriptionEstimateRequest {
    #[serde(rename = "subscription_items[item_price_id][0]")]
    pub price_id: String,
}

impl TryFrom<&GetSubscriptionEstimateRouterData> for ChargebeeSubscriptionEstimateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &GetSubscriptionEstimateRouterData) -> Result<Self, Self::Error> {
        let price_id = item.request.price_id.to_owned();
        Ok(Self { price_id })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeGetPlanPricesResponse {
    pub list: Vec<ChargebeeGetPlanPriceList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeGetPlanPriceList {
    pub item_price: ChargebeePlanPriceItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeePlanPriceItem {
    pub id: String,
    pub name: String,
    pub currency_code: common_enums::Currency,
    pub free_quantity: i64,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub created_at: Option<PrimitiveDateTime>,
    pub deleted: bool,
    pub item_id: Option<String>,
    pub period: i64,
    pub period_unit: ChargebeePeriodUnit,
    pub trial_period: Option<i64>,
    pub trial_period_unit: Option<ChargebeeTrialPeriodUnit>,
    pub price: MinorUnit,
    pub pricing_model: ChargebeePricingModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeePricingModel {
    FlatFee,
    PerUnit,
    Tiered,
    Volume,
    Stairstep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeePeriodUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeTrialPeriodUnit {
    Day,
    Month,
}

convert_connector_response_to_domain_response!(
    ChargebeeGetPlanPricesResponse,
    GetSubscriptionItemPricesResponse,
    |item: ResponseRouterData<_, ChargebeeGetPlanPricesResponse, _, _>| {
        let plan_prices = item
            .response
            .list
            .into_iter()
            .map(|prices| subscriptions::SubscriptionItemPrices {
                price_id: prices.item_price.id,
                item_id: prices.item_price.item_id,
                amount: prices.item_price.price,
                currency: prices.item_price.currency_code,
                interval: match prices.item_price.period_unit {
                    ChargebeePeriodUnit::Day => subscriptions::PeriodUnit::Day,
                    ChargebeePeriodUnit::Week => subscriptions::PeriodUnit::Week,
                    ChargebeePeriodUnit::Month => subscriptions::PeriodUnit::Month,
                    ChargebeePeriodUnit::Year => subscriptions::PeriodUnit::Year,
                },
                interval_count: prices.item_price.period,
                trial_period: prices.item_price.trial_period,
                trial_period_unit: match prices.item_price.trial_period_unit {
                    Some(ChargebeeTrialPeriodUnit::Day) => Some(subscriptions::PeriodUnit::Day),
                    Some(ChargebeeTrialPeriodUnit::Month) => Some(subscriptions::PeriodUnit::Month),
                    None => None,
                },
            })
            .collect();
        Ok(Self {
            response: Ok(GetSubscriptionItemPricesResponse { list: plan_prices }),
            ..item.data
        })
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEstimateResponse {
    pub estimate: ChargebeeEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeEstimate {
    pub created_at: i64,
    /// type of the object will be `estimate`
    pub object: String,
    pub subscription_estimate: SubscriptionEstimate,
    pub invoice_estimate: InvoiceEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEstimate {
    pub status: String,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub next_billing_at: Option<PrimitiveDateTime>,
    /// type of the object will be `subscription_estimate`
    pub object: String,
    pub currency_code: enums::Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceEstimate {
    pub recurring: bool,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub date: Option<PrimitiveDateTime>,
    pub price_type: String,
    pub sub_total: MinorUnit,
    pub total: MinorUnit,
    pub credits_applied: MinorUnit,
    pub amount_paid: MinorUnit,
    pub amount_due: MinorUnit,
    /// type of the object will be `invoice_estimate`
    pub object: String,
    pub customer_id: CustomerId,
    pub line_items: Vec<LineItem>,
    pub currency_code: enums::Currency,
    pub round_off_amount: MinorUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub id: String,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub date_from: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub date_to: Option<PrimitiveDateTime>,
    pub unit_amount: MinorUnit,
    pub quantity: i64,
    pub amount: MinorUnit,
    pub pricing_model: String,
    pub is_taxed: bool,
    pub tax_amount: MinorUnit,
    /// type of the object will be `line_item`
    pub object: String,
    pub customer_id: String,
    pub description: String,
    pub entity_type: String,
    pub entity_id: String,
    pub discount_amount: MinorUnit,
    pub item_level_discount_amount: MinorUnit,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeInvoiceStatus {
    Paid,
    Posted,
    PaymentDue,
    NotPaid,
    Voided,
    #[serde(other)]
    Pending,
}

impl From<ChargebeeInvoiceData> for SubscriptionInvoiceData {
    fn from(item: ChargebeeInvoiceData) -> Self {
        Self {
            billing_address: Some(api_models::payments::Address::from(item.clone())),
            id: item.id,
            total: item.total,
            currency_code: item.currency_code,
            status: item.status.map(connector_enums::InvoiceStatus::from),
        }
    }
}

impl From<ChargebeeInvoiceStatus> for connector_enums::InvoiceStatus {
    fn from(status: ChargebeeInvoiceStatus) -> Self {
        match status {
            ChargebeeInvoiceStatus::Paid => Self::InvoicePaid,
            ChargebeeInvoiceStatus::Posted => Self::PaymentPendingTimeout,
            ChargebeeInvoiceStatus::PaymentDue => Self::PaymentPending,
            ChargebeeInvoiceStatus::NotPaid => Self::PaymentFailed,
            ChargebeeInvoiceStatus::Voided => Self::Voided,
            ChargebeeInvoiceStatus::Pending => Self::InvoiceCreated,
        }
    }
}

// Pause Subscription structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChargebeePauseSubscriptionRequest {
    #[serde(rename = "pause_option")]
    pub pause_option: Option<api::PauseOption>,
    #[serde(rename = "resume_date", skip_serializing_if = "Option::is_none")]
    pub resume_date: Option<i64>,
}

impl From<&SubscriptionPauseRouterData> for ChargebeePauseSubscriptionRequest {
    fn from(req: &SubscriptionPauseRouterData) -> Self {
        Self {
            pause_option: req.request.pause_option.clone(),
            resume_date: req
                .request
                .pause_date
                .map(|date| date.assume_utc().unix_timestamp()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeePauseSubscriptionResponse {
    pub subscription: ChargebeeSubscriptionDetails,
}

// Resume Subscription structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChargebeeResumeSubscriptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_option: Option<api::ResumeOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_date: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charges_handling: Option<api::ChargesHandling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unpaid_invoices_handling: Option<api::UnpaidInvoicesHandling>,
}

impl From<&SubscriptionResumeRouterData> for ChargebeeResumeSubscriptionRequest {
    fn from(req: &SubscriptionResumeRouterData) -> Self {
        Self {
            resume_option: req.request.resume_option.clone(),
            resume_date: req
                .request
                .resume_date
                .map(|date| date.assume_utc().unix_timestamp()),
            charges_handling: req.request.charges_handling.clone(),
            unpaid_invoices_handling: req.request.unpaid_invoices_handling.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeResumeSubscriptionResponse {
    pub subscription: ChargebeeSubscriptionDetails,
}

// Cancel Subscription structures
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChargebeeCancelSubscriptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_option: Option<api::CancelOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unbilled_charges_option: Option<api::UnbilledChargesOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_option_for_current_term_charges: Option<api::CreditOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_receivables_handling: Option<api::AccountReceivablesHandling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refundable_credits_handling: Option<api::RefundableCreditsHandling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_reason_code: Option<String>,
}

impl From<&SubscriptionCancelRouterData> for ChargebeeCancelSubscriptionRequest {
    fn from(req: &SubscriptionCancelRouterData) -> Self {
        Self {
            cancel_at: req
                .request
                .cancel_date
                .map(|date| date.assume_utc().unix_timestamp()),
            cancel_option: req.request.cancel_option.clone(),
            unbilled_charges_option: req.request.unbilled_charges_option.clone(),
            credit_option_for_current_term_charges: req
                .request
                .credit_option_for_current_term_charges
                .clone(),
            account_receivables_handling: req.request.account_receivables_handling.clone(),
            refundable_credits_handling: req.request.refundable_credits_handling.clone(),
            cancel_reason_code: req.request.cancel_reason_code.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeCancelSubscriptionResponse {
    pub subscription: ChargebeeSubscriptionDetails,
}

convert_connector_response_to_domain_response!(
    ChargebeePauseSubscriptionResponse,
    SubscriptionPauseResponse,
    |item: ResponseRouterData<_, ChargebeePauseSubscriptionResponse, _, _>| {
        let subscription = item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionPauseResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                paused_at: subscription.pause_date,
            }),
            ..item.data
        })
    }
);

convert_connector_response_to_domain_response!(
    ChargebeeResumeSubscriptionResponse,
    SubscriptionResumeResponse,
    |item: ResponseRouterData<_, ChargebeeResumeSubscriptionResponse, _, _>| {
        let subscription = item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionResumeResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                next_billing_at: subscription.next_billing_at,
            }),
            ..item.data
        })
    }
);

convert_connector_response_to_domain_response!(
    ChargebeeCancelSubscriptionResponse,
    SubscriptionCancelResponse,
    |item: ResponseRouterData<_, ChargebeeCancelSubscriptionResponse, _, _>| {
        let subscription = item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionCancelResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                cancelled_at: subscription.cancelled_at,
            }),
            ..item.data
        })
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_non_card_transaction_payment_method() {
        let payment_method: ChargebeeTransactionPaymentMethod =
            serde_json::from_str(r#""paypal_express_checkout""#).unwrap();
        assert!(matches!(
            payment_method,
            ChargebeeTransactionPaymentMethod::PaypalExpressCheckout
        ));
        assert_eq!(
            enums::PaymentMethod::try_from(payment_method).unwrap(),
            enums::PaymentMethod::Wallet
        );
        assert_eq!(
            payment_method.payment_method_sub_type(),
            Some(enums::PaymentMethodType::Paypal)
        );
    }

    #[test]
    fn test_unknown_transaction_payment_method_falls_back_to_other() {
        let payment_method: ChargebeeTransactionPaymentMethod =
            serde_json::from_str(r#""some_method_chargebee_added_later""#).unwrap();
        assert!(matches!(
            payment_method,
            ChargebeeTransactionPaymentMethod::Other
        ));
        assert!(enums::PaymentMethod::try_from(payment_method).is_err());
    }

    /// A `paypal_express_checkout` transaction must land as Wallet / Paypal with no card details,
    /// driven through the same conversion the incoming webhook uses.
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    #[test]
    fn test_paypal_webhook_maps_to_wallet_paypal() {
        let body: ChargebeeWebhookBody = serde_json::from_str(
            r#"{
                "event_type": "payment_failed",
                "content": {
                    "transaction": {
                        "id_at_gateway": "txn_gw_123",
                        "status": "failure",
                        "error_code": "2001",
                        "error_text": "Insufficient funds",
                        "gateway_account_id": "gw_acct_123",
                        "currency_code": "USD",
                        "amount": 1000,
                        "date": 1735689600,
                        "payment_method": "paypal_express_checkout",
                        "payment_method_details": "{\"paypal_express_checkout\":{\"email\":\"buyer@example.com\"}}"
                    },
                    "invoice": {
                        "id": "inv_123",
                        "total": 1000,
                        "currency_code": "USD",
                        "status": "not_paid",
                        "linked_payments": [{"txn_status": "failure"}],
                        "customer_id": "cus_123",
                        "subscription_id": "sub_123"
                    },
                    "customer": {
                        "payment_method": {
                            "reference_id": "B-1AB23456CD789012E",
                            "gateway": "paypal_express_checkout"
                        }
                    }
                }
            }"#,
        )
        .expect("paypal webhook body should deserialize");

        let attempt = revenue_recovery::RevenueRecoveryAttemptData::try_from(body)
            .expect("paypal webhook should convert to a recovery attempt");

        assert_eq!(attempt.payment_method_type, enums::PaymentMethod::Wallet);
        assert_eq!(
            attempt.payment_method_sub_type,
            enums::PaymentMethodType::Paypal
        );
        assert_eq!(attempt.card_info.card_network, None);
        assert_eq!(attempt.card_info.card_isin, None);
        assert_eq!(attempt.status, enums::AttemptStatus::Failure);
        // Bare billing agreement id doubles as the connector customer id and the mandate token.
        assert_eq!(attempt.connector_customer_id, "B-1AB23456CD789012E");
        assert_eq!(
            attempt.processor_payment_method_token,
            "B-1AB23456CD789012E"
        );
    }

    /// External and offline transactions carry no `id_at_gateway`. Recovery dedups incoming
    /// webhooks against recorded attempts by this id, so it must fall back to Chargebee's own
    /// transaction id rather than being left unset.
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    #[test]
    fn test_missing_id_at_gateway_falls_back_to_chargebee_transaction_id() {
        let webhook = |transaction_ids: &str| {
            format!(
                r#"{{
                    "event_type": "payment_failed",
                    "content": {{
                        "transaction": {{
                            {transaction_ids}
                            "status": "failure",
                            "gateway_account_id": "gw_acct_123",
                            "currency_code": "USD",
                            "amount": 1000,
                            "date": 1735689600,
                            "payment_method": "card",
                            "payment_method_details": "{{\"card\":{{\"iin\":\"424242\",\"brand\":\"visa\",\"funding_type\":\"credit\"}}}}"
                        }},
                        "invoice": {{
                            "id": "inv_123",
                            "total": 1000,
                            "currency_code": "USD",
                            "status": "not_paid",
                            "customer_id": "cus_123",
                            "subscription_id": "sub_123"
                        }},
                        "customer": {{
                            "payment_method": {{
                                "reference_id": "cus_gw_1/tok_1",
                                "gateway": "stripe"
                            }}
                        }}
                    }}
                }}"#
            )
        };
        let attempt_for = |transaction_ids: &str| {
            let body: ChargebeeWebhookBody = serde_json::from_str(&webhook(transaction_ids))
                .expect("webhook body should deserialize");
            revenue_recovery::RevenueRecoveryAttemptData::try_from(body)
                .expect("webhook should convert to a recovery attempt")
        };

        // No `id_at_gateway`: Chargebee's transaction id becomes the dedup key.
        assert_eq!(
            attempt_for(r#""id": "txn_16BdDfSlbaZ7Y2fJ","#).connector_transaction_id,
            Some(common_utils::types::ConnectorTransactionId::TxnId(
                "txn_16BdDfSlbaZ7Y2fJ".to_string()
            ))
        );

        // `id_at_gateway` present: it still wins over Chargebee's own id.
        assert_eq!(
            attempt_for(r#""id": "txn_16BdDfSlbaZ7Y2fJ","id_at_gateway": "ch_gw_999","#)
                .connector_transaction_id,
            Some(common_utils::types::ConnectorTransactionId::TxnId(
                "ch_gw_999".to_string()
            ))
        );

        // Neither present: unchanged from the previous behaviour.
        assert_eq!(attempt_for("").connector_transaction_id, None);
    }

    #[test]
    fn test_paypal_gateway_reference_id_is_used_verbatim() {
        let customer: ChargebeeCustomer = serde_json::from_str(
            r#"{"payment_method":{"reference_id":"B-1AB23456CD789012E","gateway":"paypal_express_checkout"}}"#,
        )
        .unwrap();
        let mandate_details = customer.find_connector_ids().unwrap();
        assert_eq!(mandate_details.mandate_id, "B-1AB23456CD789012E");
        assert_eq!(mandate_details.customer_id, "B-1AB23456CD789012E");
    }

    /// Every Chargebee payment method we model must round-trip to a concrete Hyperswitch
    /// payment method and sub type. A bad serde rename would silently land in `Other`, so this
    /// asserts none of them do.
    #[test]
    fn test_modelled_payment_methods_map_to_hyperswitch_equivalents() {
        for raw in [
            "card",
            "unionpay",
            "south_korean_cards",
            "paypal_express_checkout",
            "amazon_payments",
            "apple_pay",
            "google_pay",
            "wechat_pay",
            "alipay",
            "alipay_hk",
            "venmo",
            "kakao_pay",
            "revolut_pay",
            "cash_app_pay",
            "twint",
            "go_pay",
            "gcash",
            "dana",
            "touch_n_go",
            "swish",
            "ideal",
            "sofort",
            "bancontact",
            "payconiq_by_bancontact",
            "giropay",
            "dotpay",
            "online_banking_poland",
            "trustly",
            "bizum",
            "netbanking_emandates",
            "pay_by_bank",
            "upi",
            "direct_debit",
            "pay_to",
            "faster_payments",
            "sepa_instant_transfer",
            "automated_bank_transfer",
            "pix",
            "promptpay",
            "klarna",
            "klarna_pay_now",
            "after_pay",
            "stablecoin",
        ] {
            let parsed: ChargebeeTransactionPaymentMethod =
                serde_json::from_str(&format!(r#""{raw}""#))
                    .unwrap_or_else(|error| panic!("payment method {raw} should parse: {error}"));
            assert!(
                !matches!(parsed, ChargebeeTransactionPaymentMethod::Other),
                "payment method {raw} fell through to Other - check its serde rename"
            );
            assert!(
                enums::PaymentMethod::try_from(parsed).is_ok(),
                "payment method {raw} has no Hyperswitch payment method"
            );
            // Card backed methods take their sub type from the card funding type instead.
            let is_card_backed = matches!(
                parsed,
                ChargebeeTransactionPaymentMethod::Card
                    | ChargebeeTransactionPaymentMethod::UnionPay
                    | ChargebeeTransactionPaymentMethod::SouthKoreanCards
            );
            assert_eq!(
                parsed.payment_method_sub_type().is_some(),
                !is_card_backed,
                "unexpected sub type mapping for {raw}"
            );
        }
    }

    /// Chargebee payment methods with no Hyperswitch equivalent must not decode into a
    /// concrete variant, and must be rejected explicitly rather than mislabelled.
    #[test]
    fn test_unmappable_payment_methods_are_rejected() {
        for raw in [
            "generic",
            "electronic_payment_standard",
            "kbc_payment_button",
            "naver_pay",
            "grab_pay",
            "pay_co",
            "payme",
            "paypay",
            "paynow",
            "tamara",
            "qpay",
        ] {
            let parsed: ChargebeeTransactionPaymentMethod =
                serde_json::from_str(&format!(r#""{raw}""#)).unwrap();
            assert!(matches!(parsed, ChargebeeTransactionPaymentMethod::Other));
            assert!(enums::PaymentMethod::try_from(parsed).is_err());
        }
    }

    /// Guards the serde renames: a typo would silently fall into `Other` rather than fail.
    #[test]
    fn test_common_gateway_strings_deserialize_to_named_variants() {
        for (raw, expected) in [
            ("adyen", ChargebeeGateway::Adyen),
            ("authorize_net", ChargebeeGateway::AuthorizeNet),
            ("paypal", ChargebeeGateway::Paypal),
            ("paypal_pro", ChargebeeGateway::PaypalPro),
            (
                "paypal_express_checkout",
                ChargebeeGateway::PaypalExpressCheckout,
            ),
            ("paypal_payflow_pro", ChargebeeGateway::PaypalPayflowPro),
            ("amazon_payments", ChargebeeGateway::AmazonPayments),
            ("beanstream", ChargebeeGateway::Beanstream),
            ("moneris_us", ChargebeeGateway::MonerisUs),
            ("checkout_com", ChargebeeGateway::CheckoutCom),
            ("ingenico_direct", ChargebeeGateway::IngenicoDirect),
            ("global_payments", ChargebeeGateway::GlobalPayments),
            ("bank_of_america", ChargebeeGateway::BankOfAmerica),
            ("jp_morgan", ChargebeeGateway::JpMorgan),
            ("deutsche_bank", ChargebeeGateway::DeutscheBank),
            ("vantiv", ChargebeeGateway::Vantiv),
        ] {
            let parsed: ChargebeeGateway = serde_json::from_str(&format!(r#""{raw}""#))
                .unwrap_or_else(|error| panic!("gateway {raw} should deserialize: {error}"));
            assert_eq!(
                std::mem::discriminant(&parsed),
                std::mem::discriminant(&expected),
                "gateway {raw} did not map to its named variant"
            );
        }
    }

    #[test]
    fn test_composite_reference_id_splits_into_customer_and_mandate() {
        let customer: ChargebeeCustomer = serde_json::from_str(
            r#"{"payment_method":{"reference_id":"cus_abc123/pm_xyz789","gateway":"stripe"}}"#,
        )
        .unwrap();
        let mandate_details = customer.find_connector_ids().unwrap();
        assert_eq!(mandate_details.customer_id, "cus_abc123");
        assert_eq!(mandate_details.mandate_id, "pm_xyz789");
    }

    #[test]
    fn test_unknown_gateway_falls_back_to_other_and_still_parses() {
        let customer: ChargebeeCustomer = serde_json::from_str(
            r#"{"payment_method":{"reference_id":"cus_abc123/tok_1","gateway":"some_gateway_we_do_not_model"}}"#,
        )
        .unwrap();
        assert!(matches!(
            customer.payment_method.gateway,
            ChargebeeGateway::Other
        ));
        // Parsing is driven by the reference_id shape, so an unmodelled gateway still resolves.
        let mandate_details = customer.find_connector_ids().unwrap();
        assert_eq!(mandate_details.customer_id, "cus_abc123");
        assert_eq!(mandate_details.mandate_id, "tok_1");
    }

    /// Chargebee documents `payment_method_details` as optional. A PayPal transaction that omits
    /// it entirely must still convert rather than fail the whole webhook decode.
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    #[test]
    fn test_paypal_webhook_without_payment_method_details() {
        let body: ChargebeeWebhookBody = serde_json::from_str(
            r#"{
                "event_type": "payment_failed",
                "content": {
                    "transaction": {
                        "status": "failure",
                        "gateway_account_id": "gw_acct_123",
                        "currency_code": "USD",
                        "amount": 1000,
                        "payment_method": "paypal_express_checkout"
                    },
                    "invoice": {
                        "id": "inv_123",
                        "total": 1000,
                        "currency_code": "USD",
                        "customer_id": "cus_123",
                        "subscription_id": "sub_123"
                    },
                    "customer": {
                        "payment_method": {
                            "reference_id": "B-1AB23456CD789012E",
                            "gateway": "paypal_express_checkout"
                        }
                    }
                }
            }"#,
        )
        .expect("body without payment_method_details should deserialize");

        let attempt = revenue_recovery::RevenueRecoveryAttemptData::try_from(body)
            .expect("missing payment_method_details should not block conversion");
        assert_eq!(attempt.payment_method_type, enums::PaymentMethod::Wallet);
        assert_eq!(
            attempt.payment_method_sub_type,
            enums::PaymentMethodType::Paypal
        );
    }

    /// Non-card methods carry gateway specific blobs that revenue recovery does not consume, so
    /// their shape must never be treated as card details.
    #[test]
    fn test_non_card_payment_method_details_parse_as_non_card() {
        for raw in [
            r#"{"direct_debit":{"bank_name":"Test Bank","mandate_id":"MD123"}}"#,
            r#"{"amazon_payments":{"email":"buyer@example.com"}}"#,
            r#"{}"#,
        ] {
            let details = ChargebeeTransactionPaymentMethod::DirectDebit
                .parse_payment_method_details(raw)
                .unwrap_or_else(|error| panic!("{raw} should parse: {error:?}"));
            assert!(matches!(details, ChargebeePaymentMethodDetails::NonCard));
        }
    }

    #[test]
    fn test_card_payment_method_details_parse_into_card() {
        let details = ChargebeeTransactionPaymentMethod::Card
            .parse_payment_method_details(
                r#"{"card":{"funding_type":"credit","brand":"visa","iin":"424242"}}"#,
            )
            .unwrap();
        let ChargebeePaymentMethodDetails::Card(card) = details else {
            panic!("card payment method should parse into card details");
        };
        assert!(matches!(card.funding_type, ChargebeeFundingType::Credit));
        assert!(matches!(card.brand, ChargebeeCardBrand::Visa));
        assert_eq!(card.iin, "424242");
    }

    /// Real `payment_method_details` captured from a Chargebee card transaction. Chargebee sends
    /// funding types it cannot classify, and the full card object carries far more fields than
    /// revenue recovery reads.
    #[cfg(feature = "v2")]
    #[test]
    fn test_real_card_payment_method_details() {
        let details = ChargebeeTransactionPaymentMethod::Card
            .parse_payment_method_details(
                r#"{"card":{"first_name":"fqgq","last_name":"NISHANTH","iin":"411111","last4":"1111","funding_type":"not_applicable","expiry_month":12,"expiry_year":2029,"billing_addr1":"fsef","billing_addr2":"wefwefwef","billing_city":"gfetv","billing_state":"AS","masked_number":"************1111","object":"card","brand":"visa"}}"#,
            )
            .expect("real card payment_method_details should parse");
        let ChargebeePaymentMethodDetails::Card(card) = details else {
            panic!("card payment method should parse into card details");
        };
        assert!(matches!(
            card.funding_type,
            ChargebeeFundingType::NotApplicable
        ));
        assert_eq!(card.iin, "411111");
        // An unclassifiable funding type still resolves to a card sub type.
        assert_eq!(
            enums::PaymentMethodType::from(card.funding_type),
            enums::PaymentMethodType::Card
        );
    }

    /// Funding types Chargebee cannot classify must resolve to the generic card sub type rather
    /// than failing to deserialize or being guessed as credit/debit.
    #[cfg(feature = "v2")]
    #[test]
    fn test_unclassified_funding_types_map_to_card() {
        for raw in ["prepaid", "not_known", "not_applicable", "something_new"] {
            let funding_type: ChargebeeFundingType = serde_json::from_str(&format!(r#""{raw}""#))
                .unwrap_or_else(|error| panic!("funding type {raw} should parse: {error}"));
            assert_eq!(
                enums::PaymentMethodType::from(funding_type),
                enums::PaymentMethodType::Card,
                "funding type {raw} should fall back to the card sub type"
            );
        }
    }

    /// Card brands outside the networks Hyperswitch models must not fail the webhook; they just
    /// leave the network unset.
    #[test]
    fn test_unmodelled_card_brand_yields_no_network() {
        for raw in ["other", "not_applicable", "some_new_network"] {
            let brand: ChargebeeCardBrand = serde_json::from_str(&format!(r#""{raw}""#)).unwrap();
            assert!(matches!(brand, ChargebeeCardBrand::Other));
            let network: Option<common_enums::CardNetwork> = brand.into();
            assert!(network.is_none());
        }
        let visa: ChargebeeCardBrand = serde_json::from_str(r#""visa""#).unwrap();
        let network: Option<common_enums::CardNetwork> = visa.into();
        assert_eq!(network, Some(common_enums::CardNetwork::Visa));
    }

    /// Dispatching on the payment method means a card transaction whose blob is missing the
    /// `card` object reports a decoding failure rather than being mistaken for a non-card one.
    #[test]
    fn test_card_payment_method_with_malformed_details_is_a_decode_error() {
        assert!(ChargebeeTransactionPaymentMethod::Card
            .parse_payment_method_details(r#"{"paypal_express_checkout":{"email":"a@b.com"}}"#)
            .is_err());
    }

    /// Real `payment_succeeded` webhook captured from Chargebee for a PayPal subscription.
    /// Note it carries no `payment_method_details` key at all, and its gateway is `paypal`.
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    #[test]
    fn test_real_paypal_webhook_payload() {
        let body: ChargebeeWebhookBody = serde_json::from_str(
            r#"{
              "event_type": "payment_succeeded",
              "content": {
                "transaction": {
                  "id": "txn_16A2kWVQn6Qfn8Sa",
                  "customer_id": "16A2kWVQn6QbQ8SW",
                  "subscription_id": "AzqUBWVQn68Iw1fbU",
                  "gateway_account_id": "gw_AzqDQXVQmH1gfGAg",
                  "payment_source_id": "pm_16A2kWVQn6RW28Sd",
                  "payment_method": "paypal_express_checkout",
                  "gateway": "paypal",
                  "type": "payment",
                  "date": 1785352329,
                  "exchange_rate": 1,
                  "amount": 12300,
                  "id_at_gateway": "94G36611VB1615709",
                  "status": "success",
                  "updated_at": 1785352332,
                  "resource_version": 1785352332906,
                  "deleted": false,
                  "object": "transaction",
                  "currency_code": "USD",
                  "base_currency_code": "USD",
                  "amount_unused": 0,
                  "linked_invoices": [
                    {"invoice_id": "48", "applied_amount": 12300, "invoice_status": "paid"}
                  ],
                  "linked_refunds": [],
                  "initiator_type": "customer",
                  "three_d_secure": false
                },
                "invoice": {
                  "id": "48",
                  "customer_id": "16A2kWVQn6QbQ8SW",
                  "subscription_id": "AzqUBWVQn68Iw1fbU",
                  "recurring": true,
                  "status": "paid",
                  "date": 1785352329,
                  "total": 12300,
                  "amount_due": 0,
                  "amount_paid": 12300,
                  "object": "invoice",
                  "first_invoice": true,
                  "currency_code": "USD",
                  "base_currency_code": "USD",
                  "channel": "web",
                  "tax": 0,
                  "sub_total": 12300,
                  "linked_payments": [
                    {"txn_id": "txn_16A2kWVQn6Qfn8Sa", "txn_status": "success", "txn_amount": 12300}
                  ],
                  "billing_address": {
                    "first_name": "ewfqrf",
                    "line1": "rgqergerg",
                    "city": "rbaba",
                    "state": "Alaska",
                    "country": "US",
                    "zip": "BABARGVA",
                    "object": "billing_address"
                  }
                },
                "customer": {
                  "id": "16A2kWVQn6QbQ8SW",
                  "email": "cnb03433@gmail.com",
                  "auto_collection": "on",
                  "object": "customer",
                  "primary_payment_source_id": "pm_16A2kWVQn6RW28Sd",
                  "payment_method": {
                    "object": "payment_method",
                    "type": "paypal_express_checkout",
                    "reference_id": "B-9LX22177UT804715X",
                    "gateway": "paypal",
                    "gateway_account_id": "gw_AzqDQXVQmH1gfGAg",
                    "status": "valid"
                  }
                },
                "subscription": {
                  "id": "AzqUBWVQn68Iw1fbU",
                  "customer_id": "16A2kWVQn6QbQ8SW",
                  "status": "active",
                  "current_term_start": 1785352329,
                  "current_term_end": 1788030729,
                  "next_billing_at": 1788030729,
                  "object": "subscription",
                  "currency_code": "USD"
                }
              }
            }"#,
        )
        .expect("real paypal webhook body should deserialize");

        let attempt = revenue_recovery::RevenueRecoveryAttemptData::try_from(body)
            .expect("real paypal webhook should convert to a recovery attempt");

        assert_eq!(attempt.payment_method_type, enums::PaymentMethod::Wallet);
        assert_eq!(
            attempt.payment_method_sub_type,
            enums::PaymentMethodType::Paypal
        );
        assert_eq!(attempt.status, enums::AttemptStatus::Charged);
        assert_eq!(attempt.amount, MinorUnit::new(12300));
        assert_eq!(attempt.currency, enums::Currency::USD);
        // Bare billing agreement id, so it serves as both ids.
        assert_eq!(attempt.connector_customer_id, "B-9LX22177UT804715X");
        assert_eq!(
            attempt.processor_payment_method_token,
            "B-9LX22177UT804715X"
        );
        assert_eq!(
            attempt.connector_account_reference_id,
            "gw_AzqDQXVQmH1gfGAg"
        );
        assert_eq!(attempt.retry_count, Some(1));
        assert_eq!(attempt.card_info, Default::default());
    }
}
