use std::collections::{BTreeMap, BTreeSet};

use common_types::alert_rules::{
    AlertQuery, GrafanaLokiQuery, LokiQueryMode, LokiReducer, MetricSource,
};
use common_utils::date_time::{self, convert_to_pdt};
use error_stack::{Report, ResultExt};
use external_services::metrics_provider::{
    grafana::loki::{
        GrafanaLoki, LokiMetricsQuery, LokiMetricsRequest, LokiQueryMode as ProviderMode,
    },
    MetricRange, MetricSeries, MetricsProvider,
};
use futures::{stream, StreamExt};
use hyperswitch_masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::{
    core::alert_evaluation::AlertEvaluator,
    domain::notifier::Outcome,
    domain_models::evaluator::{
        AlertEvaluationLevel, EvaluatorError, EvaluatorResult, RuleDefinition,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    services::application_alert::{preview_labels, render_application_alert},
    state::AppState,
    types::{AlertRuleEvaluationStatus, ChatNotifyRequest},
};

const MAX_RETURNED_SERIES: usize = 1000;

#[derive(Debug, Serialize)]
pub struct ApplicationInstanceObservation {
    #[serde(skip_serializing)]
    pub labels: BTreeMap<String, String>,
    #[serde(rename = "labels")]
    pub display_labels: BTreeMap<String, String>,
    pub level: AlertEvaluationLevel,
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<ApplicationNotificationReport>,
}

#[derive(Debug, Serialize)]
pub struct ApplicationNotificationReport {
    pub fingerprint: String,
    pub destination: String,
    pub text: String,
    pub details_available: bool,
    pub status: String,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApplicationEvaluationBatch {
    pub instances: Vec<ApplicationInstanceObservation>,
    pub no_data: bool,
}

#[derive(Debug, Serialize)]
pub struct ApplicationRuleReport {
    pub id: String,
    pub status: AlertRuleEvaluationStatus,
    #[serde(flatten)]
    pub evaluation: Option<ApplicationEvaluationBatch>,
    pub error: Option<String>,
    pub delivery_blocked: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationNotifyRequest {
    pub rule_ids: Vec<String>,
    pub max_messages: u32,
}

#[derive(Debug, Serialize)]
pub struct ApplicationPreviewResponse {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub evaluated_at: PrimitiveDateTime,
    pub delivery_enabled: bool,
    pub rules: Vec<ApplicationRuleReport>,
}

fn prepare_request(
    rule: &RuleDefinition,
    query: &GrafanaLokiQuery,
    datasource_uid: &str,
    now: PrimitiveDateTime,
) -> EvaluatorResult<(MetricRange, LokiMetricsRequest)> {
    let period = i64::from(rule.config.interval_seconds);
    if period == 0 {
        return Err(Report::new(EvaluatorError::InvalidTimeRange));
    }
    let end_seconds = now.assume_utc().unix_timestamp().div_euclid(period) * period;
    let to = OffsetDateTime::from_unix_timestamp(end_seconds)
        .map(convert_to_pdt)
        .change_context(EvaluatorError::InvalidTimeRange)?;
    let from = to
        .checked_sub(Duration::seconds(i64::from(query.range_seconds)))
        .ok_or(EvaluatorError::InvalidTimeRange)?;
    let range = MetricRange { from, to };
    let expression = query
        .expanded_expression()
        .map_err(|_| Report::new(EvaluatorError::InvalidTimeRange))?;
    let request = GrafanaLoki::build_request(
        LokiMetricsQuery {
            datasource_uid: datasource_uid.to_owned(),
            expression,
            mode: match query.mode {
                LokiQueryMode::Instant => ProviderMode::Instant,
                LokiQueryMode::Range => ProviderMode::Range,
            },
            step_seconds: query.step_seconds,
        },
        range,
    )
    .change_context(EvaluatorError::DatasourceRequestFailed)?;
    Ok((range, request))
}

#[async_trait::async_trait]
impl AlertEvaluator<GrafanaLokiQuery> for GrafanaLoki {
    type Evaluation = ApplicationEvaluationBatch;

    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        query: &GrafanaLokiQuery,
        datasource_uid: &str,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<Self::Evaluation> {
        let (range, request) = prepare_request(rule, query, datasource_uid, now)?;
        let series = self
            .fetch_metrics(request)
            .await
            .change_context(EvaluatorError::DatasourceRequestFailed)?;
        evaluate_series(rule, range, series)
    }
}

fn evaluate_series(
    rule: &RuleDefinition,
    range: MetricRange,
    series: Vec<MetricSeries>,
) -> EvaluatorResult<ApplicationEvaluationBatch> {
    let AlertQuery::GrafanaLoki(query) = &rule.config.query else {
        return Err(Report::new(EvaluatorError::UnexpectedResponse));
    };
    if series.len() > MAX_RETURNED_SERIES {
        return Err(Report::new(EvaluatorError::UnexpectedResponse)
                .attach_printable("Loki result exceeds the preview series limit; no instances were silently discarded"));
    }
    let mut identities = BTreeSet::new();
    let instances = series
        .into_iter()
        .map(|series| {
            if !identities.insert(series.labels.clone()) {
                return Err(Report::new(EvaluatorError::UnexpectedResponse)
                    .attach_printable("Loki returned repeated series identity"));
            }
            let mut points = series
                .points
                .into_iter()
                .filter(|p| p.timestamp >= range.from && p.timestamp <= range.to)
                .collect::<Vec<_>>();
            points.sort_by_key(|p| p.timestamp);
            if points
                .windows(2)
                .any(|pair| matches!(pair, [first, second] if first.timestamp == second.timestamp))
            {
                return Err(Report::new(EvaluatorError::UnexpectedResponse)
                    .attach_printable("Loki returned duplicate sample timestamps"));
            }
            let value = if points.is_empty() {
                None
            } else {
                let count = u32::try_from(points.len())
                    .map(f64::from)
                    .map_err(|_| Report::new(EvaluatorError::UnexpectedResponse))?;
                let value = match query.reducer {
                    LokiReducer::Last => points
                        .last()
                        .map(|p| p.value)
                        .ok_or(EvaluatorError::UnexpectedResponse)?,
                    LokiReducer::Count => count,
                    LokiReducer::Sum => points.iter().map(|p| p.value).sum(),
                    LokiReducer::Minimum => {
                        points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min)
                    }
                    LokiReducer::Maximum => points
                        .iter()
                        .map(|p| p.value)
                        .fold(f64::NEG_INFINITY, f64::max),
                    LokiReducer::Mean => points.iter().map(|p| p.value).sum::<f64>() / count,
                };
                if !value.is_finite() {
                    return Err(Report::new(EvaluatorError::UnexpectedResponse));
                }
                Some(value)
            };
            let level = value.map_or(AlertEvaluationLevel::NoData, |value| {
                rule.config
                    .thresholds
                    .severities
                    .iter()
                    .find(|(_, threshold)| {
                        threshold
                            .comparison_operator
                            .breaches(value, threshold.threshold)
                    })
                    .map_or(AlertEvaluationLevel::Normal, |(&severity, _)| {
                        severity.into()
                    })
            });
            Ok(ApplicationInstanceObservation {
                labels: series.labels,
                display_labels: BTreeMap::new(),
                level,
                value,
                notification: None,
            })
        })
        .collect::<EvaluatorResult<Vec<_>>>()?;
    let no_data = instances.iter().all(|instance| instance.value.is_none());
    Ok(ApplicationEvaluationBatch { instances, no_data })
}

pub async fn evaluate(
    state: AppState,
    _: (),
) -> ObservabilityApiResult<ApplicationPreviewResponse> {
    process(state, None).await
}

pub async fn notify(
    state: AppState,
    request: ApplicationNotifyRequest,
) -> ObservabilityApiResult<ApplicationPreviewResponse> {
    if request.rule_ids.is_empty()
        || request.rule_ids.len() > 31
        || !(1..=20).contains(&request.max_messages)
    {
        return Err(
            Report::new(ObservabilityError::InvalidRequest).attach_printable(
                "Select 1–31 rule IDs and a maximum of 1–20 messages for a one-shot delivery",
            ),
        );
    }
    process(state, Some(request)).await
}

async fn process(
    state: AppState,
    request: Option<ApplicationNotifyRequest>,
) -> ObservabilityApiResult<ApplicationPreviewResponse> {
    let settings = state.conf.alert.get_inner();
    let provider = GrafanaLoki::new(settings.grafana.clone(), state.conf.proxy.clone());
    let now = date_time::now();
    let selected = request.as_ref().map_or(&[][..], |r| r.rule_ids.as_slice());
    let definitions = crate::core::alert_rules::load_selected_rules(
        &state,
        MetricSource::GrafanaLoki,
        request.is_some(),
        now,
        selected,
    )
    .await?;
    let redactions = credential_redactions(&state);
    let secrets = redactions.iter().map(String::as_str).collect::<Vec<_>>();
    let evaluated = stream::iter(definitions.iter())
        .map(|rule| {
            let provider = &provider;
            let secrets = &secrets;
            async move {
                let result = async {
                    let datasource = settings
                        .datasources
                        .get(&MetricSource::GrafanaLoki)
                        .ok_or(EvaluatorError::UnexpectedResponse)?;
                    let AlertQuery::GrafanaLoki(query) = &rule.config.query else {
                        return Err(Report::new(EvaluatorError::UnexpectedResponse));
                    };
                    let mut evaluation =
                        provider.evaluate(rule, query, &datasource.uid, now).await?;
                    for instance in &mut evaluation.instances {
                        let mut labels = instance.labels.clone();
                        if let AlertQuery::GrafanaLoki(query) = &rule.config.query {
                            labels.extend(query.labels.clone());
                        }
                        instance.display_labels = preview_labels(&labels, secrets);
                        if let Some(threshold) = instance.value.and_then(|value| {
                            rule.config
                                .thresholds
                                .severities
                                .iter()
                                .find(|(_, threshold)| {
                                    threshold
                                        .comparison_operator
                                        .breaches(value, threshold.threshold)
                                })
                                .map(|(_, threshold)| threshold)
                        }) {
                            let rendered =
                                render_application_alert(rule, instance, threshold, now, secrets)?;
                            instance.notification = Some(ApplicationNotificationReport {
                                fingerprint: rendered.fingerprint,
                                destination: rendered.destination,
                                text: rendered.notification.text.peek().clone(),
                                details_available: rendered.details_available,
                                status: "preview".to_owned(),
                                message_id: None,
                                error: None,
                            });
                        }
                    }
                    Ok(evaluation)
                }
                .await;
                let blocked = match &rule.config.query {
                    AlertQuery::GrafanaLoki(query) if !query.delivery_enabled => {
                        Some("Delivery disabled: rule requires explicit reviewed opt-in".to_owned())
                    }
                    AlertQuery::GrafanaLoki(query) if query.pending_seconds != 0 => Some(
                        "Pending timers require persisted instance state; preview only".to_owned(),
                    ),
                    _ => None,
                };
                match result {
                    Ok(evaluation) => ApplicationRuleReport {
                        id: rule.id.clone(),
                        status: AlertRuleEvaluationStatus::Evaluated,
                        evaluation: Some(evaluation),
                        error: None,
                        delivery_blocked: blocked,
                    },
                    Err(error) => ApplicationRuleReport {
                        id: rule.id.clone(),
                        status: AlertRuleEvaluationStatus::Failed,
                        evaluation: None,
                        error: Some(super::format_error_report(&error)),
                        delivery_blocked: blocked,
                    },
                }
            }
        })
        .buffered(4)
        .collect::<Vec<_>>()
        .await;
    let mut rules = evaluated;
    if let Some(request) = &request {
        let mut remaining = usize::try_from(request.max_messages)
            .map_err(|_| Report::new(ObservabilityError::InvalidRequest))?;
        for rule in &mut rules {
            if rule.delivery_blocked.is_some() {
                continue;
            }
            let Some(evaluation) = &mut rule.evaluation else {
                continue;
            };
            let count = evaluation
                .instances
                .iter()
                .filter(|i| i.notification.is_some())
                .count();
            if count > remaining {
                rule.delivery_blocked = Some("Rule exceeds remaining one-shot message budget; no instances from this rule were sent".to_owned());
                continue;
            }
            for message in evaluation
                .instances
                .iter_mut()
                .filter_map(|instance| instance.notification.as_mut())
            {
                remaining -= 1;
                let outcome = crate::core::notifier::notify_chat(
                    state.clone(),
                    &message.destination,
                    ChatNotifyRequest {
                        text: Secret::new(message.text.clone()),
                        reply_to: None,
                    },
                )
                .await;
                match outcome {
                    Ok(Outcome::Delivered(receipt)) => {
                        message.status = "delivered".to_owned();
                        message.message_id = receipt.message_id;
                    }
                    Ok(Outcome::Refused(refusal)) => {
                        message.status = "refused".to_owned();
                        message.error = Some(refusal.code);
                    }
                    Err(_) => {
                        message.status = "failed".to_owned();
                        message.error = Some("Provider delivery failed; no retry attempted and delivery is unconfirmed".to_owned());
                    }
                }
            }
        }
        // Missing, inactive and not-due requested IDs are explicit, never silently omitted.
        for id in &request.rule_ids {
            if !rules.iter().any(|rule| &rule.id == id) {
                rules.push(ApplicationRuleReport {
                    id: id.clone(),
                    status: AlertRuleEvaluationStatus::Failed,
                    evaluation: None,
                    error: Some(
                        "Rule is absent, inactive, not due, or belongs to a different datasource"
                            .to_owned(),
                    ),
                    delivery_blocked: Some("Not evaluated or delivered".to_owned()),
                });
            }
        }
    }
    Ok(ApplicationPreviewResponse {
        evaluated_at: now,
        delivery_enabled: request.is_some(),
        rules,
    })
}

fn credential_redactions(state: &AppState) -> Vec<String> {
    let mut secrets = vec![
        state
            .conf
            .alert
            .get_inner()
            .grafana
            .api_token
            .peek()
            .clone(),
        state.conf.auth.get_inner().internal_api_key.peek().clone(),
    ];
    for destination in state.conf.chat.get_inner().destinations.values() {
        match destination {
            crate::settings::ChatDestination::Xyne(config) => {
                secrets.push(config.app_jwt.peek().clone())
            }
            crate::settings::ChatDestination::Slack(config) => {
                secrets.push(config.bot_token.peek().clone())
            }
            crate::settings::ChatDestination::Log => {}
        }
    }
    secrets
}

#[cfg(test)]
mod tests {
    use external_services::metrics_provider::MetricPoint;
    use serde_json::json;
    use time::macros::datetime;

    use super::*;

    #[test]
    fn loki_uses_the_last_completed_rule_interval() {
        let rule: RuleDefinition = serde_json::from_value(json!({
            "id":"window","name":"Window","config":{
                "interval_seconds":60,
                "query":{"datasource":"grafana_loki","expression":"vector(1)","mode":"instant","range_seconds":300,"reducer":"last"},
                "thresholds":{"severities":{"critical":{"threshold":0,"comparison_operator":"GreaterThanThreshold","description":"Test","destination":"smoke"}}}
            }
        })).unwrap();
        let AlertQuery::GrafanaLoki(query) = &rule.config.query else {
            panic!("Loki fixture required")
        };
        let (range, request) =
            prepare_request(&rule, query, "loki", datetime!(2026-10-27 08:49:08.899)).unwrap();
        assert_eq!(range.from, datetime!(2026-10-27 08:44:00));
        assert_eq!(range.to, datetime!(2026-10-27 08:49:00));
        let partial = MetricSeries {
            labels: BTreeMap::new(),
            points: vec![MetricPoint {
                timestamp: datetime!(2026-10-27 08:49:08),
                value: 2.0,
            }],
        };
        assert!(
            evaluate_series(&rule, range, vec![partial])
                .unwrap()
                .no_data
        );
        let request = serde_json::to_value(request).unwrap();
        assert_eq!(
            request["from"],
            (range.from.assume_utc().unix_timestamp() * 1000).to_string()
        );
        assert_eq!(
            request["to"],
            (range.to.assume_utc().unix_timestamp() * 1000).to_string()
        );
    }
}
