use error_stack::{report, AttachmentKind, FrameKind, Report, ResultExt};
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

use crate::{
    domain::{
        evaluator::{
            is_due, AlertEvaluator, Evaluation, EvaluationPair, EvaluatorError, EvaluatorResult,
            Level, RuleDefinition, RuleSource,
        },
        notifier::{
            chat::{ChatNotification, ChatNotifier},
            Outcome, Registry,
        },
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    settings::evaluator::EvaluatorDestinations,
    state::AppState,
    types::{EvaluatorResponse, RuleReport},
};

pub async fn run(
    state: AppState,
    now: PrimitiveDateTime,
    deliver: bool,
) -> ObservabilityApiResult<EvaluatorResponse> {
    let runtime = state.evaluator.as_deref().ok_or_else(|| {
        report!(ObservabilityError::InternalServerError)
            .attach_printable("evaluator is not configured")
    })?;
    run_with(
        runtime.rule_source.as_ref(),
        runtime.evaluators.as_ref(),
        state.chat.as_ref(),
        &runtime.destinations,
        now,
        deliver,
    )
    .await
}

async fn run_with(
    rule_source: &dyn RuleSource,
    evaluators: &Registry<dyn AlertEvaluator>,
    chat: &Registry<dyn ChatNotifier>,
    destinations: &EvaluatorDestinations,
    now: PrimitiveDateTime,
    deliver: bool,
) -> ObservabilityApiResult<EvaluatorResponse> {
    let rules = rule_source
        .rules()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("failed to load evaluator rules")?;
    let timestamp = format_timestamp(now)?;
    let context = RunContext {
        evaluators,
        chat,
        destinations,
        now,
        timestamp: &timestamp,
        deliver,
    };
    let mut reports = Vec::with_capacity(rules.len());
    for rule in &rules {
        let due = is_due(rule.config.inner().interval_seconds, now);
        if deliver && !due {
            reports.push(skipped_report(rule, due));
            continue;
        }
        reports.push(run_rule(rule, &context, due).await);
    }
    Ok(EvaluatorResponse {
        evaluated_at: now,
        rules: reports,
    })
}

async fn run_rule(rule: &RuleDefinition, context: &RunContext<'_>, due: bool) -> RuleReport {
    let outcome = evaluate_and_prepare(
        rule,
        context.evaluators,
        context.destinations,
        context.now,
        context.timestamp,
    )
    .await;
    let mut prepared = match outcome {
        Ok(prepared) => prepared,
        Err(error) => failed_rule(rule, context.destinations, context.timestamp, due, error),
    };
    prepared.report.due = due;
    if context.deliver {
        deliver_prepared(rule, context.chat, &mut prepared).await;
    }
    prepared.report
}

async fn evaluate_and_prepare(
    rule: &RuleDefinition,
    evaluators: &Registry<dyn AlertEvaluator>,
    destinations: &EvaluatorDestinations,
    now: PrimitiveDateTime,
    timestamp: &str,
) -> EvaluatorResult<PreparedRule> {
    let datasource = rule.config.inner().query.datasource().to_string();
    let evaluator = evaluators.get(&datasource).ok_or_else(|| {
        report!(EvaluatorError::DatasourceNotConfigured).attach_printable(format!(
            "no evaluator is registered for datasource `{datasource}`"
        ))
    })?;
    let pair = evaluator.evaluate(rule, now).await?;
    Ok(prepare_transition(rule, destinations, timestamp, pair))
}

fn prepare_transition(
    rule: &RuleDefinition,
    destinations: &EvaluatorDestinations,
    timestamp: &str,
    pair: EvaluationPair,
) -> PreparedRule {
    let previous_level = pair.previous.level;
    let level = pair.current.level;
    let transition = if previous_level == level {
        None
    } else if level == Level::Normal {
        Some(("RESOLVED", &pair.previous))
    } else {
        Some(("ALARM", &pair.current))
    };
    let Some((heading, selected)) = transition else {
        return PreparedRule {
            report: evaluated_report(rule, pair, None, false),
            destination: None,
            heading: None,
        };
    };
    let destination = destinations.get(selected.level).map(str::to_owned);
    let message = render_transition(heading, rule, &pair, selected, timestamp);
    PreparedRule {
        report: evaluated_report(rule, pair, Some(message), false),
        destination,
        heading: Some(heading),
    }
}

fn evaluated_report(
    rule: &RuleDefinition,
    pair: EvaluationPair,
    message: Option<String>,
    due: bool,
) -> RuleReport {
    RuleReport {
        id: rule.id.clone(),
        due,
        previous_level: Some(pair.previous.level.into()),
        level: Some(pair.current.level.into()),
        message,
        delivered: None,
        error: None,
    }
}

fn failed_rule(
    rule: &RuleDefinition,
    destinations: &EvaluatorDestinations,
    timestamp: &str,
    due: bool,
    error: Report<EvaluatorError>,
) -> PreparedRule {
    let error_text = short_error(&error);
    logger::error!(
        heading = "RULE ERROR",
        rule_id = %rule.id,
        error = %error_text,
        "Rule evaluation failed"
    );
    PreparedRule {
        report: RuleReport {
            id: rule.id.clone(),
            due,
            previous_level: None,
            level: None,
            message: Some(render_rule_error(rule, timestamp, &error_text)),
            delivered: None,
            error: Some(error_text),
        },
        destination: Some(destinations.critical.clone()),
        heading: Some("RULE ERROR"),
    }
}

async fn deliver_prepared(
    rule: &RuleDefinition,
    chat: &Registry<dyn ChatNotifier>,
    prepared: &mut PreparedRule,
) {
    let (Some(message), Some(destination), Some(heading)) = (
        prepared.report.message.as_ref(),
        prepared.destination.as_deref(),
        prepared.heading,
    ) else {
        return;
    };
    let Some(notifier) = chat.get(destination) else {
        let detail = format!("chat destination `{destination}` is not configured");
        logger::error!(heading, rule_id = %rule.id, error = %detail, "Evaluator notification failed");
        prepared.report.delivered = Some(false);
        record_delivery_error(&mut prepared.report, detail);
        return;
    };
    match notifier
        .notify(ChatNotification {
            text: Secret::new(message.clone()),
            reply_to: None,
        })
        .await
    {
        Ok(Outcome::Delivered(receipt)) => {
            logger::info!(
                heading,
                rule_id = %rule.id,
                message_id = ?receipt.message_id,
                "Evaluator notification delivered"
            );
            prepared.report.delivered = Some(true);
        }
        Ok(Outcome::Refused(refusal)) => {
            let detail = format!("chat destination refused notification: {}", refusal.code);
            logger::error!(heading, rule_id = %rule.id, error = %detail, "Evaluator notification refused");
            prepared.report.delivered = Some(false);
            record_delivery_error(&mut prepared.report, detail);
        }
        Err(error) => {
            let detail = short_error(&error);
            logger::error!(heading, rule_id = %rule.id, error = %detail, "Evaluator notification failed");
            prepared.report.delivered = Some(false);
            record_delivery_error(&mut prepared.report, detail);
        }
    }
}

fn record_delivery_error(report: &mut RuleReport, detail: String) {
    report.error = match report.error.take() {
        Some(error) => Some(format!("{error}: delivery failed: {detail}")),
        None => Some(detail),
    };
}

fn skipped_report(rule: &RuleDefinition, due: bool) -> RuleReport {
    RuleReport {
        id: rule.id.clone(),
        due,
        previous_level: None,
        level: None,
        message: None,
        delivered: None,
        error: None,
    }
}

fn render_transition(
    heading: &str,
    rule: &RuleDefinition,
    pair: &EvaluationPair,
    selected: &Evaluation,
    timestamp: &str,
) -> String {
    let previous = pair.previous.level;
    let current = pair.current.level;
    let state = match (previous, current) {
        (Level::Normal, _) => "OK → ALARM".to_owned(),
        (_, Level::Normal) => "ALARM → OK".to_owned(),
        _ => format!("ALARM ({previous}) → ALARM ({current})"),
    };
    let latest = match pair.current.latest_value {
        Some(value) => value.to_string(),
        None => "unavailable".to_owned(),
    };
    let description = selected.description.as_deref().unwrap_or_default();
    let metric_details = selected.metric_details.as_deref().unwrap_or_default();
    format!(
        "{heading}: {}\n\nSeverity: {}\nState: {state}\nEvaluated: {timestamp}\n\nDescription:\n{description}\n\n{metric_details}\nLatest value: {latest}\n\nDefinition: {}\nClassification: {}",
        rule.name,
        selected.level,
        rule.id,
        rule.config.inner().classification
    )
}

fn render_rule_error(rule: &RuleDefinition, timestamp: &str, error: &str) -> String {
    format!(
        "RULE ERROR: {}\n\nEvaluated: {timestamp}\n\nError:\n{error}\n\nDefinition: {}\nClassification: {}",
        rule.name,
        rule.id,
        rule.config.inner().classification
    )
}

fn short_error<C: error_stack::Context>(error: &Report<C>) -> String {
    let mut parts = Vec::new();
    for frame in error.frames() {
        let part = match frame.kind() {
            FrameKind::Context(context) => Some(context.to_string()),
            FrameKind::Attachment(AttachmentKind::Printable(attachment)) => {
                Some(attachment.to_string())
            }
            FrameKind::Attachment(_) => None,
        };
        if let Some(part) = part {
            if !part.is_empty() && parts.last() != Some(&part) {
                parts.push(part);
            }
        }
    }
    parts.join(": ")
}

fn format_timestamp(timestamp: PrimitiveDateTime) -> ObservabilityApiResult<String> {
    timestamp
        .format(time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] UTC"
        ))
        .change_context(ObservabilityError::InternalServerError)
}

