//! One evaluation run: read the metrics, decide, announce, report what happened.
//!
//! This is the only module in the crate that has a clock, a metrics provider and a notifier at
//! once. Everything it decides is decided by [`crate::domain::alarm`]; what lives here is the
//! order of operations and the answer to "what should happen when part of this fails".
//!
//! ## The clock
//!
//! A run evaluates through the **latest completed minute** and compares against the evaluation
//! ending one minute earlier — not one period earlier. CloudWatch evaluates every minute for any
//! period of a minute or longer, so two consecutive evaluations of a five-minute metric are two
//! overlapping five-minute windows, and the trigger that eventually drives this should run every
//! minute regardless of the periods in the catalogue.
//!
//! ## Nothing is remembered
//!
//! Both windows are reconstructed from CloudWatch on every request, and no state is written
//! anywhere. Two consequences are accepted rather than worked around:
//!
//! * Two requests in the same minute announce the same transition twice. There is no
//!   exactly-once delivery guarantee here, and adding one would mean the alert *management* store
//!   this ticket is explicitly not building.
//! * A transition that happens while a query is failing, or that is only visible in a datapoint
//!   that arrives late, is not announced at all — the next run compares two windows that both
//!   already contain it. See the delivery-hardening ticket.
//!
//! ## Failure is not missing data
//!
//! The distinction this module exists to protect: a metric that stopped reporting and a CloudWatch
//! call that did not work look identical if you flatten them, and one of them is an alarm while
//! the other is an outage. A reading whose series came back `Failed` or `Partial`, or that did not
//! come back at all, is **not evaluated** — it does not become a window of gaps for the
//! missing-data policy to turn into a breach. It is reported, and every other reading in the run
//! is evaluated normally.

use std::{collections::BTreeMap, sync::Arc};

use common_utils::date_time;
use error_stack::report;
use external_services::metrics_service::{Cursor, MetricRequest, MetricsProvider, SeriesStatus};
use hyperswitch_masking::Secret;

