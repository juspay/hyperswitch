//! Reading the catalogue's metrics and evaluating every rule against them.
//!
//! Rules under one definition share a metric and differ only in threshold, so the unit of *reading*
//! is the definition and the unit of *judging* is the rule: one query per definition, then each
//! rule slices the tail it needs from the same series.
//!
//! Definitions are batched by period. A [`MetricRequest`] carries one range for all its queries, so
//! mixing periods would size every series to the widest one; a request per period keeps each as
//! narrow as its own rules need.
//!
//! The window ends at the latest completed minute and slides with it, which is CloudWatch's own
//! default — the boundaries are not aligned to the wall clock. `GetMetricData` anchors its buckets
//! at the request's start rather than to an epoch grid, so a 300-second window ending at 12:07
//! comes back as 11:42, 11:47 … 12:02 (verified against sandbox, 2026-09-11).

use std::collections::BTreeMap;

use external_services::metrics_service::{
    Cursor, MetricQuery, MetricRequest, MetricsProvider, Period, SeriesStatus, TimeRange,
};
use time::OffsetDateTime;

use crate::{
    domain::cloudwatch::{evaluate, evaluation_range, State},
    logger,
    settings::cloudwatch::{CloudWatchSettings, Definition},
};

/// Every definition's outcome, by id.
#[derive(Debug, Default, PartialEq)]
pub struct Catalogue {
    pub definitions: Vec<Evaluation>,
}

#[derive(Debug, PartialEq)]
pub struct Evaluation {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub metric_name: String,
    pub period: u32,
    pub outcome: Outcome,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// The readings the rules were judged against, and what each of them said.
    Evaluated {
        readings: Vec<Option<f64>>,
        rules: Vec<RuleState>,
    },
    /// A definition we could not read is not a definition that is fine, so it carries no rule
    /// states rather than states derived from an absence.
    Unread { reason: Unread },
}

#[derive(Debug, PartialEq)]
pub enum Unread {
    /// The call covering this definition failed.
    QueryFailed,
    /// The series arrived, but the provider said it was incomplete.
    SeriesIncomplete,
    /// The provider said nothing about this query, which is not a series of gaps.
    SeriesMissing,
}

#[derive(Debug, PartialEq)]
pub struct RuleState {
    pub severity: String,
    pub state: State,
    pub threshold: f64,
    pub description: String,
}

/// One request and the definitions it answers for, in query order.
struct Batch<'a> {
    request: MetricRequest,
    definitions: Vec<(&'a String, &'a Definition)>,
}

/// A batch's readings by query index. `None` against an index means the provider returned that
/// series but could not complete it; an absent index means it said nothing at all.
type Readings = BTreeMap<usize, Option<Vec<Option<f64>>>>;

/// Evaluate the whole catalogue as of `now`.
///
/// `now` is a parameter so the window is a fact of the call rather than of the clock.
pub async fn evaluate_catalogue(
    provider: &dyn MetricsProvider,
    settings: &CloudWatchSettings,
    now: OffsetDateTime,
) -> Catalogue {
    let mut definitions = Vec::with_capacity(settings.definitions.len());

    for batch in plan(settings, latest_completed_minute(now)) {
        let readings = fetch(provider, &batch.request).await;
        definitions.extend(interpret(&batch, readings));
    }

    definitions.sort_by(|one, other| one.id.cmp(&other.id));
    Catalogue { definitions }
}

