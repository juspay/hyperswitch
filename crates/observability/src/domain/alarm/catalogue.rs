//! The catalogue, prepared: configuration turned into readings to fetch and rules to apply.
//!
//! *Prepared*, not resolved — the dimensions arrive already resolved, because the infrastructure
//! side renders them. What happens here is once, at boot, and it is an **inversion**.
//! Configuration is organised the way a person writes it, one entry per alarm with its severities
//! nested inside. This is organised the way a request needs it: a deduplicated list of **readings**
//! to ask CloudWatch for, and a flat list of **targets** — one per severity — each pointing at the
//! reading it is evaluated against.
//!
//! That inversion is the whole reason the module exists. The three severities of one definition
//! share a metric, dimensions, period and statistic; only their thresholds differ, and a threshold
//! is not part of a CloudWatch query. The twenty `rds-alerts` definitions carry fifty-three
//! severities between them and need **twenty** queries, not fifty-three.
//!
//! ## The two windows
//!
//! A transition is the difference between two consecutive evaluations, and CloudWatch's
//! consecutive evaluations are **one minute apart, whatever the period**. Its documentation is
//! explicit: *"if the Period is 5 minutes (300 seconds) and Evaluation Periods is 1, then at the
//! end of minute 5 the alarm evaluates based on data from minutes 1 to 5. Then at the end of
//! minute 6, the alarm is evaluated based on the data from minutes 2 to 6."* Two 300-second
//! aggregates one minute apart are not two adjacent datapoints on a five-minute grid; they are two
//! overlapping windows, and CloudWatch computes both.
//!
//! So a reading needs a grid for each. Whether one request can carry both comes down to one
//! question — does the minute the window slides by divide the period? — and
//! [`FetchPlan::build`] answers it per period rather than assuming either way:
//!
//! * **A 60-second metric**: yes. One request for `range + 1` datapoints holds both windows, one
//!   slot apart. This is the `N + 1` fetch the design started from, and it is correct *here*.
//! * **A 300-second metric**: no. The two windows sit on grids offset by a minute, and CloudWatch
//!   lays a returned datapoint on the grid the request's own start time defines. Two requests.
//!
//! Twelve of the twenty `rds-alerts` definitions are 60-second and eight are 300-second, so a run
//! is three `GetMetricData` calls.

use std::collections::BTreeMap;

use external_services::metrics_service::{
    Aggregation, Labels, MetricQuery, MetricRequest, Period, TimeRange,
};

use crate::{
    domain::alarm::{AlarmRule, ComparisonOperator, MissingDataPolicy},
    errors::ConfigurationError,
    settings::cloudwatch::{self, CloudWatchSettings},
};

/// How far apart two consecutive CloudWatch evaluations are, for any period of a minute or longer.
///
/// Not the period. AWS: *"For any period of one minute or longer, an alarm is evaluated every
/// minute"*, and the sliding window advances by a minute each time.
pub const EVALUATION_INTERVAL_SECONDS: i64 = 60;

/// One thing to ask CloudWatch for: a metric, narrowed and reduced.
///
/// Carries no threshold, because a threshold is not part of a query — which is exactly why
/// severities can share one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The namespace the metric is published under.
    pub namespace: String,
    /// The metric's name.
    pub metric_name: String,
    /// The dimensions narrowing it to one reporting stream.
    pub labels: Labels,
    /// How long one datapoint covers.
    pub period: Period,
    /// How the values inside a period are reduced.
    pub aggregation: Aggregation,
}

