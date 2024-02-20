use super::NameDescription;

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DisputeMetrics {
    DisputesChallenged,
    DisputesWon,
    DisputesLost,
    TotalAmountDisputed,
    TotalDisputeLostAmount,
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::AsRefStr,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    strum::Display,
    strum::EnumIter,
    Clone,
    Copy,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DisputeDimensions {
    // Do not change the order of these enums
    // Consult the Dashboard FE folks since these also affects the order of metrics on FE
    Connector,
    DisputeStatus,
    ConnectorStatus,
}

impl From<DisputeDimensions> for NameDescription {
    fn from(value: DisputeDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<DisputeMetrics> for NameDescription {
    fn from(value: DisputeMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}
