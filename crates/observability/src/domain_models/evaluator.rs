use common_types::alert_rules::{
    AlertQuery, AlertRuleConfig, CloudwatchStatistic, Severity, SeverityThreshold,
};
use common_utils::errors::CustomResult;
use external_services::metrics_provider::{
    grafana::cloudwatch::{CloudwatchMetricsQuery, CloudwatchStatistic as ProviderStatistic},
    MetricPoint, MetricRange,
};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
}

impl RuleDefinition {
    pub fn metrics_query(&self, datasource_uid: &str) -> CloudwatchMetricsQuery {
        let AlertQuery::GrafanaCloudwatch(query) = &self.config.query;
        CloudwatchMetricsQuery {
            datasource_uid: datasource_uid.to_owned(),
            region: query.region.clone(),
            namespace: query.namespace.clone(),
            metric_name: query.metric_name.clone(),
            dimensions: query.dimensions.clone(),
            statistic: match query.statistic {
                CloudwatchStatistic::Average => ProviderStatistic::Average,
                CloudwatchStatistic::Maximum => ProviderStatistic::Maximum,
                CloudwatchStatistic::Minimum => ProviderStatistic::Minimum,
                CloudwatchStatistic::Sum => ProviderStatistic::Sum,
            },
            period: query.period,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum AlertEvaluationLevel {
    Normal,
    NoData,
    Info,
    Warning,
    Critical,
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
