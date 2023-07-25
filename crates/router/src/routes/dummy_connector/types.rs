use api_models::enums::Currency;
use common_utils::{errors::CustomResult, generate_id_with_default_len};
use error_stack::report;
use masking::Secret;
use router_env::types::FlowMetric;
use strum::Display;
use time::PrimitiveDateTime;

use super::{consts, errors::DummyConnectorErrors};
use crate::services;

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Flow {
    DummyPaymentCreate,
    DummyPaymentRetrieve,
    DummyPaymentAuthorize,
    DummyPaymentComplete,
    DummyRefundCreate,
    DummyRefundRetrieve,
}

impl FlowMetric for Flow {}

#[derive(
    Default, serde::Serialize, serde::Deserialize, strum::Display, Clone, PartialEq, Debug, Eq,
)]
#[serde(rename_all = "lowercase")]
pub enum DummyConnectorStatus {
    Succeeded,
    #[default]
    Processing,
    Failed,
}

#[derive(Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorPaymentAttempt {
    pub timestamp: PrimitiveDateTime,
    pub attempt_id: String,
    pub payment_id: String,
    pub payment_request: DummyConnectorPaymentRequest,
}

impl From<DummyConnectorPaymentRequest> for DummyConnectorPaymentAttempt {
    fn from(payment_request: DummyConnectorPaymentRequest) -> Self {
        let timestamp = common_utils::date_time::now();
        let payment_id = generate_id_with_default_len(consts::PAYMENT_ID_PREFIX);
        let attempt_id = generate_id_with_default_len(consts::ATTEMPT_ID_PREFIX);
        DummyConnectorPaymentAttempt {
            timestamp,
            attempt_id,
            payment_id,
            payment_request,
        }
    }
}

impl DummyConnectorPaymentAttempt {
    pub fn build_payment_data(
        self,
        status: DummyConnectorStatus,
        next_action: Option<DummyConnectorNextAction>,
        return_url: Option<String>,
    ) -> DummyConnectorPaymentData {
        DummyConnectorPaymentData {
            attempt_id: self.attempt_id,
            payment_id: self.payment_id,
            status,
            amount: self.payment_request.amount,
            eligible_amount: self.payment_request.amount,
            created: self.timestamp,
            currency: self.payment_request.currency,
            payment_method_type: self.payment_request.payment_method_data.into(),
            next_action,
            return_url,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorPaymentRequest {
    pub amount: i64,
    pub currency: Currency,
    pub payment_method_data: PaymentMethodData,
    pub return_url: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodData {
    Card(DummyConnectorCard),
    Wallet(DummyConnectorWallet),
    PayLater(DummyConnectorPayLater),
}

#[derive(
    Default, serde::Serialize, serde::Deserialize, strum::Display, PartialEq, Debug, Clone,
)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    #[default]
    Card,
    Wallet(DummyConnectorWallet),
    PayLater(DummyConnectorPayLater),
}

impl From<PaymentMethodData> for PaymentMethodType {
    fn from(value: PaymentMethodData) -> Self {
        match value {
            PaymentMethodData::Card(_) => Self::Card,
            PaymentMethodData::Wallet(wallet) => Self::Wallet(wallet),
            PaymentMethodData::PayLater(pay_later) => Self::PayLater(pay_later),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub enum DummyConnectorWallet {
    GooglePay,
    Paypal,
    WeChatPay,
    MbWay,
    AliPay,
    AliPayHK,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub enum DummyConnectorPayLater {
    Klarna,
    Affirm,
    AfterPayClearPay,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorCard {
    pub name: Secret<String>,
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvc: Secret<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DummyConnectorPaymentData {
    pub attempt_id: String,
    pub payment_id: String,
    pub status: DummyConnectorStatus,
    pub amount: i64,
    pub eligible_amount: i64,
    pub currency: Currency,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
    pub payment_method_type: PaymentMethodType,
    pub next_action: Option<DummyConnectorNextAction>,
    pub return_url: Option<String>,
}

impl DummyConnectorPaymentData {
    pub fn is_eligible_for_refund(&self, refund_amount: i64) -> DummyConnectorResult<()> {
        if self.eligible_amount < refund_amount {
            return Err(
                report!(DummyConnectorErrors::RefundAmountExceedsPaymentAmount)
                    .attach_printable("Eligible amount is lesser than refund amount"),
            );
        }

        if self.status != DummyConnectorStatus::Succeeded {
            return Err(report!(DummyConnectorErrors::PaymentNotSuccessful)
                .attach_printable("Payment is not successful to process the refund"));
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum DummyConnectorNextAction {
    RedirectToUrl(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentResponse {
    pub status: DummyConnectorStatus,
    pub id: String,
    pub amount: i64,
    pub currency: Currency,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
    pub payment_method_type: PaymentMethodType,
    pub next_action: Option<DummyConnectorNextAction>,
}

impl From<DummyConnectorPaymentData> for DummyConnectorPaymentResponse {
    fn from(value: DummyConnectorPaymentData) -> Self {
        Self {
            status: value.status,
            id: value.payment_id,
            amount: value.amount,
            currency: value.currency,
            created: value.created,
            payment_method_type: value.payment_method_type,
            next_action: value.next_action,
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentRetrieveRequest {
    pub payment_id: String,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentConfirmRequest {
    pub attempt_id: String,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentCompleteRequest {
    pub attempt_id: String,
    pub confirm: bool,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentCompleteBody {
    pub confirm: bool,
}

#[derive(Default, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorRefundRequest {
    pub amount: i64,
    pub payment_id: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorRefundResponse {
    pub status: DummyConnectorStatus,
    pub id: String,
    pub currency: Currency,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
    pub payment_amount: i64,
    pub refund_amount: i64,
}

impl DummyConnectorRefundResponse {
    pub fn new(
        status: DummyConnectorStatus,
        id: String,
        currency: Currency,
        created: PrimitiveDateTime,
        payment_amount: i64,
        refund_amount: i64,
    ) -> Self {
        Self {
            status,
            id,
            currency,
            created,
            payment_amount,
            refund_amount,
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorRefundRetrieveRequest {
    pub refund_id: String,
}

pub type DummyConnectorResponse<T> =
    CustomResult<services::ApplicationResponse<T>, DummyConnectorErrors>;

pub type DummyConnectorResult<T> = CustomResult<T, DummyConnectorErrors>;

pub enum DummyConnectorFlow {
    NoThreeDS(DummyConnectorStatus, Option<DummyConnectorErrors>),
    ThreeDS(DummyConnectorStatus, Option<DummyConnectorErrors>),
}
