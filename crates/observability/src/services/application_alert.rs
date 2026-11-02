use std::collections::BTreeMap;

use common_types::alert_rules::{AlertQuery, SeverityThreshold};
use error_stack::Report;
use hyperswitch_masking::Secret;
use sha2::{Digest, Sha256};
use time::PrimitiveDateTime;

use crate::{
    core::alert_evaluation::grafana_loki::ApplicationInstanceObservation,
    domain::notifier::chat::ChatNotification,
    domain_models::evaluator::{EvaluatorError, EvaluatorResult, RuleDefinition},
};

const MAX_MESSAGE_CHARS: usize = 9000;

/// Only reviewed context labels appear in the API; provider labels remain private for identity.
pub fn preview_labels(
    labels: &BTreeMap<String, String>,
    redactions: &[&str],
) -> BTreeMap<String, String> {
    const VISIBLE: &[&str] = &[
        "app",
        "source_namespace",
        "namespace",
        "flow",
        "event",
        "method",
        "request_method",
        "status_code",
        "extra_status_code",
    ];
    labels
        .iter()
        .filter(|(key, _)| VISIBLE.contains(&key.as_str()))
        .map(|(key, value)| {
            (
                key.clone(),
                sanitize(value, redactions).chars().take(160).collect(),
            )
        })
        .collect()
}

pub struct RenderedApplicationAlert {
    pub fingerprint: String,
    pub destination: String,
    pub notification: ChatNotification,
    pub details_available: bool,
}

/// Preview and delivery use the same renderer. Nothing outside the mapped field allowlist is sent.
pub fn render_application_alert(
    rule: &RuleDefinition,
    instance: &ApplicationInstanceObservation,
    threshold: &SeverityThreshold,
    now: PrimitiveDateTime,
    redactions: &[&str],
) -> EvaluatorResult<RenderedApplicationAlert> {
    let AlertQuery::GrafanaLoki(query) = &rule.config.query else {
        return Err(Report::new(EvaluatorError::UnexpectedResponse));
    };
    let mut labels = instance.labels.clone();
    labels.extend(query.labels.clone());
    let identity = serde_json::to_vec(&(&rule.id, &instance.labels, &query.labels))
        .map_err(|_| Report::new(EvaluatorError::UnexpectedResponse))?;
    let fingerprint = format!("{:x}", Sha256::digest(identity));
    let field = |name: &str, aliases: &[&str]| -> Option<String> {
        let keys = query
            .fields
            .get(name)
            .map(|key| vec![key.as_str()])
            .unwrap_or_else(|| aliases.to_vec());
        keys.into_iter()
            .find_map(|key| labels.get(key).filter(|v| !v.trim().is_empty()))
            .map(|value| sanitize(value, redactions))
    };
    let error = field(
        "error",
        &[
            "extra_error",
            "extra_validation_error",
            "error_message",
            "developer_message",
            "error",
            "message",
        ],
    );
    let mut lines = vec![
        format!("[APPLICATION ALERT] {}", sanitize(&rule.name, redactions)),
        format!("Severity: {}", instance.level),
        format!("Rule: {}", rule.id),
        format!(
            "Description: {}",
            sanitize(&threshold.description, redactions)
        ),
    ];
    for (name, title, aliases) in [
        (
            "application",
            "Application",
            &["app", "service_name", "service", "container"][..],
        ),
        (
            "environment",
            "Environment",
            &["environment", "env", "namespace", "source_namespace"][..],
        ),
        (
            "path",
            "API path",
            &[
                "extra_http_route",
                "http_route",
                "path",
                "request_url_path",
                "url_path",
                "endpoint",
            ][..],
        ),
        (
            "method",
            "Method",
            &["request_method", "method", "extra_http_method"][..],
        ),
        ("flow", "Flow", &["flow", "flow_msg"][..]),
        (
            "connector",
            "Connector",
            &["connector", "gateway", "connector_name"][..],
        ),
        (
            "error_code",
            "Error code",
            &[
                "error_code",
                "error_kind",
                "grpc_code_name",
                "extra_status_code",
                "status_code",
            ][..],
        ),
        (
            "file",
            "Source",
            &["file", "source_file", "origin", "target"][..],
        ),
        ("request_id", "Request ID", &["request_id"][..]),
        ("event_time", "Event time", &["event_time", "time"][..]),
    ] {
        if let Some(value) = field(name, aliases) {
            lines.push(format!("{title}: {value}"));
        }
    }
    lines.push(format!(
        "\nError details:\n{}",
        error
            .as_deref()
            .unwrap_or("Not present in this query result; this rule is an aggregate signal.")
    ));
    lines.push(format!("\nEvaluated: {now} UTC"));
    lines.push(format!("Lookback: {} seconds", query.range_seconds));
    lines.push(format!(
        "Condition: {:?} {} {}",
        query.reducer, threshold.comparison_operator, threshold.threshold
    ));
    lines.push(format!(
        "Observed condition value: {}",
        instance
            .value
            .map_or_else(|| "Unavailable".to_owned(), |value| value.to_string())
    ));
    lines.push(format!("Instance: {fingerprint}"));
    let text = lines.join("\n");
    if text.chars().count() > MAX_MESSAGE_CHARS {
        return Err(Report::new(EvaluatorError::UnexpectedResponse)
            .attach_printable("Application message exceeds 9000 characters; details were not silently truncated or delivered"));
    }
    Ok(RenderedApplicationAlert {
        fingerprint,
        destination: threshold.destination.clone(),
        notification: ChatNotification {
            text: Secret::new(text),
            reply_to: None,
        },
        details_available: error.is_some(),
    })
}

