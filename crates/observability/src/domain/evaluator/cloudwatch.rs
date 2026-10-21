use std::collections::BTreeMap;

use common_types::alert_rules::{
    AlertQuery, CloudwatchQuery, CloudwatchQueryV1, CloudwatchStatistic, ComparisonOperator,
    Severity, SeverityThresholdV1, TreatMissingData,
};
use common_utils::{date_time::convert_to_pdt, errors::CustomResult};
use error_stack::{Report, ResultExt};
use external_services::grafana::{GrafanaClient, GrafanaConfig, MetricPoint, MetricSeries};
use hyperswitch_interfaces::types::Proxy;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use super::{
    AlertEvaluator, Evaluation, EvaluationPair, EvaluatorError, EvaluatorResult, Level,
    RuleDefinition,
};
use crate::errors::ConfigurationError;

const SPARE_PERIODS: u32 = 5;
const MAX_WINDOW_SECONDS: u32 = 86_400;

#[derive(Clone, Debug, Deserialize)]
pub struct GrafanaCloudwatchConfig {
    #[serde(flatten)]
    pub client: GrafanaConfig,
    pub cloudwatch_datasource_uid: String,
}

#[derive(Debug, Clone)]
pub struct GrafanaCloudwatchEvaluator {
    client: GrafanaClient,
    datasource_uid: String,
}

impl GrafanaCloudwatchEvaluator {
    pub fn new(config: GrafanaCloudwatchConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient::new(config.client, proxy),
            datasource_uid: config.cloudwatch_datasource_uid,
        }
    }
}

#[async_trait::async_trait]
impl AlertEvaluator for GrafanaCloudwatchEvaluator {
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<EvaluationPair> {
        let config = rule.config.inner();
        let query = cloudwatch_query(rule);
        let range = fetch_range(
            query,
            config.interval_seconds,
            config
                .thresholds
                .severities
                .values()
                .map(|threshold| threshold.evaluation_periods)
                .max()
                .unwrap_or(1),
            now,
        )?;
        let request = GrafanaQueryRequest::new(query, &self.datasource_uid);
        let series = self
            .client
            .fetch(request, range.from, range.to)
            .await
            .change_context(EvaluatorError::DatasourceRequestFailed)?;
        let points = points_from_series(series)?;
        evaluate_pair(rule, query, &points, range)
    }
}

pub(crate) fn validate_config(config: &GrafanaCloudwatchConfig) -> Result<(), ConfigurationError> {
    if !matches!(config.client.base_url.scheme(), "http" | "https") {
        return Err(configuration_error(
            "grafana.base_url",
            "must use the http or https scheme",
        ));
    }
    if config.cloudwatch_datasource_uid.trim().is_empty() {
        return Err(configuration_error(
            "grafana.cloudwatch_datasource_uid",
            "must not be blank",
        ));
    }
    Ok(())
}

pub(crate) fn validate_rule(rule: &RuleDefinition) -> CustomResult<(), ConfigurationError> {
    let query = cloudwatch_query(rule);
    if query.period == 0 {
        return Err(rule_configuration_error(
            rule,
            "query.period",
            "must be greater than zero",
        ));
    }
    for (severity, threshold) in &rule.config.inner().thresholds.severities {
        let valid = threshold
            .evaluation_periods
            .checked_mul(query.period)
            .is_some_and(|seconds| seconds <= MAX_WINDOW_SECONDS);
        if !valid {
            return Err(rule_configuration_error(
                rule,
                &format!("thresholds.severities.{severity}.evaluation_periods × query.period"),
                "must not exceed 86400 seconds",
            ));
        }
    }
    Ok(())
}

fn configuration_error(field: &str, requirement: &str) -> ConfigurationError {
    ConfigurationError::ConfigParsingError(format!(
        "invalid evaluator configuration: field `{field}` {requirement}"
    ))
}

fn rule_configuration_error(
    rule: &RuleDefinition,
    field: &str,
    requirement: &str,
) -> Report<ConfigurationError> {
    Report::new(ConfigurationError::ConfigParsingError(
        "invalid evaluator rule".to_owned(),
    ))
    .attach_printable(format!("rule `{}` field `{field}` {requirement}", rule.id))
}

