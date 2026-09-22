//! Reading the catalogue's metrics and evaluating every rule against them.

pub mod announce;

use std::collections::BTreeMap;

use external_services::metrics_service::{
    Cursor, MetricPage, MetricQuery, MetricRequest, MetricsProvider, Period, SeriesStatus,
    TimeRange,
};
use time::OffsetDateTime;

use crate::{
    domain::cloudwatch::{
        evaluate_with_breaches, evaluation_range, Catalogue, Comparison, Evaluation, Outcome,
        RuleState, Unread,
    },
    logger,
    settings::cloudwatch::{AlarmDefinition, CloudWatchSettings},
    state::AppState,
    utils::{evaluation_cadence, latest_settled_period},
};

/// What `GetMetricData` accepts in one call, mirroring the provider's own limit. Exceeding it
/// fails the whole batch, so the catalogue is chunked rather than trusted to stay small.
const MAX_QUERIES_PER_REQUEST: usize = 500;

/// One request and the definitions it answers for, in query order.
struct Batch<'a> {
    request: MetricRequest,
    definitions: Vec<(&'a String, &'a AlarmDefinition)>,
}

/// A batch's readings by query index. `None` against an index means the provider returned that
/// series but could not complete it; an absent index means it said nothing at all.
type Readings = BTreeMap<usize, Option<Vec<Option<f64>>>>;

/// Which of the two consecutive evaluations a request covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Window {
    Current,
    /// The evaluation one cadence earlier — what CloudWatch would have said a moment ago.
    Previous,
}

/// Evaluate the catalogue at the latest publication-settled boundary before `now`, and again as
/// it stood one evaluation earlier.
///
/// Two reads rather than one wider one: a 300-second window ending a minute earlier sits on
/// a different bucket grid, since buckets anchor at the request's start. One response cannot
/// hold both.
pub async fn compare_catalogue(state: &AppState, now: OffsetDateTime) -> Comparison {
    let Some(provider) = state.metrics.as_deref() else {
        return Comparison::default();
    };
    let settings = &state.conf.cloudwatch;

    let current = read(provider, settings, now, Window::Current).await;
    let previous = read(provider, settings, now, Window::Previous).await;

    Comparison {
        transitions: current.transitions_from(&previous),
        current,
    }
}

/// Evaluate every configured definition at the latest publication-settled boundary before `now`.
pub async fn evaluate_catalogue(state: &AppState, now: OffsetDateTime) -> Catalogue {
    let Some(provider) = state.metrics.as_deref() else {
        // Boot refuses a catalogue with no client, so there is nothing to evaluate.
        return Catalogue::default();
    };

    read(provider, &state.conf.cloudwatch, now, Window::Current).await
}

async fn read(
    provider: &dyn MetricsProvider,
    settings: &CloudWatchSettings,
    now: OffsetDateTime,
    window: Window,
) -> Catalogue {
    let mut definitions = Vec::with_capacity(settings.alarms.len());

    for batch in plan(settings, now, window) {
        let readings = fetch(provider, &batch.request).await;
        definitions.extend(interpret(&batch, readings));
    }

    definitions.sort_by(|one, other| one.id.cmp(&other.id));
    Catalogue { definitions }
}

/// The requests that cover `settings`: grouped by period, each as wide as its own widest rule.
fn plan(settings: &CloudWatchSettings, now: OffsetDateTime, window: Window) -> Vec<Batch<'_>> {
    settings
        .alarms
        .iter()
        .fold(
            BTreeMap::<_, Vec<_>>::new(),
            |mut by_period, (id, definition)| {
                by_period
                    .entry(definition.period())
                    .or_default()
                    .push((id, definition));
                by_period
            },
        )
        .into_iter()
        .flat_map(|(period, definitions)| batches(period, definitions, now, window))
        .collect()
}

