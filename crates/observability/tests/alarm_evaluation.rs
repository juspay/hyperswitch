//! One evaluation run, end to end, against a stub metrics provider.
//!
//! Everything else tests a piece: the evaluator against AWS's published tables, the planner against
//! its request shapes, the catalogue against the config file. This tests the thing those pieces are
//! assembled into — `core::alarm::evaluate` — because the properties that decide whether this
//! service is *useful* only exist once a clock, a provider and a notifier are in the same room:
//!
//! * a breach that persists announces **once**, not once a minute;
//! * a query that failed is not a metric that stopped reporting;
//! * a chat destination that is down does not silence the destinations that are up.
//!
//! ## The stub is a metric history, not a canned response
//!
//! [`StubMetrics`] holds one value per minute and aggregates on demand, exactly as CloudWatch
//! would: for a request covering `[start, end)` with period P it produces `(end - start) / P` slots
//! and reduces the minutes falling in each. That matters, because the one thing this branch could
//! not check against a live account is whether two 300-second windows a minute apart really are
//! different aggregates. A stub that replayed a fixed `Vec` per call would assert that the code
//! does what it does; a stub that aggregates makes the question real.
//!
//! Announcements are observed rather than inferred — [`RecordingChat`] keeps every message it was
//! given, so "announces exactly once" is a length assertion rather than a state assertion.