impl Reading {
    /// The identity two definitions must share to be answered by one query.
    ///
    /// A string rather than a derived `Hash`, because [`Labels`] is deliberately ordered rather
    /// than hashed; its iteration order is stable, so this is too.
    fn identity(&self) -> String {
        let labels = self
            .labels
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{}|{}|{}|{:?}|{labels}",
            self.namespace,
            self.metric_name,
            self.period.seconds(),
            self.aggregation
        )
    }

    /// The query that fetches it.
    pub fn query(&self) -> MetricQuery {
        MetricQuery {
            namespace: Some(self.namespace.clone()),
            name: self.metric_name.clone(),
            labels: self.labels.clone(),
            period: self.period,
            aggregation: self.aggregation,
        }
    }

    /// How the metric reads in an announcement.
    pub fn describe(&self) -> String {
        format!("{} {}", self.namespace, self.metric_name)
    }

    /// The dimensions, as an ordered map for the response body.
    pub fn dimensions(&self) -> BTreeMap<String, String> {
        self.labels
            .iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect()
    }

    /// A stand-in for a reading a target points at but the catalogue does not hold.
    ///
    /// Resolution cannot produce that — a reading is pushed before anything indexes it — but the
    /// evaluator still has to report *something* for such a target rather than dropping it, and a
    /// dropped severity is the kind of silence this service exists to prevent.
    pub fn unknown() -> Self {
        Self {
            namespace: String::new(),
            metric_name: String::new(),
            labels: Labels::default(),
            period: Period::ONE_MINUTE,
            aggregation: Aggregation::Average,
        }
    }
}

/// One severity of one definition: a rule, the reading it applies to, and where it announces.
#[derive(Debug, Clone)]
pub struct AlarmTarget {
    /// The catalogue entry's name.
    pub definition: String,
    /// The group it belongs to, carried through from the HCL.
    pub classification: String,
    /// The severity id.
    pub severity: String,
    /// The hand-written message this severity announces.
    pub description: String,
    /// Which entry of [`Catalogue::readings`] it is evaluated against.
    pub reading: usize,
    /// The threshold and window.
    pub rule: AlarmRule,
    /// The chat destination its announcements go to.
    pub destination: String,
}

impl AlarmTarget {
    /// How this target is named in logs and in the response.
    pub fn name(&self) -> String {
        format!("{}/{}", self.definition, self.severity)
    }
}

/// The whole catalogue, ready to evaluate.
#[derive(Debug, Clone, Default)]
pub struct Catalogue {
    /// Every distinct reading, deduplicated.
    pub readings: Vec<Reading>,
    /// Every severity, pointing into [`Self::readings`].
    pub targets: Vec<AlarmTarget>,
    /// How many datapoints beyond each window to fetch.
    pub missing_data_lookback: u32,
    /// How far back from the latest completed minute a run evaluates.
    pub evaluation_delay_seconds: i64,
    /// Where a failed evaluation is reported.
    pub failure_destination: String,
}

impl Catalogue {
    /// Turn validated configuration into readings and targets.
    ///
    /// # Errors
    ///
    /// Only for what [`CloudWatchSettings::validate`] would already have refused — a definition
    /// with no dimensions, a severity with no destination, `treat_missing_data = "ignore"`. It is
    /// re-run here rather than assumed, so the two cannot drift into a state where a catalogue
    /// prepares into something validation would not have allowed.
    pub fn resolve(settings: &CloudWatchSettings) -> Result<Self, ConfigurationError> {
        let mut catalogue = Self {
            missing_data_lookback: settings.missing_data_lookback,
            evaluation_delay_seconds: i64::from(settings.evaluation_delay_seconds),
            failure_destination: settings.failure_destination.clone().unwrap_or_default(),
            ..Self::default()
        };

        if !settings.is_enabled() {
            return Ok(catalogue);
        }

        settings.validate()?;

        // Positions of readings already seen, so the second definition watching a metric reuses
        // the first's query rather than adding a duplicate to the batch.
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();

        for (name, definition) in &settings.alarms {
            // Dimensions arrive resolved, so there is nothing to look up and nothing to expand.
            // The check that they are *there* stays, because an entry that lost its dimensions in
            // rendering would query the undimensioned metric and simply never fire.
            if definition.dimensions.is_empty() {
                Err(ConfigurationError::ConfigParsingError(format!(
                    "alarm `{name}` has no dimensions, which would read the undimensioned metric \
                     rather than the stream it names"
                )))?
            }
            let labels: Labels = definition
                .dimensions
                .iter()
                .map(|dimension| (dimension.name.clone(), dimension.value.clone()))
                .collect();

            let reading = Reading {
                namespace: definition.namespace.clone(),
                metric_name: definition.metric_name.clone(),
                labels,
                period: Period::from_seconds(i32::try_from(definition.period).unwrap_or(i32::MAX)),
                aggregation: aggregation_of(definition.statistic),
            };

            let index = match seen.get(&reading.identity()) {
                Some(index) => *index,
                None => {
                    let index = catalogue.readings.len();
                    seen.insert(reading.identity(), index);
                    catalogue.readings.push(reading);
                    index
                }
            };

            for (severity, rule) in &definition.severities {
                let destination = settings
                    .severity_destinations
                    .get(severity)
                    .ok_or_else(|| {
                        ConfigurationError::ConfigParsingError(format!(
                            "alarm `{name}` uses severity `{severity}`, which has no entry in \
                             cloudwatch.severity_destinations"
                        ))
                    })?
                    .clone();

                catalogue.targets.push(AlarmTarget {
                    definition: name.clone(),
                    classification: definition.classification.clone(),
                    severity: severity.clone(),
                    description: rule.description.clone(),
                    reading: index,
                    rule: AlarmRule {
                        comparison_operator: operator_of(rule.comparison_operator),
                        threshold: rule.threshold,
                        evaluation_periods: rule.evaluation_periods,
                        datapoints_to_alarm: rule.datapoints_to_alarm(),
                        treat_missing_data: policy_of(rule.treat_missing_data, name, severity)?,
                    },
                    destination,
                });
            }
        }

        Ok(catalogue)
    }