fn batches<'a>(
    period: Period,
    mut definitions: Vec<(&'a String, &'a AlarmDefinition)>,
    now: OffsetDateTime,
    window: Window,
) -> Vec<Batch<'a>> {
    definitions.sort_by(|(one, _), (other, _)| one.cmp(other));

    let width = definitions
        .iter()
        .flat_map(|(_, definition)| definition.severities.values())
        .map(evaluation_range)
        .max()
        .unwrap_or(1);
    let settled = latest_settled_period(now, period);
    let end = match window {
        Window::Current => settled,
        Window::Previous => settled - evaluation_cadence(period),
    };
    let range = TimeRange::ending_at(end, period, width);

    definitions
        .chunks(MAX_QUERIES_PER_REQUEST)
        .map(|chunk| Batch {
            request: MetricRequest {
                range,
                queries: chunk
                    .iter()
                    .map(|(_, definition)| query(definition, period))
                    .collect(),
            },
            definitions: chunk.to_vec(),
        })
        .collect()
}

fn query(definition: &AlarmDefinition, period: Period) -> MetricQuery {
    MetricQuery {
        namespace: Some(definition.namespace.clone()),
        name: definition.metric_name.clone(),
        labels: definition.labels(),
        period,
        aggregation: definition.statistic.into(),
    }
}

/// Read every page of `request`, or `Err` if any call failed.
async fn fetch(provider: &dyn MetricsProvider, request: &MetricRequest) -> Result<Readings, ()> {
    let mut readings = Readings::new();
    let mut cursor: Option<Cursor> = None;

    loop {
        let page = provider.fetch(request, cursor.as_ref()).await.map_err(
            |error| logger::error!(error = ?error, "CloudWatch metrics could not be read"),
        )?;

        cursor = page.cursor().cloned();
        readings = absorb(readings, page);

        if cursor.is_none() {
            return Ok(readings);
        }
    }
}

/// Fold one page into what has been read so far.
///
/// **Pages merge slot by slot rather than concatenating.** Every page carries the request's whole
/// period grid with only its own datapoints filled, so appending them would produce several
/// windows back to back and the readings on earlier pages would be sliced away unread.
///
/// Incomplete retrieval is not a run of missing readings — a shortened window would be judged as
/// though the periods it lost had reported nothing — so it poisons the index instead.
fn absorb(readings: Readings, page: MetricPage) -> Readings {
    page.into_series()
        .into_iter()
        .fold(readings, |mut readings, series| {
            let merged = match (readings.remove(&series.index()).flatten(), series.status()) {
                (_, SeriesStatus::Partial | SeriesStatus::Failed) => None,
                (None, _) => Some(series.values().to_vec()),
                (Some(seen), _) => Some(merge(&seen, series.values())),
            };

            readings.insert(series.index(), merged);
            readings
        })
}

fn merge(seen: &[Option<f64>], arrived: &[Option<f64>]) -> Vec<Option<f64>> {
    (0..seen.len().max(arrived.len()))
        .map(|slot| {
            let at = |values: &[Option<f64>]| values.get(slot).copied().flatten();
            at(seen).or(at(arrived))
        })
        .collect()
}

/// Turn one batch's readings into an outcome per definition.
fn interpret(batch: &Batch<'_>, readings: Result<Readings, ()>) -> Vec<Evaluation> {
    let Ok(readings) = readings else {
        return batch
            .definitions
            .iter()
            .map(|(id, definition)| {
                unread(id, definition, batch.request.range, Unread::QueryFailed)
            })
            .collect();
    };

    batch
        .definitions
        .iter()
        .enumerate()
        .map(|(index, (id, definition))| match readings.get(&index) {
            None => unread(id, definition, batch.request.range, Unread::SeriesMissing),
            Some(None) => unread(
                id,
                definition,
                batch.request.range,
                Unread::SeriesIncomplete,
            ),
            Some(Some(series)) => evaluated(id, definition, batch.request.range, series),
        })
        .collect()
}

