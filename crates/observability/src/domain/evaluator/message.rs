use common_types::alert_rules::{GrafanaCloudwatchQuery, SeverityThreshold};

use super::{Evaluation, Notification, RuleDefinition};

pub fn alarm(
    rule: &RuleDefinition,
    query: &GrafanaCloudwatchQuery,
    evaluation: &Evaluation,
    threshold: &SeverityThreshold,
    value: f64,
    evaluated_at: &str,
) -> Notification {
    let dimensions = if query.dimensions.is_empty() {
        "None".to_owned()
    } else {
        query
            .dimensions
            .iter()
            .map(|(name, value)| format!("{}: {}", escape(name), escape(value)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Notification {
        heading: "ALARM".to_owned(),
        destination: threshold.destination.clone(),
        message: format!(
            "ALARM: {rule_name}\n\nSeverity: {severity}\nState: ALARM\nEvaluated: {evaluated_at}\n\nDescription:\n{description}\n\nMetric: {metric}\nNamespace: {namespace}\nStatistic: {statistic}\nRegion: {region}\n\nDimensions:\n{dimensions}\n\nCondition: {statistic} {operator} {threshold}\nEvaluation: 1 of 1 datapoints\nPeriod: {period} seconds\nMissing data: NoData (no notification)\nLatest value: {value}\n\nDefinition: {rule_id}",
            rule_name = escape(&rule.name),
            rule_id = escape(&rule.id),
            severity = evaluation.level,
            namespace = escape(&query.namespace),
            metric = escape(&query.metric_name),
            statistic = query.statistic,
            region = escape(&query.region),
            period = query.period,
            operator = threshold.comparison_operator.symbol(),
            threshold = threshold.threshold,
            description = escape(&threshold.description),
        ),
    }
}

pub fn rule_error(
    rule: &RuleDefinition,
    evaluated_at: &str,
    error: &str,
    destination: &str,
) -> Notification {
    Notification {
        heading: "RULE ERROR".to_owned(),
        destination: destination.to_owned(),
        message: format!(
            "RULE ERROR: {rule_name}\n\nEvaluated: {evaluated_at}\n\nError:\n{error}\n\nDefinition: {rule_id}",
            rule_name = escape(&rule.name),
            rule_id = escape(&rule.id),
            error = escape(error),
        ),
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('`', "′")
}