fn cloudwatch_query(rule: &RuleDefinition) -> &CloudwatchQueryV1 {
    match &rule.config.inner().query {
        AlertQuery::Cloudwatch(CloudwatchQuery::V1(query)) => query,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaQueryRequest<'a> {
    ref_id: &'static str,
    datasource: GrafanaDatasource<'a>,
    query_mode: &'static str,
    metric_query_type: u8,
    metric_editor_mode: u8,
    #[serde(flatten)]
    metric: GrafanaCloudwatchQuery<'a>,
    match_exact: bool,
    id: &'static str,
    expression: &'static str,
}

impl<'a> GrafanaQueryRequest<'a> {
    fn new(query: &'a CloudwatchQueryV1, datasource_uid: &'a str) -> Self {
        Self {
            ref_id: "A",
            datasource: GrafanaDatasource {
                kind: "cloudwatch",
                uid: datasource_uid,
            },
            query_mode: "Metrics",
            metric_query_type: 0,
            metric_editor_mode: 0,
            metric: query.into(),
            match_exact: true,
            id: "",
            expression: "",
        }
    }
}

#[derive(Debug, Serialize)]
struct GrafanaDatasource<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    uid: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaCloudwatchQuery<'a> {
    region: &'a str,
    namespace: &'a str,
    metric_name: &'a str,
    dimensions: BTreeMap<&'a str, Vec<&'a str>>,
    statistic: CloudwatchStatistic,
    period: String,
}