use crate::{
    domain::{
        alarm::{
            self,
            catalogue::{AlarmTarget, Catalogue, FetchPlan, Reading},
            message, AlarmState, Evaluation, Transition,
        },
        notifier::{chat::ChatNotification, Outcome},
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
};

/// How many pages one request is followed for before the run gives up on it.
///
/// A page holds up to 100,800 datapoints and a run asks for tens, so the first page is always the
/// last. The cap is here because a provider that kept handing back a cursor would otherwise be an
/// unbounded loop inside a request handler.
const MAX_PAGES: usize = 8;

/// What a caller asked of one run.
#[derive(Debug, Clone, Copy, Default)]
pub struct EvaluationOptions {
    /// Evaluate and render, but send nothing at all — not even a failure summary.
    ///
    /// A dry run is the same evaluation, so its proposed messages are the messages a real run
    /// would have sent rather than a separately built preview of them.
    pub dry_run: bool,
}

/// Evaluate every configured definition and announce what changed.
///
/// Never returns an error for a metric that could not be read or a message that could not be sent:
/// both are reported in the run's own result, because a run that read nineteen of twenty metrics
/// did nineteen twentieths of its job and saying so is more useful than a 502.
pub async fn evaluate(
    state: AppState,
    options: EvaluationOptions,
) -> ObservabilityApiResult<RunReport> {
    let catalogue = Arc::clone(&state.alarms);
    let evaluated_at = evaluation_instant(date_time::now(), catalogue.evaluation_delay_seconds);

    let mut report = RunReport {
        evaluated_at,
        previous_evaluated_at: evaluated_at
            - time::Duration::seconds(alarm::catalogue::EVALUATION_INTERVAL_SECONDS),
        dry_run: options.dry_run,
        definitions: catalogue.definitions(),
        readings: catalogue.readings.len(),
        targets: Vec::with_capacity(catalogue.targets.len()),
        failure: None,
    };

    if catalogue.is_empty() {
        logger::debug!("No CloudWatch alarms are configured; nothing to evaluate");
        return Ok(report);
    }

    let provider = state.metrics.as_ref().ok_or_else(|| {
        // Unreachable through the boot path, which builds a provider whenever the catalogue is
        // non-empty. It is an error rather than an assumption so that a future wiring mistake is a
        // 500 with a log line instead of a silent run that evaluates nothing.
        report!(ObservabilityError::InternalServerError)
            .attach_printable("Alarms are configured but no metrics provider was built")
    })?;

    let plan = FetchPlan::build(&catalogue, evaluated_at);
    let answers = fetch_all(&**provider, &plan.requests).await;

    // One report per target, in catalogue order and never skipping one, so that the announcement
    // pass can walk the two lists side by side instead of searching.
    for target in &catalogue.targets {
        report.targets.push(
            match (
                catalogue.readings.get(target.reading),
                windows_for(&catalogue, &plan, &answers, target),
            ) {
                (Some(reading), Ok((current, previous))) => {
                    decide(target, reading, &current, &previous)
                }
                (Some(reading), Err(reason)) => TargetReport::unevaluated(target, reading, reason),
                (None, _) => TargetReport::unevaluated(
                    target,
                    &Reading::unknown(),
                    "the catalogue resolved this severity against no metric".to_owned(),
                ),
            },
        );
    }

    announce(&state, &catalogue, &mut report).await;

    Ok(report)
}

/// The instant a run evaluates through: the latest completed minute, less any configured delay.
///
/// Flooring to the minute is what makes two runs inside the same minute compare the same two
/// windows, and it is also what aligns the request with CloudWatch's own rounding — it rounds a
/// start time down to the whole minute for data less than fifteen days old, so an unaligned
/// request would silently be answered as an aligned one anyway.
///
/// A naive [`PrimitiveDateTime`](time::PrimitiveDateTime), holding UTC by discipline, matching
/// `common_utils::date_time` and the rest of the repository. It is turned into an offset instant
/// only where the metrics client's window type demands one.
fn evaluation_instant(now: time::PrimitiveDateTime, delay_seconds: i64) -> time::PrimitiveDateTime {
    let instant = now.assume_utc() - time::Duration::seconds(delay_seconds);
    let seconds = instant.unix_timestamp();

    date_time::convert_to_pdt(
        time::OffsetDateTime::from_unix_timestamp(seconds - seconds.rem_euclid(60))
            .unwrap_or(instant),
    )
}

/// Evaluate both windows and work out whether the state moved.
fn decide(
    target: &AlarmTarget,
    reading: &Reading,
    current: &[Option<f64>],
    previous: &[Option<f64>],
) -> TargetReport {
    let now = target.rule.evaluate(current);
    let before = target.rule.evaluate(previous);

    TargetReport {
        definition: target.definition.clone(),
        classification: target.classification.clone(),
        severity: target.severity.clone(),
        metric: reading.describe(),
        dimensions: reading.dimensions(),
        state: Some(now.state),
        previous_state: Some(before.state),
        transition: Transition::between(before.state, now.state),
        observed: now.observed,
        threshold: target.rule.threshold,
        breaching_datapoints: now.breaching,
        evaluation_periods: target.rule.evaluation_periods,
        datapoints_to_alarm: target.rule.datapoints_to_alarm,
        evaluation: Some(now),
        error: None,
        message: None,
        delivery: None,
    }
}

/// Send what the run decided, best effort.
///
/// Every send is attempted even when an earlier one failed: a chat outage that silenced the first
/// announcement has no bearing on the second, and stopping at the first failure would turn one
/// broken destination into a silent run.
async fn announce(state: &AppState, catalogue: &Catalogue, report: &mut RunReport) {
    let evaluated_at = report.evaluated_at;
    let dry_run = report.dry_run;

    for (outcome, source) in report.targets.iter_mut().zip(&catalogue.targets) {
        let (Some(transition), Some(evaluation)) =
            (outcome.transition, outcome.evaluation.as_ref())
        else {
            continue;
        };
        let Some(reading) = catalogue.readings.get(source.reading) else {
            continue;
        };

        let text = message::announcement(source, reading, transition, evaluation, evaluated_at);

        logger::info!(
            definition = %source.definition,
            severity = %source.severity,
            from = %transition.from.label(),
            to = %transition.to.label(),
            dry_run,
            "Alarm state transition detected"
        );

        outcome.delivery = Some(deliver(state, &source.destination, &text, dry_run).await);
        outcome.message = Some(text);
    }

    // Named by definition rather than by severity: three severities of one metric share a reading,
    // so a failed query is one thing that went wrong, not three.
    let mut failed = Vec::new();
    let mut reasons = Vec::new();
    for outcome in &report.targets {
        let Some(reason) = outcome.error.as_ref() else {
            continue;
        };
        if !failed.contains(&outcome.definition) {
            failed.push(outcome.definition.clone());
        }
        if !reasons.contains(reason) {
            reasons.push(reason.clone());
        }
    }

    if failed.is_empty() {
        return;
    }

    // A single reason shared by everything is worth quoting — it is usually the one sentence that
    // explains the whole run. Several different ones are a list nobody reads in a chat message,
    // and they are already in the response and the log.
    let reason = match reasons.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    };

    let text =
        message::failure_summary(report.definitions, &failed, reason.as_deref(), evaluated_at);

    logger::error!(
        failed_definitions = failed.len(),
        definitions = report.definitions,
        dry_run,
        "CloudWatch readings could not be retrieved"
    );

    let delivery = deliver(state, &catalogue.failure_destination, &text, dry_run).await;
    report.failure = Some(FailureReport {
        definitions: failed,
        reason,
        message: text,
        delivery,
    });
}

