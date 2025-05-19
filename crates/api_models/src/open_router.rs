use std::{collections::HashMap, fmt::Debug};

use common_utils::{errors, id_type, types::MinorUnit};
pub use euclid::{
    dssa::types::EuclidAnalysable,
    frontend::{
        ast,
        dir::{DirKeyKind, EuclidDirFilter},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    enums::{Currency, PaymentMethod},
    payment_methods,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterDecideGatewayRequest {
    pub payment_info: PaymentInfo,
    pub merchant_id: id_type::ProfileId,
    pub eligible_gateway_list: Option<Vec<String>>,
    pub ranking_algorithm: Option<RankingAlgorithm>,
    pub elimination_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RankingAlgorithm {
    SrBasedRouting,
    PlBasedRouting,
    NtwBasedRouting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfo {
    pub payment_id: id_type::PaymentId,
    pub amount: MinorUnit,
    pub currency: Currency,
    // customerId: Option<ETCu::CustomerId>,
    // preferredGateway: Option<ETG::Gateway>,
    pub payment_type: String,
    pub metadata: Option<String>,
    // internalMetadata: Option<String>,
    // isEmi: Option<bool>,
    // emiBank: Option<String>,
    // emiTenure: Option<i32>,
    pub payment_method_type: String,
    pub payment_method: PaymentMethod,
    // paymentSource: Option<String>,
    // authType: Option<ETCa::txn_card_info::AuthType>,
    // cardIssuerBankName: Option<String>,
    pub card_isin: Option<String>,
    // cardType: Option<ETCa::card_type::CardType>,
    // cardSwitchProvider: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DecidedGateway {
    pub gateway_priority_map: Option<HashMap<String, f64>>,
    pub debit_routing_output: Option<DebitRoutingOutput>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DebitRoutingOutput {
    pub co_badged_card_networks: Vec<common_enums::CardNetwork>,
    pub issuer_country: common_enums::CountryAlpha2,
    pub is_regulated: bool,
    pub regulated_name: Option<common_enums::RegulatedName>,
    pub card_type: common_enums::CardType,
}

impl From<&DebitRoutingOutput> for payment_methods::CoBadgedCardData {
    fn from(output: &DebitRoutingOutput) -> Self {
        Self {
            co_badged_card_networks: output.co_badged_card_networks.clone(),
            issuer_country_code: output.issuer_country,
            is_regulated: output.is_regulated,
            regulated_name: output.regulated_name.clone(),
        }
    }
}

impl TryFrom<(payment_methods::CoBadgedCardData, String)> for DebitRoutingRequestData {
    type Error = error_stack::Report<errors::ParsingError>;

    fn try_from(
        (output, card_type): (payment_methods::CoBadgedCardData, String),
    ) -> Result<Self, Self::Error> {
        let parsed_card_type = card_type.parse::<common_enums::CardType>().map_err(|_| {
            error_stack::Report::new(errors::ParsingError::EnumParseFailure("CardType"))
        })?;

        Ok(Self {
            co_badged_card_networks: output.co_badged_card_networks,
            issuer_country: output.issuer_country_code,
            is_regulated: output.is_regulated,
            regulated_name: output.regulated_name,
            card_type: parsed_card_type,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoBadgedCardRequest {
    pub merchant_category_code: common_enums::MerchantCategoryCode,
    pub acquirer_country: common_enums::CountryAlpha2,
    pub co_badged_card_data: Option<DebitRoutingRequestData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebitRoutingRequestData {
    pub co_badged_card_networks: Vec<common_enums::CardNetwork>,
    pub issuer_country: common_enums::CountryAlpha2,
    pub is_regulated: bool,
    pub regulated_name: Option<common_enums::RegulatedName>,
    pub card_type: common_enums::CardType,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScorePayload {
    pub merchant_id: id_type::ProfileId,
    pub gateway: String,
    pub status: TxnStatus,
    pub payment_id: id_type::PaymentId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TxnStatus {
    Started,
    AuthenticationFailed,
    JuspayDeclined,
    PendingVbv,
    VBVSuccessful,
    Authorized,
    AuthorizationFailed,
    Charged,
    Authorizing,
    CODInitiated,
    Voided,
    VoidInitiated,
    Nop,
    CaptureInitiated,
    CaptureFailed,
    VoidFailed,
    AutoRefunded,
    PartialCharged,
    ToBeCharged,
    Pending,
    Failure,
    Declined,
}
