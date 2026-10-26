pub mod grafana_cloudwatch;

use common_types::alert_rules::{AlertQuery, MetricSource};
use common_utils::date_time;
use error_stack::{AttachmentKind, FrameKind, Report, ResultExt};
use external_services::metrics_provider::grafana::cloudwatch::GrafanaCloudwatch;
use futures::future::join_all;
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

use crate::{
    core::notifier,
    domain::notifier::Outcome,
    domain_models::evaluator::{
        AlertEvaluationLevel, Delivery, EvaluatorError, EvaluatorResult, Notification,
        RuleDefinition, RuleRun,
    },
    errors::ObservabilityApiResult,
    logger,
    services::chat,
    state::AppState,
    types::{AlertEvaluationResponse, AlertRuleEvaluationResponse, ChatNotifyRequest},
};

#[async_trait::async_trait]
pub trait AlertEvaluator<Q>: Send + Sync
where
    Q: Send + Sync,
{
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        query: &Q,
        datasource_uid: &str,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<(AlertEvaluationLevel, Option<Notification>)>;

    async fn notify(&self, state: AppState, notification: &Notification) -> Delivery {
        deliver_notification(state, notification).await
    }
}

pub async fn evaluate(state: AppState, _: ()) -> ObservabilityApiResult<AlertEvaluationResponse> {
    let settings = state.conf.alert.get_inner();
    let now = date_time::now();
    let provider = GrafanaCloudwatch::new(settings.grafana.clone(), state.conf.proxy.clone());
    let rules = join_all(settings.rules.iter().map(|rule| {
        let provider = &provider;
        let state = &state;
        async move {
            let run = evaluate_rule(provider, state, rule, now).await;
            AlertRuleEvaluationResponse::from((rule, run, None))
        }
    }))
    .await;
    Ok(AlertEvaluationResponse {
        evaluated_at: now,
        rules,
    })
}

pub async fn notify(state: AppState, _: ()) -> ObservabilityApiResult<AlertEvaluationResponse> {
    let settings = state.conf.alert.get_inner();
    let now = date_time::now();
    let provider = GrafanaCloudwatch::new(settings.grafana.clone(), state.conf.proxy.clone());
    let rules = join_all(settings.rules.iter().map(|rule| {
        let provider = &provider;
        let state = state.clone();
        async move {
            let run = evaluate_rule(provider, &state, rule, now).await;
            let notification = match &run {
                RuleRun::Evaluated { notification, .. } => notification.as_ref(),
                RuleRun::Failed { notification, .. } => Some(notification),
            };
            let delivery = match notification {
                Some(notification) => Some(provider.notify(state, notification).await),
                None => None,
            };
            AlertRuleEvaluationResponse::from((rule, run, delivery))
        }
    }))
    .await;
    Ok(AlertEvaluationResponse {
        evaluated_at: now,
        rules,
    })
}

#[allow(clippy::expect_used)]
async fn evaluate_rule(
    provider: &GrafanaCloudwatch,
    state: &AppState,
    rule: &RuleDefinition,
    now: PrimitiveDateTime,
) -> RuleRun {
    let result = async {
        let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query;
        let datasource = state
            .conf
            .alert
            .get_inner()
            .datasources
            .get(&MetricSource::GrafanaCloudwatch)
            .ok_or(EvaluatorError::UnexpectedResponse)
            .attach_printable("rule metric source is not configured")?;
        provider.evaluate(rule, query, &datasource.uid, now).await
    }
    .await;
    match result {
        Ok((level, notification)) => RuleRun::Evaluated {
            level,
            notification,
        },
        Err(error) => {
            let error = format_error_report(&error);
            logger::error!(tag = "evaluator_rule_error", rule_id = %rule.id, error = %error, "Rule evaluation failed");
            let destination = &rule
                .config
                .thresholds
                .severities
                .first_key_value()
                .expect("validated alert rule must have a threshold")
                .1
                .destination;
            let notification = chat::create_notification(
                rule,
                now,
                chat::ChatBody::RuleError {
                    error: &error,
                    destination,
                },
            );
            RuleRun::Failed {
                error,
                notification,
            }
        }
    }
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
