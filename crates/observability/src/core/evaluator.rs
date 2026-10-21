use error_stack::{report, AttachmentKind, FrameKind, Report, ResultExt};
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

use crate::{
    domain::{
        evaluator::{
            Delivery, EvaluationPair, Level, Notification, RuleDefinition, RuleEvaluator, RuleRun,
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

pub async fn notify(state: AppState, _: ()) -> ObservabilityApiResult<EvaluatorResponse> {
    run(&state, Mode::Deliver).await
}

pub async fn evaluate(state: AppState, _: ()) -> ObservabilityApiResult<EvaluatorResponse> {
    run(&state, Mode::DryRun).await
}

async fn run(state: &AppState, mode: Mode) -> ObservabilityApiResult<EvaluatorResponse> {
    let runtime = state.evaluator.as_ref().ok_or_else(|| {
        report!(ObservabilityError::InternalServerError)
            .attach_printable("evaluator is not configured")
    })?;
    let now = common_utils::date_time::now();
    let rules = match mode {
        Mode::Deliver => runtime.rule_source.due_rules(now).await,
        Mode::DryRun => runtime.rule_source.rules().await,
    }
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("failed to load evaluator rules")?;
    let timestamp = format_timestamp(now)?;
    let context = RunContext {
        evaluator: runtime.evaluator.as_ref(),
        chat: state.chat.as_ref(),
        destinations: &runtime.destinations,
        now,
        timestamp: &timestamp,
    };
    Ok(EvaluatorResponse {
        evaluated_at: now,
        rules: evaluate_rules(&rules, &context, mode).await,
    })
}

async fn evaluate_rules(
    rules: &[RuleDefinition],
    context: &RunContext<'_>,
    mode: Mode,
) -> Vec<RuleReport> {
    let mut reports = Vec::with_capacity(rules.len());
    for rule in rules {
        let run = run_rule(rule, context).await;
        let delivery = match (mode, run.notification()) {
            (Mode::Deliver, Some(notification)) => Some(send(context.chat, notification).await),
            _ => None,
        };
        reports.push(RuleReport::from((rule, run, delivery)));
    }
    reports
}

async fn run_rule(rule: &RuleDefinition, context: &RunContext<'_>) -> RuleRun {
    match context.evaluator.evaluate(rule, context.now).await {
        Ok(pair) => {
            let notification = transition(&pair)
                .map(|transition| notification_for(transition, rule, &pair, context));
            RuleRun::Evaluated { pair, notification }
        }
        Err(error) => {
            let error = short_error(&error);
            logger::error!(
                tag = "evaluator_rule_error",
                rule_id = %rule.id,
                error = %error,
                "Rule evaluation failed"
            );
            RuleRun::Failed {
                notification: rule_error_notification(rule, context, &error),
                error,
            }
        }
    }
}

fn transition(pair: &EvaluationPair) -> Option<Transition> {
    match (pair.previous.level, pair.current.level) {
        (Level::Normal, Level::Normal) => None,
        (_, Level::Normal) => Some(Transition::Resolved),
        _ => Some(Transition::Alarm),
    }
}

fn notification_for(
    transition: Transition,
    rule: &RuleDefinition,
    pair: &EvaluationPair,
    context: &RunContext<'_>,
) -> Notification {
    let (selected, state) = match transition {
        Transition::Alarm => {
            let state = if pair.previous.level == Level::Normal {
                "OK → ALARM".to_owned()
            } else {
                format!(
                    "ALARM ({}) → ALARM ({})",
                    pair.previous.level, pair.current.level
                )
            };
            (&pair.current, state)
        }
        Transition::Resolved => (&pair.previous, "ALARM → OK".to_owned()),
    };
    let breach = selected.breach.as_ref();
    let description = breach.map_or("", |breach| breach.description.as_str());
    let metric_details = breach.map_or("", |breach| breach.metric_details.as_str());
    let latest = pair
        .current
        .latest_value
        .map_or_else(|| "unavailable".to_owned(), |value| value.to_string());
    let heading = transition.to_string();
    let message = format!(
        "{heading}: {}\n\nSeverity: {}\nState: {state}\nEvaluated: {}\n\nDescription:\n{description}\n\n{metric_details}\nLatest value: {latest}\n\nDefinition: {}\nClassification: {}",
        rule.name,
        selected.level,
        context.timestamp,
        rule.id,
        rule.config.inner().classification
    );
    Notification {
        heading,
        destination: context.destinations.for_severity(selected.level).to_owned(),
        message,
    }
}

fn rule_error_notification(
    rule: &RuleDefinition,
    context: &RunContext<'_>,
    error: &str,
) -> Notification {
    Notification {
        heading: "RULE ERROR".to_owned(),
        destination: context
            .destinations
            .for_severity(Level::Critical)
            .to_owned(),
        message: format!(
            "RULE ERROR: {}\n\nEvaluated: {}\n\nError:\n{error}\n\nDefinition: {}\nClassification: {}",
            rule.name,
            context.timestamp,
            rule.id,
            rule.config.inner().classification
        ),
    }
}

async fn send(chat: &Registry<dyn ChatNotifier>, notification: &Notification) -> Delivery {
    let Some(notifier) = chat.get(&notification.destination) else {
        let error = format!(
            "chat destination `{}` is not configured",
            notification.destination
        );
        logger::error!(
            tag = "evaluator_notification",
            heading = %notification.heading,
            destination = %notification.destination,
            error = %error,
            "Evaluator notification failed"
        );
        return Delivery {
            delivered: false,
            error: Some(error),
        };
    };
    match notifier
        .notify(ChatNotification {
            text: Secret::new(notification.message.clone()),
            reply_to: None,
        })
        .await
    {
        Ok(Outcome::Delivered(receipt)) => {
            logger::info!(
                tag = "evaluator_notification",
                heading = %notification.heading,
                destination = %notification.destination,
                message_id = ?receipt.message_id,
                "Evaluator notification delivered"
            );
            Delivery {
                delivered: true,
                error: None,
            }
        }
        Ok(Outcome::Refused(refusal)) => {
            let error = format!("chat destination refused notification: {}", refusal.code);
            logger::error!(
                tag = "evaluator_notification",
                heading = %notification.heading,
                destination = %notification.destination,
                error = %error,
                "Evaluator notification refused"
            );
            Delivery {
                delivered: false,
                error: Some(error),
            }
        }
        Err(error) => {
            let error = short_error(&error);
            logger::error!(
                tag = "evaluator_notification",
                heading = %notification.heading,
                destination = %notification.destination,
                error = %error,
                "Evaluator notification failed"
            );
            Delivery {
                delivered: false,
                error: Some(error),
            }
        }
    }
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

#[derive(Clone, Copy)]
enum Mode {
    Deliver,
    DryRun,
}

#[derive(Clone, Copy, strum::Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum Transition {
    Alarm,
    Resolved,
}

struct RunContext<'a> {
    evaluator: &'a dyn RuleEvaluator,
    chat: &'a Registry<dyn ChatNotifier>,
    destinations: &'a EvaluatorDestinations,
    now: PrimitiveDateTime,
    timestamp: &'a str,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::{
        collections::{BTreeMap, HashMap, VecDeque},
        sync::{Arc, Mutex},
    };

    use common_types::alert_rules::{
        AlertQuery, AlertRuleConfig, AlertRuleConfigStoredV1, AlertThresholdsV1, CloudwatchQuery,
        CloudwatchQueryV1, CloudwatchStatistic,
    };
    use error_stack::report;
    use external_services::grafana::GrafanaError;
    use hyperswitch_masking::PeekInterface;
    use time::macros::datetime;

    use super::*;
    use crate::domain::{
        evaluator::{Breach, Evaluation, EvaluatorError, EvaluatorResult},
        notifier::chat::{ChatFileOutcome, ChatFileUpload, ChatOutcome, ChatReceipt},
    };

    #[derive(Debug)]
    struct FakeEvaluator {
        results: Mutex<VecDeque<EvaluatorResult<EvaluationPair>>>,
    }

    #[async_trait::async_trait]
    impl RuleEvaluator for FakeEvaluator {
        async fn evaluate(
            &self,
            _rule: &RuleDefinition,
            _now: PrimitiveDateTime,
        ) -> EvaluatorResult<EvaluationPair> {
            self.results.lock().unwrap().pop_front().unwrap()
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
            let mut failures = self.failures_remaining.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return Err(report!(ObservabilityError::ProviderUnavailable {
                    destination: "test".to_owned(),
                }));
            }
            self.messages
                .lock()
                .unwrap()
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

    fn rule(id: &str) -> RuleDefinition {
        RuleDefinition {
            id: id.to_owned(),
            name: format!("Rule {id}"),
            config: AlertRuleConfig::V1(AlertRuleConfigStoredV1 {
                classification: "test-alerts".to_owned(),
                interval_seconds: 60,
                query: AlertQuery::Cloudwatch(CloudwatchQuery::V1(CloudwatchQueryV1 {
                    region: "us-east-1".to_owned(),
                    namespace: "Test".to_owned(),
                    metric_name: "Value".to_owned(),
                    dimensions: BTreeMap::new(),
                    statistic: CloudwatchStatistic::Average,
                    period: 60,
                    end_offset_seconds: 0,
                })),
                thresholds: AlertThresholdsV1 {
                    severities: BTreeMap::new(),
                },
            }),
        }
    }

    fn evaluation(level: Level, value: Option<f64>, condition: &str) -> Evaluation {
        Evaluation {
            level,
            latest_value: value,
            breach: (level != Level::Normal).then(|| Breach {
                condition: condition.to_owned(),
                description: format!("{level} evaluator description"),
                metric_details: format!("Metric: {condition}\nCondition: {condition}"),
            }),
        }
    }

    fn pair(previous: Level, current: Level) -> EvaluationPair {
        EvaluationPair {
            previous: evaluation(previous, Some(80.0), "previous condition"),
            current: evaluation(current, Some(90.0), "current condition"),
        }
    }

    fn destinations() -> EvaluatorDestinations {
        EvaluatorDestinations {
            critical: "critical".to_owned(),
            warning: "warning".to_owned(),
            info: "info".to_owned(),
        }
    }

    fn chat_registry(
        failures: usize,
        configured: bool,
    ) -> (Registry<dyn ChatNotifier>, Arc<FakeChat>) {
        let chat = Arc::new(FakeChat {
            messages: Mutex::new(Vec::new()),
            failures_remaining: Mutex::new(failures),
        });
        let notifier: Arc<dyn ChatNotifier> = chat.clone();
        let destinations = configured.then(|| {
            HashMap::from([
                ("critical".to_owned(), notifier.clone()),
                ("warning".to_owned(), notifier.clone()),
                ("info".to_owned(), notifier),
            ])
        });
        (Registry::new(destinations.unwrap_or_default()), chat)
    }

    async fn execute(
        rules: Vec<RuleDefinition>,
        results: Vec<EvaluatorResult<EvaluationPair>>,
        chat: &Registry<dyn ChatNotifier>,
        mode: Mode,
    ) -> EvaluatorResponse {
        let evaluator = FakeEvaluator {
            results: Mutex::new(results.into()),
        };
        let timestamp = "2026-10-21 10:08:00 UTC";
        let destinations = destinations();
        let context = RunContext {
            evaluator: &evaluator,
            chat,
            destinations: &destinations,
            now: datetime!(2026-10-21 10:08:00),
            timestamp,
        };
        EvaluatorResponse {
            evaluated_at: context.now,
            rules: evaluate_rules(&rules, &context, mode).await,
        }
    }

    #[tokio::test]
    async fn alarm_repeats_changes_then_resolves() {
        let rules = vec![
            rule("tick-1"),
            rule("tick-2"),
            rule("tick-3"),
            rule("tick-4"),
        ];
        let (chat, fake) = chat_registry(0, true);
        let mut resolved = pair(Level::Warning, Level::Normal);
        resolved.current.latest_value = None;
        let response = execute(
            rules,
            vec![
                Ok(pair(Level::Normal, Level::Critical)),
                Ok(pair(Level::Critical, Level::Critical)),
                Ok(pair(Level::Critical, Level::Warning)),
                Ok(resolved),
            ],
            &chat,
            Mode::Deliver,
        )
        .await;

        let messages = fake.messages.lock().unwrap();
        assert_eq!(messages.len(), 4);
        assert!(messages[0].contains("\n\nSeverity: CRITICAL\nState: OK → ALARM"));
        assert!(!messages[0].contains("\\n"));
        assert!(messages[1].contains("State: ALARM (CRITICAL) → ALARM (CRITICAL)"));
        assert!(messages[2].contains("State: ALARM (CRITICAL) → ALARM (WARNING)"));
        assert!(messages[3].contains("RESOLVED: Rule tick-4"));
        assert!(messages[3].contains("Metric: previous condition"));
        assert!(messages[3].contains("Latest value: unavailable"));
        assert!(!messages[3].contains("Latest value: 80"));
        assert!(response
            .rules
            .iter()
            .all(|report| report.delivered == Some(true)));
    }

    #[tokio::test]
    async fn normal_to_normal_is_quiet() {
        let (chat, fake) = chat_registry(0, true);
        let response = execute(
            vec![rule("normal")],
            vec![Ok(pair(Level::Normal, Level::Normal))],
            &chat,
            Mode::Deliver,
        )
        .await;

        assert!(fake.messages.lock().unwrap().is_empty());
        assert!(response.rules[0].message.is_none());
        assert_eq!(response.rules[0].delivered, None);
    }

    #[tokio::test]
    async fn evaluation_error_is_rendered_with_safe_detail() {
        let (chat, fake) = chat_registry(0, true);
        let response = execute(
            vec![rule("evaluation-error")],
            vec![Err(Report::new(GrafanaError::QueryRefused {
                reason: "metric request error: X".to_owned(),
            })
            .change_context(EvaluatorError::DatasourceRequestFailed))],
            &chat,
            Mode::Deliver,
        )
        .await;

        let report = &response.rules[0];
        assert!(report.message.as_deref().is_some_and(
            |message| message.contains("Grafana refused the query: metric request error: X")
        ));
        assert!(report
            .error
            .as_deref()
            .is_some_and(|error| error.contains("data source request failed")));
        assert_eq!(fake.messages.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn send_failure_is_recorded_and_later_rules_continue() {
        let (chat, fake) = chat_registry(1, true);
        let response = execute(
            vec![rule("first"), rule("second")],
            vec![
                Ok(pair(Level::Normal, Level::Critical)),
                Ok(pair(Level::Normal, Level::Critical)),
            ],
            &chat,
            Mode::Deliver,
        )
        .await;

        assert_eq!(response.rules[0].delivered, Some(false));
        assert!(response.rules[0].delivery_error.is_some());
        assert_eq!(response.rules[1].delivered, Some(true));
        assert_eq!(fake.messages.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn missing_destination_is_recorded() {
        let (chat, _) = chat_registry(0, false);
        let response = execute(
            vec![rule("missing")],
            vec![Ok(pair(Level::Normal, Level::Critical))],
            &chat,
            Mode::Deliver,
        )
        .await;

        assert_eq!(response.rules[0].delivered, Some(false));
        assert!(response.rules[0]
            .delivery_error
            .as_deref()
            .is_some_and(|error| error.contains("is not configured")));
    }

    #[tokio::test]
    async fn dry_run_evaluates_without_sending() {
        let (chat, fake) = chat_registry(0, true);
        let response = execute(
            vec![rule("dry")],
            vec![Ok(pair(Level::Normal, Level::Info))],
            &chat,
            Mode::DryRun,
        )
        .await;

        assert!(response.rules[0].message.is_some());
        assert_eq!(response.rules[0].delivered, None);
        assert!(fake.messages.lock().unwrap().is_empty());
    }

    #[test]
    fn timestamp_has_the_required_utc_shape() {
        assert_eq!(
            format_timestamp(datetime!(2026-10-21 10:08:00)).unwrap(),
            "2026-10-21 10:08:00 UTC"
        );
    }
}
