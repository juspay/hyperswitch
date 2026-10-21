pub mod cloudwatch;

use common_types::alert_rules::{AlertQuery, AlertRuleConfig, AlertThresholdsV1, Severity};
use common_utils::errors::CustomResult;
use time::PrimitiveDateTime;

use self::cloudwatch::GrafanaCloudwatchEvaluator;

#[derive(Clone, Debug, PartialEq)]
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum Level {
    Normal,
    Info,
    Warning,
    Critical,
}

impl Level {
    pub fn severity(self) -> Option<Severity> {
        match self {
            Self::Normal => None,
            Self::Info => Some(Severity::Info),
            Self::Warning => Some(Severity::Warning),
            Self::Critical => Some(Severity::Critical),
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
        pair: EvaluationPair,
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
    pub breach: Option<Breach>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Breach {
    pub condition: String,
    pub description: String,
    pub metric_details: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationPair {
    pub current: Evaluation,
    pub previous: Evaluation,
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

#[async_trait::async_trait]
pub trait AlertEvaluator: Send + Sync + std::fmt::Debug {
    type Query;

    async fn evaluate(
        &self,
        query: &Self::Query,
        thresholds: &AlertThresholdsV1,
        interval_seconds: u32,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<EvaluationPair>;
}

#[async_trait::async_trait]
pub trait RuleEvaluator: Send + Sync + std::fmt::Debug {
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<EvaluationPair>;
}

#[derive(Debug)]
pub struct Evaluators {
    pub cloudwatch: GrafanaCloudwatchEvaluator,
}

#[async_trait::async_trait]
impl RuleEvaluator for Evaluators {
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<EvaluationPair> {
        let config = rule.config.inner();
        match &config.query {
            AlertQuery::Cloudwatch(query) => {
                self.cloudwatch
                    .evaluate(
                        query.inner(),
                        &config.thresholds,
                        config.interval_seconds,
                        now,
                    )
                    .await
            }
        }
    }
}

#[async_trait::async_trait]
pub trait RuleSource: Send + Sync + std::fmt::Debug {
    async fn rules(&self) -> EvaluatorResult<Vec<RuleDefinition>>;
    async fn due_rules(&self, now: PrimitiveDateTime) -> EvaluatorResult<Vec<RuleDefinition>>;
}

#[derive(Clone, Debug)]
pub struct ConfigRuleSource {
    rules: Vec<RuleDefinition>,
}

impl ConfigRuleSource {
    pub(crate) fn new(rules: Vec<RuleDefinition>) -> Self {
        Self { rules }
    }
}

#[async_trait::async_trait]
impl RuleSource for ConfigRuleSource {
    async fn rules(&self) -> EvaluatorResult<Vec<RuleDefinition>> {
        Ok(self.rules.clone())
    }

    async fn due_rules(&self, _now: PrimitiveDateTime) -> EvaluatorResult<Vec<RuleDefinition>> {
        Ok(self.rules.clone())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use common_types::alert_rules::Severity;

    use super::Level;

    #[test]
    fn levels_share_one_severity_mapping() {
        assert_eq!(Level::Normal.severity(), None);
        assert_eq!(Level::Info.severity(), Some(Severity::Info));
        assert_eq!(Level::Warning.severity(), Some(Severity::Warning));
        assert_eq!(Level::Critical.severity(), Some(Severity::Critical));
    }

    #[test]
    fn levels_follow_alert_priority() {
        assert!(Level::Critical > Level::Warning);
        assert!(Level::Warning > Level::Info);
        assert!(Level::Info > Level::Normal);
    }

    #[test]
    fn levels_display_as_operator_facing_uppercase() {
        assert_eq!(Level::Normal.to_string(), "NORMAL");
        assert_eq!(Level::Info.to_string(), "INFO");
        assert_eq!(Level::Warning.to_string(), "WARNING");
        assert_eq!(Level::Critical.to_string(), "CRITICAL");
    }
}