struct RunContext<'a> {
    evaluators: &'a Registry<dyn AlertEvaluator>,
    chat: &'a Registry<dyn ChatNotifier>,
    destinations: &'a EvaluatorDestinations,
    now: PrimitiveDateTime,
    timestamp: &'a str,
    deliver: bool,
}

struct PreparedRule {
    report: RuleReport,
    destination: Option<String>,
    heading: Option<&'static str>,
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        sync::{Arc, Mutex},
    };

    use error_stack::report;
    use hyperswitch_masking::PeekInterface;
    use time::macros::datetime;

    use super::*;
    use crate::{
        domain::{
            evaluator::{ConfigRuleSource, EvaluatorResult},
            notifier::{
                chat::{ChatFileOutcome, ChatFileUpload, ChatOutcome, ChatReceipt},
                Refusal,
            },
        },
        settings::evaluator::test_rule,
    };

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    #[derive(Debug)]
    struct FakeEvaluator {
        results: Mutex<VecDeque<EvaluatorResult<EvaluationPair>>>,
    }

    #[async_trait::async_trait]
    impl AlertEvaluator for FakeEvaluator {
        async fn evaluate(
            &self,
            _rule: &RuleDefinition,
            _now: PrimitiveDateTime,
        ) -> EvaluatorResult<EvaluationPair> {
            let mut results = self.results.lock().map_err(|_| {
                report!(EvaluatorError::UnexpectedResponse)
                    .attach_printable("fake evaluator result lock failed")
            })?;
            results.pop_front().ok_or_else(|| {
                report!(EvaluatorError::UnexpectedResponse)
                    .attach_printable("fake evaluator has no result")
            })?
        }
    }

    #[derive(Debug)]
    struct FakeChat {
        messages: Mutex<Vec<String>>,
        failures_remaining: Mutex<usize>,
    }

    #[async_trait::async_trait]
    impl ChatNotifier for FakeChat {
        async fn notify(
            &self,
            notification: ChatNotification,
        ) -> ObservabilityApiResult<ChatOutcome> {
            let mut failures = self.failures_remaining.lock().map_err(|_| {
                report!(ObservabilityError::InternalServerError)
                    .attach_printable("fake chat failure lock failed")
            })?;
            if *failures > 0 {
                *failures -= 1;
                return Err(report!(ObservabilityError::ProviderUnavailable {
                    destination: "test".to_owned(),
                }));
            }
            self.messages
                .lock()
                .map_err(|_| {
                    report!(ObservabilityError::InternalServerError)
                        .attach_printable("fake chat message lock failed")
                })?
                .push(notification.text.peek().clone());
            Ok(Outcome::Delivered(ChatReceipt {
                message_id: Some("message-1".to_owned()),
            }))
        }

        async fn upload_file(
            &self,
            _upload: ChatFileUpload,
        ) -> ObservabilityApiResult<ChatFileOutcome> {
            Err(report!(ObservabilityError::InvalidRequest))
        }
    }

    fn evaluation(level: Level, value: f64, condition: &str) -> Evaluation {
        Evaluation {
            level,
            latest_value: Some(value),
            condition: (level != Level::Normal).then(|| condition.to_owned()),
            description: (level != Level::Normal).then(|| format!("{level} evaluator description")),
            metric_details: (level != Level::Normal)
                .then(|| format!("Metric: {condition}\nCondition: {condition}")),
        }
    }

    fn pair(previous: Level, current: Level) -> EvaluationPair {
        EvaluationPair {
            previous: evaluation(previous, 80.0, "previous condition"),
            current: evaluation(current, 90.0, "current condition"),
        }
    }

    fn destinations() -> EvaluatorDestinations {
        EvaluatorDestinations {
            critical: "critical".to_owned(),
            warning: "warning".to_owned(),
            info: "info".to_owned(),
        }
    }

    fn evaluator_registry(
        results: Vec<EvaluatorResult<EvaluationPair>>,
    ) -> Registry<dyn AlertEvaluator> {
        let evaluator: Arc<dyn AlertEvaluator> = Arc::new(FakeEvaluator {
            results: Mutex::new(results.into()),
        });
        let rule = test_rule("registry", 60);
        let key = match rule {
            Ok(rule) => rule.config.inner().query.datasource().to_string(),
            Err(_) => "invalid-test-rule".to_owned(),
        };
        Registry::new(HashMap::from([(key, evaluator)]))
    }

    fn chat_registry(failures: usize) -> (Registry<dyn ChatNotifier>, Arc<FakeChat>) {
        let chat = Arc::new(FakeChat {
            messages: Mutex::new(Vec::new()),
            failures_remaining: Mutex::new(failures),
        });
        let notifier: Arc<dyn ChatNotifier> = chat.clone();
        (
            Registry::new(HashMap::from([
                ("critical".to_owned(), notifier.clone()),
                ("warning".to_owned(), notifier.clone()),
                ("info".to_owned(), notifier),
            ])),
            chat,
        )
    }

    async fn execute(
        rules: Vec<RuleDefinition>,
        evaluators: &Registry<dyn AlertEvaluator>,
        chat: &Registry<dyn ChatNotifier>,
        deliver: bool,
        now: PrimitiveDateTime,
    ) -> ObservabilityApiResult<EvaluatorResponse> {
        run_with(
            &ConfigRuleSource::new(rules),
            evaluators,
            chat,
            &destinations(),
            now,
            deliver,
        )
        .await
    }

    fn message_count(chat: &FakeChat) -> TestResult<usize> {
        Ok(chat
            .messages
            .lock()
            .map_err(|_| "fake chat message lock failed")?
            .len())
    }

    fn report_at(response: &EvaluatorResponse, index: usize) -> TestResult<&RuleReport> {
        response
            .rules
            .get(index)
            .ok_or_else(|| format!("missing report at index {index}").into())
    }

    #[tokio::test]
    async fn alarm_uses_current_description_condition_and_destination() -> TestResult {
        let rule = test_rule("alarm", 60)?;
        let evaluators = evaluator_registry(vec![Ok(pair(Level::Normal, Level::Critical))]);
        let (chat, fake) = chat_registry(0);
        let response = execute(
            vec![rule],
            &evaluators,
            &chat,
            true,
            datetime!(2026-10-21 10:08:00),
        )
        .await?;
        let Some(report) = response.rules.first() else {
            return Err("missing alarm report".into());
        };
        let Some(message) = report.message.as_deref() else {
            return Err("missing alarm message".into());
        };
        assert_eq!(
            message,
            "ALARM: Rule alarm\n\nSeverity: CRITICAL\nState: OK → ALARM\nEvaluated: 2026-10-21 10:08:00 UTC\n\nDescription:\nCRITICAL evaluator description\n\nMetric: current condition\nCondition: current condition\nLatest value: 90\n\nDefinition: alarm\nClassification: test-alerts"
        );
        assert!(!message.contains("\\n"));
        assert_eq!(report.delivered, Some(true));
        assert_eq!(message_count(&fake)?, 1);
        Ok(())
    }

    #[tokio::test]
    async fn resolved_uses_previous_description_and_condition() -> TestResult {
        let rule = test_rule("resolved", 60)?;
        let evaluators = evaluator_registry(vec![Ok(pair(Level::Warning, Level::Normal))]);
        let (chat, _) = chat_registry(0);
        let response = execute(
            vec![rule],
            &evaluators,
            &chat,
            true,
            datetime!(2026-10-21 10:08:00),
        )
        .await?;
        let Some(message) = response
            .rules
            .first()
            .and_then(|report| report.message.as_deref())
        else {
            return Err("missing resolved message".into());
        };
        assert_eq!(
            message,
            "RESOLVED: Rule resolved\n\nSeverity: WARNING\nState: ALARM → OK\nEvaluated: 2026-10-21 10:08:00 UTC\n\nDescription:\nWARNING evaluator description\n\nMetric: previous condition\nCondition: previous condition\nLatest value: 90\n\nDefinition: resolved\nClassification: test-alerts"
        );
        assert!(!message.contains("Latest value: 80"));
        let mut missing_current = pair(Level::Warning, Level::Normal);
        missing_current.current.latest_value = None;
        let message = render_transition(
            "RESOLVED",
            &test_rule("resolved", 60)?,
            &missing_current,
            &missing_current.previous,
            "2026-10-21 10:08:00 UTC",
        );
        assert!(message.contains("\nLatest value: unavailable\n"));
        assert!(!message.contains("Latest value: 80"));
        Ok(())
    }

    #[tokio::test]
    async fn severity_change_is_an_alarm() -> TestResult {
        for (previous, expected_state) in [
            (Level::Info, "ALARM (INFO) → ALARM (WARNING)"),
            (Level::Critical, "ALARM (CRITICAL) → ALARM (WARNING)"),
        ] {
            let rule = test_rule("severity-change", 60)?;
            let evaluators = evaluator_registry(vec![Ok(pair(previous, Level::Warning))]);
            let (chat, _) = chat_registry(0);
            let response = execute(
                vec![rule],
                &evaluators,
                &chat,
                true,
                datetime!(2026-10-21 10:08:00),
            )
            .await?;
            let message = report_at(&response, 0)?
                .message
                .as_deref()
                .ok_or("missing severity-change message")?;
            assert_eq!(
                message,
                format!("ALARM: Rule severity-change\n\nSeverity: WARNING\nState: {expected_state}\nEvaluated: 2026-10-21 10:08:00 UTC\n\nDescription:\nWARNING evaluator description\n\nMetric: current condition\nCondition: current condition\nLatest value: 90\n\nDefinition: severity-change\nClassification: test-alerts")
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn unchanged_level_has_no_message() -> TestResult {
        let rule = test_rule("unchanged", 60)?;
        let evaluators = evaluator_registry(vec![Ok(pair(Level::Critical, Level::Critical))]);
        let (chat, fake) = chat_registry(0);
        let response = execute(
            vec![rule],
            &evaluators,
            &chat,
            true,
            datetime!(2026-10-21 10:08:00),
        )
        .await?;
        assert!(report_at(&response, 0)?.message.is_none());
        assert_eq!(message_count(&fake)?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn missing_registry_entry_is_a_rule_error() -> TestResult {
        let rule = test_rule("missing-registry", 60)?;
        let (chat, fake) = chat_registry(0);
        let response = execute(
            vec![rule],
            &Registry::default(),
            &chat,
            true,
            datetime!(2026-10-21 10:08:00),
        )
        .await?;
        let report = report_at(&response, 0)?;
        assert!(report
            .message
            .as_deref()
            .is_some_and(|message| message.starts_with("RULE ERROR: ")));
        assert!(report
            .error
            .as_deref()
            .is_some_and(|error| error.contains("data source is not configured")));
        assert_eq!(message_count(&fake)?, 1);
        Ok(())
    }

    #[tokio::test]
    async fn evaluation_error_is_rendered_with_safe_detail() -> TestResult {
        let rule = test_rule("evaluation-error", 60)?;
        let evaluators =
            evaluator_registry(vec![Err(report!(EvaluatorError::DatasourceRequestFailed)
                .attach_printable("metric request error: X"))]);
        let (chat, _) = chat_registry(0);
        let response = execute(
            vec![rule],
            &evaluators,
            &chat,
            true,
            datetime!(2026-10-21 10:08:00),
        )
        .await?;
        assert_eq!(
            report_at(&response, 0)?.message.as_deref(),
            Some("RULE ERROR: Rule evaluation-error\n\nEvaluated: 2026-10-21 10:08:00 UTC\n\nError:\nmetric request error: X: data source request failed\n\nDefinition: evaluation-error\nClassification: test-alerts")
        );
        Ok(())
    }

    #[tokio::test]
    async fn send_failure_is_recorded_and_later_rules_continue() -> TestResult {
        let rules = vec![test_rule("first", 60)?, test_rule("second", 60)?];
        let evaluators = evaluator_registry(vec![
            Ok(pair(Level::Normal, Level::Critical)),
            Ok(pair(Level::Normal, Level::Critical)),
        ]);
        let (chat, fake) = chat_registry(1);
        let response = execute(
            rules,
            &evaluators,
            &chat,
            true,
            datetime!(2026-10-21 10:08:00),
        )
        .await?;
        assert_eq!(report_at(&response, 0)?.delivered, Some(false));
        assert_eq!(report_at(&response, 1)?.delivered, Some(true));
        assert_eq!(message_count(&fake)?, 1);
        Ok(())
    }

    #[tokio::test]
    async fn dry_run_ignores_due_and_never_sends() -> TestResult {
        let rule = test_rule("dry", 300)?;
        let evaluators = evaluator_registry(vec![Ok(pair(Level::Normal, Level::Info))]);
        let (chat, fake) = chat_registry(0);
        let response = execute(
            vec![rule],
            &evaluators,
            &chat,
            false,
            datetime!(2026-10-21 10:01:00),
        )
        .await?;
        let report = report_at(&response, 0)?;
        assert!(!report.due);
        assert!(report.message.is_some());
        assert_eq!(report.delivered, None);
        assert_eq!(message_count(&fake)?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn notify_skips_a_rule_that_is_not_due() -> TestResult {
        let rule = test_rule("skip", 300)?;
        let evaluators = evaluator_registry(Vec::new());
        let (chat, fake) = chat_registry(0);
        let response = execute(
            vec![rule],
            &evaluators,
            &chat,
            true,
            datetime!(2026-10-21 10:01:00),
        )
        .await?;
        let report = report_at(&response, 0)?;
        assert!(!report.due);
        assert!(report.message.is_none());
        assert_eq!(message_count(&fake)?, 0);
        Ok(())
    }

    #[test]
    fn timestamp_has_the_required_utc_shape() -> TestResult {
        assert_eq!(
            format_timestamp(datetime!(2026-10-21 10:08:00))?,
            "2026-10-21 10:08:00 UTC"
        );
        Ok(())
    }

    #[test]
    fn refusal_type_remains_an_outcome() {
        let outcome: ChatOutcome = Outcome::Refused(Refusal::new("rate_limited"));
        assert!(matches!(outcome, Outcome::Refused(_)));
    }
}
