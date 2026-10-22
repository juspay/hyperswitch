use common_types::alert_rules::{GrafanaCloudwatchQuery, SeverityThreshold};
use time::PrimitiveDateTime;

use crate::domain_models::evaluator::{Evaluation, Notification, RuleDefinition};

pub(crate) fn alarm(
    rule: &RuleDefinition,
    query: &GrafanaCloudwatchQuery,
    evaluation: &Evaluation,
    threshold: &SeverityThreshold,
    value: f64,
    evaluated_at: PrimitiveDateTime,
) -> Notification {
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
    Notification {
        heading: "ALARM".to_owned(),
        destination: threshold.destination.clone(),
        message: format!(
            include_str!("chat/assets/alarm.txt"),
            rule_name = rule.name,
            rule_id = rule.id,
            severity = evaluation.level,
            namespace = query.namespace,
            metric = query.metric_name,
            statistic = query.statistic,
            region = query.region,
            period = query.period,
            operator = threshold.comparison_operator.symbol(),
            threshold = threshold.threshold,
            description = threshold.description,
            dimensions = dimensions,
            value = value,
            evaluated_at = evaluated_at,
        ),
    }
}

pub(crate) fn rule_error(
    rule: &RuleDefinition,
    evaluated_at: PrimitiveDateTime,
    error: &str,
    destination: &str,
) -> Notification {
    Notification {
        heading: "RULE ERROR".to_owned(),
        destination: destination.to_owned(),
        message: format!(
            include_str!("chat/assets/rule_error.txt"),
            rule_name = rule.name,
            rule_id = rule.id,
            evaluated_at = evaluated_at,
            error = error,
        ),
    }
}
