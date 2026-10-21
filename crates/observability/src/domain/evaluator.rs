pub mod cloudwatch;

use common_types::alert_rules::AlertRuleConfig;
use common_utils::errors::CustomResult;
use time::PrimitiveDateTime;

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

#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub level: Level,
    pub latest_value: Option<f64>,
    pub condition: Option<String>,
    pub description: Option<String>,
    pub metric_details: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationPair {
    pub current: Evaluation,
    pub previous: Evaluation,
}

#[derive(Debug, thiserror::Error)]
pub enum EvaluatorError {
    #[error("data source is not configured")]
    DatasourceNotConfigured,
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
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<EvaluationPair>;
}

#[async_trait::async_trait]
pub trait RuleSource: Send + Sync + std::fmt::Debug {
    async fn rules(&self) -> EvaluatorResult<Vec<RuleDefinition>>;
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
}

pub fn is_due(interval_seconds: u32, now: PrimitiveDateTime) -> bool {
    let interval_seconds = i64::from(interval_seconds);
    (now.assume_utc().unix_timestamp().div_euclid(60) * 60).rem_euclid(interval_seconds) == 0
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::{is_due, Level};

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

    #[test]
    fn every_minute_interval_is_always_due() {
        for minute in 0..60 {
            assert!(is_due(
                60,
                datetime!(2026-10-16 06:00) + time::Duration::minutes(minute)
            ));
        }
    }

    #[test]
    fn five_minute_interval_is_due_at_zero_and_five() {
        assert!(is_due(300, datetime!(2026-10-16 06:00)));
        assert!(!is_due(300, datetime!(2026-10-16 06:01)));
        assert!(is_due(300, datetime!(2026-10-16 06:05)));
        assert!(!is_due(300, datetime!(2026-10-16 06:06)));
    }
}
