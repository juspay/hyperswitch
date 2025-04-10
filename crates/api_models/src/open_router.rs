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
    pub merchant_id: id_type::ProfileId,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateScorePayload {
    pub merchant_id: id_type::ProfileId,
    pub gateway: RoutableConnectors,
    pub status: TxnStatus,
    pub payment_id: id_type::PaymentId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TxnStatus {
    #[serde(rename = "STARTED")]
    Started,
    #[serde(rename = "AUTHENTICATION_FAILED")]
    AuthenticationFailed,
    #[serde(rename = "JUSPAY_DECLINED")]
    JuspayDeclined,
    #[serde(rename = "PENDING_VBV")]
    PendingVBV,
    #[serde(rename = "VBV_SUCCESSFUL")]
    VBVSuccessful,
    #[serde(rename = "AUTHORIZED")]
    Authorized,
    #[serde(rename = "AUTHORIZATION_FAILED")]
    AuthorizationFailed,
    #[serde(rename = "CHARGED")]
    Charged,
    #[serde(rename = "AUTHORIZING")]
    Authorizing,
    #[serde(rename = "COD_INITIATED")]
    CODInitiated,
    #[serde(rename = "VOIDED")]
    Voided,
    #[serde(rename = "VOID_INITIATED")]
    VoidInitiated,
    #[serde(rename = "NOP")]
    Nop,
    #[serde(rename = "CAPTURE_INITIATED")]
    CaptureInitiated,
    #[serde(rename = "CAPTURE_FAILED")]
    CaptureFailed,
    #[serde(rename = "VOID_FAILED")]
    VoidFailed,
    #[serde(rename = "AUTO_REFUNDED")]
    AutoRefunded,
    #[serde(rename = "PARTIAL_CHARGED")]
    PartialCharged,
    #[serde(rename = "TO_BE_CHARGED")]
    ToBeCharged,
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "FAILURE")]
    Failure,
    #[serde(rename = "DECLINED")]
    Declined,
}

impl From<bool> for TxnStatus {
    fn from(value: bool) -> Self {
        match value {
            true => TxnStatus::Charged,
            _ => TxnStatus::Failure,
        }
    }
}