// `as_conversions` is allowed here rather than worked around: most of the warnings come out of the
// `#[tokio::test]` expansion rather than from anything written below, and the one that is ours is a
// mean over a handful of readings.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::as_conversions
)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use external_services::metrics_service::{
    Aggregation, Cursor, MetricPage, MetricRequest, MetricSeries, MetricsProvider, MetricsResult,
    SeriesStatus,
};
use observability::{
    core::alarm::{self, DeliveryReport, EvaluationOptions},
    domain::{
        alarm::catalogue::Catalogue,
        notifier::{
            chat::{
                ChatFileOutcome, ChatFileReceipt, ChatFileUpload, ChatNotification, ChatNotifier,
                ChatOutcome, ChatReceipt,
            },
            Outcome, Refusal, Registry,
        },
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    settings::cloudwatch::CloudWatchSettings,
    state::AppState,
};
use serde_json::json;

const DESTINATION: &str = "smoke";

// ---------------------------------------------------------------------------------------------
// A metrics provider backed by a minute-resolution history.
// ---------------------------------------------------------------------------------------------

/// A stub CloudWatch: one reading per minute, aggregated into whatever window is asked for.
///
/// `minutes` is oldest-first and its **last entry is the minute ending at the run's evaluation
/// instant**, so a test writes the recent past directly — `[.., Some(95.0), Some(95.0)]` is "the
/// last two minutes both breached".
///
/// That instant is discovered rather than declared. `evaluate` reads the real clock, so a stub
/// anchored to a literal timestamp would answer every request with gaps, and one anchored to
/// `now()` computed in the test would be a coin-flip whenever a run straddled a minute boundary.
/// Instead the first request's `range.end` becomes the anchor: the planner always asks for the
/// current window before the one a minute behind it, so the first end it sees is the instant the
/// run is evaluating through, whatever the clock happened to say.
#[derive(Debug)]
struct StubMetrics {
    minutes: Vec<Option<f64>>,
    /// The instant the newest minute ends at, taken from the first request.
    anchor: Mutex<Option<time::OffsetDateTime>>,
    /// What to report for each query's series, by query position. Absent means `Complete`.
    statuses: HashMap<usize, SeriesStatus>,
    /// Whether the first call should hand back a cursor and only half the datapoints.
    paginate: bool,
    calls: Mutex<usize>,
}

impl StubMetrics {
    fn new(minutes: Vec<Option<f64>>) -> Self {
        Self {
            minutes,
            anchor: Mutex::new(None),
            statuses: HashMap::new(),
            paginate: false,
            calls: Mutex::new(0),
        }
    }

    fn with_status(mut self, query: usize, status: SeriesStatus) -> Self {
        self.statuses.insert(query, status);
        self
    }

    fn paginated(mut self) -> Self {
        self.paginate = true;
        self
    }

    /// The reading for the minute ending at `ends_at`, if the history covers it.
    fn minute_at(&self, ends_at: i64) -> Option<f64> {
        let anchor = (*self.anchor.lock().unwrap())?.unix_timestamp();
        let behind = (anchor - ends_at) / 60;
        let index = usize::try_from(i64::try_from(self.minutes.len()).ok()? - 1 - behind).ok()?;

        self.minutes.get(index).copied().flatten()
    }

    /// Reduce the minutes covered by `[from, to)` the way the query asks.
    fn aggregate(&self, from: i64, to: i64, aggregation: Aggregation) -> Option<f64> {
        let readings = (from..to)
            .step_by(60)
            .filter_map(|second| self.minute_at(second + 60))
            .collect::<Vec<_>>();

        if readings.is_empty() {
            return None;
        }

        Some(match aggregation {
            Aggregation::Average => readings.iter().sum::<f64>() / readings.len() as f64,
            Aggregation::Sum => readings.iter().sum(),
            Aggregation::Maximum => readings.iter().copied().fold(f64::MIN, f64::max),
            Aggregation::Minimum => readings.iter().copied().fold(f64::MAX, f64::min),
        })
    }
}

#[async_trait::async_trait]
impl MetricsProvider for StubMetrics {
    async fn fetch(
        &self,
        request: &MetricRequest,
        cursor: Option<&Cursor>,
    ) -> MetricsResult<MetricPage> {
        let is_first_page = cursor.is_none();
        *self.calls.lock().unwrap() += 1;
        self.anchor.lock().unwrap().get_or_insert(request.range.end);

        let series = request
            .queries
            .iter()
            .enumerate()
            .map(|(index, query)| {
                let period = query.period.seconds_i64();
                let start = request.range.start.unix_timestamp();
                let slots = (request.range.end.unix_timestamp() - start) / period;

                let values = (0..slots)
                    .map(|slot| {
                        // A paginated response splits the grid down the middle: the first page
                        // carries the older half and the second the newer. Neither page is a
                        // usable window on its own, so the slot-wise merge in `core::alarm::fetch`
                        // is the only thing that can put them back together.
                        let withheld = self.paginate
                            && if is_first_page {
                                slot >= slots / 2
                            } else {
                                slot < slots / 2
                            };

                        let from = start + slot * period;
                        (!withheld)
                            .then(|| self.aggregate(from, from + period, query.aggregation))
                            .flatten()
                    })
                    .collect();

                MetricSeries::new(
                    index,
                    self.statuses
                        .get(&index)
                        .copied()
                        .unwrap_or(SeriesStatus::Complete),
                    values,
                )
            })
            .collect();

        Ok(MetricPage::new(
            series,
            (self.paginate && is_first_page).then(|| Cursor::new("more")),
        ))
    }
}

// ---------------------------------------------------------------------------------------------
// Chat destinations that record, refuse, or fail.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct RecordingChat {
    sent: Mutex<Vec<String>>,
}

