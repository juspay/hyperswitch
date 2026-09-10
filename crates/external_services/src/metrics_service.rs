//! Reading metric datapoints from a monitoring provider.
//!
//! A [`MetricsProvider`] answers one question: *what values did these metrics take over this
//! window?* It holds no thresholds, no severities and no notion that a number is "bad" — deciding
//! what a value means belongs to whatever reads it, which is what lets the same provider serve an
//! alarm evaluator, a dashboard and a one-off investigation without any of them knowing about the
//! others.
//!
//! # The vocabulary is deliberately not any one vendor's
//!
//! Amazon calls the key/value selectors on a metric *dimensions*, its aggregations *statistics*,
//! and groups metrics into *namespaces*; Google and Prometheus both say *labels*, and Prometheus
//! has no namespace at all. The names here are the ones that survive translation: [`Labels`],
//! [`Aggregation`], and a [`namespace`](MetricQuery::namespace) that is optional because not every
//! provider has one.
//!
//! The cost of that choice is real and worth stating: a trait is the *intersection* of what its
//! implementations can do. Where one provider is more capable than another — several time ranges
//! in one request, say — this interface offers the narrower of the two. Provider-specific
//! capabilities belong as inherent methods on the concrete provider rather than smuggled into the
//! trait as an opaque string, which would claim a portability the type does not have.
//!
//! # Gaps are not zeroes
//!
//! The load-bearing property of the types here is that a [`MetricSeries`] can tell "no datapoint"
//! from "a datapoint whose value is 0". Providers report a period with no data by *omitting* it,
//! so a type that flattened a response into `Vec<f64>` would quietly turn a metric that stopped
//! reporting into a run of zeroes — and a metric that stopped reporting is exactly the failure an
//! infrastructure alarm exists to catch. Datapoints therefore keep their timestamps, nothing is
//! defaulted or filled, and [`MetricSeries::window`] hands back `Option<f64>` per period so a
//! caller has to decide, in the open, what a missing datapoint means.
//!
//! # Pagination is the caller's
//!
//! [`MetricsProvider::fetch`] returns one [`MetricPage`]. If the provider had more to say the page
//! carries a [`Cursor`], and the caller feeds it back to get the next one. Nothing loops on the
//! caller's behalf, so nothing accumulates an unbounded response in memory without the caller
//! having asked for it page by page.
//!
//! One consequence to know about: **a single series can span pages**, so a caller that does
//! paginate must concatenate the datapoints it finds under the same
//! [`index`](MetricSeries::index) across pages. In practice one page is enough — what fills a page
//! is the number of datapoints, not the elapsed time, so even a request covering several days at a
//! daily period is a handful of values.

/// Reading metrics from Amazon CloudWatch.
pub mod aws_cloudwatch;

use std::collections::BTreeMap;

use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};

/// Result type for metric operations.
pub type MetricsResult<T> = CustomResult<T, MetricsError>;

/// Reads metric datapoints from a monitoring provider.
///
/// Object-safe on purpose, so a caller that resolves its provider from configuration can hold an
/// `Arc<dyn MetricsProvider>`. Resist adding an associated type: it is what stops
/// [`crate::email::EmailClient`] being usable as a trait object.
#[async_trait::async_trait]
pub trait MetricsProvider: Send + Sync + std::fmt::Debug {
    /// Fetch one page of datapoints for `request`.
    ///
    /// Pass `None` as `cursor` for the first page, then the previous page's
    /// [`MetricPage::cursor`] for each page after it. A page whose cursor is `None` is the last.
    ///
    /// The same `request` must be passed for every page of one traversal; providers derive the
    /// continuation from both the cursor and the request that produced it.
    async fn fetch(
        &self,
        request: &MetricRequest,
        cursor: Option<&Cursor>,
    ) -> MetricsResult<MetricPage>;
}

/// How the raw values inside one period are reduced to a single number.
///
/// The four the alarm catalogue actually uses. Percentiles are absent deliberately rather than by
/// oversight: they are a different concept — a quantile over the distribution rather than a
/// reduction of it — and every provider expresses them through a separate mechanism, so they earn
/// their own modelling if and when something needs one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aggregation {
    /// Arithmetic mean of the values in the period.
    Average,
    /// Largest value in the period.
    Maximum,
    /// Smallest value in the period.
    Minimum,
    /// Every value in the period added together.
    Sum,
}