    /// Whether there is anything to evaluate.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// How many catalogue entries the targets came from.
    ///
    /// Counted rather than stored, because the flat list of targets is the authority on what a run
    /// actually covers and a second field could disagree with it.
    pub fn definitions(&self) -> usize {
        let mut names = self
            .targets
            .iter()
            .map(|target| target.definition.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();

        names.len()
    }

    /// The widest evaluation range any target reads `index` over.
    fn range_for(&self, index: usize) -> usize {
        self.targets
            .iter()
            .filter(|target| target.reading == index)
            .map(|target| target.rule.evaluation_range(self.missing_data_lookback))
            .max()
            .unwrap_or_default()
    }
}

/// Where one reading's two windows are to be found once the requests come back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridRef {
    /// Which of [`FetchPlan::requests`] holds it.
    pub request: usize,
    /// How many slots before the grid's end the window ends.
    pub back: usize,
}

/// One reading's place in the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadingPlan {
    /// Which of [`Catalogue::readings`] this is for.
    pub reading: usize,
    /// The evaluation ending at the run's instant.
    pub current: GridRef,
    /// The evaluation ending one minute earlier.
    pub previous: GridRef,
}

/// Every request a run makes, and where each reading's windows land in the answers.
#[derive(Debug, Clone, Default)]
pub struct FetchPlan {
    /// The requests to issue, in order. Each carries the queries for one period and one alignment.
    pub requests: Vec<MetricRequest>,
    /// Which query position inside each request answers which reading.
    ///
    /// Indexed as `positions[request][query] = reading`, mirroring the provider's promise that a
    /// series comes back keyed by its query's position.
    pub positions: Vec<Vec<usize>>,
    /// One entry per reading.
    pub readings: Vec<ReadingPlan>,
}

