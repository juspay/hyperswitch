pub mod grafana_cloudwatch;
pub mod grafana_loki;

use common_types::alert_rules::{AlertQuery, AlertThresholds, MetricSource};
use common_utils::date_time::{self, convert_to_pdt};
use error_stack::{AttachmentKind, FrameKind, Report, ResultExt};
use external_services::metrics_provider::{
    grafana::{cloudwatch::GrafanaCloudwatch, loki::GrafanaLoki},
    MetricRange, MetricSeries, MetricsProvider,
};
use futures::future::join_all;
use hyperswitch_masking::Secret;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::{
    core::notifier,
    domain::notifier::{
        chat::{AlertState, ChatAlert},
        Outcome,
    },
    domain_models::evaluator::{
        AlertEvaluationLevel, Delivery, Evaluation, EvaluatorError, EvaluatorResult, Notification,
        RuleDefinition, RuleRun,
    },
    errors::ObservabilityApiResult,
    logger,
    services::chat,
    state::AppState,
    types::{AlertEvaluationResponse, AlertRuleEvaluationResponse, ChatNotifyRequest},
};

const LOOKBACK_SECONDS: u32 = 300;

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
    ) -> EvaluatorResult<Vec<Evaluation>>;
}

pub async fn evaluate(
    state: AppState,
    source: MetricSource,
) -> ObservabilityApiResult<AlertEvaluationResponse> {
    let settings = state.conf.alert.get_inner();
    let now = date_time::now();
    let rules = join_all(
        settings
            .rules
            .iter()
            .filter(|rule| rule.config.query.metric_source() == source)
            .map(|rule| {
                let state = &state;
                async move {
                    evaluate_rule(state, rule, now)
                        .await
                        .into_iter()
                        .map(|run| AlertRuleEvaluationResponse::from((rule, run, None)))
                        .collect::<Vec<_>>()
                }
            }),
    )
    .await
    .into_iter()
    .flatten()
    .collect();
    Ok(AlertEvaluationResponse {
        evaluated_at: now,
        rules,
    })
}

pub async fn notify(
    state: AppState,
    source: MetricSource,
) -> ObservabilityApiResult<AlertEvaluationResponse> {
    let settings = state.conf.alert.get_inner();
    let now = date_time::now();
    let rules = join_all(
        settings
            .rules
            .iter()
            .filter(|rule| rule.config.query.metric_source() == source)
            .map(|rule| {
                let state = state.clone();
                async move {
                    let mut responses = Vec::new();
                    for run in evaluate_rule(&state, rule, now).await {
                        let notification = match &run {
                            RuleRun::Evaluated { notification, .. } => notification.as_ref(),
                            RuleRun::Failed { notification, .. } => Some(notification),
                        };
                        let delivery = match notification {
                            Some(notification) => {
                                Some(deliver_notification(state.clone(), notification).await)
                            }
                            None => None,
                        };
                        responses.push(AlertRuleEvaluationResponse::from((rule, run, delivery)));
                    }
                    responses
                }
            }),
    )
    .await
    .into_iter()
    .flatten()
    .collect();
    Ok(AlertEvaluationResponse {
        evaluated_at: now,
        rules,
    })
}

