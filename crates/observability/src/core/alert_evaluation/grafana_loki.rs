use std::collections::BTreeMap;

use common_types::alert_rules::{AlertThresholds, GrafanaLokiQuery};
use external_services::metrics_provider::{
    grafana::loki::{GrafanaLoki, LokiLogsQuery, LokiMetricsQuery},
    MetricPoint, MetricRange, MetricSeries,
};
use time::{Duration, PrimitiveDateTime};

use crate::{
    core::alert_evaluation::{evaluate_series, fetch_window, AlertEvaluator},
    domain_models::evaluator::{Evaluation, EvaluatorResult, RuleDefinition},
    logger,
};

const LOG_LINES: u32 = 3;

#[async_trait::async_trait]
impl AlertEvaluator<GrafanaLokiQuery> for GrafanaLoki {
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        query: &GrafanaLokiQuery,
        datasource_uid: &str,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<Evaluation> {
        let (series, window, last) = fetch_window(
            self,
            LokiMetricsQuery {
                datasource_uid: datasource_uid.to_owned(),
                expr: query.expr.clone(),
                period: query.period,
            },
            query.period,
            query.end_offset_seconds,
            now,
        )
        .await?;
        let mut evaluation = evaluate_total(series, &rule.config.thresholds, window, last)?;
        if let (Some(logs), Some(_), Some(latest)) =
            (&query.logs, &evaluation.breach, &evaluation.latest_point)
        {
            match self
                .fetch_logs(
                    LokiLogsQuery {
                        datasource_uid: datasource_uid.to_owned(),
                        expr: logs.clone(),
                        limit: LOG_LINES,
                    },
                    MetricRange {
                        from: latest.timestamp,
                        to: latest.timestamp + Duration::seconds(i64::from(query.period)),
                    },
                )
                .await
            {
                Ok(lines) => evaluation.logs = lines,
                Err(error) => {
                    logger::warn!(tag = "evaluator_logs", rule_id = %rule.id, error = ?error, "Sample log lines unavailable")
                }
            }
        }
        Ok(evaluation)
    }
}

fn evaluate_total(
    series: Vec<MetricSeries>,
    thresholds: &AlertThresholds,
    window: MetricRange,
    last: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation> {
    let mut totals = BTreeMap::<PrimitiveDateTime, f64>::new();
    for point in series.iter().flat_map(|series| &series.points) {
        *totals.entry(point.timestamp).or_default() += point.value;
    }
    let total = MetricSeries {
        points: totals
            .into_iter()
            .map(|(timestamp, value)| MetricPoint { timestamp, value })
            .collect(),
        labels: BTreeMap::new(),
    };
    let mut evaluation = evaluate_series(vec![total], thresholds, window, last)?;
    if let Some(latest) = &evaluation.latest_point {
        evaluation.breakdown = series
            .into_iter()
            .filter_map(|series| {
                series
                    .points
                    .iter()
                    .find(|point| point.timestamp == latest.timestamp)
                    .map(|point| (series.labels.clone(), point.value))
            })
            .collect();
        evaluation
            .breakdown
            .sort_by(|left, right| right.1.total_cmp(&left.1));
    }
    Ok(evaluation)
}

#[cfg(test)]
mod tests {
    use common_types::alert_rules::{ComparisonOperator, Severity, SeverityThreshold};
    use time::macros::datetime;

    use super::*;
    use crate::domain_models::evaluator::AlertEvaluationLevel;

    #[test]
    fn labelled_series_are_summed_for_the_threshold_and_kept_as_a_sorted_breakdown() {
        let series = |app: &str, values: [(PrimitiveDateTime, f64); 2]| MetricSeries {
            points: values
                .into_iter()
                .map(|(timestamp, value)| MetricPoint { timestamp, value })
                .collect(),
            labels: BTreeMap::from([("app".to_owned(), app.to_owned())]),
        };
        let thresholds = AlertThresholds {
            severities: BTreeMap::from([(
                Severity::Warning,
                SeverityThreshold {
                    threshold: 30.0,
                    comparison_operator: ComparisonOperator::GreaterThanThreshold,
                    description: String::new(),
                    destination: "oncall".to_owned(),
                },
            )]),
        };
        let evaluation = evaluate_total(
            vec![
                series(
                    "small",
                    [
                        (datetime!(2026-03-18 10:03:00), 50.0),
                        (datetime!(2026-03-18 10:04:00), 9.0),
                    ],
                ),
                series(
                    "large",
                    [
                        (datetime!(2026-03-18 10:03:00), 1.0),
                        (datetime!(2026-03-18 10:04:00), 22.0),
                    ],
                ),
            ],
            &thresholds,
            MetricRange {
                from: datetime!(2026-03-18 10:00:00),
                to: datetime!(2026-03-18 10:05:00),
            },
            datetime!(2026-03-18 10:04:00),
        )
        .unwrap();
        assert_eq!(evaluation.level, AlertEvaluationLevel::Warning);
        assert_eq!(evaluation.latest_point.map(|point| point.value), Some(31.0));
        assert_eq!(
            evaluation.breakdown,
            vec![
                (
                    BTreeMap::from([("app".to_owned(), "large".to_owned())]),
                    22.0
                ),
                (
                    BTreeMap::from([("app".to_owned(), "small".to_owned())]),
                    9.0
                ),
            ]
        );
    }
}
