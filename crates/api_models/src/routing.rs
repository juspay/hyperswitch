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
        if let Some(mca_id) = &self.merchant_connector_id {
            return write!(f, "{}:{}", base, mca_id.get_string_repr());
        }
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RoutableConnectorChoiceWithStatus {
    pub routable_connector_choice: RoutableConnectorChoice,
    pub status: bool,
}

impl RoutableConnectorChoiceWithStatus {
    pub fn new(routable_connector_choice: RoutableConnectorChoice, status: bool) -> Self {
        Self {
            routable_connector_choice,
            status,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DynamicAlgorithmWithTimestamp<T> {
    pub algorithm_id: Option<T>,
    pub timestamp: i64,
}

impl<T> DynamicAlgorithmWithTimestamp<T> {
    pub fn new(algorithm_id: Option<T>) -> Self {
        Self {
            algorithm_id,
            timestamp: common_utils::date_time::now_unix_timestamp(),
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DynamicRoutingAlgorithmRef {
    pub success_based_algorithm: Option<SuccessBasedAlgorithm>,
    pub dynamic_routing_volume_split: Option<u8>,
    pub elimination_routing_algorithm: Option<EliminationRoutingAlgorithm>,
    pub contract_based_routing: Option<ContractRoutingAlgorithm>,
}

pub trait DynamicRoutingAlgoAccessor {
    fn get_algorithm_id_with_timestamp(
        self,
    ) -> DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId>;
    fn get_enabled_features(&mut self) -> &mut DynamicRoutingFeatures;
}

impl DynamicRoutingAlgoAccessor for SuccessBasedAlgorithm {
    fn get_algorithm_id_with_timestamp(
        self,
    ) -> DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId> {
        self.algorithm_id_with_timestamp
    }
    fn get_enabled_features(&mut self) -> &mut DynamicRoutingFeatures {
        &mut self.enabled_feature
    }
}

impl DynamicRoutingAlgoAccessor for EliminationRoutingAlgorithm {
    fn get_algorithm_id_with_timestamp(
        self,
    ) -> DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId> {
        self.algorithm_id_with_timestamp
    }
    fn get_enabled_features(&mut self) -> &mut DynamicRoutingFeatures {
        &mut self.enabled_feature
    }
}

impl DynamicRoutingAlgoAccessor for ContractRoutingAlgorithm {
    fn get_algorithm_id_with_timestamp(
        self,
    ) -> DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId> {
        self.algorithm_id_with_timestamp
    }
    fn get_enabled_features(&mut self) -> &mut DynamicRoutingFeatures {
        &mut self.enabled_feature
    }
}

impl EliminationRoutingAlgorithm {
    pub fn new(
        algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp<
            common_utils::id_type::RoutingId,
        >,
    ) -> Self {
        Self {
            algorithm_id_with_timestamp,
            enabled_feature: DynamicRoutingFeatures::None,
        }
    }
}

impl SuccessBasedAlgorithm {
    pub fn new(
        algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp<
            common_utils::id_type::RoutingId,
        >,
    ) -> Self {
        Self {
            algorithm_id_with_timestamp,
            enabled_feature: DynamicRoutingFeatures::None,
        }
    }
}

impl DynamicRoutingAlgorithmRef {
    pub fn update(&mut self, new: Self) {
        if let Some(elimination_routing_algorithm) = new.elimination_routing_algorithm {
            self.elimination_routing_algorithm = Some(elimination_routing_algorithm)
        }
        if let Some(success_based_algorithm) = new.success_based_algorithm {
            self.success_based_algorithm = Some(success_based_algorithm)
        }
        if let Some(contract_based_routing) = new.contract_based_routing {
            self.contract_based_routing = Some(contract_based_routing)
        }
    }

    pub fn update_enabled_features(
        &mut self,
        algo_type: DynamicRoutingType,
        feature_to_enable: DynamicRoutingFeatures,
    ) {
        match algo_type {
            DynamicRoutingType::SuccessRateBasedRouting => {
                self.success_based_algorithm
                    .as_mut()
                    .map(|algo| algo.enabled_feature = feature_to_enable);
            }
            DynamicRoutingType::EliminationRouting => {
                self.elimination_routing_algorithm
                    .as_mut()
                    .map(|algo| algo.enabled_feature = feature_to_enable);
            }
            DynamicRoutingType::ContractBasedRouting => {
                self.contract_based_routing
                    .as_mut()
                    .map(|algo| algo.enabled_feature = feature_to_enable);
            }
        }
    }

    pub fn update_volume_split(&mut self, volume: Option<u8>) {
        self.dynamic_routing_volume_split = volume
    }
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RoutingVolumeSplit {
    pub routing_type: RoutingType,
    pub split: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingVolumeSplitWrapper {
    pub routing_info: RoutingVolumeSplit,
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingType {
    #[default]
    Static,
    Dynamic,
}

impl RoutingType {
    pub fn is_dynamic_routing(self) -> bool {
        self == Self::Dynamic
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuccessBasedAlgorithm {
    pub algorithm_id_with_timestamp:
        DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId>,
    #[serde(default)]
    pub enabled_feature: DynamicRoutingFeatures,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractRoutingAlgorithm {
    pub algorithm_id_with_timestamp:
        DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId>,
    #[serde(default)]
    pub enabled_feature: DynamicRoutingFeatures,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EliminationRoutingAlgorithm {
    pub algorithm_id_with_timestamp:
        DynamicAlgorithmWithTimestamp<common_utils::id_type::RoutingId>,
    #[serde(default)]
    pub enabled_feature: DynamicRoutingFeatures,
}

impl EliminationRoutingAlgorithm {
    pub fn update_enabled_features(&mut self, feature_to_enable: DynamicRoutingFeatures) {
        self.enabled_feature = feature_to_enable
    }
}

impl SuccessBasedAlgorithm {
    pub fn update_enabled_features(&mut self, feature_to_enable: DynamicRoutingFeatures) {
        self.enabled_feature = feature_to_enable
    }
}

impl DynamicRoutingAlgorithmRef {
    pub fn update_algorithm_id(
        &mut self,
        new_id: common_utils::id_type::RoutingId,
        enabled_feature: DynamicRoutingFeatures,
        dynamic_routing_type: DynamicRoutingType,
    ) {
        match dynamic_routing_type {
            DynamicRoutingType::SuccessRateBasedRouting => {
                self.success_based_algorithm = Some(SuccessBasedAlgorithm {
                    algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp::new(Some(new_id)),
                    enabled_feature,
                })
            }
            DynamicRoutingType::EliminationRouting => {
                self.elimination_routing_algorithm = Some(EliminationRoutingAlgorithm {
                    algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp::new(Some(new_id)),
                    enabled_feature,
                })
            }
            DynamicRoutingType::ContractBasedRouting => {
                self.contract_based_routing = Some(ContractRoutingAlgorithm {
                    algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp::new(Some(new_id)),
                    enabled_feature,
                })
            }
        };
    }

    pub fn disable_algorithm_id(&mut self, dynamic_routing_type: DynamicRoutingType) {
        match dynamic_routing_type {
            DynamicRoutingType::SuccessRateBasedRouting => {
                if let Some(success_based_algo) = &self.success_based_algorithm {
                    self.success_based_algorithm = Some(SuccessBasedAlgorithm {
                        algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp::new(None),
                        enabled_feature: success_based_algo.enabled_feature,
                    });
                }
            }
            DynamicRoutingType::EliminationRouting => {
                if let Some(elimination_based_algo) = &self.elimination_routing_algorithm {
                    self.elimination_routing_algorithm = Some(EliminationRoutingAlgorithm {
                        algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp::new(None),
                        enabled_feature: elimination_based_algo.enabled_feature,
                    });
                }
            }
            DynamicRoutingType::ContractBasedRouting => {
                if let Some(contract_based_algo) = &self.contract_based_routing {
                    self.contract_based_routing = Some(ContractRoutingAlgorithm {
                        algorithm_id_with_timestamp: DynamicAlgorithmWithTimestamp::new(None),
                        enabled_feature: contract_based_algo.enabled_feature,
                    });
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ToggleDynamicRoutingQuery {
    pub enable: DynamicRoutingFeatures,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct DynamicRoutingVolumeSplitQuery {
    pub split: u8,
}

#[derive(
    Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DynamicRoutingFeatures {
    Metrics,
    DynamicConnectorSelection,
    #[default]
    None,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct DynamicRoutingUpdateConfigQuery {
    #[schema(value_type = String)]
    pub algorithm_id: common_utils::id_type::RoutingId,
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ToggleDynamicRoutingWrapper {
    pub profile_id: common_utils::id_type::ProfileId,
    pub feature_to_enable: DynamicRoutingFeatures,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct ToggleDynamicRoutingPath {
    #[schema(value_type = String)]
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct EliminationRoutingConfig {
    pub params: Option<Vec<DynamicRoutingConfigParams>>,
    pub elimination_analyser_config: Option<EliminationAnalyserConfig>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct EliminationAnalyserConfig {
    pub bucket_size: Option<u64>,
    pub bucket_leak_interval_in_secs: Option<u64>,
}

impl Default for EliminationRoutingConfig {
    fn default() -> Self {
        Self {
            params: Some(vec![DynamicRoutingConfigParams::PaymentMethod]),
            elimination_analyser_config: Some(EliminationAnalyserConfig {
                bucket_size: Some(5),
                bucket_leak_interval_in_secs: Some(2),
            }),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct SuccessBasedRoutingConfig {
    pub params: Option<Vec<DynamicRoutingConfigParams>>,
    pub config: Option<SuccessBasedRoutingConfigBody>,
}

impl Default for SuccessBasedRoutingConfig {
    fn default() -> Self {
        Self {
            params: Some(vec![DynamicRoutingConfigParams::PaymentMethod]),
            config: Some(SuccessBasedRoutingConfigBody {
                min_aggregates_size: Some(2),
                default_success_rate: Some(100.0),
                max_aggregates_size: Some(3),
                current_block_threshold: Some(CurrentBlockThreshold {
                    duration_in_mins: Some(5),
                    max_total_count: Some(2),
                }),
                specificity_level: SuccessRateSpecificityLevel::default(),
            }),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema, strum::Display)]
pub enum DynamicRoutingConfigParams {
    PaymentMethod,
    PaymentMethodType,
    AuthenticationType,
    Currency,
    Country,
    CardNetwork,
    CardBin,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct SuccessBasedRoutingConfigBody {
    pub min_aggregates_size: Option<u32>,
    pub default_success_rate: Option<f64>,
    pub max_aggregates_size: Option<u32>,
    pub current_block_threshold: Option<CurrentBlockThreshold>,
    #[serde(default)]
    pub specificity_level: SuccessRateSpecificityLevel,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
pub struct CurrentBlockThreshold {
    pub duration_in_mins: Option<u64>,
    pub max_total_count: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuccessRateSpecificityLevel {
    #[default]
    Merchant,
    Global,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuccessBasedRoutingPayloadWrapper {
    pub updated_config: SuccessBasedRoutingConfig,
    pub algorithm_id: common_utils::id_type::RoutingId,
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractBasedRoutingPayloadWrapper {
    pub updated_config: ContractBasedRoutingConfig,
    pub algorithm_id: common_utils::id_type::RoutingId,
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractBasedRoutingSetupPayloadWrapper {
    pub config: Option<ContractBasedRoutingConfig>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub features_to_enable: DynamicRoutingFeatures,
}

#[derive(
    Debug, Clone, Copy, strum::Display, serde::Serialize, serde::Deserialize, PartialEq, Eq,
)]
pub enum DynamicRoutingType {
    SuccessRateBasedRouting,
    EliminationRouting,
    ContractBasedRouting,
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
        self.specificity_level = new.specificity_level
    }
}

impl CurrentBlockThreshold {
    pub fn update(&mut self, new: Self) {
        if let Some(max_total_count) = new.max_total_count {
            self.max_total_count = Some(max_total_count)
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractBasedRoutingConfig {
    pub config: Option<ContractBasedRoutingConfigBody>,
    pub label_info: Option<Vec<LabelInformation>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractBasedRoutingConfigBody {
    pub constants: Option<Vec<f64>>,
    pub time_scale: Option<ContractBasedTimeScale>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct LabelInformation {
    pub label: String,
    pub target_count: u64,
    pub target_time: u64,
    #[schema(value_type = String)]
    pub mca_id: common_utils::id_type::MerchantConnectorAccountId,
}

impl LabelInformation {
    pub fn update_target_time(&mut self, new: &Self) {
        self.target_time = new.target_time;
    }

    pub fn update_target_count(&mut self, new: &Self) {
        self.target_count = new.target_count;
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractBasedTimeScale {
    Day,
    Month,
}

impl Default for ContractBasedRoutingConfig {
    fn default() -> Self {
        Self {
            config: Some(ContractBasedRoutingConfigBody {
                constants: Some(vec![0.7, 0.35]),
                time_scale: Some(ContractBasedTimeScale::Day),
            }),
            label_info: None,
        }
    }
}

impl ContractBasedRoutingConfig {
    pub fn update(&mut self, new: Self) {
        if let Some(new_config) = new.config {
            self.config.as_mut().map(|config| config.update(new_config));
        }
        if let Some(new_label_info) = new.label_info {
            new_label_info.iter().for_each(|new_label_info| {
                if let Some(existing_label_infos) = &mut self.label_info {
                    for existing_label_info in existing_label_infos {
                        if existing_label_info.mca_id == new_label_info.mca_id {
                            existing_label_info.update_target_time(new_label_info);
                            existing_label_info.update_target_count(new_label_info);
                        }
                    }
                } else {
                    self.label_info = Some(vec![new_label_info.clone()]);
                }
            });
        }
    }
}

impl ContractBasedRoutingConfigBody {
    pub fn update(&mut self, new: Self) {
        if let Some(new_cons) = new.constants {
            self.constants = Some(new_cons)
        }
        if let Some(new_time_scale) = new.time_scale {
            self.time_scale = Some(new_time_scale)
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RoutableConnectorChoiceWithBucketName {
    pub routable_connector_choice: RoutableConnectorChoice,
    pub bucket_name: String,
}

impl RoutableConnectorChoiceWithBucketName {
    pub fn new(routable_connector_choice: RoutableConnectorChoice, bucket_name: String) -> Self {
        Self {
            routable_connector_choice,
            bucket_name,
        }
    }
}