/// One delivery attempt, reported rather than raised.
async fn deliver(state: &AppState, destination: &str, text: &str, dry_run: bool) -> DeliveryReport {
    if dry_run {
        return DeliveryReport::SkippedDryRun {
            destination: destination.to_owned(),
        };
    }

    let Some(notifier) = state.chat.get(destination) else {
        // Boot validation makes this unreachable; reporting it beats a panic and beats silence.
        return DeliveryReport::Failed {
            destination: destination.to_owned(),
            error: "no chat destination is configured under this id".to_owned(),
        };
    };

    let notification = ChatNotification {
        text: Secret::new(text.to_owned()),
        reply_to: None,
    };

    match notifier.notify(notification).await {
        Ok(Outcome::Delivered(receipt)) => DeliveryReport::Delivered {
            destination: destination.to_owned(),
            message_id: receipt.message_id,
        },
        Ok(Outcome::Refused(refusal)) => {
            logger::warn!(
                destination = %destination,
                code = %refusal.code,
                "Chat destination refused an alarm announcement"
            );
            DeliveryReport::Refused {
                destination: destination.to_owned(),
                code: refusal.code,
            }
        }
        Err(error) => {
            logger::error!(
                destination = %destination,
                error = ?error,
                "Chat destination could not be reached for an alarm announcement"
            );
            DeliveryReport::Failed {
                destination: destination.to_owned(),
                error: error.current_context().to_string(),
            }
        }
    }
}

/// One window's readings, laid on its period grid, `None` where a period had no datapoint.
type Window = Vec<Option<f64>>;

/// A window, or the sentence explaining why there is not one.
///
/// The failure side is a `String` rather than an error type on purpose: it is not handled, it is
/// *reported* — into the response, the log and the operational summary — and every producer of one
/// is a different flavour of "CloudWatch did not give us this".
type WindowResult = Result<Window, String>;

/// The two windows one target is evaluated over, or why it cannot be evaluated.
fn windows_for(
    catalogue: &Catalogue,
    plan: &FetchPlan,
    answers: &[RequestAnswer],
    target: &AlarmTarget,
) -> Result<(Window, Window), String> {
    let entry = plan
        .readings
        .iter()
        .find(|entry| entry.reading == target.reading)
        .ok_or_else(|| "the run planned no request for this metric".to_owned())?;

    let span = target
        .rule
        .evaluation_range(catalogue.missing_data_lookback);

    let grid = |reference: alarm::catalogue::GridRef| -> WindowResult {
        let position = plan
            .position_of(reference.request, target.reading)
            .ok_or_else(|| "the run planned no query for this metric".to_owned())?;

        let series = answers
            .get(reference.request)
            .ok_or_else(|| "the run made no such request".to_owned())?
            .series
            .get(position)
            .ok_or_else(|| "CloudWatch answered nothing for this query".to_owned())?
            .as_ref()
            .map_err(Clone::clone)?;

        alarm::window(series, span, reference.back)
            .map(<[Option<f64>]>::to_vec)
            .ok_or_else(|| {
                format!("CloudWatch returned fewer than the {span} datapoints the window needs")
            })
    };

    Ok((grid(entry.current)?, grid(entry.previous)?))
}

/// What one `GetMetricData` request produced, per query position.
struct RequestAnswer {
    series: Vec<WindowResult>,
}

/// Issue every planned request.
///
/// Sequential: a run makes a handful of calls, and running them concurrently would trade a
/// readable failure attribution for latency nobody is waiting on.
async fn fetch_all(
    provider: &dyn MetricsProvider,
    requests: &[MetricRequest],
) -> Vec<RequestAnswer> {
    let mut answers = Vec::with_capacity(requests.len());

    for request in requests {
        answers.push(fetch(provider, request).await);
    }

    answers
}

