use std::{collections::BTreeMap, fmt};

use common_types::alert_rules::{AlertRuleConfig, Severity, SeverityThreshold};
use common_utils::errors::CustomResult;
use external_services::metrics_provider::{grafana::loki::LogLine, MetricPoint, MetricRange};
use serde::Deserialize;

use crate::domain::notifier::chat::AlertSeverity;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertEvaluationLevel {
    Normal,
    NoData,
    Info,
    Warning,
    Critical,
}

impl fmt::Display for AlertEvaluationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Normal => "NORMAL",
            Self::NoData => "NO_DATA",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Critical => "CRITICAL",
        })
    }
}

impl From<Severity> for AlertEvaluationLevel {
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
    pub title: String,
    pub severity: AlertSeverity,
    pub region: String,
    pub message: String,
}

pub enum RuleRun {
    Evaluated {
        level: AlertEvaluationLevel,
        notification: Option<Notification>,
    },
    Failed {
        error: String,
        notification: Notification,
    },
}

pub struct Delivery {
    pub delivered: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub level: AlertEvaluationLevel,
    pub latest_point: Option<MetricPoint>,
    pub datapoints: Vec<MetricPoint>,
    pub breach: Option<SeverityThreshold>,
    pub window: MetricRange,
    pub breakdown: Vec<(BTreeMap<String, String>, f64)>,
    pub logs: Vec<LogLine>,
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

#[cfg(test)]
mod display_tests {
    use super::AlertEvaluationLevel;

    #[test]
    fn alert_level_display_keeps_message_format() {
        for (level, expected) in [
            (AlertEvaluationLevel::Normal, "NORMAL"),
            (AlertEvaluationLevel::NoData, "NO_DATA"),
            (AlertEvaluationLevel::Info, "INFO"),
            (AlertEvaluationLevel::Warning, "WARNING"),
            (AlertEvaluationLevel::Critical, "CRITICAL"),
        ] {
            assert_eq!(level.to_string(), expected);
        }
    }
}