fn evaluated(
    id: &str,
    definition: &AlarmDefinition,
    range: TimeRange,
    readings: &[Option<f64>],
) -> Evaluation {
    let mut rules: Vec<RuleState> = definition
        .severities
        .iter()
        .map(|(severity, rule)| {
            let (state, breaching_datapoints) =
                evaluate_with_breaches(rule, window(readings, evaluation_range(rule)));

            RuleState {
                severity: severity.clone(),
                state,
                threshold: rule.threshold,
                comparison_operator: rule.comparison_operator,
                evaluation_periods: rule.evaluation_periods,
                datapoints_to_alarm: rule.datapoints_to_alarm,
                treat_missing_data: rule.treat_missing_data,
                breaching_datapoints,
                description: rule.description.clone(),
            }
        })
        .collect();
    rules.sort_by(|one, other| one.severity.cmp(&other.severity));

    Evaluation {
        outcome: Outcome::Evaluated {
            readings: readings.to_vec(),
            rules,
        },
        ..describe(id, definition, range)
    }
}

/// The `width` most recent readings: the tail one rule needs out of what its definition fetched
/// for the widest rule under it.
fn window(readings: &[Option<f64>], width: u32) -> &[Option<f64>] {
    let width = usize::try_from(width).unwrap_or(usize::MAX);

    readings
        .get(readings.len().saturating_sub(width)..)
        .unwrap_or(readings)
}

fn unread(id: &str, definition: &AlarmDefinition, range: TimeRange, reason: Unread) -> Evaluation {
    Evaluation {
        outcome: Outcome::Unread { reason },
        ..describe(id, definition, range)
    }
}

