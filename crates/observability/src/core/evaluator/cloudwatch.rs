use common_types::alert_rules::{AlertThresholds, GrafanaCloudwatchQuery, Severity};
use common_utils::date_time::convert_to_pdt;
use error_stack::{Report, ResultExt};
use external_services::metrics_provider::{
    grafana::cloudwatch::CloudwatchMetricRequest, MetricRange, MetricSeries, MetricsError,
    MetricsProvider,
};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::domain_models::evaluator::{
    AlertEvaluationLevel, Evaluation, EvaluatorError, EvaluatorResult,
};

const LOOKBACK_SECONDS: u32 = 300;

pub async fn evaluate<P>(
    provider: &P,
    request: CloudwatchMetricRequest,
    query: &GrafanaCloudwatchQuery,
    thresholds: &AlertThresholds,
    now: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation>
where
    P: MetricsProvider<
        Request = CloudwatchMetricRequest,
        Response = Vec<MetricSeries>,
        Error = MetricsError,
    >,
{
    let (window, last) = evaluation_window(query.period, query.end_offset_seconds, now)?;
    let series = provider
        .fetch(request, window)
        .await
        .change_context(EvaluatorError::DatasourceRequestFailed)?;
    evaluate_series(series, thresholds, window, last)
}

fn evaluate_series(
    series: Vec<MetricSeries>,
    thresholds: &AlertThresholds,
    window: MetricRange,
    last: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation> {
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
        .filter(|point| (window.from..=last).contains(&point.timestamp))
        .max_by_key(|point| point.timestamp);
    let Some(latest) = latest else {
        return Ok(Evaluation {
            level: AlertEvaluationLevel::NoData,
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
        level: AlertEvaluationLevel::Normal,
        latest_value: Some(latest.value),
        breach: None,
        window,
    })
}

fn evaluation_window(
    period_seconds: u32,
    end_offset_seconds: u32,
    at: PrimitiveDateTime,
) -> EvaluatorResult<(MetricRange, PrimitiveDateTime)> {
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
    let last = OffsetDateTime::from_unix_timestamp(last_seconds)
        .map(convert_to_pdt)
        .change_context(EvaluatorError::InvalidTimeRange)?;
    let to = last
        .checked_add(Duration::seconds(period_seconds))
        .ok_or_else(invalid_time_range)?;
    let from = to
        .checked_sub(Duration::seconds(
            i64::from(LOOKBACK_SECONDS).max(period_seconds),
        ))
        .ok_or_else(invalid_time_range)?;
    Ok((MetricRange { from, to }, last))
}

fn invalid_time_range() -> Report<EvaluatorError> {
    Report::new(EvaluatorError::InvalidTimeRange)
        .attach_printable("query evaluation time range is outside supported bounds")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common_types::alert_rules::{ComparisonOperator, SeverityThreshold};
    use external_services::metrics_provider::MetricPoint;
    use time::macros::datetime;

    use super::*;

    fn thresholds(entries: &[(Severity, f64, ComparisonOperator)]) -> AlertThresholds {
        AlertThresholds {
            severities: entries
                .iter()
                .map(|(severity, threshold, operator)| {
                    (
                        *severity,
                        SeverityThreshold {
                            threshold: *threshold,
                            comparison_operator: *operator,
                            description: String::new(),
                            destination: severity.to_string(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn point(timestamp: PrimitiveDateTime, value: f64) -> MetricPoint {
        MetricPoint { timestamp, value }
    }

    #[test]
    fn period_and_offset_define_the_expected_complete_windows() {
        let now = datetime!(2026-03-18 10:08:00);
        let (minute, last) = evaluation_window(60, 180, now).unwrap();
        assert_eq!(last, datetime!(2026-03-18 10:05:00));
        assert_eq!(minute.from, datetime!(2026-03-18 10:01:00));
        assert_eq!(minute.to, datetime!(2026-03-18 10:06:00));

        let (five_minutes, last) = evaluation_window(300, 180, now).unwrap();
        assert_eq!(last, datetime!(2026-03-18 10:00:00));
        assert_eq!(five_minutes.from, datetime!(2026-03-18 10:00:00));
        assert_eq!(five_minutes.to, datetime!(2026-03-18 10:05:00));
    }

    #[test]
    fn newest_complete_unsorted_sample_drives_priority_and_boundaries() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let result = evaluate_series(
            vec![MetricSeries {
                points: vec![
                    point(datetime!(2026-03-18 10:05:00), 90.0),
                    point(datetime!(2026-03-18 10:04:00), 100.0),
                    point(datetime!(2026-03-18 10:06:00), 200.0),
                ],
            }],
            &thresholds(&[
                (
                    Severity::Info,
                    80.0,
                    ComparisonOperator::GreaterThanThreshold,
                ),
                (
                    Severity::Critical,
                    90.0,
                    ComparisonOperator::GreaterThanOrEqualToThreshold,
                ),
            ]),
            window,
            datetime!(2026-03-18 10:05:00),
        )
        .unwrap();
        assert_eq!(result.level, AlertEvaluationLevel::Critical);
        assert_eq!(result.latest_value, Some(90.0));
    }

    #[test]
    fn empty_is_no_data_and_normal_is_not_a_severity() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let configured = thresholds(&[(
            Severity::Warning,
            10.0,
            ComparisonOperator::GreaterThanThreshold,
        )]);
        assert_eq!(
            evaluate_series(vec![], &configured, window, datetime!(2026-03-18 10:05:00))
                .unwrap()
                .level,
            AlertEvaluationLevel::NoData
        );
        assert_eq!(
            evaluate_series(
                vec![MetricSeries {
                    points: vec![point(datetime!(2026-03-18 10:05:00), 10.0)]
                }],
                &configured,
                window,
                datetime!(2026-03-18 10:05:00)
            )
            .unwrap()
            .level,
            AlertEvaluationLevel::Normal
        );
    }

    #[test]
    fn multiple_series_and_conflicting_newest_duplicates_are_rejected() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let configured = AlertThresholds {
            severities: BTreeMap::new(),
        };
        assert!(evaluate_series(
            vec![
                MetricSeries { points: vec![] },
                MetricSeries { points: vec![] }
            ],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00)
        )
        .is_err());
        assert!(evaluate_series(
            vec![MetricSeries {
                points: vec![
                    point(datetime!(2026-03-18 10:05:00), 1.0),
                    point(datetime!(2026-03-18 10:05:00), 2.0),
                ]
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00)
        )
        .is_err());
    }
}