/// How long a single datapoint covers, in seconds.
///
/// Validated at construction because the alternative is a request the provider rejects as a whole:
/// one malformed period can take down every metric sharing the call, which is a poor way to find
/// out about a typo in configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Period(i32);

impl Period {
    /// One minute, the period most of the alarm catalogue uses.
    pub const ONE_MINUTE: Self = Self(60);
    /// Five minutes.
    pub const FIVE_MINUTES: Self = Self(300);
    /// One hour.
    pub const ONE_HOUR: Self = Self(3600);

    /// Build a period from a number of seconds.
    ///
    /// Sub-minute periods are accepted only at the resolutions high-resolution metrics support
    /// (1, 5, 10, 20 and 30 seconds); anything else must be a positive multiple of 60.
    pub fn from_seconds(seconds: i32) -> MetricsResult<Self> {
        let valid = matches!(seconds, 1 | 5 | 10 | 20 | 30) || (seconds > 0 && seconds % 60 == 0);

        if valid {
            Ok(Self(seconds))
        } else {
            Err(report!(MetricsError::InvalidRequest))
                .attach_printable_lazy(|| format!("{seconds} is not a usable metric period"))
        }
    }

    /// The period in seconds.
    pub fn seconds(self) -> i32 {
        self.0
    }

    /// The period in seconds, widened for timestamp arithmetic.
    pub fn seconds_i64(self) -> i64 {
        i64::from(self.0)
    }
}

/// The key/value selectors narrowing a metric to one reporting stream.
///
/// Ordered rather than hashed, so a request built two different ways compares and serialises
/// identically — which is what makes the translation to a provider's wire format testable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels(BTreeMap<String, String>);

impl Labels {
    /// No labels — the metric is read across every stream reporting it.
    pub fn none() -> Self {
        Self::default()
    }

    /// Add one label, replacing any previous value under the same key.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(key.into(), value.into());
        self
    }

    /// The labels, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Whether any label is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K, V> FromIterator<(K, V)> for Labels
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }
}

/// One metric to read, and how to reduce it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricQuery {
    namespace: Option<String>,
    name: String,
    labels: Labels,
    period: Period,
    aggregation: Aggregation,
}

impl MetricQuery {
    /// A metric read at `period`, reduced by `aggregation`.
    pub fn new(name: impl Into<String>, period: Period, aggregation: Aggregation) -> Self {
        Self {
            namespace: None,
            name: name.into(),
            labels: Labels::none(),
            period,
            aggregation,
        }
    }

    /// Place the metric in a namespace.
    ///
    /// Required by providers that group metrics that way, ignored by those that do not.
    #[must_use]
    pub fn in_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Narrow the metric to one reporting stream.
    #[must_use]
    pub fn with_labels(mut self, labels: Labels) -> Self {
        self.labels = labels;
        self
    }

    /// The namespace, if the caller set one.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// The metric's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The labels narrowing this metric.
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    /// How long each datapoint covers.
    pub fn period(&self) -> Period {
        self.period
    }

    /// How the values inside each period are reduced.
    pub fn aggregation(&self) -> Aggregation {
        self.aggregation
    }
}

/// The half-open window `[start, end)` a request covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    start: time::OffsetDateTime,
    end: time::OffsetDateTime,
}

impl TimeRange {
    /// Build a range, rejecting one that does not move forwards.
    pub fn new(start: time::OffsetDateTime, end: time::OffsetDateTime) -> MetricsResult<Self> {
        if start < end {
            Ok(Self { start, end })
        } else {
            Err(report!(MetricsError::InvalidRequest))
                .attach_printable("A metric time range must start before it ends")
        }
    }