impl<'a> From<&'a CloudwatchQueryV1> for GrafanaCloudwatchQuery<'a> {
    fn from(query: &'a CloudwatchQueryV1) -> Self {
        Self {
            region: &query.region,
            namespace: &query.namespace,
            metric_name: &query.metric_name,
            dimensions: query
                .dimensions
                .iter()
                .map(|(name, value)| (name.as_str(), vec![value.as_str()]))
                .collect(),
            statistic: query.statistic,
            period: query.period.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FetchRange {
    from: PrimitiveDateTime,
    to: PrimitiveDateTime,
    current_last: PrimitiveDateTime,
    previous_last: PrimitiveDateTime,
}

fn fetch_range(
    query: &CloudwatchQueryV1,
    interval_seconds: u32,
    maximum_evaluation_periods: u32,
    now: PrimitiveDateTime,
) -> EvaluatorResult<FetchRange> {
    let current_last = last_complete_period_start(query.period, query.end_offset_seconds, now)?;
    let previous_at = shift_timestamp(now, -i64::from(interval_seconds))?;
    let previous_last =
        last_complete_period_start(query.period, query.end_offset_seconds, previous_at)?;
    let range_seconds = evaluation_range_seconds(query.period, maximum_evaluation_periods)?;
    let from = shift_timestamp(
        previous_last,
        range_seconds.checked_neg().ok_or_else(invalid_time_range)?,
    )?;
    let to = shift_timestamp(current_last, i64::from(query.period))?;
    Ok(FetchRange {
        from,
        to,
        current_last,
        previous_last,
    })
}

fn last_complete_period_start(
    period_seconds: u32,
    end_offset_seconds: u32,
    at: PrimitiveDateTime,
) -> EvaluatorResult<PrimitiveDateTime> {
    let period_seconds = i64::from(period_seconds);
    let at_seconds = at.assume_utc().unix_timestamp();
    let shifted_seconds = at_seconds
        .checked_sub(i64::from(end_offset_seconds))
        .ok_or_else(invalid_time_range)?;
    let mut last_seconds = shifted_seconds
        .checked_div_euclid(period_seconds)
        .and_then(|value| value.checked_mul(period_seconds))
        .ok_or_else(invalid_time_range)?;
    while last_seconds
        .checked_add(period_seconds)
        .ok_or_else(invalid_time_range)?
        > at_seconds
    {
        last_seconds = last_seconds
            .checked_sub(period_seconds)
            .ok_or_else(invalid_time_range)?;
    }
    let timestamp = OffsetDateTime::from_unix_timestamp(last_seconds)
        .change_context(EvaluatorError::InvalidTimeRange)?;
    Ok(convert_to_pdt(timestamp))
}

fn evaluation_range_seconds(period_seconds: u32, evaluation_periods: u32) -> EvaluatorResult<i64> {
    let range_periods = evaluation_periods
        .checked_add(SPARE_PERIODS)
        .and_then(|periods| periods.checked_sub(1))
        .ok_or_else(invalid_time_range)?;
    i64::from(period_seconds)
        .checked_mul(i64::from(range_periods))
        .ok_or_else(invalid_time_range)
}

fn shift_timestamp(
    timestamp: PrimitiveDateTime,
    seconds: i64,
) -> EvaluatorResult<PrimitiveDateTime> {
    timestamp
        .checked_add(Duration::seconds(seconds))
        .ok_or_else(invalid_time_range)
}

fn invalid_time_range() -> Report<EvaluatorError> {
    Report::new(EvaluatorError::InvalidTimeRange)
        .attach_printable("query evaluation time range is outside supported bounds")
}

fn points_from_series(series: Vec<MetricSeries>) -> EvaluatorResult<Vec<MetricPoint>> {
    let count = series.len();
    if count > 1 {
        return Err(Report::new(EvaluatorError::UnexpectedResponse)
            .attach_printable(format!("query returned {count} series; expected 1")));
    }
    Ok(series
        .into_iter()
        .flat_map(|metric_series| metric_series.points)
        .collect())
}

fn evaluate_pair(
    rule: &RuleDefinition,
    query: &CloudwatchQueryV1,
    points: &[MetricPoint],
    range: FetchRange,
) -> EvaluatorResult<EvaluationPair> {
    Ok(EvaluationPair {
        current: evaluate_at(rule, query, points, range.current_last)?,
        previous: evaluate_at(rule, query, points, range.previous_last)?,
    })
}

fn evaluate_at(
    rule: &RuleDefinition,
    query: &CloudwatchQueryV1,
    points: &[MetricPoint],
    last: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation> {
    let mut latest_point = None;
    for (severity, level) in [
        (Severity::Critical, Level::Critical),
        (Severity::Warning, Level::Warning),
        (Severity::Info, Level::Info),
    ] {
        let Some(threshold) = rule.config.inner().thresholds.severities.get(&severity) else {
            continue;
        };
        let judged = evaluate_severity(threshold, points, last, query.period)?;
        latest_point = newest_point(latest_point, judged.latest_point);
        if judged.breaching {
            let condition = condition(query, threshold);
            let metric_details = metric_details(query, threshold, &condition);
            return Ok(Evaluation {
                level,
                latest_value: judged.latest_point.map(|point| point.value),
                condition: Some(condition),
                description: Some(threshold.description.clone()),
                metric_details: Some(metric_details),
            });
        }
    }
    Ok(Evaluation {
        level: Level::Normal,
        latest_value: latest_point.map(|point| point.value),
        condition: None,
        description: None,
        metric_details: None,
    })
}

#[derive(Clone, Copy, Debug)]
struct SeverityEvaluation<'a> {
    breaching: bool,
    latest_point: Option<&'a MetricPoint>,
}

fn evaluate_severity<'a>(
    threshold: &SeverityThresholdV1,
    points: &'a [MetricPoint],
    last: PrimitiveDateTime,
    period_seconds: u32,
) -> EvaluatorResult<SeverityEvaluation<'a>> {
    let range_seconds = evaluation_range_seconds(period_seconds, threshold.evaluation_periods)?;
    let range_start = shift_timestamp(
        last,
        range_seconds.checked_neg().ok_or_else(invalid_time_range)?,
    )?;
    let mut real = points
        .iter()
        .filter(|point| point.timestamp >= range_start && point.timestamp <= last)
        .collect::<Vec<_>>();
    real.sort_by_key(|point| point.timestamp);
    let evaluation_periods = usize::try_from(threshold.evaluation_periods)
        .change_context(EvaluatorError::UnexpectedResponse)
        .attach_printable("evaluation_periods does not fit this platform")?;
    let selected = real
        .iter()
        .rev()
        .take(evaluation_periods)
        .copied()
        .collect::<Vec<_>>();
    let real_breaches = selected
        .iter()
        .filter(|point| breaches(point.value, threshold))
        .count();
    let missing = evaluation_periods.saturating_sub(selected.len());
    let filled_breaches = if threshold.treat_missing_data == TreatMissingData::Breaching {
        missing
    } else {
        0
    };
    let breaching_points = real_breaches
        .checked_add(filled_breaches)
        .ok_or_else(invalid_time_range)?;
    let datapoints_to_alarm = usize::try_from(
        threshold
            .datapoints_to_alarm
            .unwrap_or(threshold.evaluation_periods),
    )
    .change_context(EvaluatorError::UnexpectedResponse)
    .attach_printable("datapoints_to_alarm does not fit this platform")?;
    Ok(SeverityEvaluation {
        breaching: breaching_points >= datapoints_to_alarm,
        latest_point: selected.first().copied(),
    })
}

fn newest_point<'a>(
    first: Option<&'a MetricPoint>,
    second: Option<&'a MetricPoint>,
) -> Option<&'a MetricPoint> {
    match (first, second) {
        (Some(first), Some(second)) if first.timestamp >= second.timestamp => Some(first),
        (_, Some(second)) => Some(second),
        (Some(first), None) => Some(first),
        (None, None) => None,
    }
}

