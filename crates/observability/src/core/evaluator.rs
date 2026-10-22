use common_types::alert_rules::AlertQuery;
use common_utils::date_time;
use error_stack::{AttachmentKind, FrameKind, Report};
use external_services::metrics_provider::grafana::GrafanaClient;
use hyperswitch_masking::Secret;

use crate::{
    domain::{
        evaluator::{
            cloudwatch::GrafanaCloudwatchEvaluator, message, Delivery, EvaluatorError, RuleRun,
        },
        notifier::{
            chat::{ChatNotification, ChatNotifier},
            Outcome, Registry,
        },
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
    types::{EvaluatorResponse, RuleReport},
};

pub async fn notify(state: AppState, _: ()) -> ObservabilityApiResult<EvaluatorResponse> {
    run(&state, true).await
}

pub async fn evaluate(state: AppState, _: ()) -> ObservabilityApiResult<EvaluatorResponse> {
    run(&state, false).await
}

async fn run(state: &AppState, deliver: bool) -> ObservabilityApiResult<EvaluatorResponse> {
    let settings = state.conf.alert.get_inner();
    let now = date_time::now();
    let timestamp = format!(
        "{} {:02}:{:02}:{:02} UTC",
        now.date(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let evaluator = GrafanaCloudwatchEvaluator::new(GrafanaClient::new(
        settings.grafana.clone(),
        state.conf.proxy.clone(),
    ));
    let mut reports = Vec::with_capacity(settings.rules().len());
    for rule in settings.rules() {
        let error_destination = &rule
            .config
            .thresholds
            .severities
            .first_key_value()
            .ok_or_else(|| {
                Report::new(ObservabilityError::InternalServerError)
                    .attach_printable("rule must configure at least one threshold")
            })?
            .1
            .destination;
        let result = async {
            let uid = settings
                .datasource_uid(rule.config.query.datasource())
                .ok_or_else(|| {
                    Report::new(EvaluatorError::UnexpectedResponse)
                        .attach_printable("rule datasource is not configured")
                })?;
            let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query;
            let evaluation = evaluator
                .evaluate(query, uid, &rule.config.thresholds, now)
                .await?;
            let notification = match (&evaluation.breach, evaluation.latest_value) {
                (Some(threshold), Some(value)) => Some(message::alarm(
                    rule,
                    query,
                    &evaluation,
                    threshold,
                    value,
                    &timestamp,
                )),
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
                let error = short_error(&error);
                logger::error!(tag = "evaluator_rule_error", rule_id = %rule.id, error = %error, "Rule evaluation failed");
                RuleRun::Failed {
                    notification: message::rule_error(rule, &timestamp, &error, error_destination),
                    error,
                }
            }
        };
        let delivery = if deliver {
            match run.notification() {
                Some(notification) => Some(send(state.chat.as_ref(), notification).await),
                None => None,
            }
        } else {
            None
        };
        reports.push(RuleReport::from((rule, run, delivery)));
    }
    Ok(EvaluatorResponse {
        evaluated_at: now,
        rules: reports,
    })
}

async fn send(
    chat: &Registry<dyn ChatNotifier>,
    notification: &crate::domain::evaluator::Notification,
) -> Delivery {
    let Some(notifier) = chat.get(&notification.destination) else {
        let error = format!(
            "chat destination `{}` is not configured",
            notification.destination
        );
        logger::error!(tag = "evaluator_notification", heading = %notification.heading, destination = %notification.destination, error = %error, "Evaluator notification failed");
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
            let error = short_error(&error);
            logger::error!(tag = "evaluator_notification", heading = %notification.heading, destination = %notification.destination, error = %error, "Evaluator notification failed");
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
