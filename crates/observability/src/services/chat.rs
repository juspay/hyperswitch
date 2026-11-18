use std::collections::BTreeMap;

use common_types::alert_rules::{
    AlertQuery, GrafanaCloudwatchQuery, GrafanaLokiQuery, SeverityThreshold,
};
use external_services::metrics_provider::MetricPoint;
use time::PrimitiveDateTime;

use crate::domain_models::evaluator::{Evaluation, Notification, RuleDefinition};

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

const LOG_LINE_CHARS: usize = 500;

pub(crate) fn create_notification(
    rule: &RuleDefinition,
    evaluated_at: PrimitiveDateTime,
    body: ChatBody<'_>,
) -> Notification {
    match &rule.config.query {
        AlertQuery::GrafanaCloudwatch(query) => {
            cloudwatch_notification(rule, query, evaluated_at, body)
        }
        AlertQuery::GrafanaLoki(query) => loki_notification(rule, query, evaluated_at, body),
    }
}

fn cloudwatch_notification(
    rule: &RuleDefinition,
    query: &GrafanaCloudwatchQuery,
    evaluated_at: PrimitiveDateTime,
    body: ChatBody<'_>,
) -> Notification {
    let dimensions = key_value_lines(&query.dimensions);
    match body {
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
    }
}

