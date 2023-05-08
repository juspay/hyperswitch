use common_utils::errors::CustomResult;
use masking::Secret;
use router_env::types::FlowMetric;
use strum::Display;

use super::errors::DummyConnectorErrors;
use crate::services;

#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum Flow {
    DummyPaymentCreate,
}

impl FlowMetric for Flow {}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct DummyConnectorPaymentRequest {
    pub amount: i64,
    pub payment_method_data: DummyConnectorPaymentMethodData,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub enum DummyConnectorPaymentMethodData {
    Card(DummyConnectorCard),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorCard {
    pub name: Secret<String>,
    pub number: Secret<String, common_utils::pii::CardNumber>,
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
    pub payemnt_method_type: String,
}

impl DummyConnectorPaymentData {
    pub fn new(
        status: String,
        amount: i64,
        eligible_amount: i64,
        payemnt_method_type: String,
    ) -> Self {
        Self {
            status,
            amount,
            eligible_amount,
            payemnt_method_type,
        }
    }
}

#[allow(dead_code)]
pub enum DummyConnectorTransactionStatus {
    Success,
    InProcess,
    Fail,
}

impl std::fmt::Display for DummyConnectorTransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "succeeded"),
            Self::InProcess => write!(f, "processing"),
            Self::Fail => write!(f, "failed"),
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DummyConnectorPaymentResponse {
    pub status: String,
    pub id: String,
    pub amount: i64,
    pub payment_method_type: String,
}

impl DummyConnectorPaymentResponse {
    pub fn new(status: String, id: String, amount: i64, payment_method_type: String) -> Self {
        Self {
            status,
            id,
            amount,
            payment_method_type,
        }
    }
}

pub type DummyConnectorResponse<T> =
    CustomResult<services::ApplicationResponse<T>, DummyConnectorErrors>;