/// The requests that cover `settings`: one per period, each as wide as its own widest rule.
fn plan(settings: &CloudWatchSettings, end: OffsetDateTime) -> Vec<Batch<'_>> {
    let mut by_period: BTreeMap<Period, Vec<(&String, &Definition)>> = BTreeMap::new();

    for (id, definition) in &settings.definitions {
        by_period
            .entry(definition.period())
            .or_default()
            .push((id, definition));
    }

    by_period
        .into_iter()
        .map(|(period, mut definitions)| {
            definitions.sort_by(|(one, _), (other, _)| one.cmp(other));

            let width = definitions
                .iter()
                .flat_map(|(_, definition)| definition.severities.values())
                .map(evaluation_range)
                .max()
                .unwrap_or(1);

            Batch {
                request: MetricRequest {
                    range: TimeRange::ending_at(end, period, width),
                    queries: definitions
                        .iter()
                        .map(|(_, definition)| MetricQuery {
                            namespace: Some(definition.namespace.clone()),
                            name: definition.metric_name.clone(),
                            labels: definition.labels(),
                            period,
                            aggregation: definition.statistic.into(),
                        })
                        .collect(),
                },
                definitions,
            }
        })
        .collect()
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
        for series in page.into_series() {
            absorb(
                &mut readings,
                series.index(),
                series.status(),
                series.values(),
            );
        }

        if cursor.is_none() {
            return Ok(readings);
        }
    }
}

/// Fold one series into what has been read so far.
///
/// A series can span pages, so values accumulate under their index. Incomplete retrieval is not a
/// run of missing readings — a shortened window would be judged as though the periods it lost had
/// reported nothing — so it poisons the index instead.
fn absorb(readings: &mut Readings, index: usize, status: SeriesStatus, values: &[Option<f64>]) {
    let entry = readings.entry(index).or_insert_with(|| Some(vec![]));

    match status {
        SeriesStatus::Complete => {
            if let Some(collected) = entry {
                collected.extend_from_slice(values);
            }
        }
        SeriesStatus::Partial | SeriesStatus::Failed => *entry = None,
    }
}

/// Turn one batch's readings into an outcome per definition.
fn interpret(batch: &Batch<'_>, readings: Result<Readings, ()>) -> Vec<Evaluation> {
    let readings = match readings {
        Ok(readings) => readings,
        Err(()) => {
            return batch
                .definitions
                .iter()
                .map(|(id, definition)| unread(id, definition, Unread::QueryFailed))
                .collect()
        }
    };

    batch
        .definitions
        .iter()
        .enumerate()
        .map(|(index, (id, definition))| match readings.get(&index) {
            None => unread(id, definition, Unread::SeriesMissing),
            Some(None) => unread(id, definition, Unread::SeriesIncomplete),
            Some(Some(series)) => evaluated(id, definition, series),
        })
        .collect()
}