fn loki_notification(
    rule: &RuleDefinition,
    query: &GrafanaLokiQuery,
    evaluated_at: PrimitiveDateTime,
    body: ChatBody<'_>,
) -> Notification {
    let context = key_value_lines(&query.context);
    match body {
        ChatBody::Alarm {
            evaluation,
            threshold,
            point,
        } => {
            let logs = if evaluation.logs.is_empty() {
                "None".to_owned()
            } else {
                evaluation
                    .logs
                    .iter()
                    .map(|log| {
                        format!(
                            "{} UTC | {}",
                            log.timestamp,
                            log_text(&log.line)
                                .trim()
                                .chars()
                                .take(LOG_LINE_CHARS)
                                .collect::<String>()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Notification {
                heading: "ALARM".to_owned(),
                destination: threshold.destination.clone(),
                message: format!(
                    include_str!("chat/assets/loki_alert.txt"),
                    heading = "ALARM",
                    rule_name = instance_name(&rule.name, &evaluation.labels),
                    rule_id = rule.id,
                    severity = evaluation.level,
                    evaluated_at = evaluated_at,
                    description = threshold.description,
                    context = context,
                    condition = format_args!(
                        "value {} {}",
                        threshold.comparison_operator, threshold.threshold
                    ),
                    period = query.period,
                    value = point.value,
                    range = format_args!(
                        "{} UTC to {} UTC",
                        evaluation.window.from, evaluation.window.to
                    ),
                    labels = key_value_lines(&evaluation.labels),
                    logs = logs,
                    error = "None",
                ),
            }
        }
        ChatBody::RuleError { error, destination } => Notification {
            heading: "RULE ERROR".to_owned(),
            destination: destination.to_owned(),
            message: format!(
                include_str!("chat/assets/loki_alert.txt"),
                heading = "RULE ERROR",
                rule_name = rule.name,
                rule_id = rule.id,
                severity = "Not evaluated",
                evaluated_at = evaluated_at,
                description = "Rule evaluation failed",
                context = context,
                condition = "Not evaluated",
                period = query.period,
                value = "Unavailable",
                range = "Unavailable",
                labels = "Unavailable: rule evaluation failed",
                logs = "Unavailable: rule evaluation failed",
                error = error,
            ),
        },
    }
}

fn instance_name(name: &str, labels: &BTreeMap<String, String>) -> String {
    if labels.is_empty() {
        name.to_owned()
    } else {
        format!(
            "{name} ({})",
            labels
                .iter()
                .map(|(label, value)| format!("{label}={value}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn log_text(line: &str) -> String {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|log| {
            log.pointer("/extra/error")
                .and_then(serde_json::Value::as_str)
                .or_else(|| log.get("message").and_then(serde_json::Value::as_str))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| line.to_owned())
}

fn key_value_lines(entries: &BTreeMap<String, String>) -> String {
    if entries.is_empty() {
        "None".to_owned()
    } else {
        entries
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use external_services::metrics_provider::{grafana::loki::LogLine, MetricRange};
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
                labels: BTreeMap::new(),
                logs: Vec::new(),
            };
            let notification = create_notification(
                &rule,
                datetime!(2026-03-18 10:08:00),
                ChatBody::Alarm {
                    evaluation: &evaluation,
                    threshold,
                    point: &point,
                },
            );
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

    #[test]
    fn loki_alarm_names_its_instance_and_lists_context_and_latest_logs_without_the_query() {
        let rule: RuleDefinition = serde_json::from_value(json!({
            "id": "router-errors",
            "name": "Router errors",
            "config": {
                "interval_seconds": 60,
                "query": {
                    "datasource": "grafana_loki",
                    "expr": "sum by (app) (count_over_time({app=~\"router\"} |= `error` [1m]))",
                    "period": 60,
                    "context": {"Service": "hyperswitch-router"},
                    "logs": "{app=~\"router\"} |= `error`"
                },
                "thresholds": {"severities": {"warning": {
                    "threshold": 20,
                    "comparison_operator": "GreaterThanThreshold",
                    "description": "Router logged errors",
                    "destination": "oncall"
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
            timestamp: datetime!(2026-03-18 10:04:00),
            value: 42.0,
        };
        let evaluation = Evaluation {
            level: AlertEvaluationLevel::Warning,
            latest_point: Some(point.clone()),
            datapoints: vec![point.clone()],
            breach: Some(threshold.clone()),
            window: MetricRange {
                from: datetime!(2026-03-18 10:00:00),
                to: datetime!(2026-03-18 10:05:00),
            },
            labels: BTreeMap::from([
                ("extra_status_code".to_owned(), "500".to_owned()),
                ("flow".to_owned(), "PaymentsConfirm".to_owned()),
            ]),
            logs: vec![
                LogLine {
                    timestamp: datetime!(2026-03-18 10:04:41),
                    line: "ERROR payment failed\n".to_owned(),
                },
                LogLine {
                    timestamp: datetime!(2026-03-18 10:04:12),
                    line: "é".repeat(LOG_LINE_CHARS + 1),
                },
                LogLine {
                    timestamp: datetime!(2026-03-18 10:04:05),
                    line: json!({
                        "level": "ERROR",
                        "message": "[SERVER_WRAP_UTIL - EVENT] router::core",
                        "extra": {"error": "Failed to parse URL\n╰╴at crates/router/src/services/api/client.rs:146:14"}
                    })
                    .to_string(),
                },
            ],
        };
        let alarm = create_notification(
            &rule,
            datetime!(2026-03-18 10:07:12),
            ChatBody::Alarm {
                evaluation: &evaluation,
                threshold,
                point: &point,
            },
        );
        assert_eq!(alarm.heading, "ALARM");
        assert_eq!(alarm.destination, "oncall");
        assert!(alarm
            .message
            .starts_with("ALARM: Router errors (extra_status_code=500, flow=PaymentsConfirm)\n"));
        assert!(alarm
            .message
            .contains("Context:\nService: hyperswitch-router\n"));
        assert!(alarm.message.contains("Condition: value > 20\n"));
        assert!(alarm.message.contains("Latest value: 42\n"));
        assert!(alarm
            .message
            .contains("Labels:\nextra_status_code: 500\nflow: PaymentsConfirm\n"));
        assert!(alarm.message.contains(&format!(
            "Latest logs:\n{} UTC | ERROR payment failed\n",
            datetime!(2026-03-18 10:04:41)
        )));
        assert!(alarm.message.contains(&format!(
            "{} UTC | {}\n",
            datetime!(2026-03-18 10:04:12),
            "é".repeat(LOG_LINE_CHARS)
        )));
        assert!(!alarm.message.contains(&"é".repeat(LOG_LINE_CHARS + 1)));
        assert!(alarm.message.contains(&format!(
            "{} UTC | Failed to parse URL\n╰╴at crates/router/src/services/api/client.rs:146:14\n",
            datetime!(2026-03-18 10:04:05)
        )));
        assert!(!alarm.message.contains("SERVER_WRAP_UTIL"));
        assert!(!alarm.message.contains("count_over_time"));

        let unlabelled = Evaluation {
            labels: BTreeMap::new(),
            ..evaluation.clone()
        };
        let alarm = create_notification(
            &rule,
            datetime!(2026-03-18 10:07:12),
            ChatBody::Alarm {
                evaluation: &unlabelled,
                threshold,
                point: &point,
            },
        );
        assert!(alarm.message.starts_with("ALARM: Router errors\n"));
        assert!(alarm.message.contains("Labels:\nNone\n"));

        let error = create_notification(
            &rule,
            datetime!(2026-03-18 10:07:12),
            ChatBody::RuleError {
                error: "data source request failed",
                destination: "oncall",
            },
        );
        assert_eq!(error.heading, "RULE ERROR");
        assert!(error.message.starts_with("RULE ERROR: Router errors\n"));
        assert!(error
            .message
            .contains("Error:\ndata source request failed\n"));
    }
}