    /// The `count` periods immediately before `end`.
    ///
    /// The shape alarm evaluation asks for — "the last N periods, as of now". Asking for one more
    /// period than an alarm's evaluation window returns the window before it in the same call,
    /// which is how a state *transition* is detected without remembering anything between ticks.
    pub fn ending_at(end: time::OffsetDateTime, period: Period, count: u32) -> MetricsResult<Self> {
        let span = period
            .seconds_i64()
            .checked_mul(i64::from(count))
            .ok_or_else(|| report!(MetricsError::InvalidRequest))
            .attach_printable("The requested window is too wide to represent")?;

        Self::new(end - time::Duration::seconds(span), end)
    }

    /// The inclusive start of the window.
    pub fn start(&self) -> time::OffsetDateTime {
        self.start
    }

    /// The exclusive end of the window.
    pub fn end(&self) -> time::OffsetDateTime {
        self.end
    }
}

/// A batch of metrics to read over one window.
///
/// One range covers every query, because that is what the providers behind this interface accept.
/// Periods stay per-query, so a single call can mix a metric sampled every minute with one sampled
/// hourly — which is what the alarm catalogue needs, since a tick reads metrics of both kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricRequest {
    range: TimeRange,
    queries: Vec<MetricQuery>,
}

impl MetricRequest {
    /// Build a request, rejecting one with nothing to fetch.
    ///
    /// An empty batch is refused here rather than sent, so a caller that filtered its metric list
    /// down to nothing finds out at the call site instead of paying for a round trip that can only
    /// come back empty.
    pub fn new(range: TimeRange, queries: Vec<MetricQuery>) -> MetricsResult<Self> {
        if queries.is_empty() {
            return Err(report!(MetricsError::InvalidRequest))
                .attach_printable("A metric request needs at least one query");
        }

        Ok(Self { range, queries })
    }

    /// The window every query in this request covers.
    pub fn range(&self) -> TimeRange {
        self.range
    }

    /// The metrics to read, in the order results are reported under.
    pub fn queries(&self) -> &[MetricQuery] {
        &self.queries
    }
}

/// A provider's opaque marker for "there is more after this page".
///
/// Deliberately not a `String`: what a provider puts in here is its own business, and a caller
/// that could read it would end up depending on the shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(String);

impl Cursor {
    /// Wrap a provider's continuation marker.
    pub(crate) fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The marker, for the provider that minted it.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A single aggregated value and the period it covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Datapoint {
    timestamp: time::OffsetDateTime,
    value: f64,
}

impl Datapoint {
    /// Build a datapoint.
    pub fn new(timestamp: time::OffsetDateTime, value: f64) -> Self {
        Self { timestamp, value }
    }

    /// The start of the period this value aggregates.
    pub fn timestamp(&self) -> time::OffsetDateTime {
        self.timestamp
    }

    /// The aggregated value.
    pub fn value(&self) -> f64 {
        self.value
    }
}

/// How much of what was asked for a series actually contains.
///
/// A series that failed arrives as data rather than as an error, so one metric the credentials
/// cannot read — or one the provider stumbled on — does not blind the caller to every other metric
/// in the same batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesStatus {
    /// Everything the provider had for the requested window is here.
    Complete,
    /// Some of the window is here; the rest was not returned.
    Partial,
    /// The provider could not produce this series. Any datapoints it carries are incomplete.
    Failed,
}

/// The datapoints one query produced.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricSeries {
    index: usize,
    datapoints: Vec<Datapoint>,
    status: SeriesStatus,
}

impl MetricSeries {
    /// Build a series, for providers translating a response.
    pub(crate) fn new(index: usize, datapoints: Vec<Datapoint>, status: SeriesStatus) -> Self {
        Self {
            index,
            datapoints,
            status,
        }
    }

    /// Which query in [`MetricRequest::queries`] this series answers.
    pub fn index(&self) -> usize {
        self.index
    }

    /// How complete this series is.
    pub fn status(&self) -> SeriesStatus {
        self.status
    }

    /// The datapoints, oldest first.
    ///
    /// **Periods with no datapoint are absent, not zero.** A series with a gap is shorter than the
    /// window that produced it; [`Self::window`] is the way to see where the gaps fall.
    pub fn datapoints(&self) -> &[Datapoint] {
        &self.datapoints
    }

    /// The most recent datapoint, if any.
    pub fn latest(&self) -> Option<&Datapoint> {
        self.datapoints.last()
    }