fn evaluated(id: &str, definition: &Definition, readings: &[Option<f64>]) -> Evaluation {
    let mut rules: Vec<RuleState> = definition
        .severities
        .iter()
        .map(|(severity, rule)| {
            let width = usize::try_from(evaluation_range(rule)).unwrap_or(usize::MAX);
            let window = readings
                .get(readings.len().saturating_sub(width)..)
                .unwrap_or(readings);

            RuleState {
                severity: severity.clone(),
                state: evaluate(rule, window),
                threshold: rule.threshold,
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
        ..describe(id, definition)
    }
}

fn unread(id: &str, definition: &Definition, reason: Unread) -> Evaluation {
    Evaluation {
        outcome: Outcome::Unread { reason },
        ..describe(id, definition)
    }
}

fn describe(id: &str, definition: &Definition) -> Evaluation {
    Evaluation {
        id: id.to_owned(),
        name: definition.name.clone(),
        classification: definition.classification.clone(),
        metric_name: definition.metric_name.clone(),
        period: definition.period,
        outcome: Outcome::Unread {
            reason: Unread::SeriesMissing,
        },
    }
}

/// `TimeRange` is half-open, so this as the end makes the last slot the minute before it — one
/// CloudWatch has had a chance to receive readings for.
fn latest_completed_minute(now: OffsetDateTime) -> OffsetDateTime {
    now.replace_second(0)
        .and_then(|minute| minute.replace_nanosecond(0))
        .unwrap_or(now)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use external_services::metrics_service::{Aggregation, Labels};
    use time::macros::datetime;

    use super::*;
    use crate::settings::cloudwatch::{
        ComparisonOperator, Dimension, MissingDataPolicy, Rule, Statistic,
    };

    const NOW: OffsetDateTime = datetime!(2026-09-11 12:07:42.5 UTC);

    fn rule(evaluation_periods: u32, threshold: f64) -> Rule {
        Rule {
            threshold,
            comparison_operator: ComparisonOperator::GreaterThanOrEqualToThreshold,
            evaluation_periods,
            datapoints_to_alarm: None,
            treat_missing_data: MissingDataPolicy::NotBreaching,
            description: "SEV: something is wrong.".to_owned(),
        }
    }

    fn definition(period: u32, severities: Vec<(&str, Rule)>) -> Definition {
        Definition {
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

    fn catalogue(definitions: Vec<(&str, Definition)>) -> CloudWatchSettings {
        CloudWatchSettings {
            client: Default::default(),
            definitions: definitions
                .into_iter()
                .map(|(id, definition)| (id.to_owned(), definition))
                .collect(),
        }
    }

    fn complete(values: Vec<Option<f64>>) -> Result<Readings, ()> {
        let mut readings = Readings::new();
        absorb(&mut readings, 0, SeriesStatus::Complete, &values);
        Ok(readings)
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

    #[test]
    fn one_request_per_period_each_as_wide_as_its_own_widest_rule() {
        let settings = catalogue(vec![
            ("minutely", definition(60, vec![("sev1", rule(1, 90.0))])),
            ("slow", definition(60, vec![("sev1", rule(5, 90.0))])),
            (
                "five_minutely",
                definition(300, vec![("sev1", rule(3, 90.0))]),
            ),
        ]);

        let batches = plan(&settings, latest_completed_minute(NOW));

        assert_eq!(batches.len(), 2, "one per period, not one per definition");

        // 60s, widest window 5, so 5 + 2 periods back from the latest completed minute.
        assert_eq!(batches[0].request.queries.len(), 2);
        assert_eq!(
            batches[0].request.range.end,
            datetime!(2026-09-11 12:07:00 UTC)
        );
        assert_eq!(
            batches[0].request.range.start,
            datetime!(2026-09-11 12:00:00 UTC)
        );

        // 300s, window 3, so 5 buckets — ending on the same minute, off the five-minute grid,
        // which is what CloudWatch's sliding window does.
        assert_eq!(batches[1].request.queries.len(), 1);
        assert_eq!(
            batches[1].request.range.start,
            datetime!(2026-09-11 11:42:00 UTC)
        );
    }

    #[test]
    fn a_query_names_the_stream_its_definition_configured() {
        let settings = catalogue(vec![("cpu", definition(60, vec![("sev1", rule(1, 90.0))]))]);

        let batches = plan(&settings, NOW);
        let query = &batches[0].request.queries[0];

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

    #[test]
    fn a_definition_is_read_once_and_judged_by_every_severity() {
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
        let batches = plan(&settings, NOW);

        assert_eq!(
            batches[0].request.queries.len(),
            1,
            "one query, three thresholds"
        );
        assert_eq!(
            states(&interpret(&batches[0], complete(vec![Some(87.0)]))[0]),
            vec![
                ("sev1", State::Ok),
                ("sev2", State::Alarm),
                ("sev3", State::Alarm)
            ]
        );
    }

    /// A narrow rule sharing a batch with a wider one reads only its own tail.
    #[test]
    fn a_rule_slices_its_own_window_not_the_batch_s() {
        let settings = catalogue(vec![(
            "cpu",
            definition(60, vec![("sev1", rule(1, 90.0)), ("sev3", rule(5, 90.0))]),
        )]);
        let batches = plan(&settings, NOW);

        // Breaching two periods ago, fine since. The five-period rule still sees the breach in its
        // window and needs every period of it, so it stays Ok; the one-period rule sees only the
        // last reading.
        let readings = complete(vec![
            Some(99.0),
            Some(99.0),
            Some(10.0),
            Some(10.0),
            Some(10.0),
            Some(10.0),
            Some(10.0),
        ]);

        assert_eq!(
            states(&interpret(&batches[0], readings)[0]),
            vec![("sev1", State::Ok), ("sev3", State::Ok)]
        );
    }

    #[test]
    fn an_incomplete_series_yields_no_state_rather_than_a_short_window() {
        let settings = catalogue(vec![
            ("first", definition(60, vec![("sev1", rule(3, 90.0))])),
            ("second", definition(60, vec![("sev1", rule(3, 90.0))])),
        ]);
        let batches = plan(&settings, NOW);

        let mut readings = Readings::new();
        absorb(&mut readings, 0, SeriesStatus::Partial, &[Some(10.0)]);
        absorb(&mut readings, 1, SeriesStatus::Complete, &[Some(10.0); 5]);

        let outcomes = interpret(&batches[0], Ok(readings));

        assert_eq!(
            outcomes[0].outcome,
            Outcome::Unread {
                reason: Unread::SeriesIncomplete
            }
        );
        assert_eq!(states(&outcomes[1]), vec![("sev1", State::Ok)]);
    }

    #[test]
    fn a_failed_series_poisons_its_index_even_after_a_complete_page() {
        let mut readings = Readings::new();
        absorb(&mut readings, 0, SeriesStatus::Complete, &[Some(1.0)]);
        absorb(&mut readings, 0, SeriesStatus::Failed, &[]);

        assert_eq!(readings.get(&0), Some(&None));
    }

    #[test]
    fn a_series_spanning_pages_is_concatenated_under_its_index() {
        let mut readings = Readings::new();
        absorb(&mut readings, 0, SeriesStatus::Complete, &[Some(1.0), None]);
        absorb(&mut readings, 0, SeriesStatus::Complete, &[Some(3.0)]);

        assert_eq!(
            readings.get(&0),
            Some(&Some(vec![Some(1.0), None, Some(3.0)]))
        );
    }

    #[test]
    fn a_query_the_provider_ignored_is_not_a_series_of_gaps() {
        let settings = catalogue(vec![("cpu", definition(60, vec![("sev1", rule(1, 90.0))]))]);
        let batches = plan(&settings, NOW);

        assert_eq!(
            interpret(&batches[0], Ok(Readings::new()))[0].outcome,
            Outcome::Unread {
                reason: Unread::SeriesMissing
            }
        );
    }

    #[test]
    fn a_failed_call_leaves_every_definition_in_its_batch_unread() {
        let settings = catalogue(vec![
            ("first", definition(60, vec![("sev1", rule(1, 90.0))])),
            ("second", definition(60, vec![("sev1", rule(1, 90.0))])),
        ]);
        let batches = plan(&settings, NOW);

        for outcome in interpret(&batches[0], Err(())) {
            assert_eq!(
                outcome.outcome,
                Outcome::Unread {
                    reason: Unread::QueryFailed
                }
            );
        }
    }

    /// Batches are read independently, so one failing call cannot blind the other period.
    #[test]
    fn batches_are_independent() {
        let settings = catalogue(vec![
            ("minutely", definition(60, vec![("sev1", rule(1, 90.0))])),
            (
                "five_minutely",
                definition(300, vec![("sev1", rule(1, 90.0))]),
            ),
        ]);
        let batches = plan(&settings, NOW);

        let failed = interpret(&batches[0], Err(()));
        let answered = interpret(&batches[1], complete(vec![Some(99.0)]));

        assert_eq!(
            failed[0].outcome,
            Outcome::Unread {
                reason: Unread::QueryFailed
            }
        );
        assert_eq!(states(&answered[0]), vec![("sev1", State::Alarm)]);
    }

    #[test]
    fn an_empty_catalogue_asks_for_nothing() {
        assert!(plan(&catalogue(vec![]), NOW).is_empty());
    }

    #[test]
    fn the_window_ends_at_the_last_minute_that_finished() {
        assert_eq!(
            latest_completed_minute(NOW),
            datetime!(2026-09-11 12:07:00 UTC)
        );
    }
}
