//! What to ask a metrics provider for.
//!
//! These are plain data. They carry no provider's rules about which periods are legal or how many
//! queries fit in a batch, because those differ per provider and belong where they are enforced —
//! see [`MetricsError::InvalidRequest`](super::MetricsError::InvalidRequest).

use std::collections::BTreeMap;

/// How the raw values inside one period are reduced to a single number.
///
/// The four the alarm catalogue uses. Percentiles are a quantile over the distribution rather than
/// a reduction of it, and every provider expresses them separately, so they would need their own
/// modelling.
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
/// Any value can be constructed; which ones a provider accepts is the provider's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Period(i32);

impl Period {
    /// One minute, the period most of the alarm catalogue uses.
    pub const ONE_MINUTE: Self = Self(60);
    /// Five minutes.
    pub const FIVE_MINUTES: Self = Self(300);
    /// One hour.
    pub const ONE_HOUR: Self = Self(3600);

    /// A period of `seconds`.
    pub fn from_seconds(seconds: i32) -> Self {
        Self(seconds)
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
/// Ordered rather than hashed, so a request built two different ways serialises identically.
///
/// An empty set is **not** a request to aggregate across streams. What it selects is a provider's
/// own business — on CloudWatch it names the stream published with no dimensions at all, which is
/// a different metric from the same name published per instance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels(BTreeMap<String, String>);

impl Labels {
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
    /// The grouping a provider files the metric under, where it has one.
    pub namespace: Option<String>,
    /// The metric's name.
    pub name: String,
    /// The labels narrowing it to one reporting stream.
    pub labels: Labels,
    /// How long each datapoint covers.
    pub period: Period,
    /// How the values inside each period are reduced.
    pub aggregation: Aggregation,
}

/// The half-open window `[start, end)` a request covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// The inclusive start of the window.
    pub start: time::OffsetDateTime,
    /// The exclusive end of the window.
    pub end: time::OffsetDateTime,
}

impl TimeRange {
    /// The `count` periods immediately before `end`.
    ///
    /// The shape alarm evaluation asks for. Asking for one more period than an alarm's evaluation
    /// window returns the window before it in the same call, which is how a state transition is
    /// detected without remembering anything between ticks.
    pub fn ending_at(end: time::OffsetDateTime, period: Period, count: u32) -> Self {
        let span = period.seconds_i64().saturating_mul(i64::from(count));

        Self {
            start: end - time::Duration::seconds(span),
            end,
        }
    }
}

/// A batch of metrics to read over one window.
///
/// One range covers every query, because that is what the providers behind this interface accept.
/// Periods stay per-query, so one call can mix a metric sampled every minute with one sampled
/// hourly — which the alarm catalogue needs, since a tick reads metrics of both kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricRequest {
    /// The window every query covers.
    pub range: TimeRange,
    /// The metrics to read. Results come back keyed by each query's position here.
    pub queries: Vec<MetricQuery>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn a_range_ending_at_covers_the_periods_before_its_end() {
        let range =
            TimeRange::ending_at(datetime!(2026-09-09 12:00:00 UTC), Period::FIVE_MINUTES, 3);

        assert_eq!(range.start, datetime!(2026-09-09 11:45:00 UTC));
        assert_eq!(range.end, datetime!(2026-09-09 12:00:00 UTC));
    }

    #[test]
    fn labels_keep_a_stable_order_however_they_were_built() {
        let one: Labels = [
            ("TargetGroup", "targetgroup/envoy-tg/1"),
            ("LoadBalancer", "app/sbx-hyp-envoy-alb/1"),
        ]
        .into_iter()
        .collect();
        let other: Labels = [
            ("LoadBalancer", "app/sbx-hyp-envoy-alb/1"),
            ("TargetGroup", "targetgroup/envoy-tg/1"),
        ]
        .into_iter()
        .collect();

        assert_eq!(one, other);
        assert_eq!(
            one.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            vec!["LoadBalancer", "TargetGroup"]
        );
    }
}