    /// The series laid out on a period grid, oldest first, `None` where there is no datapoint.
    ///
    /// This lives here rather than in each caller because laying sparse datapoints back onto their
    /// periods is precisely the step where an `unwrap_or(0.0)` gets typed by accident, and a
    /// missing datapoint is emphatically not a zero.
    ///
    /// Buckets cover `[end - count * period, end)` and a datapoint falls in the bucket containing
    /// its timestamp, so exact grid alignment on the provider's side is not assumed. If two
    /// datapoints share a bucket — which means `period` is wider than the one the query asked for
    /// — the later one wins.
    pub fn window(
        &self,
        end: time::OffsetDateTime,
        period: Period,
        count: usize,
    ) -> Vec<Option<f64>> {
        let mut buckets = vec![None; count];

        let Ok(count) = i64::try_from(count) else {
            return buckets;
        };
        let period_seconds = period.seconds_i64();
        let start = end.unix_timestamp() - count * period_seconds;

        for datapoint in &self.datapoints {
            let offset = datapoint.timestamp.unix_timestamp() - start;
            if offset < 0 || offset / period_seconds >= count {
                continue;
            }

            if let Ok(index) = usize::try_from(offset / period_seconds) {
                if let Some(bucket) = buckets.get_mut(index) {
                    *bucket = Some(datapoint.value);
                }
            }
        }

        buckets
    }
}

/// One page of a provider's answer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetricPage {
    series: Vec<MetricSeries>,
    cursor: Option<Cursor>,
}

impl MetricPage {
    /// Build a page, for providers translating a response.
    pub(crate) fn new(series: Vec<MetricSeries>, cursor: Option<Cursor>) -> Self {
        Self { series, cursor }
    }

    /// Every series on this page.
    pub fn series(&self) -> &[MetricSeries] {
        &self.series
    }

    /// Take ownership of this page's series.
    pub fn into_series(self) -> Vec<MetricSeries> {
        self.series
    }

    /// The series answering the query at `index`, if this page carries one.
    ///
    /// `None` means the provider said nothing about that query *on this page* — which is different
    /// from a series that came back empty, and different again from one full of zeroes.
    pub fn series_at(&self, index: usize) -> Option<&MetricSeries> {
        self.series.iter().find(|series| series.index() == index)
    }

    /// The marker for the next page, or `None` if this is the last.
    pub fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }
}

