use std::fmt::Debug;

use common_utils::id_type;
pub use euclid::{
    dssa::types::EuclidAnalysable,
    frontend::{
        ast,
        dir::{DirKeyKind, EuclidDirFilter},
    },
};
use serde::{Deserialize, Serialize};

use crate::enums::{Currency, PaymentMethod, PaymentMethodType, RoutableConnectors};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterDecideGatewayRequest {
    pub payment_info: PaymentInfo,
    pub merchant_id: id_type::MerchantId,
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
pub struct PaymentInfo {
    payment_id: id_type::PaymentId,
    amount: f64,
    currency: Currency,
    // customerId: Option<ETCu::CustomerId>,
    // preferredGateway: Option<ETG::Gateway>,
    payment_type: String,
    // metadata: Option<String>,
    // internalMetadata: Option<String>,
    // isEmi: Option<bool>,
    // emiBank: Option<String>,
    // emiTenure: Option<i32>,
    payment_method_type: PaymentMethodType,
    payment_method: PaymentMethod,
    // paymentSource: Option<String>,
    // authType: Option<ETCa::txn_card_info::AuthType>,
    // cardIssuerBankName: Option<String>,
    // cardIsin: Option<String>,
    // cardType: Option<ETCa::card_type::CardType>,
    // cardSwitchProvider: Option<Secret<String>>,
}