impl RecordingChat {
    fn sent(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl ChatNotifier for RecordingChat {
    async fn notify(&self, notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome> {
        use hyperswitch_masking::ExposeInterface;

        self.sent.lock().unwrap().push(notification.text.expose());

        Ok(Outcome::Delivered(ChatReceipt {
            message_id: Some("1.0".to_owned()),
        }))
    }

    async fn upload_file(
        &self,
        _upload: ChatFileUpload,
    ) -> ObservabilityApiResult<ChatFileOutcome> {
        Ok(Outcome::Delivered(ChatFileReceipt { file_id: None }))
    }
}

/// Reached, and said no. A refusal is an outcome, not an error.
#[derive(Debug)]
struct RefusingChat;

#[async_trait::async_trait]
impl ChatNotifier for RefusingChat {
    async fn notify(&self, _notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome> {
        Ok(Outcome::Refused(Refusal::new("channel_not_found")))
    }

    async fn upload_file(
        &self,
        _upload: ChatFileUpload,
    ) -> ObservabilityApiResult<ChatFileOutcome> {
        Ok(Outcome::Refused(Refusal::new("channel_not_found")))
    }
}

/// Could not be reached at all, so whether the message arrived is unknown.
#[derive(Debug)]
struct FailingChat;

#[async_trait::async_trait]
impl ChatNotifier for FailingChat {
    async fn notify(&self, _notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome> {
        Err(error_stack::report!(
            ObservabilityError::ProviderUnavailable {
                destination: DESTINATION.to_owned(),
            }
        ))
    }

    async fn upload_file(
        &self,
        _upload: ChatFileUpload,
    ) -> ObservabilityApiResult<ChatFileOutcome> {
        Err(error_stack::report!(
            ObservabilityError::ProviderUnavailable {
                destination: DESTINATION.to_owned(),
            }
        ))
    }
}

// ---------------------------------------------------------------------------------------------
// Building a run.
// ---------------------------------------------------------------------------------------------

/// A catalogue of one definition and one severity, breaching above `threshold`.
fn catalogue_of(period: u32, evaluation_periods: u32, threshold: f64) -> Catalogue {
    catalogue_from(json!({
        "cpu": {
            "classification": "rds-alerts",
            "metric_name": "CPUUtilization",
            "namespace": "AWS/RDS",
            "dimensions": [{ "name": "DBInstanceIdentifier", "value": "hyperswitchdb-primary" }],
            "period": period,
            "statistic": "Average",
            "severities": {
                "sev1": {
                    "threshold": threshold,
                    "comparison_operator": "GreaterThanThreshold",
                    "description": "SEV1: CPU is high",
                    "evaluation_periods": evaluation_periods,
                },
            },
        },
    }))
}

fn catalogue_from(alarms: serde_json::Value) -> Catalogue {
    let settings: CloudWatchSettings = serde_json::from_value(json!({
        "region": "ap-south-1",
        "failure_destination": DESTINATION,
        "severity_destinations": { "sev1": DESTINATION },
        "alarms": alarms,
    }))
    .expect("the test catalogue should deserialize");

    Catalogue::resolve(&settings).expect("the test catalogue should resolve")
}

fn state_with(
    catalogue: Catalogue,
    metrics: Arc<dyn MetricsProvider>,
    chat: Arc<dyn ChatNotifier>,
) -> AppState {
    AppState {
        conf: Arc::new(
            serde_json::from_value(json!({ "auth": { "internal_api_key": "k" } }))
                .expect("the test configuration should deserialize"),
        ),
        chat: Arc::new(Registry::new(HashMap::from([(
            DESTINATION.to_owned(),
            chat,
        )]))),
        email: Arc::new(Registry::default()),
        alarms: Arc::new(catalogue),
        metrics: Some(metrics),
    }
}

async fn run(state: &AppState) -> alarm::RunReport {
    alarm::evaluate(state.clone(), EvaluationOptions::default())
        .await
        .expect("a run should not fail as a whole")
}

// ---------------------------------------------------------------------------------------------
// The properties.
// ---------------------------------------------------------------------------------------------

/// **The ticket's acceptance criterion.** A condition that stays breaching across two consecutive
/// evaluations announces exactly once.
///
/// This is the whole justification for reconstructing both windows instead of remembering the last
/// state: the previous window is already breaching, so there is no transition and nothing to say. A
/// regression here turns a single alert into one alert per minute for as long as the incident
/// lasts, which is how an on-call channel becomes unreadable.
#[tokio::test]
async fn a_breach_that_persists_across_two_evaluations_announces_once() {
    // Four minutes of breach: both the current window and the one ending a minute earlier are
    // fully breaching, so the state was already ALARM before this run.
    let metrics = Arc::new(StubMetrics::new(vec![
        Some(95.0),
        Some(95.0),
        Some(95.0),
        Some(95.0),
        Some(95.0),
    ]));
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue_of(60, 1, 90.0),
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    let report = run(&state).await;

    assert_eq!(report.targets[0].state.map(|s| s.label()), Some("ALARM"));
    assert_eq!(
        report.targets[0].previous_state.map(|s| s.label()),
        Some("ALARM")
    );
    assert!(report.targets[0].transition.is_none());
    assert!(chat.sent().is_empty(), "{:?}", chat.sent());

    // Evaluating again changes nothing, because nothing is remembered *and* nothing moved.
    let second = run(&state).await;
    assert!(second.targets[0].transition.is_none());
    assert!(chat.sent().is_empty(), "{:?}", chat.sent());
}

/// The other half: a breach that has just started *is* a transition, and it says so.
#[tokio::test]
async fn a_new_breach_announces_with_the_value_that_caused_it() {
    // The previous window was fine; only the latest minute breached.
    let metrics = Arc::new(StubMetrics::new(vec![
        Some(10.0),
        Some(10.0),
        Some(10.0),
        Some(95.5),
    ]));
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue_of(60, 1, 90.0),
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    let report = run(&state).await;

    assert_eq!(report.targets[0].state.map(|s| s.label()), Some("ALARM"));
    assert_eq!(
        report.targets[0].previous_state.map(|s| s.label()),
        Some("OK")
    );

    let sent = chat.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].contains("OK → ALARM"), "{}", sent[0]);
    assert!(sent[0].contains("Observed: 95.5"), "{}", sent[0]);
    assert!(matches!(
        report.targets[0].delivery,
        Some(DeliveryReport::Delivered { .. })
    ));
}

/// Recovery is a transition like any other and is announced, so an alert that opened also closes.
#[tokio::test]
async fn a_recovery_announces() {
    let metrics = Arc::new(StubMetrics::new(vec![
        Some(95.0),
        Some(95.0),
        Some(95.0),
        Some(4.0),
    ]));
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue_of(60, 1, 90.0),
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    run(&state).await;

    let sent = chat.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].contains("ALARM → OK"), "{}", sent[0]);
}

/// The claim this branch could not check against a live account: for a 300-second metric the two
/// evaluations are two *overlapping* aggregates a minute apart, not two adjacent datapoints.
///
/// The history is built so the distinction is the whole answer. The five minutes ending at 12:00
/// average above the threshold; the five ending at 11:59 do not. Comparing adjacent five-minute
/// datapoints instead would compare 11:55–12:00 with 11:50–11:55 and reach a different conclusion.
#[tokio::test]
async fn a_five_minute_alarm_compares_windows_one_minute_apart() {
    // minutes ending 11:52 .. 12:00
    let metrics = Arc::new(StubMetrics::new(vec![
        Some(0.0),   // 11:52
        Some(0.0),   // 11:53
        Some(0.0),   // 11:54
        Some(0.0),   // 11:55  <- in the previous window, not the current one
        Some(100.0), // 11:56
        Some(100.0), // 11:57
        Some(100.0), // 11:58
        Some(100.0), // 11:59
        Some(100.0), // 12:00
    ]));
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue_of(300, 1, 90.0),
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    let report = run(&state).await;

    // Current window 11:55–12:00 averages 100; previous window 11:54–11:59 averages 80.
    assert_eq!(report.targets[0].state.map(|s| s.label()), Some("ALARM"));
    assert_eq!(
        report.targets[0].previous_state.map(|s| s.label()),
        Some("OK")
    );
    assert_eq!(chat.sent().len(), 1);
}

/// A query that failed is not a metric that stopped reporting.
///
/// The alarm's policy is `notBreaching`, so treating the failure as a window of gaps would report a
/// confident OK about a database nobody actually looked at. It must report *nothing* about the
/// state, and say why.
#[tokio::test]
async fn a_failed_series_is_not_evaluated_as_missing_data() {
    let metrics = Arc::new(
        StubMetrics::new(vec![Some(95.0), Some(95.0)]).with_status(0, SeriesStatus::Failed),
    );
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue_of(60, 1, 90.0),
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    let report = run(&state).await;

    assert!(report.targets[0].state.is_none());
    assert!(report.targets[0].transition.is_none());
    assert!(report.targets[0].error.is_some());

    // One operational summary, and it is not dressed up as an alarm about the estate.
    let failure = report.failure.expect("a failed reading should be reported");
    assert_eq!(failure.definitions, vec!["cpu"]);
    assert_eq!(chat.sent().len(), 1);
    assert!(
        chat.sent()[0].contains("CloudWatch evaluation"),
        "{:?}",
        chat.sent()
    );
}

/// A partially incomplete series is treated the same way, because half a window evaluated as if it
/// were whole is a confident answer about data that was never received.
#[tokio::test]
async fn a_partial_series_is_refused_like_a_failed_one() {
    let metrics = Arc::new(
        StubMetrics::new(vec![Some(95.0), Some(95.0)]).with_status(0, SeriesStatus::Partial),
    );
    let state = state_with(
        catalogue_of(60, 1, 90.0),
        metrics,
        Arc::new(RecordingChat::default()),
    );

    assert!(run(&state).await.targets[0].error.is_some());
}

/// Best-effort progress: one broken reading does not blind the run to the others. Only the total
/// failure path was ever exercised by hand, and it is the *partial* one that has to keep working.
#[tokio::test]
async fn one_failed_reading_does_not_stop_the_others_being_evaluated() {
    let catalogue = catalogue_from(json!({
        "broken": {
            "classification": "rds-alerts",
            "metric_name": "CPUUtilization",
            "namespace": "AWS/RDS",
            "dimensions": [{ "name": "DBInstanceIdentifier", "value": "one" }],
            "period": 60,
            "statistic": "Average",
            "severities": { "sev1": {
                "threshold": 90.0, "comparison_operator": "GreaterThanThreshold",
                "description": "d", "evaluation_periods": 1,
            }},
        },
        "healthy": {
            "classification": "rds-alerts",
            "metric_name": "CPUUtilization",
            "namespace": "AWS/RDS",
            "dimensions": [{ "name": "DBInstanceIdentifier", "value": "two" }],
            "period": 60,
            "statistic": "Average",
            "severities": { "sev1": {
                "threshold": 90.0, "comparison_operator": "GreaterThanThreshold",
                "description": "d", "evaluation_periods": 1,
            }},
        },
    }));

    // Query 0 is `broken`; the catalogue is ordered by definition name.
    let metrics = Arc::new(
        StubMetrics::new(vec![Some(10.0), Some(10.0), Some(95.0)])
            .with_status(0, SeriesStatus::Failed),
    );
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue,
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    let report = run(&state).await;

    let broken = &report.targets[0];
    let healthy = &report.targets[1];
    assert_eq!(broken.definition, "broken");
    assert!(broken.error.is_some());

    assert_eq!(healthy.definition, "healthy");
    assert_eq!(healthy.state.map(|s| s.label()), Some("ALARM"));
    assert!(healthy.transition.is_some());

    // The announcement for `healthy` and the summary naming `broken` — not one instead of the
    // other.
    let failure = report
        .failure
        .expect("the partial failure should be summarised");
    assert_eq!(failure.definitions, vec!["broken"]);
    assert_eq!(chat.sent().len(), 2, "{:?}", chat.sent());
}

/// A dry run performs the same evaluation and returns the messages it *would* have sent, including
/// the failure summary. Nothing reaches chat — a dry run that announced a CloudWatch outage would
/// not be a dry run.
#[tokio::test]
async fn a_dry_run_returns_the_messages_and_sends_none_of_them() {
    let metrics = Arc::new(StubMetrics::new(vec![Some(10.0), Some(10.0), Some(95.0)]));
    let chat = Arc::new(RecordingChat::default());
    let state = state_with(
        catalogue_of(60, 1, 90.0),
        metrics,
        Arc::clone(&chat) as Arc<dyn ChatNotifier>,
    );

    let report = alarm::evaluate(state, EvaluationOptions { dry_run: true })
        .await
        .unwrap();

    assert!(report.dry_run);
    assert!(report.targets[0].transition.is_some());
    // The message is built by the code that would have sent it, not a separate preview.
    assert!(report.targets[0]
        .message
        .as_ref()
        .is_some_and(|message| message.contains("OK → ALARM")));
    assert!(matches!(
        report.targets[0].delivery,
        Some(DeliveryReport::SkippedDryRun { .. })
    ));
    assert!(chat.sent().is_empty());
}

/// A destination that refuses, and one that cannot be reached, are both reported rather than
/// raised — the run still returns its evaluation, and the response says the alert did not arrive.
#[tokio::test]
async fn a_refusal_and_an_unreachable_destination_are_reported_not_raised() {
    let history = vec![Some(10.0), Some(10.0), Some(95.0)];

    let refused = run(&state_with(
        catalogue_of(60, 1, 90.0),
        Arc::new(StubMetrics::new(history.clone())),
        Arc::new(RefusingChat),
    ))
    .await;
    assert!(matches!(
        refused.targets[0].delivery,
        Some(DeliveryReport::Refused { ref code, .. }) if code == "channel_not_found"
    ));

    let failed = run(&state_with(
        catalogue_of(60, 1, 90.0),
        Arc::new(StubMetrics::new(history)),
        Arc::new(FailingChat),
    ))
    .await;
    assert!(matches!(
        failed.targets[0].delivery,
        Some(DeliveryReport::Failed { .. })
    ));
    // The evaluation itself still succeeded; only the delivery did not.
    assert_eq!(failed.targets[0].state.map(|s| s.label()), Some("ALARM"));
}

/// A series spread over pages is merged slot-wise rather than concatenated.
///
/// Each page lays its datapoints on the *whole* window's grid and fills only the slots it carries,
/// so appending one page to the next would produce a grid twice the right length with the readings
/// in the wrong periods — and the window would then be sliced out of the wrong end of it.
#[tokio::test]
async fn a_series_split_across_pages_is_merged_into_one_grid() {
    let metrics = Arc::new(
        StubMetrics::new(vec![
            Some(95.0),
            Some(95.0),
            Some(95.0),
            Some(95.0),
            Some(95.0),
        ])
        .paginated(),
    );
    let state = state_with(
        catalogue_of(60, 3, 90.0),
        Arc::clone(&metrics) as Arc<dyn MetricsProvider>,
        Arc::new(RecordingChat::default()),
    );

    let report = run(&state).await;

    assert!(
        *metrics.calls.lock().unwrap() >= 2,
        "the stub should paginate"
    );
    assert!(
        report.targets[0].error.is_none(),
        "{:?}",
        report.targets[0].error
    );
    assert_eq!(report.targets[0].state.map(|s| s.label()), Some("ALARM"));
}

/// An empty catalogue is a normal, successful run that touches nothing.
#[tokio::test]
async fn an_empty_catalogue_evaluates_nothing_and_succeeds() {
    let state = AppState {
        conf: Arc::new(
            serde_json::from_value(json!({ "auth": { "internal_api_key": "k" } })).unwrap(),
        ),
        chat: Arc::new(Registry::default()),
        email: Arc::new(Registry::default()),
        alarms: Arc::new(Catalogue::default()),
        metrics: None,
    };

    let report = run(&state).await;

    assert!(report.targets.is_empty());
    assert!(report.failure.is_none());
    assert_eq!(report.definitions, 0);
}
