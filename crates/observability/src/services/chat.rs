use common_types::alert_rules::{AlertQuery, SeverityThreshold};
use external_services::metrics_provider::MetricPoint;
use time::PrimitiveDateTime;

use crate::domain_models::evaluator::{
    Evaluation, EvaluatorError, EvaluatorResult, Notification, RuleDefinition,
};

pub(crate) enum ChatBody<'a> {
    Alarm {
        evaluation: &'a Evaluation,
        threshold: &'a SeverityThreshold,
        point: &'a MetricPoint,
    },
    RuleError {
        error: &'a str,
        destination: &'a str,
    },
}

pub(crate) fn create_notification(
    rule: &RuleDefinition,
    evaluated_at: PrimitiveDateTime,
    body: ChatBody<'_>,
) -> EvaluatorResult<Notification> {
    let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query else {
        return Err(error_stack::Report::new(EvaluatorError::UnexpectedResponse));
    };
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
    Ok(match body {
        ChatBody::Alarm {
            evaluation,
            threshold,
            point,
        } => {
            let mut points = evaluation.datapoints.iter().collect::<Vec<_>>();
            points.sort_by_key(|sample| sample.timestamp);
            let selected = points.iter().position(|sample| *sample == point);
            let datapoints = points
                .into_iter()
                .enumerate()
                .map(|(index, sample)| {
                    let marker = if Some(index) == selected {
                        "SELECTED / BREACHING"
                    } else {
                        "CONTEXT / NOT EVALUATED"
                    };
                    format!("{} UTC | {} | {marker}", sample.timestamp, sample.value)
                })
                .collect::<Vec<_>>()
                .join("\n");
            Notification {
                heading: "ALARM".to_owned(),
                destination: threshold.destination.clone(),
                message: format!(
                    include_str!("chat/assets/alert.txt"),
                    heading = "ALARM",
                    rule_name = rule.name,
                    rule_id = rule.id,
                    severity = evaluation.level,
                    evaluated_at = evaluated_at,
                    description = threshold.description,
                    metric = query.metric_name,
                    namespace = query.namespace,
                    statistic = query.statistic,
                    region = query.region,
                    dimensions = dimensions,
                    condition = format_args!(
                        "{} {} {}",
                        query.statistic, threshold.comparison_operator, threshold.threshold
                    ),
                    evaluation = "1 of 1 datapoints",
                    period = query.period,
                    value = point.value,
                    range = format_args!(
                        "{} UTC to {} UTC",
                        evaluation.window.from, evaluation.window.to
                    ),
                    datapoints = datapoints,
                    error = "None",
                ),
            }
        }
        ChatBody::RuleError { error, destination } => Notification {
            heading: "RULE ERROR".to_owned(),
            destination: destination.to_owned(),
            message: format!(
                include_str!("chat/assets/alert.txt"),
                heading = "RULE ERROR",
                rule_name = rule.name,
                rule_id = rule.id,
                severity = "Not evaluated",
                evaluated_at = evaluated_at,
                description = "Rule evaluation failed",
                metric = query.metric_name,
                namespace = query.namespace,
                statistic = query.statistic,
                region = query.region,
                dimensions = dimensions,
                condition = "Not evaluated",
                evaluation = "Failed",
                period = query.period,
                value = "Unavailable",
                range = "Unavailable",
                datapoints = "Unavailable: rule evaluation failed",
                error = error,
            ),
        },
    })
}

#[cfg(test)]
mod tests {
    use external_services::metrics_provider::MetricRange;
    use serde_json::json;
    use time::macros::datetime;

    use super::*;
    use crate::domain_models::evaluator::AlertEvaluationLevel;

    #[test]
    fn all_metrics_render_actual_points_in_time_order_and_select_only_one() {
        for (metric, statistic) in [
            ("HTTPCode_Target_5XX_Count", "Sum"),
            ("HTTPCode_ELB_5XX_Count", "Sum"),
            ("CPUUtilization", "Average"),
        ] {
            let rule: RuleDefinition = serde_json::from_value(json!({
                "id": "test-rule",
                "name": "Raw `{name}` <&>",
                "config": {
                    "interval_seconds": 60,
                    "query": {
                        "datasource": "grafana_cloudwatch",
                        "region": "ap-south-2",
                        "namespace": "Synthetic/Test",
                        "metric_name": metric,
                        "statistic": statistic,
                        "period": 60
                    },
                    "thresholds": {"severities": {"critical": {
                        "threshold": 0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "description": "Raw `{description}` <&>",
                        "destination": "test-only"
                    }}}
                }
            }))
            .unwrap();
            let threshold = rule
                .config
                .thresholds
                .severities
                .first_key_value()
                .unwrap()
                .1;
            let point = MetricPoint {
                timestamp: datetime!(2026-03-18 10:05:00),
                value: 2.0,
            };
            let evaluation = Evaluation {
                level: AlertEvaluationLevel::Critical,
                latest_point: Some(point.clone()),
                datapoints: vec![
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:06:00),
                        value: 200.0,
                    },
                    point.clone(),
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 09:59:00),
                        value: 999.0,
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:04:00),
                        value: 100.0,
                    },
                    point.clone(),
                ],
                breach: Some(threshold.clone()),
                window: MetricRange {
                    from: datetime!(2026-03-18 10:01:00),
                    to: datetime!(2026-03-18 10:06:00),
                },
            };
            let notification = create_notification(
                &rule,
                datetime!(2026-03-18 10:08:00),
                ChatBody::Alarm {
                    evaluation: &evaluation,
                    threshold,
                    point: &point,
                },
            )
            .unwrap();
            let message = notification.message;
            assert!(message.contains(&format!("Metric: {metric}\n")));
            assert!(message.contains(&format!("Statistic: {statistic}\n")));
            assert!(message.contains("Raw `{name}` <&>"));
            assert!(message.contains("Raw `{description}` <&>"));
            assert!(message.contains(&format!(
                "Evaluated: {} UTC\n",
                datetime!(2026-03-18 10:08:00)
            )));
            assert!(message.contains(&format!(
                "Query range: {} UTC to {} UTC\n",
                evaluation.window.from, evaluation.window.to
            )));
            let rows = evaluation
                .datapoints
                .iter()
                .map(|sample| format!("{} UTC | {} |", sample.timestamp, sample.value))
                .collect::<Vec<_>>();
            for row in &rows {
                assert!(message.contains(row));
            }
            assert!(message.find(&rows[2]).unwrap() < message.find(&rows[3]).unwrap());
            assert!(message.find(&rows[3]).unwrap() < message.find(&rows[1]).unwrap());
            assert!(message.find(&rows[1]).unwrap() < message.find(&rows[0]).unwrap());
            assert!(message.contains(&format!(
                "{} UTC | 2 | SELECTED / BREACHING\n",
                point.timestamp
            )));
            assert_eq!(message.matches("SELECTED / BREACHING").count(), 1);
            assert_eq!(message.matches("CONTEXT / NOT EVALUATED").count(), 4);
            assert!(message.contains("Evaluation: 1 of 1 datapoints\n"));
            assert_eq!(notification.destination, "test-only");
        }
    }
}
