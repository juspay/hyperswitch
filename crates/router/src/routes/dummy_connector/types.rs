use api_models::enums::Currency;
use common_utils::errors::CustomResult;
use masking::Secret;
use router_env::types::FlowMetric;
use strum::Display;
use time::PrimitiveDateTime;

use super::errors::DummyConnectorErrors;
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

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorPaymentRequest {
    pub amount: i64,
    pub currency: Currency,
    pub payment_method_data: DummyConnectorPaymentMethodData,
    pub return_url: Option<String>,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub enum DummyConnectorPaymentMethodData {
    Card(DummyConnectorCard),
    Wallet(DummyConnectorWallet),
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub enum DummyConnectorWallet {
    GooglePay
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

#[derive(
    Default, serde::Serialize, serde::Deserialize, strum::Display, PartialEq, Debug, Clone,
)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    #[default]
    Card,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DummyConnectorPaymentData {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum DummyConnectorNextAction {
    RedirectToUrl(String),
}

impl DummyConnectorPaymentData {
    pub fn new(
        payment_id: String,
        status: DummyConnectorStatus,
        amount: i64,
        eligible_amount: i64,
        currency: Currency,
        created: PrimitiveDateTime,
        payment_method_type: PaymentMethodType,
        redirect_url: Option<DummyConnectorNextAction>,
        return_url: Option<String>,
    ) -> Self {
        Self {
            payment_id,
            status,
            amount,
            eligible_amount,
            currency,
            created,
            payment_method_type,
            next_action: redirect_url,
            return_url
        }
    }
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