impl FetchPlan {
    /// Plan the requests for an evaluation ending at `end`.
    ///
    /// Readings are grouped by period, because one `GetMetricData` call covers one time range and
    /// a range is only meaningful against a period. Within a period, whether the two windows share
    /// a grid is decided by whether the minute the window slides by divides that period — see the
    /// module docs.
    pub fn build(catalogue: &Catalogue, end: time::PrimitiveDateTime) -> Self {
        // The one place a naive UTC instant becomes an offset one: `TimeRange` is the metrics
        // client's type and it takes an `OffsetDateTime`. Everything on this side of that call
        // stays naive, as `common_utils::date_time` and the rest of the repository do.
        let end = end.assume_utc();
        let mut plan = Self::default();

        let mut by_period: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
        for (index, reading) in catalogue.readings.iter().enumerate() {
            by_period
                .entry(reading.period.seconds())
                .or_default()
                .push(index);
        }

        for (seconds, readings) in by_period {
            let period = Period::from_seconds(seconds);
            let range = readings
                .iter()
                .map(|index| catalogue.range_for(*index))
                .max()
                .unwrap_or_default();

            // How many whole periods the window slides by between two evaluations. `None` means
            // the slide is shorter than a period, so the two windows do not share a grid.
            let stride = (period.seconds_i64() > 0
                && EVALUATION_INTERVAL_SECONDS % period.seconds_i64() == 0)
                .then(|| EVALUATION_INTERVAL_SECONDS / period.seconds_i64())
                .and_then(|stride| usize::try_from(stride).ok());

            // A rejected batch takes every metric in it down, so the cap is respected here rather
            // than discovered by the provider on a catalogue that has grown past it.
            for chunk in readings.chunks(MAX_QUERIES_PER_REQUEST) {
                match stride {
                    // One grid holds both windows: the previous evaluation is the same grid read
                    // `stride` slots further back.
                    Some(stride) => {
                        let slots = range.saturating_add(stride);
                        let request = plan.push(
                            catalogue,
                            TimeRange::ending_at(end, period, slot_count(slots)),
                            chunk,
                        );

                        plan.readings.extend(chunk.iter().map(|index| ReadingPlan {
                            reading: *index,
                            current: GridRef { request, back: 0 },
                            previous: GridRef {
                                request,
                                back: stride,
                            },
                        }));
                    }

                    // Two grids, a minute apart. Asking for one and reading it twice would compare
                    // a window against itself shifted by a whole period, which is a different
                    // question from the one CloudWatch asks.
                    None => {
                        let shift = time::Duration::seconds(EVALUATION_INTERVAL_SECONDS);
                        let current = plan.push(
                            catalogue,
                            TimeRange::ending_at(end, period, slot_count(range)),
                            chunk,
                        );
                        let previous = plan.push(
                            catalogue,
                            TimeRange::ending_at(end - shift, period, slot_count(range)),
                            chunk,
                        );

                        plan.readings.extend(chunk.iter().map(|index| ReadingPlan {
                            reading: *index,
                            current: GridRef {
                                request: current,
                                back: 0,
                            },
                            previous: GridRef {
                                request: previous,
                                back: 0,
                            },
                        }));
                    }
                }
            }
        }

        plan
    }

    /// Which query position inside `request` answers `reading`, if it is in that request at all.
    pub fn position_of(&self, request: usize, reading: usize) -> Option<usize> {
        self.positions
            .get(request)?
            .iter()
            .position(|candidate| *candidate == reading)
    }

    fn push(&mut self, catalogue: &Catalogue, range: TimeRange, readings: &[usize]) -> usize {
        let queries = readings
            .iter()
            .filter_map(|index| catalogue.readings.get(*index))
            .map(Reading::query)
            .collect::<Vec<_>>();

        let index = self.requests.len();
        self.requests.push(MetricRequest { range, queries });
        self.positions.push(readings.to_vec());
        index
    }
}

/// The most queries one `GetMetricData` call takes. The provider enforces the same number; naming
/// it here is what keeps a grown catalogue from ever reaching that check.
const MAX_QUERIES_PER_REQUEST: usize = 500;

fn slot_count(slots: usize) -> u32 {
    u32::try_from(slots).unwrap_or(u32::MAX)
}

fn aggregation_of(statistic: cloudwatch::Statistic) -> Aggregation {
    match statistic {
        cloudwatch::Statistic::Average => Aggregation::Average,
        cloudwatch::Statistic::Maximum => Aggregation::Maximum,
        cloudwatch::Statistic::Minimum => Aggregation::Minimum,
        cloudwatch::Statistic::Sum => Aggregation::Sum,
    }
}

