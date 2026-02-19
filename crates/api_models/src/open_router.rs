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

/// Request to decide the optimal gateway for routing a payment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterDecideGatewayRequest {
    /// Payment information for routing decision
    pub payment_info: PaymentInfo,

    /// Profile ID of the merchant
    #[schema(value_type = String, example = "pro_aMoPnEkgCVnh2WVsFe32")]
    pub merchant_id: id_type::ProfileId,

    /// List of eligible gateways for routing consideration
    #[schema(value_type = Option<Vec<String>>, example = "[\"stripe:mca_123\", \"adyen:mca_456\"]")]
    pub eligible_gateway_list: Option<Vec<String>>,

    /// Algorithm to use for ranking and selecting gateways
    #[schema(value_type = Option<RankingAlgorithm>, example = "SR_BASED_ROUTING")]
    pub ranking_algorithm: Option<RankingAlgorithm>,

    /// Whether elimination logic is enabled for filtering gateways
    #[schema(value_type = Option<bool>, example = true)]
    pub elimination_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DecideGatewayResponse {
    /// The gateway decided by the routing engine
    #[schema(value_type = Option<String>, example = "stripe:mca1")]
    pub decided_gateway: Option<String>,

    /// Map of gateways with their priority scores
    #[schema(value_type = Option<HashMap<String, f64>>, example = json!({"stripe:mca1": 1.0, "adyen:mca2": 1.0}))]
    pub gateway_priority_map: Option<serde_json::Value>,

    /// Gateways organized by filter criteria
    #[schema(value_type = Option<Object>)]
    pub filter_wise_gateways: Option<serde_json::Value>,

    /// Tag identifying the priority logic used
    #[schema(value_type = Option<String>)]
    pub priority_logic_tag: Option<String>,

    /// The routing approach used for decision making
    #[schema(value_type = Option<String>, example = "SR_SELECTION_V3_ROUTING")]
    pub routing_approach: Option<String>,

    /// The gateway that was evaluated before the final decision
    #[schema(value_type = Option<String>, example = "adyen:mca2")]
    pub gateway_before_evaluation: Option<String>,

    /// Detailed output from the priority logic evaluation
    #[schema(value_type = Option<PriorityLogicOutput>)]
    pub priority_logic_output: Option<PriorityLogicOutput>,

    /// The reset approach applied during routing
    #[schema(value_type = Option<String>, example = "NO_RESET")]
    pub reset_approach: Option<String>,

    /// Dimensions used for routing decision (payment type, method, etc.)
    #[schema(value_type = Option<String>, example = "ORDER_PAYMENT, UPI, upi")]
    pub routing_dimension: Option<String>,

    /// Level at which routing dimension is evaluated
    #[schema(value_type = Option<String>, example = "PM_LEVEL")]
    pub routing_dimension_level: Option<String>,

    /// Indicates if routing decision was affected by scheduled outage
    #[schema(value_type = Option<bool>, example = false)]
    pub is_scheduled_outage: Option<bool>,

    /// Indicates if dynamic merchant gateway account is enabled
    #[schema(value_type = Option<bool>, example = false)]
    pub is_dynamic_mga_enabled: Option<bool>,

    /// Map of gateways to their MGA IDs
    #[schema(value_type = Option<Object>)]
    pub gateway_mga_id_map: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriorityLogicOutput {
    /// Whether enforcement mode is enabled
    #[schema(value_type = Option<bool>, example = false)]
    pub is_enforcement: Option<bool>,

    /// List of gateways returned by the priority logic
    #[schema(value_type = Option<Vec<String>>, example = json!(["stripe:mca1", "adyen:mca2"]))]
    pub gws: Option<Vec<String>>,

    /// Tag identifying the priority logic used
    #[schema(value_type = Option<String>)]
    pub priority_logic_tag: Option<String>,

    /// Map of gateway reference IDs
    #[schema(value_type = Option<Object>, example = json!({}))]
    pub gateway_reference_ids: Option<HashMap<String, String>>,

    /// Primary logic details
    #[schema(value_type = Option<PriorityLogicData>)]
    pub primary_logic: Option<PriorityLogicData>,

    /// Fallback logic details
    #[schema(value_type = Option<PriorityLogicData>)]
    pub fallback_logic: Option<PriorityLogicData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriorityLogicData {
    /// Name of the logic
    #[schema(value_type = Option<String>, example = "success_rate_logic")]
    pub name: Option<String>,

    /// Status of the logic execution
    #[schema(value_type = Option<String>, example = "success")]
    pub status: Option<String>,

    /// Reason for failure if the logic failed
    #[schema(value_type = Option<String>, example = "insufficient_data")]
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RankingAlgorithm {
    SrBasedRouting,
    PlBasedRouting,
    NtwBasedRouting,
}

/// Payment information used for routing decision-making
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfo {
    /// Unique identifier for the payment transaction
    #[schema(value_type = String, example = "pay_12345")]
    pub payment_id: id_type::PaymentId,
    /// Payment amount in minor units
    #[schema(value_type = i64, example = "100")]
    pub amount: MinorUnit,
    /// Currency code for the payment
    #[schema(value_type = String, example = "USD")]
    pub currency: Currency,
    // customerId: Option<ETCu::CustomerId>,
    // preferredGateway: Option<ETG::Gateway>,
    /// Type of payment transaction being processed
    #[schema(value_type = String, example = "ORDER_PAYMENT")]
    pub payment_type: String,
    /// Optional metadata associated with the payment
    #[schema(value_type = String, example = "metadata")]
    pub metadata: Option<String>,
    // internalMetadata: Option<String>,
    // isEmi: Option<bool>,
    // emiBank: Option<String>,
    // emiTenure: Option<i32>,
    /// Specific payment method type being used
    #[schema(value_type = String, example = "upi")]
    pub payment_method_type: String,
    /// General payment method category
    #[schema(value_type = String, example = "upi")]
    pub payment_method: PaymentMethod,
    // paymentSource: Option<String>,
    // authType: Option<ETCa::txn_card_info::AuthType>,
    // cardIssuerBankName: Option<String>,
    /// Card Issuer Identification Number (first 6 digits of card)
    #[schema(value_type = String, example = "424242")]
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
    pub saving_percentage: Option<f64>,
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

/// Request payload to update gateway performance score based on transaction outcome
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScorePayload {
    /// Profile ID of the merchant
    #[schema(value_type = String, example = "pro_aMoPnEkgCVnh2WVsFe32")]
    pub merchant_id: id_type::ProfileId,

    /// Payment Gateway identifier
    #[schema(value_type = String, example = "stripe:mca1")]
    pub gateway: String,

    /// Transaction status for feedback scoring
    #[schema(value_type = TxnStatus, example = "CHARGED")]
    pub status: TxnStatus,

    /// Payment ID associated with the transaction
    #[schema(value_type = String, example = "pay_1234")]
    pub payment_id: id_type::PaymentId,
}

/// Response after updating gateway score
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateScoreResponse {
    /// Status message indicating the result of the score update
    #[schema(value_type = String, example = "Gateway score updated successfully")]
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

/// Configuration for Decision Engine success rate based routing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngineSuccessRateData {
    /// Default latency threshold in percentile for gateway selection
    #[schema(value_type = Option<f64>, example = 90.0)]
    pub default_latency_threshold: Option<f64>,

    /// Default number of transactions to consider for success rate calculation
    #[schema(value_type = Option<i32>, example = 100)]
    pub default_bucket_size: Option<i32>,

    /// Default percentage of traffic to route for exploration/hedging
    #[schema(value_type = Option<f64>, example = 5.0)]
    pub default_hedging_percent: Option<f64>,

    /// Lower reset factor for adjusting gateway scores
    #[schema(value_type = Option<f64>, example = 0.5)]
    pub default_lower_reset_factor: Option<f64>,

    /// Upper reset factor for adjusting gateway scores
    #[schema(value_type = Option<f64>, example = 1.5)]
    pub default_upper_reset_factor: Option<f64>,

    /// Gateway-specific extra scoring factors
    #[schema(value_type = Option<Vec<DecisionEngineGatewayWiseExtraScore>>)]
    pub default_gateway_extra_score: Option<Vec<DecisionEngineGatewayWiseExtraScore>>,

    /// Payment method level specific configurations
    #[schema(value_type = Option<Vec<DecisionEngineSRSubLevelInputConfig>>)]
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

/// Payment method level configuration for success rate based routing
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngineSRSubLevelInputConfig {
    /// Payment method type (e.g., "card", "wallet")
    #[schema(value_type = Option<String>, example = "card")]
    pub payment_method_type: Option<String>,

    /// Specific payment method (e.g., "credit", "debit")
    #[schema(value_type = Option<String>, example = "credit")]
    pub payment_method: Option<String>,

    /// Latency threshold in percentile for this payment method
    #[schema(value_type = Option<f64>, example = 90.0)]
    pub latency_threshold: Option<f64>,

    /// Number of transactions to consider for this payment method
    #[schema(value_type = Option<i32>, example = 100)]
    pub bucket_size: Option<i32>,

    /// Percentage of traffic to route for exploration for this payment method
    #[schema(value_type = Option<f64>, example = 5.0)]
    pub hedging_percent: Option<f64>,

    /// Lower reset factor for this payment method
    #[schema(value_type = Option<f64>, example = 0.5)]
    pub lower_reset_factor: Option<f64>,

    /// Upper reset factor for this payment method
    #[schema(value_type = Option<f64>, example = 1.5)]
    pub upper_reset_factor: Option<f64>,

    /// Gateway-specific extra scoring factors for this payment method
    #[schema(value_type = Option<Vec<DecisionEngineGatewayWiseExtraScore>>)]
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
    /// Threshold for elimination logic in gateway selection
    #[schema(value_type = f64, example = 0.3)]
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
