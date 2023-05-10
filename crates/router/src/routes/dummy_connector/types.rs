use api_models::enums::Currency;
use common_utils::errors::CustomResult;
use masking::Secret;
use router_env::types::FlowMetric;
use strum::Display;

use super::errors::DummyConnectorErrors;
use crate::services;

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Flow {
    DummyPaymentCreate,
    DummyPaymentRetrieve,
    DummyRefundCreate,
    DummyRefundRetrieve,
}

impl FlowMetric for Flow {}

#[allow(dead_code)]
#[derive(Default)]
pub enum DummyConnectorStatus {
    Succeeded,
    #[default]
    Processing,
    Failed,
}

impl std::fmt::Display for DummyConnectorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Succeeded => write!(f, "succeeded"),
            Self::Processing => write!(f, "processing"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorPaymentRequest {
    pub amount: i64,
    pub currency: Currency,
    pub payment_method_data: DummyConnectorPaymentMethodData,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub enum DummyConnectorPaymentMethodData {
    Card(DummyConnectorCard),
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

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DummyConnectorPaymentData {
    pub status: String,
    pub amount: i64,
    pub eligible_amount: i64,
    pub currency: Currency,
    pub created: String,
    pub payment_method_type: String,
}

impl DummyConnectorPaymentData {
    pub fn new(
        status: String,
        amount: i64,
        eligible_amount: i64,
        currency: Currency,
        created: String,
        payment_method_type: String,
    ) -> Self {
        Self {
            status,
            amount,
            eligible_amount,
            currency,
            created,
            payment_method_type,
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentResponse {
    pub status: String,
    pub id: String,
    pub amount: i64,
    pub currency: Currency,
    pub created: String,
    pub payment_method_type: String,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentRetrieveRequest {
    pub payment_id: String,
}

impl DummyConnectorPaymentResponse {
    pub fn new(
        status: String,
        id: String,
        amount: i64,
        currency: Currency,
        created: String,
        payment_method_type: String,
    ) -> Self {
        Self {
            status,
            id,
            amount,
            currency,
            created,
            payment_method_type,
        }
    }
}

#[derive(Default, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorRefundRequest {
    pub amount: i64,
    pub payment_id: Option<String>,
}

#[derive(Default, Clone, Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorRefundResponse {
    pub status: String,
    pub id: String,
    pub currency: Currency,
    pub created: String,
    pub payment_amount: i64,
    pub refund_amount: i64,
}

impl DummyConnectorRefundResponse {
    pub fn new(
        status: String,
        id: String,
        currency: Currency,
        created: String,
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