fn operator_of(operator: cloudwatch::ComparisonOperator) -> ComparisonOperator {
    match operator {
        cloudwatch::ComparisonOperator::GreaterThanThreshold => {
            ComparisonOperator::GreaterThanThreshold
        }
        cloudwatch::ComparisonOperator::GreaterThanOrEqualToThreshold => {
            ComparisonOperator::GreaterThanOrEqualToThreshold
        }
        cloudwatch::ComparisonOperator::LessThanThreshold => ComparisonOperator::LessThanThreshold,
        cloudwatch::ComparisonOperator::LessThanOrEqualToThreshold => {
            ComparisonOperator::LessThanOrEqualToThreshold
        }
    }
}

fn policy_of(
    treatment: cloudwatch::TreatMissingData,
    alarm: &str,
    severity: &str,
) -> Result<MissingDataPolicy, ConfigurationError> {
    match treatment {
        cloudwatch::TreatMissingData::Breaching => Ok(MissingDataPolicy::Breaching),
        cloudwatch::TreatMissingData::NotBreaching => Ok(MissingDataPolicy::NotBreaching),
        cloudwatch::TreatMissingData::Missing => Ok(MissingDataPolicy::Missing),
        cloudwatch::TreatMissingData::Ignore => {
            Err(ConfigurationError::ConfigParsingError(format!(
                "alarm `{alarm}` severity `{severity}` asks for treat_missing_data `ignore`, \
                 which this evaluator cannot reproduce without a memory of the previous state"
            )))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn settings(alarms: serde_json::Value) -> CloudWatchSettings {
        serde_json::from_value(serde_json::json!({
            "region": "ap-south-1",
            "failure_destination": "smoke",
            "severity_destinations": { "sev1": "smoke", "sev2": "smoke", "sev3": "smoke" },
            "alarms": alarms,
        }))
        .unwrap()
    }

    fn definition(
        metric: &str,
        instance: &str,
        period: u32,
        severities: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "classification": "rds-alerts",
            "metric_name": metric,
            "namespace": "AWS/RDS",
            "dimensions": [{ "name": "DBInstanceIdentifier", "value": instance }],
            "period": period,
            "statistic": "Average",
            "severities": severities,
        })
    }

    fn severity(threshold: f64, evaluation_periods: u32) -> serde_json::Value {
        serde_json::json!({
            "threshold": threshold,
            "description": "d",
            "evaluation_periods": evaluation_periods,
        })
    }

    /// The inversion this module exists for: severities differ only by threshold, and a threshold
    /// is not part of a CloudWatch query.
    #[test]
    fn severities_of_one_definition_share_a_single_reading() {
        let catalogue = Catalogue::resolve(&settings(serde_json::json!({
            "cpu": definition("CPUUtilization", "rds-primary", 60, serde_json::json!({
                "sev1": severity(90.0, 1),
                "sev2": severity(85.0, 1),
                "sev3": severity(65.0, 1),
            })),
        })))
        .unwrap();

        assert_eq!(catalogue.readings.len(), 1);
        assert_eq!(catalogue.targets.len(), 3);
        assert!(catalogue.targets.iter().all(|target| target.reading == 0));
    }

    /// The same metric on two instances is two readings, because the dimensions identify it.
    #[test]
    fn the_same_metric_on_different_dimensions_is_a_second_reading() {
        let catalogue = Catalogue::resolve(&settings(serde_json::json!({
            "primary": definition("CPUUtilization", "rds-primary", 60, serde_json::json!({
                "sev1": severity(90.0, 1),
            })),
            "failover": definition("CPUUtilization", "rds-failover", 60, serde_json::json!({
                "sev1": severity(90.0, 1),
            })),
        })))
        .unwrap();

        assert_eq!(catalogue.readings.len(), 2);
    }

    /// A 60-second metric slides by exactly one slot, so one grid holds both evaluations — the
    /// `N + 1` fetch, and the only case where it is correct.
    #[test]
    fn a_one_minute_metric_needs_one_request_for_both_windows() {
        let catalogue = Catalogue::resolve(&settings(serde_json::json!({
            "cpu": definition("CPUUtilization", "rds-primary", 60, serde_json::json!({
                "sev1": severity(90.0, 3),
            })),
        })))
        .unwrap();

        let plan = FetchPlan::build(&catalogue, datetime!(2026-09-10 12:00:00));

        assert_eq!(plan.requests.len(), 1);
        // Evaluation range 3 + 2 lookback, plus the one slot the window slides by.
        assert_eq!(
            plan.requests[0].range.start,
            datetime!(2026-09-10 11:54:00 UTC)
        );
        assert_eq!(
            plan.readings[0].current,
            GridRef {
                request: 0,
                back: 0
            }
        );
        assert_eq!(
            plan.readings[0].previous,
            GridRef {
                request: 0,
                back: 1
            }
        );
    }

    /// A 300-second metric does not. Its two windows sit on grids a minute apart, and one
    /// response cannot represent both — reading one grid twice would compare windows a whole
    /// period apart, which is not what CloudWatch compares.
    #[test]
    fn a_five_minute_metric_needs_two_differently_aligned_requests() {
        let catalogue = Catalogue::resolve(&settings(serde_json::json!({
            "iops": definition("ReadIOPS", "rds-primary", 300, serde_json::json!({
                "sev1": severity(18000.0, 3),
            })),
        })))
        .unwrap();

        let plan = FetchPlan::build(&catalogue, datetime!(2026-09-10 12:00:00));

        assert_eq!(plan.requests.len(), 2);
        assert_eq!(
            plan.requests[0].range.end,
            datetime!(2026-09-10 12:00:00 UTC)
        );
        // Exactly one minute earlier, not one period earlier.
        assert_eq!(
            plan.requests[1].range.end,
            datetime!(2026-09-10 11:59:00 UTC)
        );
        assert_eq!(
            plan.requests[0].range.start,
            datetime!(2026-09-10 11:35:00 UTC)
        );
        assert_eq!(
            plan.readings[0].current,
            GridRef {
                request: 0,
                back: 0
            }
        );
        assert_eq!(
            plan.readings[0].previous,
            GridRef {
                request: 1,
                back: 0
            }
        );
    }

    /// Readings of one period travel in one request, and the request is sized by the widest window
    /// any of them needs.
    #[test]
    fn one_request_per_period_carries_every_reading_of_that_period() {
        let catalogue = Catalogue::resolve(&settings(serde_json::json!({
            "cpu": definition("CPUUtilization", "rds-primary", 60, serde_json::json!({
                "sev1": severity(90.0, 1),
            })),
            "memory": definition("FreeableMemory", "rds-failover", 60, serde_json::json!({
                "sev1": severity(1.0, 5),
            })),
            "iops": definition("ReadIOPS", "rds-primary", 300, serde_json::json!({
                "sev1": severity(18000.0, 3),
            })),
        })))
        .unwrap();

        let plan = FetchPlan::build(&catalogue, datetime!(2026-09-10 12:00:00));

        // One for the minute metrics, two for the five-minute one.
        assert_eq!(plan.requests.len(), 3);
        assert_eq!(plan.requests[0].queries.len(), 2);
        // Sized by the widest: 5 evaluation periods + 2 lookback + 1 slide = 8 minutes.
        assert_eq!(
            plan.requests[0].range.start,
            datetime!(2026-09-10 11:52:00 UTC)
        );
    }

    #[test]
    fn an_empty_catalogue_plans_nothing() {
        let catalogue = Catalogue::resolve(&CloudWatchSettings::default()).unwrap();

        assert!(catalogue.is_empty());
        assert!(FetchPlan::build(&catalogue, datetime!(2026-09-10 12:00:00))
            .requests
            .is_empty());
    }

    #[test]
    fn a_target_carries_the_destination_its_severity_maps_to() {
        let catalogue = Catalogue::resolve(&settings(serde_json::json!({
            "cpu": definition("CPUUtilization", "rds-primary", 60, serde_json::json!({
                "sev2": severity(85.0, 1),
            })),
        })))
        .unwrap();

        assert_eq!(catalogue.targets[0].destination, "smoke");
        assert_eq!(catalogue.targets[0].name(), "cpu/sev2");
    }
}