fn sanitize(value: &str, redactions: &[&str]) -> String {
    // Redact configured credentials even if a remote error echoes them. No raw labels are logged.
    let mut value = value.replace('\0', "");
    for secret in redactions.iter().filter(|secret| !secret.is_empty()) {
        value = value.replace(secret, "[REDACTED]");
    }
    let mut output = String::new();
    for line in value.lines() {
        let lower = line.to_ascii_lowercase();
        if [
            "authorization",
            "bearer ",
            "password",
            "api_key",
            "api-key",
            "access_token",
            "app_jwt",
            "client_secret",
            "cookie",
            "card_number",
            "cardnumber",
            "\"pan\"",
            "\"cvc\"",
            "\"cvv\"",
        ]
        .iter()
        .any(|key| lower.contains(key))
        {
            output.push_str("[REDACTED SENSITIVE LINE]");
        } else {
            for word in line.split_inclusive(char::is_whitespace) {
                let jwt = word.split('.').count() == 3 && word.contains("eyJ");
                let digits = word.chars().filter(char::is_ascii_digit).count();
                if jwt
                    || (13..=19).contains(&digits)
                        && word.chars().all(|c| {
                            c.is_ascii_digit() || c.is_whitespace() || matches!(c, '-' | '"' | ',')
                        })
                {
                    output.push_str("[REDACTED]");
                } else {
                    output.push_str(word);
                }
            }
        }
        output.push('\n');
    }
    output
        .trim_end()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace("@channel", "@\u{200b}channel")
        .replace("@here", "@\u{200b}here")
        .replace("@everyone", "@\u{200b}everyone")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use hyperswitch_masking::PeekInterface;
    use serde_json::json;

    use super::*;
    use crate::domain_models::evaluator::AlertEvaluationLevel;

    fn fixture(error: Option<&str>) -> (RuleDefinition, ApplicationInstanceObservation) {
        let rule: RuleDefinition=serde_json::from_value(json!({"id":"fixture","name":"Application error","config":{
            "interval_seconds":60,"query":{"datasource":"grafana_loki","expression":"vector(1)","mode":"instant","range_seconds":300,"reducer":"last","fields":{"error":"custom_error"}},
            "thresholds":{"severities":{"critical":{"threshold":0,"comparison_operator":"GreaterThanThreshold","description":"Operation failed","destination":"test"}}}
        }})).unwrap();
        let labels = error.map_or_else(BTreeMap::new, |error| {
            BTreeMap::from([("custom_error".to_owned(), error.to_owned())])
        });
        (
            rule,
            ApplicationInstanceObservation {
                labels,
                display_labels: BTreeMap::new(),
                level: AlertEvaluationLevel::Critical,
                value: Some(2.0),
                notification: None,
            },
        )
    }

    #[test]
    fn reviewed_labels_win_without_merging_distinct_provider_identities() {
        let (mut rule, mut instance) = fixture(None);
        let AlertQuery::GrafanaLoki(query) = &mut rule.config.query else {
            panic!("fixture must use Loki");
        };
        query.labels.insert("app".into(), "reviewed-app".into());
        instance.labels.insert("app".into(), "provider-one".into());
        let threshold = rule.config.thresholds.severities.values().next().unwrap();
        let first = render_application_alert(
            &rule,
            &instance,
            threshold,
            common_utils::date_time::now(),
            &[],
        )
        .unwrap();
        instance.labels.insert("app".into(), "provider-two".into());
        let second = render_application_alert(
            &rule,
            &instance,
            threshold,
            common_utils::date_time::now(),
            &[],
        )
        .unwrap();
        assert!(first
            .notification
            .text
            .peek()
            .contains("Application: reviewed-app"));
        assert!(second
            .notification
            .text
            .peek()
            .contains("Application: reviewed-app"));
        assert_ne!(first.fingerprint, second.fingerprint);
    }

    #[test]
    fn mapped_error_is_complete_and_oversize_is_a_failure_not_a_truncated_alert() {
        let (rule, instance) = fixture(Some("Actual error\nCaused by database constraint"));
        let threshold = rule.config.thresholds.severities.values().next().unwrap();
        let rendered = render_application_alert(
            &rule,
            &instance,
            threshold,
            common_utils::date_time::now(),
            &[],
        )
        .unwrap();
        assert!(rendered.details_available);
        assert!(rendered
            .notification
            .text
            .peek()
            .contains("Actual error\nCaused by database constraint"));
        let (_, large) = fixture(Some(&"x".repeat(MAX_MESSAGE_CHARS + 1)));
        assert!(render_application_alert(
            &rule,
            &large,
            threshold,
            common_utils::date_time::now(),
            &[]
        )
        .is_err());
    }

    #[test]
    fn missing_details_are_explicit_not_fabricated() {
        let (rule, instance) = fixture(None);
        let threshold = rule.config.thresholds.severities.values().next().unwrap();
        let rendered = render_application_alert(
            &rule,
            &instance,
            threshold,
            common_utils::date_time::now(),
            &[],
        )
        .unwrap();
        assert!(!rendered.details_available);
        assert!(rendered
            .notification
            .text
            .peek()
            .contains("aggregate signal"));
    }

    #[test]
    fn credentials_sensitive_lines_card_numbers_and_broadcasts_are_redacted() {
        let value=sanitize("Operation failed\nAuthorization: Bearer private\nsecret-credential\n4242424242424242\n<@channel>",&["secret-credential"]);
        assert!(value.contains("Operation failed"));
        for hidden in [
            "private",
            "secret-credential",
            "4242424242424242",
            "<@channel>",
            "@channel",
        ] {
            assert!(!value.contains(hidden));
        }
        assert!(value.contains("[REDACTED"));
    }
}