/// Fetch one request, following its pages, and report each query's series separately.
///
/// A whole-request failure becomes the same failure for every query in it, so one broken call does
/// not have to be distinguished from many at the point where a definition asks what happened.
async fn fetch(provider: &dyn MetricsProvider, request: &MetricRequest) -> RequestAnswer {
    let mut merged: Vec<Option<(SeriesStatus, Vec<Option<f64>>)>> =
        vec![None; request.queries.len()];
    let mut cursor: Option<Cursor> = None;

    for page_number in 0..MAX_PAGES {
        let page = match provider.fetch(request, cursor.as_ref()).await {
            Ok(page) => page,
            Err(error) => {
                let reason = error.current_context().to_string();
                logger::error!(
                    error = ?error,
                    page = page_number,
                    queries = request.queries.len(),
                    "A CloudWatch request failed"
                );

                return RequestAnswer {
                    series: vec![Err(reason); request.queries.len()],
                };
            }
        };

        for series in page.series() {
            let Some(slot) = merged.get_mut(series.index()) else {
                continue;
            };

            match slot {
                // Every page lays its datapoints on the *whole* window's grid, filling only the
                // slots it carries, so pages are merged slot-wise rather than concatenated.
                Some((status, values)) => {
                    *status = coarsest(*status, series.status());
                    for (existing, arriving) in values.iter_mut().zip(series.values()) {
                        if existing.is_none() {
                            *existing = *arriving;
                        }
                    }
                }
                None => *slot = Some((series.status(), series.values().to_vec())),
            }
        }

        cursor = page.cursor().cloned();
        if cursor.is_none() {
            break;
        }
    }

    if cursor.is_some() {
        logger::warn!(
            pages = MAX_PAGES,
            "A CloudWatch request still had pages after the page limit; treating it as incomplete"
        );

        return RequestAnswer {
            series: vec![
                Err("the response did not finish paginating".to_owned());
                request.queries.len()
            ],
        };
    }

    RequestAnswer {
        series: merged
            .into_iter()
            .map(|slot| match slot {
                Some((SeriesStatus::Complete, values)) => Ok(values),
                // Neither of these is a metric that stopped reporting. `Partial` is a window we
                // only half received and `Failed` is one we did not receive; evaluating either
                // would let a CloudWatch problem be announced as a database problem.
                Some((SeriesStatus::Partial, _)) => {
                    Err("CloudWatch returned only part of this series".to_owned())
                }
                Some((SeriesStatus::Failed, _)) => {
                    Err("CloudWatch could not produce this series".to_owned())
                }
                None => Err("CloudWatch reported nothing for this query".to_owned()),
            })
            .collect(),
    }
}

/// The less complete of two statuses, for a series spread over pages.
fn coarsest(one: SeriesStatus, other: SeriesStatus) -> SeriesStatus {
    match (one, other) {
        (SeriesStatus::Failed, _) | (_, SeriesStatus::Failed) => SeriesStatus::Failed,
        (SeriesStatus::Partial, _) | (_, SeriesStatus::Partial) => SeriesStatus::Partial,
        _ => SeriesStatus::Complete,
    }
}

/// Everything one run found and did.
#[derive(Debug)]
pub struct RunReport {
    /// The instant the current evaluation ends at, in UTC.
    pub evaluated_at: time::PrimitiveDateTime,
    /// The instant the evaluation it was compared against ends at — one minute earlier.
    pub previous_evaluated_at: time::PrimitiveDateTime,
    /// Whether every send was skipped.
    pub dry_run: bool,
    /// How many catalogue entries the run covered.
    pub definitions: usize,
    /// How many distinct metric readings those entries needed. Fewer, because severities and
    /// definitions watching one metric share a query.
    pub readings: usize,
    /// One entry per severity, in catalogue order.
    pub targets: Vec<TargetReport>,
    /// The operational summary, when anything failed to read.
    pub failure: Option<FailureReport>,
}