pub async fn evaluate_metrics<P: MetricsProvider>(
    provider: &P,
    metrics_query: P::MetricsQuery,
    period: u32,
    end_offset_seconds: u32,
    thresholds: &AlertThresholds,
    now: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation> {
    let (series, window, last) =
        fetch_window(provider, metrics_query, period, end_offset_seconds, now).await?;
    evaluate_series(series, thresholds, window, last)
}

async fn fetch_window<P: MetricsProvider>(
    provider: &P,
    metrics_query: P::MetricsQuery,
    period: u32,
    end_offset_seconds: u32,
    now: PrimitiveDateTime,
) -> EvaluatorResult<(Vec<MetricSeries>, MetricRange, PrimitiveDateTime)> {
    if period == 0 || end_offset_seconds > 3600 {
        return Err(Report::new(EvaluatorError::InvalidTimeRange)
            .attach_printable("query period or end offset is outside supported bounds"));
    }
    let seconds =
        now.assume_utc().unix_timestamp() - i64::from(end_offset_seconds.saturating_sub(period));
    let period = i64::from(period);
    let end_seconds = seconds
        .checked_div_euclid(period)
        .map(|bucket| bucket * period)
        .ok_or(EvaluatorError::InvalidTimeRange)
        .attach_printable("evaluation period must be greater than zero")?;
    let to = OffsetDateTime::from_unix_timestamp(end_seconds)
        .map(convert_to_pdt)
        .change_context(EvaluatorError::InvalidTimeRange)?;
    let (from, last) = to
        .checked_sub(Duration::seconds(i64::from(LOOKBACK_SECONDS).max(period)))
        .zip(to.checked_sub(Duration::seconds(period)))
        .ok_or(EvaluatorError::InvalidTimeRange)
        .attach_printable("query evaluation time range is outside supported bounds")?;
    let window = MetricRange { from, to };
    let request = P::build_request(metrics_query, window)
        .change_context(EvaluatorError::DatasourceRequestFailed)?;
    let series = provider
        .fetch_metrics(request)
        .await
        .change_context(EvaluatorError::DatasourceRequestFailed)?;
    Ok((series, window, last))
}

fn rule_run(
    rule: &RuleDefinition,
    evaluation: Evaluation,
    now: PrimitiveDateTime,
) -> EvaluatorResult<RuleRun> {
    let notification = match (&evaluation.breach, evaluation.latest_point.as_ref()) {
        (Some(threshold), Some(point)) => Some(chat::create_notification(
            rule,
            now,
            chat::ChatBody::Alarm {
                evaluation: &evaluation,
                threshold,
                point,
            },
        )),
        (Some(_), None) => {
            return Err(Report::new(EvaluatorError::UnexpectedResponse)
                .attach_printable("a measured breach is missing its current datapoint"))
        }
        (None, _) => None,
    };
    Ok(RuleRun::Evaluated {
        level: evaluation.level,
        labels: evaluation.labels,
        notification,
    })
}

#[allow(clippy::expect_used)]
async fn evaluate_rule(
    state: &AppState,
    rule: &RuleDefinition,
    now: PrimitiveDateTime,
) -> Vec<RuleRun> {
    let settings = state.conf.alert.get_inner();
    let result = async {
        let uid = &settings
            .datasources
            .get(&rule.config.query.metric_source())
            .ok_or(EvaluatorError::UnexpectedResponse)
            .attach_printable("rule metric source is not configured")?
            .uid;
        match &rule.config.query {
            AlertQuery::GrafanaCloudwatch(query) => {
                let provider =
                    GrafanaCloudwatch::new(settings.grafana.clone(), state.conf.proxy.clone());
                AlertEvaluator::evaluate(&provider, rule, query, uid, now).await
            }
            AlertQuery::GrafanaLoki(query) => {
                let provider = GrafanaLoki::new(settings.grafana.clone(), state.conf.proxy.clone());
                AlertEvaluator::evaluate(&provider, rule, query, uid, now).await
            }
        }
    }
    .await
    .and_then(|evaluations| {
        evaluations
            .into_iter()
            .map(|evaluation| rule_run(rule, evaluation, now))
            .collect::<EvaluatorResult<Vec<_>>>()
    });
    match result {
        Ok(runs) => runs,
        Err(error) => {
            let error = format_error_report(&error);
            logger::error!(tag = "evaluator_rule_error", rule_id = %rule.id, error = %error, "Rule evaluation failed");
            let destination = rule
                .config
                .thresholds
                .error_destination()
                .expect("validated alert rule must have a threshold");
            let notification = chat::create_notification(
                rule,
                now,
                chat::ChatBody::RuleError {
                    error: &error,
                    destination,
                },
            );
            vec![RuleRun::Failed {
                error,
                notification,
            }]
        }
    }
}

fn evaluate_series(
    series: Vec<MetricSeries>,
    thresholds: &AlertThresholds,
    window: MetricRange,
    last: PrimitiveDateTime,
) -> EvaluatorResult<Evaluation> {
    if series.len() > 1 {
        return Err(
            Report::new(EvaluatorError::UnexpectedResponse).attach_printable(format!(
                "query returned {} series; expected at most 1",
                series.len()
            )),
        );
    }
    let (points, labels) = series
        .into_iter()
        .next()
        .map(|series| (series.points, series.labels))
        .unwrap_or_default();
    let latest = points
        .iter()
        .filter(|point| (window.from..=last).contains(&point.timestamp))
        .max_by_key(|point| point.timestamp)
        .cloned();
    let Some(latest) = latest else {
        return Ok(Evaluation {
            level: AlertEvaluationLevel::NoData,
            latest_point: None,
            datapoints: points,
            breach: None,
            window,
            labels,
        });
    };
    if points
        .iter()
        .any(|point| point.timestamp == latest.timestamp && point.value != latest.value)
    {
        return Err(Report::new(EvaluatorError::UnexpectedResponse)
            .attach_printable("query returned conflicting values for the current timestamp"));
    }
    for (&severity, threshold) in &thresholds.severities {
        if threshold
            .comparison_operator
            .breaches(latest.value, threshold.threshold)
        {
            return Ok(Evaluation {
                level: severity.into(),
                latest_point: Some(latest),
                datapoints: points,
                breach: Some(threshold.clone()),
                window,
                labels,
            });
        }
    }
    Ok(Evaluation {
        level: AlertEvaluationLevel::Normal,
        latest_point: Some(latest),
        datapoints: points,
        breach: None,
        window,
        labels,
    })
}

async fn deliver_notification(state: AppState, notification: &Notification) -> Delivery {
    match notifier::notify_chat(
        state,
        &notification.destination,
        ChatNotifyRequest {
            text: Secret::new(notification.message.clone()),
            reply_to: None,
            alert: Some(ChatAlert {
                state: AlertState::Firing,
                severity: notification.severity,
                title: Secret::new(notification.title.clone()),
                region: Some(notification.region.clone()),
            }),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common_types::alert_rules::{ComparisonOperator, Severity, SeverityThreshold};
    use external_services::metrics_provider::MetricPoint;
    use time::macros::datetime;

    use super::*;

    fn thresholds(entries: &[(Severity, f64, ComparisonOperator)]) -> AlertThresholds {
        AlertThresholds {
            severities: entries
                .iter()
                .map(|(severity, threshold, operator)| {
                    (
                        *severity,
                        SeverityThreshold {
                            threshold: *threshold,
                            comparison_operator: *operator,
                            description: String::new(),
                            destination: severity.to_string(),
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn newest_complete_unsorted_sample_drives_priority_and_boundaries() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let result = evaluate_series(
            vec![MetricSeries {
                labels: Default::default(),
                points: vec![
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 90.0,
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:04:00),
                        value: 100.0,
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:06:00),
                        value: 200.0,
                    },
                ],
            }],
            &thresholds(&[
                (
                    Severity::Info,
                    80.0,
                    ComparisonOperator::GreaterThanThreshold,
                ),
                (
                    Severity::Critical,
                    90.0,
                    ComparisonOperator::GreaterThanOrEqualToThreshold,
                ),
            ]),
            window,
            datetime!(2026-03-18 10:05:00),
        )
        .unwrap();
        assert_eq!(result.level, AlertEvaluationLevel::Critical);
        assert_eq!(
            result.latest_point,
            Some(MetricPoint {
                timestamp: datetime!(2026-03-18 10:05:00),
                value: 90.0,
            })
        );
        // Preserve older and too-new returned points as context, not evaluation inputs.
        assert_eq!(result.datapoints.len(), 3);
        assert_eq!(result.datapoints[1].value, 100.0);
        assert_eq!(result.datapoints[2].timestamp, window.to);
    }

    #[test]
    fn empty_is_no_data_and_normal_is_not_a_severity() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let configured = thresholds(&[(
            Severity::Warning,
            10.0,
            ComparisonOperator::GreaterThanThreshold,
        )]);
        assert_eq!(
            evaluate_series(vec![], &configured, window, datetime!(2026-03-18 10:05:00))
                .unwrap()
                .level,
            AlertEvaluationLevel::NoData
        );
        assert_eq!(
            evaluate_series(
                vec![MetricSeries {
                    labels: Default::default(),
                    points: vec![MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 10.0
                    }]
                }],
                &configured,
                window,
                datetime!(2026-03-18 10:05:00)
            )
            .unwrap()
            .level,
            AlertEvaluationLevel::Normal
        );
    }

    #[test]
    fn context_points_do_not_turn_no_data_or_normal_into_an_alarm() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let configured = thresholds(&[(
            Severity::Critical,
            10.0,
            ComparisonOperator::GreaterThanThreshold,
        )]);
        let points = vec![
            MetricPoint {
                timestamp: datetime!(2026-03-18 09:59:00),
                value: 100.0,
            },
            MetricPoint {
                timestamp: datetime!(2026-03-18 10:06:00),
                value: 200.0,
            },
        ];
        let no_data = evaluate_series(
            vec![MetricSeries {
                labels: Default::default(),
                points: points.clone(),
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00),
        )
        .unwrap();
        assert_eq!(no_data.level, AlertEvaluationLevel::NoData);
        assert_eq!(no_data.latest_point, None);
        assert_eq!(no_data.datapoints, points);
        assert_eq!(no_data.breach, None);

        let selected = MetricPoint {
            timestamp: datetime!(2026-03-18 10:05:00),
            value: 1.0,
        };
        let mut points = points;
        points.push(MetricPoint {
            timestamp: datetime!(2026-03-18 10:04:00),
            value: 100.0,
        });
        points.push(selected.clone());
        let normal = evaluate_series(
            vec![MetricSeries {
                labels: Default::default(),
                points: points.clone(),
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00),
        )
        .unwrap();
        assert_eq!(normal.level, AlertEvaluationLevel::Normal);
        assert_eq!(normal.latest_point, Some(selected));
        assert_eq!(normal.datapoints, points);
        assert_eq!(normal.breach, None);
    }

    #[test]
    fn multiple_series_and_conflicting_newest_duplicates_are_rejected() {
        let window = MetricRange {
            from: datetime!(2026-03-18 10:00:00),
            to: datetime!(2026-03-18 10:06:00),
        };
        let configured = AlertThresholds {
            severities: BTreeMap::new(),
        };
        assert!(evaluate_series(
            vec![
                MetricSeries {
                    points: vec![],
                    labels: Default::default()
                },
                MetricSeries {
                    points: vec![],
                    labels: Default::default()
                }
            ],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00)
        )
        .is_err());
        assert!(evaluate_series(
            vec![MetricSeries {
                labels: Default::default(),
                points: vec![
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 1.0
                    },
                    MetricPoint {
                        timestamp: datetime!(2026-03-18 10:05:00),
                        value: 2.0
                    },
                ]
            }],
            &configured,
            window,
            datetime!(2026-03-18 10:05:00)
        )
        .is_err());
    }
}
