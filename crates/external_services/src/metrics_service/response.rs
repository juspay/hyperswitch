//! What a metrics provider answers with.
//!
//! Providers report a period with no data by *omitting* it, so a type that flattened a response
//! into `Vec<f64>` would turn a metric that stopped reporting into a run of zeroes — and a metric
//! that stopped reporting is what an infrastructure alarm exists to catch. Datapoints therefore
//! keep their timestamps and [`MetricSeries::window`] hands back `Option<f64>` per period.

use super::Period;

/// A provider's opaque marker for "there is more after this page".
///
/// Not a `String`: what a provider puts in here is its own business, and a caller that could read
/// it would end up depending on the shape.
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
    /// The start of the period this value aggregates.
    pub timestamp: time::OffsetDateTime,
    /// The aggregated value.
    pub value: f64,
}

/// How much of what was asked for a series actually contains.
///
/// A failed series arrives as data rather than as an error, so one metric the credentials cannot
/// read does not blind the caller to every other metric in the same batch.
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

    /// Which query in [`MetricRequest::queries`](super::MetricRequest::queries) this answers.
    pub fn index(&self) -> usize {
        self.index
    }

    /// How complete this series is.
    pub fn status(&self) -> SeriesStatus {
        self.status
    }

    /// The datapoints, oldest first.
    ///
    /// **Periods with no datapoint are absent, not zero**, so a series with a gap is shorter than
    /// the window that produced it. [`Self::window`] shows where the gaps fall.
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
    /// periods is where an `unwrap_or(0.0)` gets typed by accident.
    ///
    /// Buckets cover `[end - count * period, end)`, and a datapoint falls in the bucket containing
    /// its timestamp, so exact grid alignment is not assumed. If two share a bucket, the later
    /// wins. A period of zero or less has no grid, so every bucket comes back `None`.
    pub fn window(
        &self,
        end: time::OffsetDateTime,
        period: Period,
        count: usize,
    ) -> Vec<Option<f64>> {
        let mut buckets = vec![None; count];

        let period_seconds = period.seconds_i64();
        let Ok(count) = i64::try_from(count) else {
            return buckets;
        };
        if period_seconds <= 0 {
            return buckets;
        }

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
    /// `None` means the provider said nothing about that query *on this page*, which differs from
    /// a series that came back empty.
    pub fn series_at(&self, index: usize) -> Option<&MetricSeries> {
        self.series.iter().find(|series| series.index() == index)
    }

    /// The marker for the next page, or `None` if this is the last.
    pub fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn series_of(datapoints: Vec<Datapoint>) -> MetricSeries {
        MetricSeries::new(0, datapoints, SeriesStatus::Complete)
    }

    fn at(timestamp: time::OffsetDateTime, value: f64) -> Datapoint {
        Datapoint { timestamp, value }
    }

    #[test]
    fn a_missing_datapoint_is_none_and_a_zero_datapoint_is_some_zero() {
        // 12:02 is missing from the response; 12:01 really did aggregate to 0.
        let series = series_of(vec![
            at(datetime!(2026-09-09 12:00:00 UTC), 4.0),
            at(datetime!(2026-09-09 12:01:00 UTC), 0.0),
            at(datetime!(2026-09-09 12:03:00 UTC), 7.0),
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
            at(datetime!(2026-09-09 11:58:00 UTC), 1.0),
            at(datetime!(2026-09-09 12:01:00 UTC), 2.0),
            // At the exclusive end, so outside.
            at(datetime!(2026-09-09 12:02:00 UTC), 3.0),
        ]);

        // Buckets are [12:00, 12:01) and [12:01, 12:02), so 12:01 lands in the second.
        let window = series.window(datetime!(2026-09-09 12:02:00 UTC), Period::ONE_MINUTE, 2);

        assert_eq!(window, vec![None, Some(2.0)]);
    }

    #[test]
    fn a_window_lands_an_unaligned_datapoint_in_the_period_containing_it() {
        let series = series_of(vec![at(datetime!(2026-09-09 12:00:37 UTC), 9.0)]);

        let window = series.window(datetime!(2026-09-09 12:02:00 UTC), Period::ONE_MINUTE, 2);

        assert_eq!(window, vec![Some(9.0), None]);
    }

    #[test]
    fn a_period_with_no_grid_yields_gaps_rather_than_dividing_by_zero() {
        let series = series_of(vec![at(datetime!(2026-09-09 12:00:00 UTC), 1.0)]);

        let window = series.window(
            datetime!(2026-09-09 12:02:00 UTC),
            Period::from_seconds(0),
            2,
        );

        assert_eq!(window, vec![None, None]);
    }

    #[test]
    fn a_series_never_returned_is_distinguishable_from_one_that_came_back_empty() {
        let page = MetricPage::new(vec![series_of(Vec::new())], None);

        assert!(page.series_at(0).is_some_and(|s| s.datapoints().is_empty()));
        assert!(page.series_at(1).is_none());
    }
}