fn describe(id: &str, definition: &AlarmDefinition, range: TimeRange) -> Evaluation {
    Evaluation {
        id: id.to_owned(),
        name: definition.name.clone(),
        classification: definition.classification.clone(),
        namespace: definition.namespace.clone(),
        metric_name: definition.metric_name.clone(),
        statistic: definition.statistic,
        dimensions: definition
            .dimensions
            .iter()
            .map(|dimension| (dimension.name.clone(), dimension.value.clone()))
            .collect(),
        period: definition.period,
        range_start: range.start,
        range_end: range.end,
        outcome: Outcome::Unread {
            reason: Unread::SeriesMissing,
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use external_services::metrics_service::{
        Aggregation, Labels, MetricSeries, MetricsError, MetricsResult,
    };
    use time::macros::datetime;

    use super::*;
    use crate::{
        domain::cloudwatch::State,
        settings::cloudwatch::{
            ComparisonOperator, Dimension, MissingDataPolicy, SeverityRule, Statistic,
        },
    };

    const NOW: OffsetDateTime = datetime!(2026-09-11 12:07:42.5 UTC);

    /// Answers from a script and records what it was asked.
    #[derive(Debug)]
    struct StubProvider {
        pages: Mutex<Vec<MetricsResult<MetricPage>>>,
        seen: Mutex<Vec<MetricRequest>>,
    }

    impl StubProvider {
        fn answering(pages: Vec<MetricsResult<MetricPage>>) -> Arc<Self> {
            Arc::new(Self {
                pages: Mutex::new(pages),
                seen: Mutex::new(vec![]),
            })
        }

        fn requests(&self) -> Vec<MetricRequest> {
            self.seen.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl MetricsProvider for StubProvider {
        async fn fetch(
            &self,
            request: &MetricRequest,
            _cursor: Option<&Cursor>,
        ) -> MetricsResult<MetricPage> {
            self.seen.lock().unwrap().push(request.clone());
            let mut pages = self.pages.lock().unwrap();
            if pages.is_empty() {
                Ok(MetricPage::default())
            } else {
                pages.remove(0)
            }
        }
    }

    fn page(series: Vec<MetricSeries>) -> MetricsResult<MetricPage> {
        Ok(MetricPage::new(series, None))
    }

    fn complete(index: usize, values: Vec<Option<f64>>) -> MetricSeries {
        MetricSeries::new(index, SeriesStatus::Complete, values)
    }

    fn rule(evaluation_periods: u32, threshold: f64) -> SeverityRule {
        SeverityRule {
            threshold,
            comparison_operator: ComparisonOperator::GreaterThanOrEqualToThreshold,
            evaluation_periods,
            datapoints_to_alarm: None,
            treat_missing_data: MissingDataPolicy::NotBreaching,
            description: "SEV: something is wrong.".to_owned(),
        }
    }

    fn definition(period: u32, severities: Vec<(&str, SeverityRule)>) -> AlarmDefinition {
        AlarmDefinition {
            name: "rds-primary-cpu".to_owned(),
            classification: "rds-alerts".to_owned(),
            metric_name: "CPUUtilization".to_owned(),
            namespace: "AWS/RDS".to_owned(),
            dimensions: [Dimension {
                name: "DBInstanceIdentifier".to_owned(),
                value: "hyperswitchdb-primary".to_owned(),
            }]
            .into_iter()
            .collect(),
            period,
            statistic: Statistic::Average,
            severities: severities
                .into_iter()
                .map(|(severity, rule)| (severity.to_owned(), rule))
                .collect(),
        }
    }

    fn catalogue(alarms: Vec<(&str, AlarmDefinition)>) -> CloudWatchSettings {
        CloudWatchSettings {
            client: Default::default(),
            destinations: HashMap::new(),
            alarms: alarms
                .into_iter()
                .map(|(id, definition)| (id.to_owned(), definition))
                .collect(),
        }
    }

    fn states(evaluation: &Evaluation) -> Vec<(&str, State)> {
        match &evaluation.outcome {
            Outcome::Evaluated { rules, .. } => rules
                .iter()
                .map(|rule| (rule.severity.as_str(), rule.state))
                .collect(),
            Outcome::Unread { .. } => vec![],
        }
    }

    fn readings(evaluation: &Evaluation) -> Vec<Option<f64>> {
        match &evaluation.outcome {
            Outcome::Evaluated { readings, .. } => readings.clone(),
            Outcome::Unread { .. } => vec![],
        }
    }

    #[tokio::test]
    async fn one_request_per_period_each_as_wide_as_its_own_widest_rule() {
        let settings = catalogue(vec![
            ("minutely", definition(60, vec![("sev1", rule(1, 90.0))])),
            ("slow", definition(60, vec![("sev1", rule(5, 90.0))])),
            (
                "five_minutely",
                definition(300, vec![("sev1", rule(3, 90.0))]),
            ),
        ]);
        let provider = StubProvider::answering(vec![]);

        read(provider.as_ref(), &settings, NOW, Window::Current).await;
        let requests = provider.requests();

        assert_eq!(requests.len(), 2, "one per period, not one per definition");

        // 60s, widest window 5, so 5 + 2 periods back from the latest settled minute.
        assert_eq!(requests[0].queries.len(), 2);
        assert_eq!(requests[0].range.end, datetime!(2026-09-11 12:05:00 UTC));
        assert_eq!(requests[0].range.start, datetime!(2026-09-11 11:58:00 UTC));

        // 300s, window 3, so 5 buckets ending on the same minute — off the five-minute grid,
        // which is what CloudWatch's sliding window does.
        assert_eq!(requests[1].queries.len(), 1);
        assert_eq!(requests[1].range.start, datetime!(2026-09-11 11:40:00 UTC));
    }

    #[tokio::test]
    async fn a_query_names_the_stream_its_definition_configured() {
        let settings = catalogue(vec![("cpu", definition(60, vec![("sev1", rule(1, 90.0))]))]);
        let provider = StubProvider::answering(vec![]);

        read(provider.as_ref(), &settings, NOW, Window::Current).await;
        let query = &provider.requests()[0].queries[0];

        assert_eq!(query.namespace.as_deref(), Some("AWS/RDS"));
        assert_eq!(query.name, "CPUUtilization");
        assert_eq!(query.period, Period::ONE_MINUTE);
        assert_eq!(query.aggregation, Aggregation::Average);
        assert_eq!(
            query.labels,
            [("DBInstanceIdentifier", "hyperswitchdb-primary")]
                .into_iter()
                .collect::<Labels>()
        );
    }

    /// The provider rejects a batch above its own limit, taking every definition in it down.
    #[tokio::test]
    async fn a_period_group_is_chunked_below_the_providers_query_limit() {
        let alarms: Vec<(String, AlarmDefinition)> = (0..MAX_QUERIES_PER_REQUEST + 10)
            .map(|n| {
                (
                    format!("cpu_{n:04}"),
                    definition(60, vec![("sev1", rule(1, 90.0))]),
                )
            })
            .collect();
        let settings = CloudWatchSettings {
            client: Default::default(),
            destinations: HashMap::new(),
            alarms: alarms.into_iter().collect(),
        };
        let provider = StubProvider::answering(vec![]);

        read(provider.as_ref(), &settings, NOW, Window::Current).await;

        let sizes: Vec<usize> = provider
            .requests()
            .iter()
            .map(|request| request.queries.len())
            .collect();
        assert_eq!(sizes, vec![MAX_QUERIES_PER_REQUEST, 10]);
    }

    #[tokio::test]
    async fn a_definition_is_read_once_and_judged_by_every_severity() {
        let settings = catalogue(vec![(
            "cpu",
            definition(
                60,
                vec![
                    ("sev1", rule(1, 90.0)),
                    ("sev2", rule(1, 85.0)),
                    ("sev3", rule(1, 65.0)),
                ],
            ),
        )]);
        let provider = StubProvider::answering(vec![page(vec![complete(0, vec![Some(87.0)])])]);

        let catalogue = read(provider.as_ref(), &settings, NOW, Window::Current).await;

        assert_eq!(
            provider.requests()[0].queries.len(),
            1,
            "one query, three thresholds"
        );
        assert_eq!(
            states(&catalogue.definitions[0]),
            vec![
                ("sev1", State::Ok),
                ("sev2", State::Alarm),
                ("sev3", State::Alarm)
            ]
        );
    }

    /// A narrow rule sharing a batch with a wider one reads only its own tail.
    #[tokio::test]
    async fn a_rule_slices_its_own_window_not_the_batchs() {
        let settings = catalogue(vec![(
            "cpu",
            definition(60, vec![("sev1", rule(1, 90.0)), ("sev3", rule(5, 90.0))]),
        )]);
        let provider = StubProvider::answering(vec![page(vec![complete(
            0,
            vec![
                Some(99.0),
                Some(99.0),
                Some(10.0),
                Some(10.0),
                Some(10.0),
                Some(10.0),
                Some(10.0),
            ],
        )])]);

        let catalogue = read(provider.as_ref(), &settings, NOW, Window::Current).await;

        assert_eq!(
            states(&catalogue.definitions[0]),
            vec![("sev1", State::Ok), ("sev3", State::Ok)]
        );
    }

    /// Every page carries the whole grid, so appending them would bury the earlier ones.
    #[tokio::test]
    async fn a_series_spanning_pages_merges_slot_by_slot() {
        let settings = catalogue(vec![("cpu", definition(60, vec![("sev1", rule(3, 90.0))]))]);
        let provider = StubProvider::answering(vec![
            Ok(MetricPage::new(
                vec![complete(0, vec![Some(99.0), Some(99.0), None, None, None])],
                Some(Cursor::new("more")),
            )),
            page(vec![complete(0, vec![None, None, Some(99.0), None, None])]),
        ]);

        let catalogue = read(provider.as_ref(), &settings, NOW, Window::Current).await;

        assert_eq!(
            readings(&catalogue.definitions[0]),
            vec![Some(99.0), Some(99.0), Some(99.0), None, None],
            "five slots, not ten"
        );
        assert_eq!(
            states(&catalogue.definitions[0]),
            vec![("sev1", State::Alarm)],
            "the breach on the first page still counts"
        );
    }

    #[tokio::test]
    async fn an_incomplete_series_yields_no_state_rather_than_a_short_window() {
        let settings = catalogue(vec![
            ("first", definition(60, vec![("sev1", rule(3, 90.0))])),
            ("second", definition(60, vec![("sev1", rule(3, 90.0))])),
        ]);
        let provider = StubProvider::answering(vec![page(vec![
            MetricSeries::new(0, SeriesStatus::Partial, vec![Some(10.0)]),
            complete(1, vec![Some(10.0); 5]),
        ])]);

        let catalogue = read(provider.as_ref(), &settings, NOW, Window::Current).await;

        assert_eq!(
            catalogue.definitions[0].outcome,
            Outcome::Unread {
                reason: Unread::SeriesIncomplete
            }
        );
        assert_eq!(states(&catalogue.definitions[1]), vec![("sev1", State::Ok)]);
    }

    #[tokio::test]
    async fn a_query_the_provider_ignored_is_not_a_series_of_gaps() {
        let settings = catalogue(vec![("cpu", definition(60, vec![("sev1", rule(1, 90.0))]))]);
        let provider = StubProvider::answering(vec![page(vec![])]);

        let catalogue = read(provider.as_ref(), &settings, NOW, Window::Current).await;

        assert_eq!(
            catalogue.definitions[0].outcome,
            Outcome::Unread {
                reason: Unread::SeriesMissing
            }
        );
    }

    /// One failing call must not blind the other period.
    #[tokio::test]
    async fn a_failed_call_leaves_only_its_own_batch_unread() {
        let settings = catalogue(vec![
            ("minutely", definition(60, vec![("sev1", rule(1, 90.0))])),
            (
                "five_minutely",
                definition(300, vec![("sev1", rule(1, 90.0))]),
            ),
        ]);
        let provider = StubProvider::answering(vec![
            Err(error_stack::report!(MetricsError::Transport)),
            page(vec![complete(0, vec![Some(99.0)])]),
        ]);

        let catalogue = read(provider.as_ref(), &settings, NOW, Window::Current).await;

        assert_eq!(
            catalogue.definitions[1].outcome,
            Outcome::Unread {
                reason: Unread::QueryFailed
            },
            "the 60s batch failed"
        );
        assert_eq!(
            states(&catalogue.definitions[0]),
            vec![("sev1", State::Alarm)],
            "the 300s batch still answered"
        );
    }

    #[tokio::test]
    async fn an_empty_catalogue_asks_for_nothing() {
        let provider = StubProvider::answering(vec![]);

        let catalogue = read(provider.as_ref(), &catalogue(vec![]), NOW, Window::Current).await;

        assert_eq!(catalogue, Catalogue::default());
        assert!(provider.requests().is_empty());
    }

    /// The previous evaluation is one cadence back — a minute, even for a five-minute period,
    /// because that is how often CloudWatch re-evaluates it.
    #[tokio::test]
    async fn the_previous_window_ends_one_cadence_before_the_current_one() {
        let settings = catalogue(vec![
            ("minutely", definition(60, vec![("sev1", rule(1, 90.0))])),
            (
                "five_minutely",
                definition(300, vec![("sev1", rule(3, 90.0))]),
            ),
        ]);
        let provider = StubProvider::answering(vec![]);

        read(provider.as_ref(), &settings, NOW, Window::Current).await;
        read(provider.as_ref(), &settings, NOW, Window::Previous).await;

        let seen = provider.requests();
        let (minutely, five_minutely) = (&seen[0], &seen[1]);
        let (minutely_before, five_minutely_before) = (&seen[2], &seen[3]);

        assert_eq!(
            minutely.range.end - minutely_before.range.end,
            time::Duration::minutes(1)
        );
        assert_eq!(
            five_minutely.range.end - five_minutely_before.range.end,
            time::Duration::minutes(1),
            "a five-minute period still steps a minute at a time"
        );

        // The shifted 300s window sits on a different bucket grid, which is why it cannot be
        // sliced out of the current one.
        assert_eq!(
            five_minutely_before.range.start,
            datetime!(2026-09-11 11:39:00 UTC)
        );
        assert_eq!(
            five_minutely.range.start,
            datetime!(2026-09-11 11:40:00 UTC)
        );
    }
}
