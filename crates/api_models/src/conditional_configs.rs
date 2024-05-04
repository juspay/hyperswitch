use common_utils::events;
use euclid::{
    dssa::types::EuclidAnalysable,
    enums,
    frontend::{
        ast::Program,
        dir::{DirKeyKind, DirValue, EuclidDirFilter},
    },
    types::Metadata,
};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumVariantNames,
    strum::EnumIter,
    strum::EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticationType {
    ThreeDs,
    NoThreeDs,
}
impl AuthenticationType {
    pub fn to_dir_value(&self) -> DirValue {
        match self {
            Self::ThreeDs => DirValue::AuthenticationType(enums::AuthenticationType::ThreeDs),
            Self::NoThreeDs => DirValue::AuthenticationType(enums::AuthenticationType::NoThreeDs),
        }
    }
}

impl EuclidAnalysable for AuthenticationType {
    fn get_dir_value_for_analysis(&self, rule_name: String) -> Vec<(DirValue, Metadata)> {
        let auth = self.to_string();

        vec![(
            self.to_dir_value(),
            std::collections::HashMap::from_iter([(
                "AUTHENTICATION_TYPE".to_string(),
                serde_json::json!({
                    "rule_name":rule_name,
                    "Authentication_type": auth,
                }),
            )]),
        )]
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConditionalConfigs {
    pub override_3ds: Option<AuthenticationType>,
}
impl EuclidDirFilter for ConditionalConfigs {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardType,
        DirKeyKind::CardNetwork,
        DirKeyKind::MetaData,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::CaptureMethod,
        DirKeyKind::BillingCountry,
        DirKeyKind::BusinessCountry,
    ];
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionManagerRecord {
    pub name: String,
    pub program: Program<ConditionalConfigs>,
    pub created_at: i64,
    pub modified_at: i64,
}
impl events::ApiEventMetric for DecisionManagerRecord {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalConfigReq {
    pub name: Option<String>,
    pub algorithm: Option<Program<ConditionalConfigs>>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]

pub struct DecisionManagerRequest {
    pub name: Option<String>,
    pub program: Option<Program<ConditionalConfigs>>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DecisionManager {
    DecisionManagerv0(ConditionalConfigReq),
    DecisionManagerv1(DecisionManagerRequest),
}

impl events::ApiEventMetric for DecisionManager {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}

pub type DecisionManagerResponse = DecisionManagerRecord;