/// What happened to one severity.
#[derive(Debug)]
pub struct TargetReport {
    /// The catalogue entry's name.
    pub definition: String,
    /// The group it belongs to.
    pub classification: String,
    /// The severity id.
    pub severity: String,
    /// Namespace and metric name.
    pub metric: String,
    /// The dimensions it read.
    pub dimensions: BTreeMap<String, String>,
    /// The state the current window puts it in, absent if it could not be evaluated.
    pub state: Option<AlarmState>,
    /// The state the previous window puts it in.
    pub previous_state: Option<AlarmState>,
    /// The change between them, if any.
    pub transition: Option<Transition>,
    /// The most recent real reading in the current window.
    pub observed: Option<f64>,
    /// What it was compared against.
    pub threshold: f64,
    /// How many of the window's readings breached.
    pub breaching_datapoints: usize,
    /// N.
    pub evaluation_periods: u32,
    /// M.
    pub datapoints_to_alarm: u32,
    /// The full evaluation, kept so the announcement and the response agree.
    pub evaluation: Option<Evaluation>,
    /// Why it could not be evaluated, when it could not.
    pub error: Option<String>,
    /// The message that was sent, or that a dry run would have sent.
    pub message: Option<String>,
    /// What came of sending it.
    pub delivery: Option<DeliveryReport>,
}

impl TargetReport {
    fn unevaluated(target: &AlarmTarget, reading: &Reading, error: String) -> Self {
        Self {
            definition: target.definition.clone(),
            classification: target.classification.clone(),
            severity: target.severity.clone(),
            metric: reading.describe(),
            dimensions: reading.dimensions(),
            state: None,
            previous_state: None,
            transition: None,
            observed: None,
            threshold: target.rule.threshold,
            breaching_datapoints: 0,
            evaluation_periods: target.rule.evaluation_periods,
            datapoints_to_alarm: target.rule.datapoints_to_alarm,
            evaluation: None,
            error: Some(error),
            message: None,
            delivery: None,
        }
    }
}

/// The operational summary a run sends when it could not read everything.
#[derive(Debug)]
pub struct FailureReport {
    /// The definitions that went unevaluated.
    pub definitions: Vec<String>,
    /// The one reason behind all of them, when there was only one.
    pub reason: Option<String>,
    /// The message that was sent, or that a dry run would have sent.
    pub message: String,
    /// What came of sending it.
    pub delivery: DeliveryReport,
}

/// What came of one attempt to send a message.
#[derive(Debug)]
pub enum DeliveryReport {
    /// The provider accepted it.
    Delivered {
        /// The destination it went to.
        destination: String,
        /// The provider's id for the message, when it named one.
        message_id: Option<String>,
    },
    /// The provider was reached and refused it.
    Refused {
        /// The destination that refused.
        destination: String,
        /// The stable code it refused with.
        code: String,
    },
    /// The provider could not be reached, so whether it arrived is unknown.
    Failed {
        /// The destination that could not be reached.
        destination: String,
        /// What went wrong.
        error: String,
    },
    /// Nothing was sent, because this was a dry run.
    SkippedDryRun {
        /// Where it would have gone.
        destination: String,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use time::macros::datetime;

    use super::*;

    /// A run evaluates through the last minute that finished, so two runs inside one minute
    /// compare the same two windows rather than two slightly different ones.
    #[test]
    fn a_run_evaluates_through_the_latest_completed_minute() {
        assert_eq!(
            evaluation_instant(datetime!(2026-09-10 12:34:56.789), 0),
            datetime!(2026-09-10 12:34:00)
        );
        assert_eq!(
            evaluation_instant(datetime!(2026-09-10 12:34:00), 0),
            datetime!(2026-09-10 12:34:00)
        );
    }

    /// The knob for a metric whose readings are known to land late. Still floored, so the two
    /// windows stay a whole minute apart.
    #[test]
    fn a_configured_delay_moves_the_instant_back_whole_minutes() {
        assert_eq!(
            evaluation_instant(datetime!(2026-09-10 12:34:56), 120),
            datetime!(2026-09-10 12:32:00)
        );
        assert_eq!(
            evaluation_instant(datetime!(2026-09-10 12:34:56), 30),
            datetime!(2026-09-10 12:34:00)
        );
    }

    #[test]
    fn a_partial_page_makes_the_whole_series_incomplete() {
        assert_eq!(
            coarsest(SeriesStatus::Complete, SeriesStatus::Partial),
            SeriesStatus::Partial
        );
        assert_eq!(
            coarsest(SeriesStatus::Partial, SeriesStatus::Failed),
            SeriesStatus::Failed
        );
        assert_eq!(
            coarsest(SeriesStatus::Complete, SeriesStatus::Complete),
            SeriesStatus::Complete
        );
    }
}
