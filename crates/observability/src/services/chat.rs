use common_types::alert_rules::{AlertQuery, SeverityThreshold};
use time::PrimitiveDateTime;

use crate::domain_models::evaluator::{Evaluation, Notification, RuleDefinition};

pub(crate) enum ChatBody<'a> {
    Alarm {
        evaluation: &'a Evaluation,
        threshold: &'a SeverityThreshold,
        value: f64,
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
) -> Notification {
    let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query;
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
    match body {
        ChatBody::Alarm {
            evaluation,
            threshold,
            value,
        } => Notification {
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
                value = value,
                error = "None",
            ),
        },
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
                error = error,
            ),
        },
    }
}
