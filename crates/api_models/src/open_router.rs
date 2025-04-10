use std::{collections::HashMap, fmt::Debug};

use common_utils::types::MinorUnit;
pub use euclid::{
    dssa::types::EuclidAnalysable,
    frontend::{
        ast,
        dir::{DirKeyKind, EuclidDirFilter},
    },
};
use serde::{Deserialize, Serialize};

use crate::enums::{Currency, PaymentMethod, RoutableConnectors};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterDecideGatewayRequest {
    pub payment_info: PaymentInfo,
    pub merchant_id: String,
    pub eligible_gateway_list: Option<Vec<RoutableConnectors>>,
    pub ranking_algorithm: Option<RankingAlgorithm>,
    pub elimination_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RankingAlgorithm {
    SrBasedRouting,
    PlBasedRouting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfo {
    pub payment_id: String,
    pub amount: MinorUnit,
    pub currency: Currency,
    // customerId: Option<ETCu::CustomerId>,
    // preferredGateway: Option<ETG::Gateway>,
    pub payment_type: String,
    // metadata: Option<String>,
    // internalMetadata: Option<String>,
    // isEmi: Option<bool>,
    // emiBank: Option<String>,
    // emiTenure: Option<i32>,
    pub payment_method_type: String,
    pub payment_method: PaymentMethod,
    // paymentSource: Option<String>,
    // authType: Option<ETCa::txn_card_info::AuthType>,
    // cardIssuerBankName: Option<String>,
    // cardIsin: Option<String>,
    // cardType: Option<ETCa::card_type::CardType>,
    // cardSwitchProvider: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DecidedGateway {
    pub gateway_priority_map: Option<HashMap<String, f64>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub status: String,
    pub error_code: String,
    pub error_message: String,
    pub priority_logic_tag: Option<String>,
    pub filter_wise_gateways: Option<serde_json::Value>,
    pub error_info: UnifiedError,
    pub is_dynamic_mga_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnifiedError {
    pub code: String,
    pub user_message: String,
    pub developer_message: String,
}
