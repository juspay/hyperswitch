pub mod cloudwatch;
pub mod message;

use common_types::alert_rules::{AlertRuleConfig, Severity, SeverityThreshold};
use common_utils::errors::CustomResult;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum Level {
    Normal,
    NoData,
    Info,
    Warning,
    Critical,
}

impl From<Severity> for Level {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Critical => Self::Critical,
            Severity::Warning => Self::Warning,
            Severity::Info => Self::Info,
        }
    }
}

pub struct Notification {
    pub heading: String,
    pub destination: String,
    pub message: String,
}

pub enum RuleRun {
    Evaluated {
        evaluation: Evaluation,
        notification: Option<Notification>,
    },
    Failed {
        error: String,
        notification: Notification,
    },
}

impl RuleRun {
    pub fn notification(&self) -> Option<&Notification> {
        match self {
            Self::Evaluated { notification, .. } => notification.as_ref(),
            Self::Failed { notification, .. } => Some(notification),
        }
    }
}

pub struct Delivery {
    pub delivered: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub level: Level,
    pub latest_value: Option<f64>,
    pub breach: Option<SeverityThreshold>,
    pub window: external_services::metrics_provider::MetricRange,
}

#[derive(Debug, thiserror::Error)]
pub enum EvaluatorError {
    #[error("data source request failed")]
    DatasourceRequestFailed,
    #[error("data source returned an unexpected response")]
    UnexpectedResponse,
    #[error("evaluation time range is invalid")]
    InvalidTimeRange,
}

pub type EvaluatorResult<T> = CustomResult<T, EvaluatorError>;
