use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use router_derive::Setter;
use time::PrimitiveDateTime;

use crate::{
    core::errors,
    pii,
    services::api,
    types::{self, api as api_types, api::enums as api_enums, storage},
    utils::custom_serde,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentOp {
    Create,
    Update,
    Confirm,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PaymentsRequest {
    #[serde(
        default,
        deserialize_with = "custom_serde::payment_id_type::deserialize_option"
    )]
    pub payment_id: Option<PaymentIdType>,
    pub merchant_id: Option<String>,
    #[serde(default, deserialize_with = "custom_serde::amount::deserialize_option")]
    pub amount: Option<Amount>,
    pub currency: Option<String>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub amount_to_capture: Option<i32>,
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

impl PaymentsRequest {
    pub fn is_mandate(&self) -> Option<MandateTxnType> {
        match (&self.mandate_data, &self.mandate_id) {
            (None, None) => None,
            (_, Some(_)) => Some(MandateTxnType::RecurringMandateTxn),
            (Some(_), _) => Some(MandateTxnType::NewMandateTxn),
        }
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    Value(i32),
    #[default]
    Zero,
}

impl From<Amount> for i32 {
    fn from(amount: Amount) -> Self {
        match amount {
            Amount::Value(v) => v,
            Amount::Zero => 0,
        }
    }
}
impl From<i32> for Amount {
    fn from(val: i32) -> Self {
        match val {
            0 => Amount::Zero,
            amount => Amount::Value(amount),
        }
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
#[serde(deny_unknown_fields)]
pub struct MandateData {
    pub customer_acceptance: CustomerAcceptance,
    pub mandate_type: MandateType,
}

#[derive(Eq, PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum MandateType {
    SingleUse(storage::MandateAmountData),
    MultiUse(Option<storage::MandateAmountData>),
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

impl CustomerAcceptance {
    pub fn get_ip_address(&self) -> Option<String> {
        self.online
            .as_ref()
            .map(|data| data.ip_address.peek().to_owned())
    }
    pub fn get_user_agent(&self) -> Option<String> {
        self.online.as_ref().map(|data| data.user_agent.clone())
    }
    pub fn get_accepted_at(&self) -> PrimitiveDateTime {
        self.accepted_at
            .unwrap_or_else(common_utils::date_time::now)
    }
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

impl super::Router for PaymentsRequest {}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CCard {
    pub card_number: Secret<String, pii::CardNumber>,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_holder_name: Secret<String>,
    pub card_cvc: Secret<String>,
}

#[derive(Default, Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PayLaterData {
    pub billing_email: String,
    pub country: String,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum PaymentMethod {
    #[serde(rename(deserialize = "card"))]
    Card(CCard),
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

impl Default for PaymentMethod {
    fn default() -> Self {
        PaymentMethod::BankTransfer
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PaymentIdType {
    PaymentIntentId(String),
    ConnectorTransactionId(String),
    PaymentTxnId(String),
}

impl PaymentIdType {
    pub fn get_payment_intent_id(&self) -> errors::CustomResult<String, errors::ValidationError> {
        match self {
            Self::PaymentIntentId(id) => Ok(id.clone()),
            Self::ConnectorTransactionId(_) | Self::PaymentTxnId(_) => {
                Err(errors::ValidationError::IncorrectValueProvided {
                    field_name: "payment_id",
                })
                .into_report()
                .attach_printable("Expected payment intent ID but got connector transaction ID")
            }
        }
    }
}

impl Default for PaymentIdType {
    fn default() -> Self {
        Self::PaymentIntentId(Default::default())
    }
}

// Core related api layer.
#[derive(Debug, Clone)]
pub struct Authorize;
#[derive(Debug, Clone)]
pub struct Capture;

#[derive(Debug, Clone)]
pub struct PSync;
#[derive(Debug, Clone)]
pub struct Void;

#[derive(Debug, Clone)]
pub struct Session;

#[derive(Debug, Clone)]
pub struct Verify;

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

#[derive(Default, Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub address: Option<AddressDetails>,
    pub phone: Option<PhoneDetails>,
}

// used by customers also, could be moved outside
#[derive(Clone, Default, Debug, Eq, serde::Deserialize, serde::Serialize, PartialEq)]
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

#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PhoneDetails {
    pub number: Option<Secret<String>>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Deserialize)]
pub(crate) struct PaymentsCaptureRequest {
    pub payment_id: Option<String>,
    pub merchant_id: Option<String>,
    pub amount_to_capture: Option<i32>,
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
    pub amount: i32,
    pub amount_capturable: Option<i32>,
    pub amount_received: Option<i32>,
    pub client_secret: Option<Secret<String>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    pub currency: String,
    pub customer_id: Option<String>,
    pub description: Option<String>,
    pub refunds: Option<Vec<api_types::RefundResponse>>,
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
    pub error_code: Option<String>, //TODO: Add error code column to the database
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

impl MandateValidationFields {
    pub fn is_mandate(&self) -> Option<MandateTxnType> {
        match (&self.mandate_data, &self.mandate_id) {
            (None, None) => None,
            (_, Some(_)) => Some(MandateTxnType::RecurringMandateTxn),
            (Some(_), _) => Some(MandateTxnType::NewMandateTxn),
        }
    }
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

impl PaymentsRedirectionResponse {
    pub fn new(redirect_url: &str) -> Self {
        Self {
            redirect_url: redirect_url.to_owned(),
        }
    }
}

impl From<PaymentsRequest> for PaymentsResponse {
    fn from(item: PaymentsRequest) -> Self {
        let payment_id = match item.payment_id {
            Some(api_types::PaymentIdType::PaymentIntentId(id)) => Some(id),
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
        let payment_id = match item.payment_id {
            api_types::PaymentIdType::PaymentIntentId(id) => Some(id),
            _ => None,
        };

        Self {
            payment_id,
            ..Default::default()
        }
    }
}

impl From<PaymentsSessionRequest> for PaymentsSessionResponse {
    fn from(_item: PaymentsSessionRequest) -> Self {
        Self {
            session_token: vec![],
        }
    }
}

impl From<types::storage::PaymentIntent> for PaymentsResponse {
    fn from(item: types::storage::PaymentIntent) -> Self {
        Self {
            payment_id: Some(item.payment_id),
            merchant_id: Some(item.merchant_id),
            status: item.status.into(),
            amount: item.amount,
            amount_capturable: item.amount_captured,
            client_secret: item.client_secret.map(|s| s.into()),
            created: Some(item.created_at),
            currency: item.currency.map(|c| c.to_string()).unwrap_or_default(),
            description: item.description,
            metadata: item.metadata,
            customer_id: item.customer_id,
            ..Self::default()
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct PgRedirectResponse {
    pub payment_id: String,
    pub status: api_enums::IntentStatus,
    pub gateway_id: String,
    pub customer_id: Option<String>,
    pub amount: Option<i32>,
}

#[derive(Debug, serde::Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct RedirectionResponse {
    pub return_url: String,
    pub params: Vec<(String, String)>,
    pub return_url_with_query_params: String,
    pub http_method: api::Method,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PaymentsResponseForm {
    pub transaction_id: String,
    // pub transaction_reference_id: String,
    pub merchant_id: String,
    pub order_id: String,
}

// Extract only the last 4 digits of card
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
            PaymentMethod::Card(card) => PaymentMethodDataResponse::Card(CCardResponse::from(card)),
            PaymentMethod::BankTransfer => PaymentMethodDataResponse::BankTransfer,
            PaymentMethod::PayLater(pay_later_data) => {
                PaymentMethodDataResponse::PayLater(pay_later_data)
            }
            PaymentMethod::Wallet(wallet_data) => PaymentMethodDataResponse::Wallet(wallet_data),
            PaymentMethod::Paypal => PaymentMethodDataResponse::Paypal,
        }
    }
}

pub trait PaymentAuthorize:
    api::ConnectorIntegration<Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
}

pub trait PaymentSync:
    api::ConnectorIntegration<PSync, types::PaymentsSyncData, types::PaymentsResponseData>
{
}

pub trait PaymentVoid:
    api::ConnectorIntegration<Void, types::PaymentsCancelData, types::PaymentsResponseData>
{
}

pub trait PaymentCapture:
    api::ConnectorIntegration<Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
}

pub trait PaymentSession:
    api::ConnectorIntegration<Session, types::PaymentsSessionData, types::PaymentsResponseData>
{
}

pub trait PreVerify:
    api::ConnectorIntegration<Verify, types::VerifyRequestData, types::PaymentsResponseData>
{
}

pub trait Payment:
    api_types::ConnectorCommon
    + PaymentAuthorize
    + PaymentSync
    + PaymentCapture
    + PaymentVoid
    + PreVerify
    + PaymentSession
{
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentsRetrieveRequest {
    pub resource_id: PaymentIdType,
    pub merchant_id: Option<String>,
    pub force_sync: bool,
    pub param: Option<String>,
    pub connector: Option<String>,
}

#[derive(Default, Debug, serde::Deserialize, Clone)]
pub struct PaymentsSessionRequest {
    pub payment_id: PaymentIdType,
    pub client_secret: String,
}

#[derive(Default, Debug, serde::Serialize, Clone)]
pub struct PaymentsSessionResponse {
    pub session_token: Vec<types::ConnectorSessionToken>,
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

#[cfg(test)]
mod payments_test {
    #![allow(clippy::expect_used)]

    use super::*;

    #[allow(dead_code)]
    fn card() -> CCard {
        CCard {
            card_number: "1234432112344321".to_string().into(),
            card_exp_month: "12".to_string().into(),
            card_exp_year: "99".to_string().into(),
            card_holder_name: "JohnDoe".to_string().into(),
            card_cvc: "123".to_string().into(),
        }
    }

    #[allow(dead_code)]
    fn payments_request() -> PaymentsRequest {
        PaymentsRequest {
            amount: Some(Amount::Value(200)),
            payment_method_data: Some(PaymentMethod::Card(card())),
            ..PaymentsRequest::default()
        }
    }

    //#[test] // FIXME: Fix test
    #[allow(dead_code)]
    fn verify_payments_request() {
        let pay_req = payments_request();
        let serialized =
            serde_json::to_string(&pay_req).expect("error serializing payments request");
        let _deserialized_pay_req: PaymentsRequest =
            serde_json::from_str(&serialized).expect("error de-serializing payments response");
        //assert_eq!(pay_req, deserialized_pay_req)
    }

    // Intended to test the serialization and deserialization of the enum PaymentIdType
    #[test]
    fn test_connector_id_type() {
        let sample_1 = PaymentIdType::PaymentIntentId("test_234565430uolsjdnf48i0".to_string());
        let s_sample_1 = serde_json::to_string(&sample_1).unwrap();
        let ds_sample_1 = serde_json::from_str::<PaymentIdType>(&s_sample_1).unwrap();
        assert_eq!(ds_sample_1, sample_1)
    }
}
