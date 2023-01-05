use std::num::NonZeroI64;

use common_utils::pii;
use masking::{PeekInterface, Secret};
use router_derive::Setter;
use time::PrimitiveDateTime;

use crate::{enums as api_enums, refunds};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentOp {
    Create,
    Update,
    Confirm,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Metadata {
    pub order_details: OrderDetails,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PaymentsRequest {
    #[serde(default, deserialize_with = "payment_id_type::deserialize_option")]
    pub payment_id: Option<PaymentIdType>,
    pub merchant_id: Option<String>,
    #[serde(default, deserialize_with = "amount::deserialize_option")]
    pub amount: Option<Amount>,
    pub connector: Option<api_enums::Connector>,
    pub currency: Option<api_enums::Currency>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub amount_to_capture: Option<i64>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,
    pub confirm: Option<bool>,
    pub customer_id: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub name: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub off_session: Option<bool>,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
    pub payment_method_data: Option<PaymentMethod>,
    pub payment_method: Option<api_enums::PaymentMethodType>,
    pub payment_token: Option<String>,
    pub card_cvc: Option<Secret<String>>,
    pub shipping: Option<Address>,
    pub billing: Option<Address>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub client_secret: Option<String>,
    pub mandate_data: Option<MandateData>,
    pub mandate_id: Option<String>,
    pub browser_info: Option<serde_json::Value>,
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
    pub email: Option<Secret<String, pii::Email>>,
    pub name: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub payment_method: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: Option<PaymentMethod>,
    pub payment_token: Option<String>,
    pub mandate_data: Option<MandateData>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub client_secret: Option<String>,
}

impl From<PaymentsRequest> for VerifyRequest {
    fn from(item: PaymentsRequest) -> Self {
        Self {
            client_secret: item.client_secret,
            merchant_id: item.merchant_id,
            customer_id: item.customer_id,
            email: item.email,
            name: item.name,
            phone: item.phone,
            phone_country_code: item.phone_country_code,
            payment_method: item.payment_method,
            payment_method_data: item.payment_method_data,
            payment_token: item.payment_token,
            mandate_data: item.mandate_data,
            setup_future_usage: item.setup_future_usage,
            off_session: item.off_session,
        }
    }
}

pub enum MandateTxnType {
    NewMandateTxn,
    RecurringMandateTxn,
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct MandateIds {
    pub mandate_id: String,
    pub connector_mandate_id: Option<String>,
}

impl MandateIds {
    pub fn new(mandate_id: String) -> Self {
        Self {
            mandate_id,
            connector_mandate_id: None,
        }
    }
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MandateData {
    pub customer_acceptance: CustomerAcceptance,
    pub mandate_type: MandateType,
}

#[derive(Clone, Eq, PartialEq, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SingleUseMandate {
    pub amount: i64,
    pub currency: api_enums::Currency,
}

#[derive(Clone, Eq, PartialEq, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MandateAmountData {
    pub amount: i64,
    pub currency: api_enums::Currency,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MandateType {
    SingleUse(MandateAmountData),
    MultiUse(Option<MandateAmountData>),
}

impl Default for MandateType {
    fn default() -> Self {
        Self::MultiUse(None)
    }
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CustomerAcceptance {
    pub acceptance_type: AcceptanceType,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub accepted_at: Option<PrimitiveDateTime>,
    pub online: Option<OnlineMandate>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AcceptanceType {
    Online,
    #[default]
    Offline,
}

#[derive(Default, Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct OnlineMandate {
    pub ip_address: Secret<String, pii::IpAddress>,
    pub user_agent: String,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CCard {
    pub card_number: Secret<String, pii::CardNumber>,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_holder_name: Secret<String>,
    pub card_cvc: Secret<String>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KlarnaRedirectIssuer {
    Stripe,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KlarnaSdkIssuer {
    Klarna,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayLaterData {
    KlarnaRedirect {
        issuer_name: KlarnaRedirectIssuer,
        billing_email: String,
        billing_country: String,
    },
    KlarnaSdk {
        issuer_name: KlarnaSdkIssuer,
        token: String,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub enum PaymentMethod {
    #[serde(rename(deserialize = "card"))]
    Card(CCard),
    #[default]
    #[serde(rename(deserialize = "bank_transfer"))]
    BankTransfer,
    #[serde(rename(deserialize = "wallet"))]
    Wallet(WalletData),
    #[serde(rename(deserialize = "pay_later"))]
    PayLater(PayLaterData),
    #[serde(rename(deserialize = "paypal"))]
    Paypal,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WalletData {
    pub issuer_name: api_enums::WalletIssuer,
    pub token: String,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize)]
pub struct CCardResponse {
    last4: String,
    exp_month: String,
    exp_year: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize)]
pub enum PaymentMethodDataResponse {
    #[serde(rename = "card")]
    Card(CCardResponse),
    #[serde(rename(deserialize = "bank_transfer"))]
    BankTransfer,
    Wallet(WalletData),
    PayLater(PayLaterData),
    Paypal,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PaymentIdType {
    PaymentIntentId(String),
    ConnectorTransactionId(String),
    PaymentAttemptId(String),
}

impl Default for PaymentIdType {
    fn default() -> Self {
        Self::PaymentIntentId(Default::default())
    }
}

//#[derive(Debug, serde::Deserialize, serde::Serialize)]
//#[serde(untagged)]
//pub enum enums::CaptureMethod {
//Automatic,
//Manual,
//}

//impl Default for enums::CaptureMethod {
//fn default() -> Self {
//enums::CaptureMethod::Manual
//}
//}

#[derive(
    Default,
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    frunk::LabelledGeneric,
)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub address: Option<AddressDetails>,
    pub phone: Option<PhoneDetails>,
}

// used by customers also, could be moved outside
#[derive(
    Clone,
    Default,
    Debug,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    frunk::LabelledGeneric,
)]
#[serde(deny_unknown_fields)]
pub struct AddressDetails {
    pub city: Option<String>,
    pub country: Option<String>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
}

#[derive(
    Debug,
    Clone,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    frunk::LabelledGeneric,
)]
pub struct PhoneDetails {
    pub number: Option<Secret<String>>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize)]
pub struct PaymentsCaptureRequest {
    pub payment_id: Option<String>,
    pub merchant_id: Option<String>,
    pub amount_to_capture: Option<i64>,
    pub refund_uncaptured_amount: Option<bool>,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor_prefix: Option<String>,
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
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionType {
    RedirectToUrl,
    DisplayQrCode,
    InvokeSdkClient,
    TriggerApi,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NextAction {
    #[serde(rename = "type")]
    pub next_action_type: NextActionType,
    pub redirect_to_url: Option<String>,
}

#[derive(Setter, Clone, Default, Debug, Eq, PartialEq, serde::Serialize)]
pub struct PaymentsResponse {
    pub payment_id: Option<String>,
    pub merchant_id: Option<String>,
    pub status: api_enums::IntentStatus,
    pub amount: i64,
    pub amount_capturable: Option<i64>,
    pub amount_received: Option<i64>,
    pub connector: Option<String>,
    pub client_secret: Option<Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    pub currency: String,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub refunds: Option<Vec<refunds::RefundResponse>>,
    pub mandate_id: Option<String>,
    pub mandate_data: Option<MandateData>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub off_session: Option<bool>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub capture_on: Option<PrimitiveDateTime>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    #[auth_based]
    pub payment_method: Option<api_enums::PaymentMethodType>,
    #[auth_based]
    pub payment_method_data: Option<PaymentMethodDataResponse>,
    pub payment_token: Option<String>,
    pub shipping: Option<Address>,
    pub billing: Option<Address>,
    pub metadata: Option<serde_json::Value>,
    pub email: Option<Secret<String, pii::Email>>,
    pub name: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub return_url: Option<String>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
    pub statement_descriptor_name: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub next_action: Option<NextAction>,
    pub cancellation_reason: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentListConstraints {
    pub customer_id: Option<String>,
    pub starting_after: Option<String>,
    pub ending_before: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.lt")]
    pub created_lt: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.gt")]
    pub created_gt: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.lte")]
    pub created_lte: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[serde(rename = "created.gte")]
    pub created_gte: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentListResponse {
    pub size: usize,
    pub data: Vec<PaymentsResponse>,
}

#[derive(Setter, Clone, Default, Debug, Eq, PartialEq, serde::Serialize)]
pub struct VerifyResponse {
    pub verify_id: Option<String>,
    pub merchant_id: Option<String>,
    // pub status: enums::VerifyStatus,
    pub client_secret: Option<Secret<String>>,
    pub customer_id: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub name: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub mandate_id: Option<String>,
    #[auth_based]
    pub payment_method: Option<api_enums::PaymentMethodType>,
    #[auth_based]
    pub payment_method_data: Option<PaymentMethodDataResponse>,
    pub payment_token: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

fn default_limit() -> i64 {
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
            customer_id: req.customer_id.clone(),
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

impl From<PaymentsRequest> for PaymentsResponse {
    fn from(item: PaymentsRequest) -> Self {
        let payment_id = match item.payment_id {
            Some(PaymentIdType::PaymentIntentId(id)) => Some(id),
            _ => None,
        };

        Self {
            payment_id,
            merchant_id: item.merchant_id,
            setup_future_usage: item.setup_future_usage,
            off_session: item.off_session,
            shipping: item.shipping,
            billing: item.billing,
            metadata: item.metadata,
            capture_method: item.capture_method,
            payment_method: item.payment_method,
            capture_on: item.capture_on,
            payment_method_data: item
                .payment_method_data
                .map(PaymentMethodDataResponse::from),
            email: item.email,
            name: item.name,
            phone: item.phone,
            payment_token: item.payment_token,
            return_url: item.return_url,
            authentication_type: item.authentication_type,
            statement_descriptor_name: item.statement_descriptor_name,
            statement_descriptor_suffix: item.statement_descriptor_suffix,
            mandate_data: item.mandate_data,
            ..Default::default()
        }
    }
}

impl From<VerifyRequest> for VerifyResponse {
    fn from(item: VerifyRequest) -> Self {
        Self {
            merchant_id: item.merchant_id,
            customer_id: item.customer_id,
            email: item.email,
            name: item.name,
            phone: item.phone,
            payment_method: item.payment_method,
            payment_method_data: item
                .payment_method_data
                .map(PaymentMethodDataResponse::from),
            payment_token: item.payment_token,
            ..Default::default()
        }
    }
}

impl From<PaymentsStartRequest> for PaymentsResponse {
    fn from(item: PaymentsStartRequest) -> Self {
        Self {
            payment_id: Some(item.payment_id),
            merchant_id: Some(item.merchant_id),
            ..Default::default()
        }
    }
}

impl From<PaymentsSessionRequest> for PaymentsResponse {
    fn from(item: PaymentsSessionRequest) -> Self {
        Self {
            payment_id: Some(item.payment_id),
            ..Default::default()
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

impl From<PaymentsRetrieveRequest> for PaymentsResponse {
    // After removing the request from the payments_to_payments_response this will no longer be needed
    fn from(item: PaymentsRetrieveRequest) -> Self {
        let payment_id = match item.resource_id {
            PaymentIdType::PaymentIntentId(id) => Some(id),
            _ => None,
        };

        Self {
            payment_id,
            merchant_id: item.merchant_id,
            ..Default::default()
        }
    }
}

impl From<PaymentsCancelRequest> for PaymentsResponse {
    fn from(item: PaymentsCancelRequest) -> Self {
        Self {
            payment_id: Some(item.payment_id),
            cancellation_reason: item.cancellation_reason,
            ..Default::default()
        }
    }
}

impl From<PaymentsCaptureRequest> for PaymentsResponse {
    // After removing the request from the payments_to_payments_response this will no longer be needed
    fn from(item: PaymentsCaptureRequest) -> Self {
        Self {
            payment_id: item.payment_id,
            amount_received: item.amount_to_capture,
            ..Self::default()
        }
    }
}

impl From<CCard> for CCardResponse {
    fn from(card: CCard) -> Self {
        let card_number_length = card.card_number.peek().clone().len();
        Self {
            last4: card.card_number.peek().clone()[card_number_length - 4..card_number_length]
                .to_string(),
            exp_month: card.card_exp_month.peek().clone(),
            exp_year: card.card_exp_year.peek().clone(),
        }
    }
}

impl From<PaymentMethod> for PaymentMethodDataResponse {
    fn from(payment_method_data: PaymentMethod) -> Self {
        match payment_method_data {
            PaymentMethod::Card(card) => Self::Card(CCardResponse::from(card)),
            PaymentMethod::BankTransfer => Self::BankTransfer,
            PaymentMethod::PayLater(pay_later_data) => Self::PayLater(pay_later_data),
            PaymentMethod::Wallet(wallet_data) => Self::Wallet(wallet_data),
            PaymentMethod::Paypal => Self::Paypal,
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

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentsRetrieveRequest {
    pub resource_id: PaymentIdType,
    pub merchant_id: Option<String>,
    pub force_sync: bool,
    pub param: Option<String>,
    pub connector: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SupportedWallets {
    Paypal,
    ApplePay,
    Klarna,
    Gpay,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct OrderDetails {
    pub product_name: String,
    pub quantity: u16,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentsSessionRequest {
    pub payment_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayAllowedMethodsParameters {
    pub allowed_auth_methods: Vec<String>,
    pub allowed_card_networks: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayTokenParameters {
    pub gateway: String,
    pub gateway_merchant_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayTokenizationSpecification {
    #[serde(rename = "type")]
    pub token_specification_type: String,
    pub parameters: GpayTokenParameters,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayAllowedPaymentMethods {
    #[serde(rename = "type")]
    pub payment_method_type: String,
    pub parameters: GpayAllowedMethodsParameters,
    pub tokenization_specification: GpayTokenizationSpecification,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayTransactionInfo {
    pub country_code: String,
    pub currency_code: String,
    pub total_price_status: String,
    pub total_price: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayMerchantInfo {
    pub merchant_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpayMetadata {
    pub merchant_info: GpayMerchantInfo,
    pub allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpaySessionTokenData {
    pub gpay: GpayMetadata,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "wallet_name")]
#[serde(rename_all = "lowercase")]
pub enum SessionToken {
    Gpay {
        allowed_payment_methods: Vec<GpayAllowedPaymentMethods>,
        transaction_info: GpayTransactionInfo,
    },
    Klarna {
        session_token: String,
        session_id: String,
    },
    Paypal {
        session_token: String,
    },
    Applepay {
        epoch_timestamp: u64,
        expires_at: u64,
        merchant_session_identifier: String,
        nonce: String,
        merchant_identifier: String,
        domain_name: String,
        display_name: String,
        signature: String,
        operational_analytics_identifier: String,
        retries: u8,
        psp_id: String,
    },
}

#[derive(Default, Debug, serde::Serialize, Clone)]
pub struct PaymentsSessionResponse {
    pub payment_id: String,
    pub client_secret: Secret<String, pii::ClientSecret>,
    pub session_token: Vec<SessionToken>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentRetrieveBody {
    pub merchant_id: Option<String>,
    pub force_sync: Option<bool>,
}
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentsCancelRequest {
    #[serde(skip)]
    pub payment_id: String,
    pub cancellation_reason: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentsStartRequest {
    pub payment_id: String,
    pub merchant_id: String,
    pub txn_id: String,
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

mod amount {
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
