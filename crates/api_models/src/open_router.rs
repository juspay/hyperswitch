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
use utoipa::ToSchema;

use crate::{
    enums::{Currency, PaymentMethod},
    payment_methods,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterDecideGatewayRequest {
    pub payment_info: PaymentInfo,
    #[schema(value_type = String)]
    pub merchant_id: id_type::ProfileId,
    pub eligible_gateway_list: Option<Vec<String>>,
    pub ranking_algorithm: Option<RankingAlgorithm>,
    pub elimination_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DecideGatewayResponse {
    pub decided_gateway: Option<String>,
    pub gateway_priority_map: Option<serde_json::Value>,
    pub filter_wise_gateways: Option<serde_json::Value>,
    pub priority_logic_tag: Option<String>,
    pub routing_approach: Option<String>,
    pub gateway_before_evaluation: Option<String>,
    pub priority_logic_output: Option<PriorityLogicOutput>,
    pub reset_approach: Option<String>,
    pub routing_dimension: Option<String>,
    pub routing_dimension_level: Option<String>,
    pub is_scheduled_outage: Option<bool>,
    pub is_dynamic_mga_enabled: Option<bool>,
    pub gateway_mga_id_map: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriorityLogicOutput {
    pub is_enforcement: Option<bool>,
    pub gws: Option<Vec<String>>,
    pub priority_logic_tag: Option<String>,
    pub gateway_reference_ids: Option<HashMap<String, String>>,
    pub primary_logic: Option<PriorityLogicData>,
    pub fallback_logic: Option<PriorityLogicData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriorityLogicData {
    pub name: Option<String>,
    pub status: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RankingAlgorithm {
    SrBasedRouting,
    PlBasedRouting,
    NtwBasedRouting,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfo {
    #[schema(value_type = String)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DecidedGateway {
    pub gateway_priority_map: Option<HashMap<String, f64>>,
    pub debit_routing_output: Option<DebitRoutingOutput>,
    pub routing_approach: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DebitRoutingOutput {
    pub co_badged_card_networks_info: CoBadgedCardNetworks,
    pub issuer_country: common_enums::CountryAlpha2,
    pub is_regulated: bool,
    pub regulated_name: Option<common_enums::RegulatedName>,
    pub card_type: common_enums::CardType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoBadgedCardNetworksInfo {
    pub network: common_enums::CardNetwork,
    pub saving_percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoBadgedCardNetworks(pub Vec<CoBadgedCardNetworksInfo>);

impl CoBadgedCardNetworks {
    pub fn get_card_networks(&self) -> Vec<common_enums::CardNetwork> {
        self.0.iter().map(|info| info.network.clone()).collect()
    }

    pub fn get_signature_network(&self) -> Option<common_enums::CardNetwork> {
        self.0
            .iter()
            .find(|info| info.network.is_signature_network())
            .map(|info| info.network.clone())
    }
}

impl From<&DebitRoutingOutput> for payment_methods::CoBadgedCardData {
    fn from(output: &DebitRoutingOutput) -> Self {
        Self {
            co_badged_card_networks_info: output.co_badged_card_networks_info.clone(),
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
            co_badged_card_networks_info: output.co_badged_card_networks_info.get_card_networks(),
            issuer_country: output.issuer_country_code,
            is_regulated: output.is_regulated,
            regulated_name: output.regulated_name,
            card_type: parsed_card_type,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoBadgedCardRequest {
    pub merchant_category_code: common_enums::DecisionEngineMerchantCategoryCode,
    pub acquirer_country: common_enums::CountryAlpha2,
    pub co_badged_card_data: Option<DebitRoutingRequestData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebitRoutingRequestData {
    pub co_badged_card_networks_info: Vec<common_enums::CardNetwork>,
    pub issuer_country: common_enums::CountryAlpha2,
    pub is_regulated: bool,
    pub regulated_name: Option<common_enums::RegulatedName>,
    pub card_type: common_enums::CardType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
    pub status: String,
    pub error_code: String,
    pub error_message: String,
    pub priority_logic_tag: Option<String>,
    pub filter_wise_gateways: Option<serde_json::Value>,
    pub error_info: UnifiedError,
    pub is_dynamic_mga_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnifiedError {
    pub code: String,
    pub user_message: String,
    pub developer_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScorePayload {
    #[schema(value_type = String)]
    pub merchant_id: id_type::ProfileId,
    pub gateway: String,
    pub status: TxnStatus,
    #[schema(value_type = String)]
    pub payment_id: id_type::PaymentId,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateScoreResponse {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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
    VoidedPostCharge,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecisionEngineConfigSetupRequest {
    pub merchant_id: String,
    pub config: DecisionEngineConfigVariant,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetDecisionEngineConfigRequest {
    pub merchant_id: String,
    pub algorithm: DecisionEngineDynamicAlgorithmType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum DecisionEngineDynamicAlgorithmType {
    SuccessRate,
    Elimination,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum DecisionEngineConfigVariant {
    SuccessRate(DecisionEngineSuccessRateData),
    Elimination(DecisionEngineEliminationData),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngineSuccessRateData {
    pub default_latency_threshold: Option<f64>,
    pub default_bucket_size: Option<i32>,
    pub default_hedging_percent: Option<f64>,
    pub default_lower_reset_factor: Option<f64>,
    pub default_upper_reset_factor: Option<f64>,
    pub default_gateway_extra_score: Option<Vec<DecisionEngineGatewayWiseExtraScore>>,
    pub sub_level_input_config: Option<Vec<DecisionEngineSRSubLevelInputConfig>>,
}

impl DecisionEngineSuccessRateData {
    pub fn update(&mut self, new_config: Self) {
        if let Some(threshold) = new_config.default_latency_threshold {
            self.default_latency_threshold = Some(threshold);
        }
        if let Some(bucket_size) = new_config.default_bucket_size {
            self.default_bucket_size = Some(bucket_size);
        }
        if let Some(hedging_percent) = new_config.default_hedging_percent {
            self.default_hedging_percent = Some(hedging_percent);
        }
        if let Some(lower_reset_factor) = new_config.default_lower_reset_factor {
            self.default_lower_reset_factor = Some(lower_reset_factor);
        }
        if let Some(upper_reset_factor) = new_config.default_upper_reset_factor {
            self.default_upper_reset_factor = Some(upper_reset_factor);
        }
        if let Some(gateway_extra_score) = new_config.default_gateway_extra_score {
            self.default_gateway_extra_score
                .as_mut()
                .map(|score| score.extend(gateway_extra_score));
        }
        if let Some(sub_level_input_config) = new_config.sub_level_input_config {
            self.sub_level_input_config.as_mut().map(|config| {
                config.extend(sub_level_input_config);
            });
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngineSRSubLevelInputConfig {
    pub payment_method_type: Option<String>,
    pub payment_method: Option<String>,
    pub latency_threshold: Option<f64>,
    pub bucket_size: Option<i32>,
    pub hedging_percent: Option<f64>,
    pub lower_reset_factor: Option<f64>,
    pub upper_reset_factor: Option<f64>,
    pub gateway_extra_score: Option<Vec<DecisionEngineGatewayWiseExtraScore>>,
}

impl DecisionEngineSRSubLevelInputConfig {
    pub fn update(&mut self, new_config: Self) {
        if let Some(payment_method_type) = new_config.payment_method_type {
            self.payment_method_type = Some(payment_method_type);
        }
        if let Some(payment_method) = new_config.payment_method {
            self.payment_method = Some(payment_method);
        }
        if let Some(latency_threshold) = new_config.latency_threshold {
            self.latency_threshold = Some(latency_threshold);
        }
        if let Some(bucket_size) = new_config.bucket_size {
            self.bucket_size = Some(bucket_size);
        }
        if let Some(hedging_percent) = new_config.hedging_percent {
            self.hedging_percent = Some(hedging_percent);
        }
        if let Some(lower_reset_factor) = new_config.lower_reset_factor {
            self.lower_reset_factor = Some(lower_reset_factor);
        }
        if let Some(upper_reset_factor) = new_config.upper_reset_factor {
            self.upper_reset_factor = Some(upper_reset_factor);
        }
        if let Some(gateway_extra_score) = new_config.gateway_extra_score {
            self.gateway_extra_score
                .as_mut()
                .map(|score| score.extend(gateway_extra_score));
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngineGatewayWiseExtraScore {
    pub gateway_name: String,
    pub gateway_sigma_factor: f64,
}

impl DecisionEngineGatewayWiseExtraScore {
    pub fn update(&mut self, new_config: Self) {
        self.gateway_name = new_config.gateway_name;
        self.gateway_sigma_factor = new_config.gateway_sigma_factor;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngineEliminationData {
    pub threshold: f64,
}

impl DecisionEngineEliminationData {
    pub fn update(&mut self, new_config: Self) {
        self.threshold = new_config.threshold;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantAccount {
    pub merchant_id: String,
    pub gateway_success_rate_based_decider_input: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FetchRoutingConfig {
    pub merchant_id: String,
    pub algorithm: AlgorithmType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum AlgorithmType {
    SuccessRate,
    Elimination,
    DebitRouting,
}
