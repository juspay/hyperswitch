use common_types::alert_rules::{AlertThresholds, GrafanaCloudwatchQuery};
use common_utils::date_time::convert_to_pdt;
use error_stack::{Report, ResultExt};
use external_services::metrics_provider::{
    grafana::cloudwatch::CloudwatchMetricsQuery, MetricRange, MetricSeries, MetricsError,
    MetricsProvider,
};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::domain_models::evaluator::{
    AlertEvaluationLevel, Evaluation, EvaluatorError, EvaluatorResult,
};

const LOOKBACK_SECONDS: u32 = 300;

pub async fn evaluate<P>(
    provider: &P,
    metrics_query: CloudwatchMetricsQuery,
    query: &GrafanaCloudwatchQuery,
    thresholds: &AlertThresholds,
    now: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation>
where
    P: MetricsProvider<MetricsQuery = CloudwatchMetricsQuery, MetricsError = MetricsError>,
{
    let period = i64::from(query.period);
    let seconds = now.assume_utc().unix_timestamp()
        - i64::from(query.end_offset_seconds.saturating_sub(query.period));
    let end_seconds = seconds
        .checked_div_euclid(period)
        .map(|bucket| bucket * period)
        .ok_or(EvaluatorError::InvalidTimeRange)
        .attach_printable("evaluation period must be greater than zero")?;
    let to = OffsetDateTime::from_unix_timestamp(end_seconds)
        .map(convert_to_pdt)
        .change_context(EvaluatorError::InvalidTimeRange)?;
    let (from, last) = to
        .checked_sub(Duration::seconds(i64::from(LOOKBACK_SECONDS).max(period)))
        .zip(to.checked_sub(Duration::seconds(period)))
        .ok_or(EvaluatorError::InvalidTimeRange)
        .attach_printable("query evaluation time range is outside supported bounds")?;
    let window = MetricRange { from, to };
    let request = P::build_request(metrics_query, window)
        .change_context(EvaluatorError::DatasourceRequestFailed)?;
    let series = provider
        .fetch_metrics(request)
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
        .into_iter()
        .next()
        .map(|series| series.points)
        .unwrap_or_default();
    let latest = points
        .iter()
        .filter(|point| (window.from..=last).contains(&point.timestamp))
        .max_by_key(|point| point.timestamp)
        .cloned();
    let Some(latest) = latest else {
        return Ok(Evaluation {
            level: AlertEvaluationLevel::NoData,
            latest_point: None,
            datapoints: points,
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
    for (&severity, threshold) in &thresholds.severities {
        if threshold
            .comparison_operator
            .breaches(latest.value, threshold.threshold)
        {
            return Ok(Evaluation {
                level: severity.into(),
                latest_point: Some(latest),
                datapoints: points,
                breach: Some(threshold.clone()),
                window,
            });
        }
    }
    Ok(Evaluation {
        level: AlertEvaluationLevel::Normal,
        latest_point: Some(latest),
        datapoints: points,
        breach: None,
        window,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common_types::alert_rules::{ComparisonOperator, Severity, SeverityThreshold};
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

    #[test]
    fn newest_complete_unsorted_sample_drives_priority_and_boundaries() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let result = evaluate_series(
            vec![MetricSeries {
                points: vec![
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 90.0,
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:04:00),
                        value: 100.0,
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:06:00),
                        value: 200.0,
                    },
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
        assert_eq!(
            result.latest_point,
            Some(MetricPoint {
                timestamp: datetime!(2026-03-18 10:05:00),
                value: 90.0,
            })
        );
        // Preserve older and too-new returned points as context, not evaluation inputs.
        assert_eq!(result.datapoints.len(), 3);
        assert_eq!(result.datapoints[1].value, 100.0);
        assert_eq!(result.datapoints[2].timestamp, window.to);
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
                    points: vec![MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 10.0
                    }]
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
    fn context_points_do_not_turn_no_data_or_normal_into_an_alarm() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let configured = thresholds(&[(
            Severity::Critical,
            10.0,
            ComparisonOperator::GreaterThanThreshold,
        )]);
        let points = vec![
            MetricPoint {
                timestamp: datetime!(2026-03-18 09:59:00),
                value: 100.0,
            },
            MetricPoint {
                timestamp: datetime!(2026-03-18 10:06:00),
                value: 200.0,
            },
        ];
        let no_data = evaluate_series(
            vec![MetricSeries {
                points: points.clone(),
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00),
        )
        .unwrap();
        assert_eq!(no_data.level, AlertEvaluationLevel::NoData);
        assert_eq!(no_data.latest_point, None);
        assert_eq!(no_data.datapoints, points);
        assert_eq!(no_data.breach, None);

        let selected = MetricPoint {
            timestamp: datetime!(2026-03-18 10:05:00),
            value: 1.0,
        };
        let mut points = points;
        points.push(MetricPoint {
            timestamp: datetime!(2026-03-18 10:04:00),
            value: 100.0,
        });
        points.push(selected.clone());
        let normal = evaluate_series(
            vec![MetricSeries {
                points: points.clone(),
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00),
        )
        .unwrap();
        assert_eq!(normal.level, AlertEvaluationLevel::Normal);
        assert_eq!(normal.latest_point, Some(selected));
        assert_eq!(normal.datapoints, points);
        assert_eq!(normal.breach, None);
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
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 1.0
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 2.0
                    },
                ]
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00)
        )
        .is_err());
    }
}
