use common_utils::events::ApiEventMetric;
use serde_json::{Map, Value};
use superposition_types::Config;

use crate::{
    enums as api_enums,
    payment_methods::{BankDebitTypes, BankTransferTypes},
    payments::BankCodeResponse,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SdkPaymentMethodType {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub payment_experience: Option<api_enums::PaymentExperience>,
    pub eligible_connectors: Vec<String>,
    pub card_networks: Option<Vec<api_enums::CardNetwork>>,
    pub bank_names: Option<Vec<BankCodeResponse>>,
    pub bank_debits: Option<BankDebitTypes>,
    pub bank_transfers: Option<BankTransferTypes>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SdkPaymentMethod {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_types: Vec<SdkPaymentMethodType>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SuperPositionConfigResponse {
    pub raw_configs: Config,
    pub resolved_configs: Option<Map<String, Value>>,
    pub context_used: Map<String, Value>,
    pub payment_methods: Option<Vec<SdkPaymentMethod>>,
}

impl ApiEventMetric for SuperPositionConfigResponse {}
