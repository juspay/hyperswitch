use std::collections::BTreeMap;

use common_types::alert_rules::{AlertThresholds, GrafanaLokiQuery};
use external_services::metrics_provider::{
    grafana::loki::{GrafanaLoki, LokiLogsQuery, LokiMetricsQuery},
    MetricPoint, MetricRange, MetricSeries,
};
use time::PrimitiveDateTime;

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
    ) -> EvaluatorResult<Vec<Evaluation>> {
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
        let window = MetricRange {
            from: last,
            ..window
        };
        let mut evaluations = evaluate_instances(series, &rule.config.thresholds, window, last)?;
        if let Some(logs) = query.logs.as_ref().filter(|_| {
            evaluations
                .iter()
                .any(|evaluation| evaluation.breach.is_some())
        }) {
            match self
                .fetch_logs(
                    LokiLogsQuery {
                        datasource_uid: datasource_uid.to_owned(),
                        expr: logs.clone(),
                        limit: LOG_LINES,
                    },
                    window,
                )
                .await
            {
                Ok(lines) => evaluations
                    .iter_mut()
                    .filter(|evaluation| evaluation.breach.is_some())
                    .for_each(|evaluation| evaluation.logs = lines.clone()),
                Err(error) => {
                    logger::warn!(tag = "evaluator_logs", rule_id = %rule.id, error = ?error, "Sample log lines unavailable")
                }
            }
        }
        Ok(evaluations)
    }
}

fn evaluate_instances(
    mut series: Vec<MetricSeries>,
    thresholds: &AlertThresholds,
    window: MetricRange,
    last: PrimitiveDateTime,
) -> EvaluatorResult<Vec<Evaluation>> {
    series.retain(|series| !series.points.is_empty());
    if series.is_empty() {
        let empty = MetricSeries {
            points: vec![MetricPoint {
                timestamp: last,
                value: 0.0,
            }],
            labels: BTreeMap::new(),
        };
        return evaluate_series(vec![empty], thresholds, window, last)
            .map(|evaluation| vec![evaluation]);
    }
    series
        .into_iter()
        .map(|series| evaluate_series(vec![series], thresholds, window, last))
        .collect()
}

#[cfg(test)]
mod tests {
    use common_types::alert_rules::{ComparisonOperator, Severity, SeverityThreshold};
    use time::macros::datetime;

    use super::*;
    use crate::domain_models::evaluator::AlertEvaluationLevel;

    fn thresholds() -> AlertThresholds {
        AlertThresholds {
            severities: BTreeMap::from([
                (
                    Severity::Critical,
                    SeverityThreshold {
                        threshold: 20.0,
                        comparison_operator: ComparisonOperator::GreaterThanThreshold,
                        description: String::new(),
                        destination: "oncall".to_owned(),
                    },
                ),
                (
                    Severity::Warning,
                    SeverityThreshold {
                        threshold: 5.0,
                        comparison_operator: ComparisonOperator::GreaterThanThreshold,
                        description: String::new(),
                        destination: "oncall".to_owned(),
                    },
                ),
            ]),
        }
    }

    fn window() -> MetricRange {
        MetricRange {
            from: datetime!(2026-03-18 10:04:00),
            to: datetime!(2026-03-18 10:05:00),
        }
    }

    #[test]
    fn each_series_is_its_own_instance_judged_against_the_thresholds() {
        let last = datetime!(2026-03-18 10:04:00);
        let series = |flow: &str, value: f64| MetricSeries {
            points: vec![MetricPoint {
                timestamp: last,
                value,
            }],
            labels: BTreeMap::from([("flow".to_owned(), flow.to_owned())]),
        };
        let evaluations = evaluate_instances(
            vec![
                series("PaymentsCreate", 3.0),
                series("PaymentsConfirm", 9.0),
                series("Refunds", 25.0),
            ],
            &thresholds(),
            window(),
            last,
        )
        .unwrap();
        assert_eq!(
            evaluations
                .iter()
                .map(|evaluation| (
                    evaluation.labels.get("flow").map(String::as_str),
                    evaluation.level,
                    evaluation.latest_point.as_ref().map(|point| point.value)
                ))
                .collect::<Vec<_>>(),
            [
                (
                    Some("PaymentsCreate"),
                    AlertEvaluationLevel::Normal,
                    Some(3.0)
                ),
                (
                    Some("PaymentsConfirm"),
                    AlertEvaluationLevel::Warning,
                    Some(9.0)
                ),
                (Some("Refunds"), AlertEvaluationLevel::Critical, Some(25.0)),
            ]
        );
    }

    #[test]
    fn no_matching_logs_is_one_unlabelled_instance_valued_zero() {
        let last = datetime!(2026-03-18 10:04:00);
        let placeholder = MetricSeries {
            points: Vec::new(),
            labels: BTreeMap::new(),
        };
        let evaluations =
            evaluate_instances(vec![placeholder], &thresholds(), window(), last).unwrap();
        assert_eq!(evaluations.len(), 1);
        let evaluation = evaluations.first().unwrap();
        assert_eq!(evaluation.level, AlertEvaluationLevel::Normal);
        assert!(evaluation.labels.is_empty());
        assert_eq!(
            evaluation.latest_point,
            Some(MetricPoint {
                timestamp: last,
                value: 0.0
            })
        );
    }
}
