pub mod cloudwatch;

use common_types::alert_rules::{AlertQuery, Severity};
use common_utils::date_time;
use error_stack::{AttachmentKind, FrameKind, Report};
use external_services::metrics_provider::grafana::cloudwatch::GrafanaCloudwatch;
use futures::future::join_all;
use hyperswitch_masking::Secret;

use crate::{
    core::notifier,
    domain::notifier::Outcome,
    domain_models::evaluator::{Delivery, EvaluatorError, Notification, RuleRun},
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    services::chat,
    state::AppState,
    types::{AlertEvaluationResponse, AlertRuleEvaluationResponse, ChatNotifyRequest},
};

pub async fn notify(state: AppState, _: ()) -> ObservabilityApiResult<AlertEvaluationResponse> {
    evaluate_alert_rules(&state, true).await
}

pub async fn evaluate(state: AppState, _: ()) -> ObservabilityApiResult<AlertEvaluationResponse> {
    evaluate_alert_rules(&state, false).await
}

async fn evaluate_alert_rules(
    state: &AppState,
    deliver: bool,
) -> ObservabilityApiResult<AlertEvaluationResponse> {
    let settings = state.conf.alert.get_inner();
    let now = date_time::now();
    let provider = GrafanaCloudwatch::new(settings.grafana.clone(), state.conf.proxy.clone());
    let rules = settings
        .rules()
        .iter()
        .map(|rule| {
            let destination = [Severity::Critical, Severity::Warning, Severity::Info]
                .iter()
                .find_map(|severity| {
                    rule.config
                        .thresholds
                        .severities
                        .get(severity)
                        .map(|threshold| threshold.destination.as_str())
                })
                .ok_or_else(|| {
                    Report::new(ObservabilityError::InternalServerError)
                        .attach_printable("rule must configure at least one threshold")
                })?;
            Ok((rule, destination))
        })
        .collect::<ObservabilityApiResult<Vec<_>>>()?;

    let reports = join_all(rules.into_iter().map(|(rule, error_destination)| {
        let provider = &provider;
        let state = state.clone();
        async move {
            let result = async {
                let uid = settings
                    .datasource_uid(rule.config.query.metric_source())
                    .ok_or_else(|| {
                        Report::new(EvaluatorError::UnexpectedResponse)
                            .attach_printable("rule metric source is not configured")
                    })?;
                let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query;
                let evaluation = cloudwatch::evaluate(
                    provider,
                    rule.metric_request(uid),
                    query,
                    &rule.config.thresholds,
                    now,
                )
                .await?;
                let notification = match (&evaluation.breach, evaluation.latest_value) {
                    (Some(threshold), Some(value)) => {
                        Some(chat::alarm(rule, query, &evaluation, threshold, value, now))
                    }
                    (Some(_), None) => {
                        return Err(Report::new(EvaluatorError::UnexpectedResponse)
                            .attach_printable("a measured breach is missing its current value"))
                    }
                    (None, _) => None,
                };
                Ok(RuleRun::Evaluated {
                    evaluation,
                    notification,
                })
            }
            .await;
            let run = match result {
                Ok(run) => run,
                Err(error) => {
                    let error = format_error_report(&error);
                    logger::error!(tag = "evaluator_rule_error", rule_id = %rule.id, error = %error, "Rule evaluation failed");
                    RuleRun::Failed {
                        notification: chat::rule_error(rule, now, &error, error_destination),
                        error,
                    }
                }
            };
            let delivery = match (deliver, run.notification()) {
                (true, Some(notification)) => {
                    Some(deliver_notification(state, notification).await)
                }
                _ => None,
            };
            AlertRuleEvaluationResponse::from((rule, run, delivery))
        }
    }))
    .await;

    Ok(AlertEvaluationResponse {
        evaluated_at: now,
        rules: reports,
    })
}

async fn deliver_notification(state: AppState, notification: &Notification) -> Delivery {
    match notifier::notify_chat(
        state,
        &notification.destination,
        ChatNotifyRequest {
            text: Secret::new(notification.message.clone()),
            reply_to: None,
        },
    )
    .await
    {
        Ok(Outcome::Delivered(receipt)) => {
            logger::info!(tag = "evaluator_notification", heading = %notification.heading, destination = %notification.destination, message_id = ?receipt.message_id, "Evaluator notification delivered");
            Delivery {
                delivered: true,
                error: None,
            }
        }
        Ok(Outcome::Refused(refusal)) => {
            let error = format!("chat destination refused notification: {}", refusal.code);
            logger::error!(tag = "evaluator_notification", heading = %notification.heading, destination = %notification.destination, error = %error, "Evaluator notification refused");
            Delivery {
                delivered: false,
                error: Some(error),
            }
        }
        Err(error) => {
            let error = format_error_report(&error);
            logger::error!(tag = "evaluator_notification", heading = %notification.heading, destination = %notification.destination, error = %error, "Evaluator notification failed");
            Delivery {
                delivered: false,
                error: Some(error),
            }
        }
    }
}

fn format_error_report<C: error_stack::Context>(error: &Report<C>) -> String {
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