fn breaches(value: f64, threshold: &SeverityThresholdV1) -> bool {
    match threshold.comparison_operator {
        ComparisonOperator::GreaterThanThreshold => value > threshold.threshold,
        ComparisonOperator::GreaterThanOrEqualToThreshold => value >= threshold.threshold,
        ComparisonOperator::LessThanThreshold => value < threshold.threshold,
        ComparisonOperator::LessThanOrEqualToThreshold => value <= threshold.threshold,
    }
}

fn condition(query: &CloudwatchQueryV1, threshold: &SeverityThresholdV1) -> String {
    let operator = match threshold.comparison_operator {
        ComparisonOperator::GreaterThanThreshold => ">",
        ComparisonOperator::GreaterThanOrEqualToThreshold => ">=",
        ComparisonOperator::LessThanThreshold => "<",
        ComparisonOperator::LessThanOrEqualToThreshold => "<=",
    };
    format!("{} {operator} {}", query.statistic, threshold.threshold)
}

fn metric_details(
    query: &CloudwatchQueryV1,
    threshold: &SeverityThresholdV1,
    condition: &str,
) -> String {
    let dimensions = if query.dimensions.is_empty() {
        "None".to_owned()
    } else {
        query
            .dimensions
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let datapoints_to_alarm = threshold
        .datapoints_to_alarm
        .unwrap_or(threshold.evaluation_periods);
    format!(
        "Metric: {}\nNamespace: {}\nStatistic: {}\nRegion: {}\n\nDimensions:\n{dimensions}\n\nCondition: {condition}\nEvaluation: {datapoints_to_alarm} of {} datapoints\nPeriod: {} seconds\nMissing data: {}",
        query.metric_name,
        query.namespace,
        query.statistic,
        query.region,
        threshold.evaluation_periods,
        query.period,
        threshold.treat_missing_data
    )
}

#[cfg(test)]
mod tests {
    use common_types::alert_rules::{AlertRuleConfig, AlertRuleConfigStoredV1, AlertThresholdsV1};
    use hyperswitch_masking::Secret;
    use serde_json::json;
    use time::macros::datetime;
    use url::Url;

    use super::*;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    fn base() -> PrimitiveDateTime {
        datetime!(2026-10-16 06:45:00)
    }

    fn threshold(
        value: f64,
        comparison_operator: ComparisonOperator,
        evaluation_periods: u32,
        datapoints_to_alarm: Option<u32>,
        treat_missing_data: TreatMissingData,
    ) -> SeverityThresholdV1 {
        SeverityThresholdV1 {
            threshold: value,
            comparison_operator,
            evaluation_periods,
            datapoints_to_alarm,
            treat_missing_data,
            description: "severity".to_owned(),
        }
    }

    fn rule(
        period: u32,
        end_offset_seconds: u32,
        interval_seconds: u32,
        severities: Vec<(Severity, SeverityThresholdV1)>,
    ) -> RuleDefinition {
        RuleDefinition {
            id: "rule-1".to_owned(),
            name: "Rule 1".to_owned(),
            config: AlertRuleConfig::V1(AlertRuleConfigStoredV1 {
                classification: "test".to_owned(),
                interval_seconds,
                query: AlertQuery::Cloudwatch(CloudwatchQuery::V1(CloudwatchQueryV1 {
                    region: "us-east-1".to_owned(),
                    namespace: "Test".to_owned(),
                    metric_name: "Value".to_owned(),
                    dimensions: BTreeMap::new(),
                    statistic: CloudwatchStatistic::Average,
                    period,
                    end_offset_seconds,
                })),
                thresholds: AlertThresholdsV1 {
                    severities: severities.into_iter().collect(),
                },
            }),
        }
    }

    fn critical_rule(
        value: f64,
        comparison_operator: ComparisonOperator,
        evaluation_periods: u32,
        datapoints_to_alarm: Option<u32>,
        treat_missing_data: TreatMissingData,
    ) -> RuleDefinition {
        rule(
            60,
            0,
            60,
            vec![(
                Severity::Critical,
                threshold(
                    value,
                    comparison_operator,
                    evaluation_periods,
                    datapoints_to_alarm,
                    treat_missing_data,
                ),
            )],
        )
    }

    fn point(minute: i64, value: f64) -> MetricPoint {
        MetricPoint {
            timestamp: base() + Duration::minutes(minute),
            value,
        }
    }

    fn assert_level_order(
        data: Vec<Option<f64>>,
        rule: RuleDefinition,
        expected: &[Level],
    ) -> TestResult {
        let data_len = i64::try_from(data.len())?;
        let mut points = Vec::new();
        for (minute, value) in data.into_iter().enumerate() {
            if let Some(value) = value {
                points.push(point(i64::try_from(minute)?, value));
            }
        }
        let query = cloudwatch_query(&rule);
        let mut actual = Vec::new();
        for minute in -1..=data_len + 10 {
            let evaluation =
                evaluate_at(&rule, query, &points, base() + Duration::minutes(minute))?;
            if actual.last() != Some(&evaluation.level) {
                actual.push(evaluation.level);
            }
        }
        assert_eq!(actual, expected);
        Ok(())
    }

    fn dense_data(length: usize, value: impl Fn(usize) -> f64) -> Vec<Option<f64>> {
        (0..length).map(|minute| Some(value(minute))).collect()
    }

    #[test]
    fn s01_dense_sustained() -> TestResult {
        assert_level_order(
            dense_data(32, |minute| {
                if (5..=9).contains(&minute) {
                    90.0
                } else {
                    50.0
                }
            }),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::Missing,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s02_dense_one_minute_spike() -> TestResult {
        assert_level_order(
            dense_data(32, |minute| if minute == 5 { 95.0 } else { 50.0 }),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::Missing,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s03_dense_sustained_n3() -> TestResult {
        assert_level_order(
            dense_data(32, |minute| {
                if (5..=10).contains(&minute) {
                    90.0
                } else {
                    50.0
                }
            }),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                3,
                Some(3),
                TreatMissingData::Missing,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s04_dense_short_n3() -> TestResult {
        assert_level_order(
            dense_data(32, |minute| {
                if (5..=6).contains(&minute) {
                    90.0
                } else {
                    50.0
                }
            }),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                3,
                Some(3),
                TreatMissingData::Missing,
            ),
            &[Level::Normal],
        )
    }

    #[test]
    fn s05_sparse_single_n1() -> TestResult {
        assert_level_order(
            (0..32).map(|minute| (minute == 5).then_some(4.0)).collect(),
            critical_rule(
                0.0,
                ComparisonOperator::GreaterThanThreshold,
                1,
                Some(1),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s06_sparse_single_n3() -> TestResult {
        assert_level_order(
            (0..32).map(|minute| (minute == 5).then_some(4.0)).collect(),
            critical_rule(
                1.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                3,
                Some(3),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal],
        )
    }

    #[test]
    fn s07_sparse_burst_n3() -> TestResult {
        assert_level_order(
            (0..32)
                .map(|minute| (5..=9).contains(&minute).then_some(3.0))
                .collect(),
            critical_rule(
                1.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                3,
                Some(3),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s08_stops_breaching() -> TestResult {
        assert_level_order(
            (0..32)
                .map(|minute| (minute <= 7).then_some(50.0))
                .collect(),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::Breaching,
            ),
            &[Level::Critical, Level::Normal, Level::Critical],
        )
    }

    #[test]
    fn s09_stops_missing() -> TestResult {
        assert_level_order(
            (0..32)
                .map(|minute| (minute <= 7).then_some(50.0))
                .collect(),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::Missing,
            ),
            &[Level::Normal],
        )
    }

    #[test]
    fn s10_three_of_five() -> TestResult {
        assert_level_order(
            dense_data(32, |minute| {
                if [5, 7, 9].contains(&minute) {
                    90.0
                } else {
                    50.0
                }
            }),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                5,
                Some(3),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s11_ec2_every_five_minutes() -> TestResult {
        assert_level_order(
            (0..32)
                .map(|minute| (minute % 5 == 0).then_some(if minute == 15 { 90.0 } else { 50.0 }))
                .collect(),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::Missing,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn s12_stops_while_alarm() -> TestResult {
        assert_level_order(
            (0..32)
                .map(|minute| (minute <= 7).then_some(90.0))
                .collect(),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn f2_sparse_burst_n3() -> TestResult {
        assert_level_order(
            (0..16)
                .map(|minute| (3..=7).contains(&minute).then_some(3.0))
                .collect(),
            critical_rule(
                1.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                3,
                Some(3),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn f3_three_of_five() -> TestResult {
        assert_level_order(
            dense_data(16, |minute| {
                if [3, 5, 7].contains(&minute) {
                    90.0
                } else {
                    50.0
                }
            }),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                5,
                Some(3),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal, Level::Critical, Level::Normal],
        )
    }

    #[test]
    fn f4_only_two_of_five() -> TestResult {
        assert_level_order(
            dense_data(
                16,
                |minute| {
                    if [3, 5].contains(&minute) {
                        90.0
                    } else {
                        50.0
                    }
                },
            ),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                5,
                Some(3),
                TreatMissingData::NotBreaching,
            ),
            &[Level::Normal],
        )
    }

    #[test]
    fn f5_stops_breaching() -> TestResult {
        assert_level_order(
            (0..16)
                .map(|minute| (minute <= 4).then_some(50.0))
                .collect(),
            critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                1,
                Some(1),
                TreatMissingData::Breaching,
            ),
            &[Level::Critical, Level::Normal, Level::Critical],
        )
    }

    #[test]
    fn last_period_examples_never_use_unfinished_periods() -> TestResult {
        assert_eq!(
            last_complete_period_start(60, 180, datetime!(2026-10-16 10:08:00))?,
            datetime!(2026-10-16 10:05:00)
        );
        assert_eq!(
            last_complete_period_start(300, 180, datetime!(2026-10-16 10:08:00))?,
            datetime!(2026-10-16 10:00:00)
        );
        Ok(())
    }

    #[test]
    fn zero_period_returns_an_invalid_time_range() {
        let result = last_complete_period_start(0, 0, base());
        assert!(matches!(
            result,
            Err(ref error) if matches!(error.current_context(), EvaluatorError::InvalidTimeRange)
        ));
    }

    #[test]
    fn fetch_range_covers_previous_and_current_from_one_request() -> TestResult {
        let rule = rule(
            60,
            180,
            300,
            vec![
                (
                    Severity::Critical,
                    threshold(
                        80.0,
                        ComparisonOperator::GreaterThanOrEqualToThreshold,
                        1,
                        Some(1),
                        TreatMissingData::Missing,
                    ),
                ),
                (
                    Severity::Warning,
                    threshold(
                        70.0,
                        ComparisonOperator::GreaterThanOrEqualToThreshold,
                        3,
                        Some(2),
                        TreatMissingData::Missing,
                    ),
                ),
            ],
        );
        let range = fetch_range(
            cloudwatch_query(&rule),
            rule.config.inner().interval_seconds,
            3,
            datetime!(2026-10-16 10:08:00),
        )?;
        assert_eq!(range.from, datetime!(2026-10-16 09:53:00));
        assert_eq!(range.to, datetime!(2026-10-16 10:06:00));
        assert_eq!(range.previous_last, datetime!(2026-10-16 10:00:00));
        assert_eq!(range.current_last, datetime!(2026-10-16 10:05:00));
        Ok(())
    }

    #[test]
    fn current_and_previous_share_points_and_ignore_future_points() -> TestResult {
        let rule = critical_rule(
            80.0,
            ComparisonOperator::GreaterThanOrEqualToThreshold,
            1,
            Some(1),
            TreatMissingData::Missing,
        );
        let range = FetchRange {
            from: base(),
            to: base() + Duration::minutes(3),
            previous_last: base(),
            current_last: base() + Duration::minutes(1),
        };
        let pair = evaluate_pair(
            &rule,
            cloudwatch_query(&rule),
            &[point(0, 50.0), point(1, 90.0), point(2, 100.0)],
            range,
        )?;
        assert_eq!(pair.previous.level, Level::Normal);
        assert_eq!(pair.previous.latest_value, Some(50.0));
        assert_eq!(pair.current.level, Level::Critical);
        assert_eq!(pair.current.latest_value, Some(90.0));
        Ok(())
    }

    #[test]
    fn critical_wins_without_comparing_severity_values() -> TestResult {
        let rule = rule(
            60,
            0,
            60,
            vec![
                (
                    Severity::Warning,
                    threshold(
                        70.0,
                        ComparisonOperator::GreaterThanOrEqualToThreshold,
                        1,
                        Some(1),
                        TreatMissingData::Missing,
                    ),
                ),
                (
                    Severity::Critical,
                    threshold(
                        80.0,
                        ComparisonOperator::GreaterThanOrEqualToThreshold,
                        1,
                        Some(1),
                        TreatMissingData::Missing,
                    ),
                ),
            ],
        );
        let evaluation = evaluate_at(&rule, cloudwatch_query(&rule), &[point(0, 90.0)], base())?;
        assert_eq!(evaluation.level, Level::Critical);
        Ok(())
    }

    #[test]
    fn condition_and_latest_value_belong_to_chosen_severity() -> TestResult {
        let rule = critical_rule(
            90.0,
            ComparisonOperator::GreaterThanOrEqualToThreshold,
            2,
            Some(1),
            TreatMissingData::Missing,
        );
        let evaluation = evaluate_at(
            &rule,
            cloudwatch_query(&rule),
            &[point(0, 91.0), point(1, 50.0)],
            base() + Duration::minutes(1),
        )?;
        assert_eq!(evaluation.level, Level::Critical);
        assert_eq!(evaluation.latest_value, Some(50.0));
        assert_eq!(evaluation.condition.as_deref(), Some("Average >= 90"));
        assert_eq!(
            evaluation.metric_details.as_deref(),
            Some("Metric: Value\nNamespace: Test\nStatistic: Average\nRegion: us-east-1\n\nDimensions:\nNone\n\nCondition: Average >= 90\nEvaluation: 1 of 2 datapoints\nPeriod: 60 seconds\nMissing data: missing")
        );
        Ok(())
    }

    #[test]
    fn metric_details_preserve_all_dimensions_and_default_evaluation_count() {
        let query = CloudwatchQueryV1 {
            region: "ap-south-1".to_owned(),
            namespace: "AWS/ApplicationELB".to_owned(),
            metric_name: "HTTPCode_Target_5XX_Count".to_owned(),
            dimensions: BTreeMap::from([
                ("AvailabilityZone".to_owned(), "ap-south-1a".to_owned()),
                (
                    "LoadBalancer".to_owned(),
                    "app/test-envoy-alb/0123456789abcdef".to_owned(),
                ),
            ]),
            statistic: CloudwatchStatistic::Sum,
            period: 60,
            end_offset_seconds: 0,
        };
        let threshold = threshold(
            10.0,
            ComparisonOperator::GreaterThanOrEqualToThreshold,
            1,
            None,
            TreatMissingData::NotBreaching,
        );
        assert_eq!(
            metric_details(&query, &threshold, &condition(&query, &threshold)),
            "Metric: HTTPCode_Target_5XX_Count\nNamespace: AWS/ApplicationELB\nStatistic: Sum\nRegion: ap-south-1\n\nDimensions:\nAvailabilityZone: ap-south-1a\nLoadBalancer: app/test-envoy-alb/0123456789abcdef\n\nCondition: Sum >= 10\nEvaluation: 1 of 1 datapoints\nPeriod: 60 seconds\nMissing data: notBreaching"
        );
    }

    #[test]
    fn normal_uses_latest_real_point_across_all_judged_severities() -> TestResult {
        let rule = rule(
            60,
            0,
            60,
            vec![
                (
                    Severity::Critical,
                    threshold(
                        100.0,
                        ComparisonOperator::GreaterThanOrEqualToThreshold,
                        1,
                        Some(1),
                        TreatMissingData::Missing,
                    ),
                ),
                (
                    Severity::Info,
                    threshold(
                        80.0,
                        ComparisonOperator::GreaterThanOrEqualToThreshold,
                        3,
                        Some(3),
                        TreatMissingData::Missing,
                    ),
                ),
            ],
        );
        let evaluation = evaluate_at(
            &rule,
            cloudwatch_query(&rule),
            &[point(0, 50.0), point(1, 60.0)],
            base() + Duration::minutes(1),
        )?;
        assert_eq!(evaluation.level, Level::Normal);
        assert_eq!(evaluation.latest_value, Some(60.0));
        assert_eq!(evaluation.condition, None);
        assert_eq!(evaluation.metric_details, None);
        Ok(())
    }

    #[test]
    fn all_missing_data_modes_and_default_m_follow_the_recipe() -> TestResult {
        for (treatment, expected) in [
            (TreatMissingData::Breaching, Level::Critical),
            (TreatMissingData::NotBreaching, Level::Normal),
            (TreatMissingData::Missing, Level::Normal),
        ] {
            let rule = critical_rule(
                80.0,
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                3,
                None,
                treatment,
            );
            let evaluation = evaluate_at(&rule, cloudwatch_query(&rule), &[], base())?;
            assert_eq!(evaluation.level, expected);
            assert_eq!(evaluation.latest_value, None);
        }
        Ok(())
    }

    #[test]
    fn comparison_operators_are_applied() -> TestResult {
        for (operator, value, expected) in [
            (ComparisonOperator::GreaterThanThreshold, 80.0, false),
            (
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                80.0,
                true,
            ),
            (ComparisonOperator::LessThanThreshold, 80.0, false),
            (ComparisonOperator::LessThanOrEqualToThreshold, 80.0, true),
        ] {
            let rule = critical_rule(80.0, operator, 1, Some(1), TreatMissingData::Missing);
            let evaluation =
                evaluate_at(&rule, cloudwatch_query(&rule), &[point(0, value)], base())?;
            assert_eq!(evaluation.level == Level::Critical, expected);
        }
        Ok(())
    }

    #[test]
    fn multiple_series_has_typed_context_and_safe_count() -> TestResult {
        let result = points_from_series(vec![
            MetricSeries { points: Vec::new() },
            MetricSeries { points: Vec::new() },
        ]);
        let Err(error) = result else {
            return Err("expected a multiple-series error".into());
        };
        assert!(matches!(
            error.current_context(),
            EvaluatorError::UnexpectedResponse
        ));
        assert!(format!("{error:?}").contains("query returned 2 series; expected 1"));
        Ok(())
    }

    #[test]
    fn zero_series_is_zero_points() -> TestResult {
        let points = points_from_series(Vec::new())?;
        assert!(points.is_empty());
        Ok(())
    }

    #[test]
    fn wire_query_matches_the_grafana_cloudwatch_contract() -> TestResult {
        let mut rule = critical_rule(
            80.0,
            ComparisonOperator::GreaterThanOrEqualToThreshold,
            1,
            Some(1),
            TreatMissingData::Missing,
        );
        let AlertRuleConfig::V1(config) = &mut rule.config;
        let AlertQuery::Cloudwatch(CloudwatchQuery::V1(query)) = &mut config.query;
        query.namespace = "AWS/EC2".to_owned();
        query.metric_name = "CPUUtilization".to_owned();
        query
            .dimensions
            .insert("InstanceId".to_owned(), "i-0123".to_owned());
        let value = serde_json::to_value(GrafanaQueryRequest::new(
            cloudwatch_query(&rule),
            "cloudwatch",
        ))?;
        assert_eq!(
            value,
            json!({
                "refId": "A",
                "datasource": {"type": "cloudwatch", "uid": "cloudwatch"},
                "queryMode": "Metrics",
                "metricQueryType": 0,
                "metricEditorMode": 0,
                "region": "us-east-1",
                "namespace": "AWS/EC2",
                "metricName": "CPUUtilization",
                "dimensions": {"InstanceId": ["i-0123"]},
                "statistic": "Average",
                "period": "60",
                "matchExact": true,
                "id": "",
                "expression": ""
            })
        );
        Ok(())
    }

    #[test]
    fn source_validators_report_typed_configuration_context() -> TestResult {
        let bad_config = GrafanaCloudwatchConfig {
            client: GrafanaConfig {
                base_url: Url::parse("ftp://grafana.example")?,
                api_token: Secret::new("token".to_owned()),
            },
            cloudwatch_datasource_uid: " ".to_owned(),
        };
        let Err(config_error) = validate_config(&bad_config) else {
            return Err("expected invalid source config".into());
        };
        assert!(matches!(
            config_error,
            ConfigurationError::ConfigParsingError(_)
        ));
        assert!(format!("{config_error:?}").contains("grafana.base_url"));

        let blank_uid_config = GrafanaCloudwatchConfig {
            client: GrafanaConfig {
                base_url: Url::parse("https://grafana.example")?,
                api_token: Secret::new("token".to_owned()),
            },
            cloudwatch_datasource_uid: " ".to_owned(),
        };
        let Err(uid_error) = validate_config(&blank_uid_config) else {
            return Err("expected a blank datasource uid error".into());
        };
        assert!(matches!(
            uid_error,
            ConfigurationError::ConfigParsingError(_)
        ));
        assert!(format!("{uid_error:?}").contains("cloudwatch_datasource_uid"));

        let zero_period_rule = rule(
            0,
            0,
            60,
            vec![(
                Severity::Critical,
                threshold(
                    80.0,
                    ComparisonOperator::GreaterThanOrEqualToThreshold,
                    1,
                    Some(1),
                    TreatMissingData::Missing,
                ),
            )],
        );
        let Err(period_error) = validate_rule(&zero_period_rule) else {
            return Err("expected a zero-period rule error".into());
        };
        assert!(matches!(
            period_error.current_context(),
            ConfigurationError::ConfigParsingError(_)
        ));
        assert!(format!("{period_error:?}").contains("rule `rule-1` field `query.period`"));

        let rule = rule(
            43_201,
            0,
            60,
            vec![(
                Severity::Critical,
                threshold(
                    80.0,
                    ComparisonOperator::GreaterThanOrEqualToThreshold,
                    2,
                    Some(1),
                    TreatMissingData::Missing,
                ),
            )],
        );
        let Err(rule_error) = validate_rule(&rule) else {
            return Err("expected invalid rule window".into());
        };
        assert!(matches!(
            rule_error.current_context(),
            ConfigurationError::ConfigParsingError(_)
        ));
        assert!(format!("{rule_error:?}").contains("rule `rule-1` field"));
        Ok(())
    }
}