/// Errors raised while reading metrics.
///
/// Kept to what happened to the request, rather than to what any particular caller should do
/// about it.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// The configuration could not be turned into a usable provider.
    #[error("Invalid metrics provider configuration: {0}")]
    Configuration(&'static str),

    /// The request was refused before it was sent.
    #[error("The metric request is not one the provider can be asked")]
    InvalidRequest,

    /// The call did not produce a usable response — credentials, network, throttling, or a
    /// request the provider rejected.
    #[error("The call to the metrics provider failed")]
    Transport,

    /// A response arrived but could not be read as a set of series.
    #[error("Could not interpret the metrics provider's response")]
    MalformedResponse,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn series_of(datapoints: Vec<Datapoint>) -> MetricSeries {
        MetricSeries::new(0, datapoints, SeriesStatus::Complete)
    }

    #[test]
    fn a_period_must_be_one_a_provider_can_aggregate_on() {
        for seconds in [1, 5, 10, 20, 30, 60, 300, 3600, 86400] {
            assert_eq!(
                Period::from_seconds(seconds).unwrap().seconds(),
                seconds,
                "{seconds} should be accepted"
            );
        }

        for seconds in [0, -60, 2, 45, 61, 359] {
            assert!(
                Period::from_seconds(seconds).is_err(),
                "{seconds} should be rejected"
            );
        }
    }

    #[test]
    fn a_missing_datapoint_is_none_and_a_zero_datapoint_is_some_zero() {
        // A provider reports a gap by omitting the period entirely. 12:02 is missing; 12:01
        // really did aggregate to 0.
        let series = series_of(vec![
            Datapoint::new(datetime!(2026-09-09 12:00:00 UTC), 4.0),
            Datapoint::new(datetime!(2026-09-09 12:01:00 UTC), 0.0),
            Datapoint::new(datetime!(2026-09-09 12:03:00 UTC), 7.0),
        ]);

        let window = series.window(datetime!(2026-09-09 12:04:00 UTC), Period::ONE_MINUTE, 4);

        assert_eq!(window, vec![Some(4.0), Some(0.0), None, Some(7.0)]);
    }

    #[test]
    fn an_empty_series_is_all_gaps_rather_than_all_zeroes() {
        let window =
            series_of(Vec::new()).window(datetime!(2026-09-09 12:05:00 UTC), Period::ONE_MINUTE, 3);

        assert_eq!(window, vec![None, None, None]);
    }

    #[test]
    fn a_window_ignores_datapoints_outside_it() {
        let series = series_of(vec![
            // Before the window starts.
            Datapoint::new(datetime!(2026-09-09 11:58:00 UTC), 1.0),
            Datapoint::new(datetime!(2026-09-09 12:01:00 UTC), 2.0),
            // At the exclusive end, so outside.
            Datapoint::new(datetime!(2026-09-09 12:02:00 UTC), 3.0),
        ]);

        // Two buckets ending at 12:02, so [12:00, 12:01) and [12:01, 12:02): the 12:01 datapoint
        // belongs to the second, and the two outside the window are dropped rather than clamped
        // into the nearest bucket.
        let window = series.window(datetime!(2026-09-09 12:02:00 UTC), Period::ONE_MINUTE, 2);

        assert_eq!(window, vec![None, Some(2.0)]);
    }

    #[test]
    fn a_window_lands_an_unaligned_datapoint_in_the_period_containing_it() {
        let series = series_of(vec![Datapoint::new(
            datetime!(2026-09-09 12:00:37 UTC),
            9.0,
        )]);

        let window = series.window(datetime!(2026-09-09 12:02:00 UTC), Period::ONE_MINUTE, 2);

        assert_eq!(window, vec![Some(9.0), None]);
    }

    #[test]
    fn a_time_range_covers_the_periods_before_its_end() {
        let range =
            TimeRange::ending_at(datetime!(2026-09-09 12:00:00 UTC), Period::FIVE_MINUTES, 3)
                .unwrap();

        assert_eq!(range.start(), datetime!(2026-09-09 11:45:00 UTC));
        assert_eq!(range.end(), datetime!(2026-09-09 12:00:00 UTC));
    }

    #[test]
    fn a_time_range_must_move_forwards() {
        let instant = datetime!(2026-09-09 12:00:00 UTC);

        assert!(TimeRange::new(instant, instant).is_err());
        assert!(TimeRange::new(instant, instant - time::Duration::seconds(1)).is_err());
        assert!(TimeRange::ending_at(instant, Period::ONE_MINUTE, 0).is_err());
    }

    #[test]
    fn labels_keep_a_stable_order_however_they_were_built() {
        let built = Labels::none()
            .with("TargetGroup", "targetgroup/envoy-tg/1")
            .with("LoadBalancer", "app/sbx-hyp-envoy-alb/1");
        let collected: Labels = [
            ("LoadBalancer", "app/sbx-hyp-envoy-alb/1"),
            ("TargetGroup", "targetgroup/envoy-tg/1"),
        ]
        .into_iter()
        .collect();

        assert_eq!(built, collected);
        assert_eq!(
            built.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            vec!["LoadBalancer", "TargetGroup"]
        );
    }

    #[test]
    fn a_request_needs_something_to_fetch() {
        let range = TimeRange::ending_at(datetime!(2026-09-09 12:00:00 UTC), Period::ONE_MINUTE, 2)
            .unwrap();

        assert!(MetricRequest::new(range, Vec::new()).is_err());
        assert!(MetricRequest::new(
            range,
            vec![MetricQuery::new(
                "CPUUtilization",
                Period::ONE_MINUTE,
                Aggregation::Average
            )],
        )
        .is_ok());
    }

    #[test]
    fn a_series_never_returned_is_distinguishable_from_one_that_came_back_empty() {
        let page = MetricPage::new(vec![series_of(Vec::new())], None);

        assert!(page.series_at(0).is_some_and(|s| s.datapoints().is_empty()));
        assert!(page.series_at(1).is_none());
    }
}
