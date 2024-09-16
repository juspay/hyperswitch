use std::fmt::Debug;

use common_utils::{errors::ParsingError, ext_traits::ValueExt, pii};
pub use euclid::{
    dssa::types::EuclidAnalysable,
    frontend::{
        ast,
        dir::{DirKeyKind, EuclidDirFilter},
    },
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums::{RoutableConnectors, TransactionType};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ConnectorSelection {
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
}

impl ConnectorSelection {
    pub fn get_connector_list(&self) -> Vec<RoutableConnectorChoice> {
        match self {
            Self::Priority(list) => list.clone(),
            Self::VolumeSplit(splits) => {
                splits.iter().map(|split| split.connector.clone()).collect()
            }
        }
    }
}
#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct RoutingConfigRequest {
    pub name: String,
    pub description: String,
    pub algorithm: RoutingAlgorithm,
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct RoutingConfigRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub algorithm: Option<RoutingAlgorithm>,
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<common_utils::id_type::ProfileId>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ProfileDefaultRoutingConfig {
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
    pub connectors: Vec<RoutableConnectorChoice>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RoutingRetrieveQuery {
    pub limit: Option<u16>,
    pub offset: Option<u8>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RoutingRetrieveLinkQuery {
    pub profile_id: Option<common_utils::id_type::ProfileId>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RoutingRetrieveLinkQueryWrapper {
    pub routing_query: RoutingRetrieveQuery,
    pub profile_id: common_utils::id_type::ProfileId,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
/// Response of the retrieved routing configs for a merchant account
pub struct RoutingRetrieveResponse {
    pub algorithm: Option<MerchantRoutingAlgorithm>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(untagged)]
pub enum LinkedRoutingConfigRetrieveResponse {
    MerchantAccountBased(RoutingRetrieveResponse),
    ProfileBased(Vec<RoutingDictionaryRecord>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
/// Routing Algorithm specific to merchants
pub struct MerchantRoutingAlgorithm {
    #[schema(value_type = String)]
    pub id: common_utils::id_type::RoutingId,
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
    pub name: String,
    pub description: String,
    pub algorithm: RoutingAlgorithm,
    pub created_at: i64,
    pub modified_at: i64,
    pub algorithm_for: TransactionType,
}

impl EuclidDirFilter for ConnectorSelection {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardBin,
        DirKeyKind::CardType,
        DirKeyKind::CardNetwork,
        DirKeyKind::PayLaterType,
        DirKeyKind::WalletType,
        DirKeyKind::UpiType,
        DirKeyKind::BankRedirectType,
        DirKeyKind::BankDebitType,
        DirKeyKind::CryptoType,
        DirKeyKind::MetaData,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::AuthenticationType,
        DirKeyKind::MandateAcceptanceType,
        DirKeyKind::MandateType,
        DirKeyKind::PaymentType,
        DirKeyKind::SetupFutureUsage,
        DirKeyKind::CaptureMethod,
        DirKeyKind::BillingCountry,
        DirKeyKind::BusinessCountry,
        DirKeyKind::BusinessLabel,
        DirKeyKind::MetaData,
        DirKeyKind::RewardType,
        DirKeyKind::VoucherType,
        DirKeyKind::CardRedirectType,
        DirKeyKind::BankTransferType,
        DirKeyKind::RealTimePaymentType,
    ];
}

impl EuclidAnalysable for ConnectorSelection {
    fn get_dir_value_for_analysis(
        &self,
        rule_name: String,
    ) -> Vec<(euclid::frontend::dir::DirValue, euclid::types::Metadata)> {
        self.get_connector_list()
            .into_iter()
            .map(|connector_choice| {
                let connector_name = connector_choice.connector.to_string();
                let mca_id = connector_choice.merchant_connector_id.clone();

                (
                    euclid::frontend::dir::DirValue::Connector(Box::new(connector_choice.into())),
                    std::collections::HashMap::from_iter([(
                        "CONNECTOR_SELECTION".to_string(),
                        serde_json::json!({
                            "rule_name": rule_name,
                            "connector_name": connector_name,
                            "mca_id": mca_id,
                        }),
                    )]),
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConnectorVolumeSplit {
    pub connector: RoutableConnectorChoice,
    pub split: u8,
}

/// Routable Connector chosen for a payment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(from = "RoutableChoiceSerde", into = "RoutableChoiceSerde")]
pub struct RoutableConnectorChoice {
    #[serde(skip)]
    pub choice_kind: RoutableChoiceKind,
    pub connector: RoutableConnectors,
    #[schema(value_type = Option<String>)]
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub enum RoutableChoiceKind {
    OnlyConnector,
    FullStruct,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum RoutableChoiceSerde {
    OnlyConnector(Box<RoutableConnectors>),
    FullStruct {
        connector: RoutableConnectors,
        merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    },
}

impl std::fmt::Display for RoutableConnectorChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = self.connector.to_string();

        write!(f, "{}", base)
    }
}

impl From<RoutableConnectorChoice> for ast::ConnectorChoice {
    fn from(value: RoutableConnectorChoice) -> Self {
        Self {
            connector: value.connector,
        }
    }
}

impl PartialEq for RoutableConnectorChoice {
    fn eq(&self, other: &Self) -> bool {
        self.connector.eq(&other.connector)
            && self.merchant_connector_id.eq(&other.merchant_connector_id)
    }
}

impl Eq for RoutableConnectorChoice {}

impl From<RoutableChoiceSerde> for RoutableConnectorChoice {
    fn from(value: RoutableChoiceSerde) -> Self {
        match value {
            RoutableChoiceSerde::OnlyConnector(connector) => Self {
                choice_kind: RoutableChoiceKind::OnlyConnector,
                connector: *connector,
                merchant_connector_id: None,
            },

            RoutableChoiceSerde::FullStruct {
                connector,
                merchant_connector_id,
            } => Self {
                choice_kind: RoutableChoiceKind::FullStruct,
                connector,
                merchant_connector_id,
            },
        }
    }
}

impl From<RoutableConnectorChoice> for RoutableChoiceSerde {
    fn from(value: RoutableConnectorChoice) -> Self {
        match value.choice_kind {
            RoutableChoiceKind::OnlyConnector => Self::OnlyConnector(Box::new(value.connector)),
            RoutableChoiceKind::FullStruct => Self::FullStruct {
                connector: value.connector,
                merchant_connector_id: value.merchant_connector_id,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize, strum::Display, ToSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutingAlgorithmKind {
    Single,
    Priority,
    VolumeSplit,
    Advanced,
    Dynamic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingPayloadWrapper {
    pub updated_config: Vec<RoutableConnectorChoice>,
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    try_from = "RoutingAlgorithmSerde"
)]
/// Routing Algorithm kind
pub enum RoutingAlgorithm {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
    #[schema(value_type=ProgramConnectorSelection)]
    Advanced(ast::Program<ConnectorSelection>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RoutingAlgorithmSerde {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
    Advanced(ast::Program<ConnectorSelection>),
}

impl TryFrom<RoutingAlgorithmSerde> for RoutingAlgorithm {
    type Error = error_stack::Report<ParsingError>;

    fn try_from(value: RoutingAlgorithmSerde) -> Result<Self, Self::Error> {
        match &value {
            RoutingAlgorithmSerde::Priority(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Priority Algorithm",
                ))?
            }
            RoutingAlgorithmSerde::VolumeSplit(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Volume split Algorithm",
                ))?
            }
            _ => {}
        };
        Ok(match value {
            RoutingAlgorithmSerde::Single(i) => Self::Single(i),
            RoutingAlgorithmSerde::Priority(i) => Self::Priority(i),
            RoutingAlgorithmSerde::VolumeSplit(i) => Self::VolumeSplit(i),
            RoutingAlgorithmSerde::Advanced(i) => Self::Advanced(i),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    try_from = "StraightThroughAlgorithmSerde",
    into = "StraightThroughAlgorithmSerde"
)]
pub enum StraightThroughAlgorithm {
    #[schema(title = "Single")]
    Single(Box<RoutableConnectorChoice>),
    #[schema(title = "Priority")]
    Priority(Vec<RoutableConnectorChoice>),
    #[schema(title = "VolumeSplit")]
    VolumeSplit(Vec<ConnectorVolumeSplit>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum StraightThroughAlgorithmInner {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplit>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum StraightThroughAlgorithmSerde {
    Direct(StraightThroughAlgorithmInner),
    Nested {
        algorithm: StraightThroughAlgorithmInner,
    },
}

impl TryFrom<StraightThroughAlgorithmSerde> for StraightThroughAlgorithm {
    type Error = error_stack::Report<ParsingError>;

    fn try_from(value: StraightThroughAlgorithmSerde) -> Result<Self, Self::Error> {
        let inner = match value {
            StraightThroughAlgorithmSerde::Direct(algorithm) => algorithm,
            StraightThroughAlgorithmSerde::Nested { algorithm } => algorithm,
        };

        match &inner {
            StraightThroughAlgorithmInner::Priority(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Priority Algorithm",
                ))?
            }
            StraightThroughAlgorithmInner::VolumeSplit(i) if i.is_empty() => {
                Err(ParsingError::StructParseFailure(
                    "Connectors list can't be empty for Volume split Algorithm",
                ))?
            }
            _ => {}
        };

        Ok(match inner {
            StraightThroughAlgorithmInner::Single(single) => Self::Single(single),
            StraightThroughAlgorithmInner::Priority(plist) => Self::Priority(plist),
            StraightThroughAlgorithmInner::VolumeSplit(vsplit) => Self::VolumeSplit(vsplit),
        })
    }
}

impl From<StraightThroughAlgorithm> for StraightThroughAlgorithmSerde {
    fn from(value: StraightThroughAlgorithm) -> Self {
        let inner = match value {
            StraightThroughAlgorithm::Single(conn) => StraightThroughAlgorithmInner::Single(conn),
            StraightThroughAlgorithm::Priority(plist) => {
                StraightThroughAlgorithmInner::Priority(plist)
            }
            StraightThroughAlgorithm::VolumeSplit(vsplit) => {
                StraightThroughAlgorithmInner::VolumeSplit(vsplit)
            }
        };

        Self::Nested { algorithm: inner }
    }
}

impl From<StraightThroughAlgorithm> for RoutingAlgorithm {
    fn from(value: StraightThroughAlgorithm) -> Self {
        match value {
            StraightThroughAlgorithm::Single(conn) => Self::Single(conn),
            StraightThroughAlgorithm::Priority(conns) => Self::Priority(conns),
            StraightThroughAlgorithm::VolumeSplit(splits) => Self::VolumeSplit(splits),
        }
    }
}

impl RoutingAlgorithm {
    pub fn get_kind(&self) -> RoutingAlgorithmKind {
        match self {
            Self::Single(_) => RoutingAlgorithmKind::Single,
            Self::Priority(_) => RoutingAlgorithmKind::Priority,
            Self::VolumeSplit(_) => RoutingAlgorithmKind::VolumeSplit,
            Self::Advanced(_) => RoutingAlgorithmKind::Advanced,
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingAlgorithmRef {
    pub algorithm_id: Option<common_utils::id_type::RoutingId>,
    pub timestamp: i64,
    pub config_algo_id: Option<String>,
    pub surcharge_config_algo_id: Option<String>,
}

impl RoutingAlgorithmRef {
    pub fn update_algorithm_id(&mut self, new_id: common_utils::id_type::RoutingId) {
        self.algorithm_id = Some(new_id);
        self.timestamp = common_utils::date_time::now_unix_timestamp();
    }

    pub fn update_conditional_config_id(&mut self, ids: String) {
        self.config_algo_id = Some(ids);
        self.timestamp = common_utils::date_time::now_unix_timestamp();
    }

    pub fn update_surcharge_config_id(&mut self, ids: String) {
        self.surcharge_config_algo_id = Some(ids);
        self.timestamp = common_utils::date_time::now_unix_timestamp();
    }

    pub fn parse_routing_algorithm(
        value: Option<pii::SecretSerdeValue>,
    ) -> Result<Option<Self>, error_stack::Report<ParsingError>> {
        value
            .map(|val| val.parse_value::<Self>("RoutingAlgorithmRef"))
            .transpose()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]

pub struct RoutingDictionaryRecord {
    #[schema(value_type = String)]
    pub id: common_utils::id_type::RoutingId,
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
    pub name: String,
    pub kind: RoutingAlgorithmKind,
    pub description: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub algorithm_for: Option<TransactionType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct RoutingDictionary {
    #[schema(value_type = String)]
    pub merchant_id: common_utils::id_type::MerchantId,
    pub active_id: Option<String>,
    pub records: Vec<RoutingDictionaryRecord>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, ToSchema)]
#[serde(untagged)]
pub enum RoutingKind {
    Config(RoutingDictionary),
    RoutingAlgorithm(Vec<RoutingDictionaryRecord>),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct RoutingAlgorithmId {
    #[schema(value_type = String)]
    pub routing_algorithm_id: common_utils::id_type::RoutingId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingLinkWrapper {
    pub profile_id: common_utils::id_type::ProfileId,
    pub algorithm_id: RoutingAlgorithmId,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DynamicAlgorithmWithTimestamp<T> {
    pub algorithm_id: Option<T>,
    pub timestamp: i64,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DynamicRoutingAlgorithmRef {
    pub success_based_algorithm:
        Option<DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId>>,
}

impl DynamicRoutingAlgorithmRef {
    pub fn update_algorithm_id(&mut self, new_id: common_utils::id_type::RoutingId) {
        self.success_based_algorithm = Some(DynamicAlgorithmWithTimestamp {
            algorithm_id: Some(new_id),
            timestamp: common_utils::date_time::now_unix_timestamp(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ToggleSuccessBasedRoutingQuery {
    pub status: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct SuccessBasedRoutingUpdateConfigQuery {
    #[schema(value_type = String)]
    pub algorithm_id: common_utils::id_type::RoutingId,
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ToggleSuccessBasedRoutingWrapper {
    pub profile_id: common_utils::id_type::ProfileId,
    pub status: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ToggleSuccessBasedRoutingPath {
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct SuccessBasedRoutingConfig {
    pub params: Option<Vec<SuccessBasedRoutingConfigParams>>,
    pub config: Option<SuccessBasedRoutingConfigBody>,
}

impl Default for SuccessBasedRoutingConfig {
    fn default() -> Self {
        Self {
            params: Some(vec![SuccessBasedRoutingConfigParams::PaymentMethod]),
            config: Some(SuccessBasedRoutingConfigBody {
                min_aggregates_size: Some(2),
                default_success_rate: Some(100.0),
                max_aggregates_size: Some(3),
                current_block_threshold: Some(CurrentBlockThreshold {
                    duration_in_mins: Some(5),
                    max_total_count: Some(2),
                }),
            }),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub enum SuccessBasedRoutingConfigParams {
    PaymentMethod,
    PaymentMethodType,
    Currency,
    AuthenticationType,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct SuccessBasedRoutingConfigBody {
    pub min_aggregates_size: Option<u32>,
    pub default_success_rate: Option<f64>,
    pub max_aggregates_size: Option<u32>,
    pub current_block_threshold: Option<CurrentBlockThreshold>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct CurrentBlockThreshold {
    pub duration_in_mins: Option<u64>,
    pub max_total_count: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuccessBasedRoutingPayloadWrapper {
    pub updated_config: SuccessBasedRoutingConfig,
    pub algorithm_id: common_utils::id_type::RoutingId,
    pub profile_id: common_utils::id_type::ProfileId,
}

impl SuccessBasedRoutingConfig {
    pub fn update(&mut self, new: Self) {
        if let Some(params) = new.params {
            self.params = Some(params)
        }
        if let Some(new_config) = new.config {
            self.config.as_mut().map(|config| config.update(new_config));
        }
    }
}

impl SuccessBasedRoutingConfigBody {
    pub fn update(&mut self, new: Self) {
        if let Some(min_aggregates_size) = new.min_aggregates_size {
            self.min_aggregates_size = Some(min_aggregates_size)
        }
        if let Some(default_success_rate) = new.default_success_rate {
            self.default_success_rate = Some(default_success_rate)
        }
        if let Some(max_aggregates_size) = new.max_aggregates_size {
            self.max_aggregates_size = Some(max_aggregates_size)
        }
        if let Some(current_block_threshold) = new.current_block_threshold {
            self.current_block_threshold
                .as_mut()
                .map(|threshold| threshold.update(current_block_threshold));
        }
    }
}

impl CurrentBlockThreshold {
    pub fn update(&mut self, new: Self) {
        if let Some(max_total_count) = new.max_total_count {
            self.max_total_count = Some(max_total_count)
        }
    }
}
