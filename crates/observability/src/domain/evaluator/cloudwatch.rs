use common_types::alert_rules::{AlertThresholds, GrafanaCloudwatchQuery, Severity};
use common_utils::{date_time::convert_to_pdt, fp_utils::when};
use error_stack::{Report, ResultExt};
use external_services::metrics_provider::{
    grafana::{cloudwatch::CloudwatchQuery, GrafanaClient},
    MetricRange, MetricsProvider,
};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use super::{Evaluation, EvaluatorError, EvaluatorResult, Level};

const LOOKBACK_SECONDS: u32 = 300;
const MAX_WINDOW_SECONDS: u32 = 86_400;

#[derive(Debug)]
pub struct GrafanaCloudwatchEvaluator {
    client: GrafanaClient,
}

impl GrafanaCloudwatchEvaluator {
    pub fn new(client: GrafanaClient) -> Self {
        Self { client }
    }

    pub async fn evaluate(
        &self,
        query: &GrafanaCloudwatchQuery,
        datasource_uid: &str,
        thresholds: &AlertThresholds,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<Evaluation> {
        let last = last_complete_period_start(query.period, query.end_offset_seconds, now)?;
        let to = last
            .checked_add(Duration::seconds(i64::from(query.period)))
            .ok_or_else(invalid_time_range)?;
        let from = to
            .checked_sub(Duration::seconds(i64::from(
                LOOKBACK_SECONDS.max(query.period),
            )))
            .ok_or_else(invalid_time_range)?;
        let window = MetricRange { from, to };
        let series = self
            .client
            .query(
                &CloudwatchQuery {
                    datasource_uid: datasource_uid.to_owned(),
                    region: query.region.clone(),
                    namespace: query.namespace.clone(),
                    metric_name: query.metric_name.clone(),
                    dimensions: query.dimensions.clone(),
                    statistic: query.statistic.to_string(),
                    period: query.period,
                },
                window,
            )
            .await
            .change_context(EvaluatorError::DatasourceRequestFailed)?;
        if series.len() > 1 {
            return Err(
                Report::new(EvaluatorError::UnexpectedResponse).attach_printable(format!(
                    "query returned {} series; expected at most 1",
                    series.len()
                )),
            );
        }
        let points = series
            .first()
            .map(|series| series.points.as_slice())
            .unwrap_or_default();
        let latest = points
            .iter()
            .filter(|point| (from..=last).contains(&point.timestamp))
            .max_by_key(|point| point.timestamp);
        let Some(latest) = latest else {
            return Ok(Evaluation {
                level: Level::NoData,
                latest_value: None,
                breach: None,
                window,
            });
        };
        if points
            .iter()
            .any(|point| point.timestamp == latest.timestamp && point.value != latest.value)
        {
            return Err(Report::new(EvaluatorError::UnexpectedResponse)
                .attach_printable("query returned conflicting values for the current timestamp"));
        }
        for severity in [Severity::Critical, Severity::Warning, Severity::Info] {
            if let Some(threshold) = thresholds.severities.get(&severity) {
                if threshold
                    .comparison_operator
                    .breaches(latest.value, threshold.threshold)
                {
                    return Ok(Evaluation {
                        level: severity.into(),
                        latest_value: Some(latest.value),
                        breach: Some(threshold.clone()),
                        window,
                    });
                }
            }
        }
        Ok(Evaluation {
            level: Level::Normal,
            latest_value: Some(latest.value),
            breach: None,
            window,
        })
    }
}

#[derive(Debug)]
pub struct CloudwatchValidationError {
    pub field: String,
    pub requirement: &'static str,
}

impl CloudwatchValidationError {
    fn new(field: impl Into<String>, requirement: &'static str) -> Self {
        Self {
            field: field.into(),
            requirement,
        }
    }
}

pub(crate) fn validate_query(
    query: &GrafanaCloudwatchQuery,
) -> Result<(), CloudwatchValidationError> {
    for (field, value) in [
        ("region", &query.region),
        ("namespace", &query.namespace),
        ("metric_name", &query.metric_name),
    ] {
        when(value.trim().is_empty(), || {
            Err(CloudwatchValidationError::new(
                format!("config.query.{field}"),
                "must not be blank",
            ))
        })?;
    }
    when(query.dimensions.len() > 30, || {
        Err(CloudwatchValidationError::new(
            "config.query.dimensions",
            "must contain at most 30 dimensions",
        ))
    })?;
    for (name, value) in &query.dimensions {
        when(name.trim().is_empty() || value.trim().is_empty(), || {
            Err(CloudwatchValidationError::new(
                "config.query.dimensions",
                "names and values must not be blank",
            ))
        })?;
    }
    let valid_period = [1, 5, 10, 20, 30].contains(&query.period)
        || (query.period > 0 && query.period <= MAX_WINDOW_SECONDS && query.period % 60 == 0);
    when(!valid_period, || {
        Err(CloudwatchValidationError::new(
            "config.query.period",
            "must be 1, 5, 10, 20, 30, or a multiple of 60 up to 86400",
        ))
    })?;
    when(query.end_offset_seconds > 3600, || {
        Err(CloudwatchValidationError::new(
            "config.query.end_offset_seconds",
            "must not exceed 3600 seconds",
        ))
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
    let candidate = shifted_seconds
        .checked_div_euclid(period_seconds)
        .and_then(|value| value.checked_mul(period_seconds))
        .ok_or_else(invalid_time_range)?;
    let last_seconds = if candidate
        .checked_add(period_seconds)
        .ok_or_else(invalid_time_range)?
        > at_seconds
    {
        candidate
            .checked_sub(period_seconds)
            .ok_or_else(invalid_time_range)?
    } else {
        candidate
    };
    let timestamp = OffsetDateTime::from_unix_timestamp(last_seconds)
        .change_context(EvaluatorError::InvalidTimeRange)?;
    Ok(convert_to_pdt(timestamp))
}

fn invalid_time_range() -> Report<EvaluatorError> {
    Report::new(EvaluatorError::InvalidTimeRange)
        .attach_printable("query evaluation time range is outside supported bounds")
}
