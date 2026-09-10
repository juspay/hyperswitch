//! What a metrics provider answers with.
//!
//! Providers report a period with no data by *omitting* it, and they say so only in the timestamps
//! — the values alone are a run of readings with no hole visible in them. A response flattened to
//! `Vec<f64>` would therefore turn a metric that stopped reporting into a metric that was fine,
//! which is the failure an infrastructure alarm exists to catch.
//!
//! So a series arrives already laid out on its period grid: one [`Option<f64>`] per period across
//! the window that was asked for, `None` where there was no datapoint. Providers do that placement
//! themselves, using the range and period from the request rather than asking the caller to repeat
//! them.
//!
//! The constructors are public rather than crate-private, because
//! [`MetricsProvider`](super::MetricsProvider) is object-safe and provider-neutral on purpose — a
//! trait no other crate can produce a value for is one no other crate can implement, which would
//! quietly make "provider-neutral" mean "the providers in this module". It is also what lets a
//! caller stand a stub provider up in its own tests instead of reaching for a live account.

/// A provider's opaque marker for "there is more after this page".
///
/// Not a `String`: what a provider puts in here is its own business, and a caller that could read
/// it would end up depending on the shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(String);

impl Cursor {
    /// Wrap a provider's continuation marker.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The marker, for the provider that minted it.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
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
    /// The provider could not produce this series. Any values it carries are incomplete.
    Failed,
}

/// The values one query produced, one per period of the window it asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricSeries {
    index: usize,
    status: SeriesStatus,
    values: Vec<Option<f64>>,
}

impl MetricSeries {
    /// Build a series, for providers translating a response.
    pub fn new(index: usize, status: SeriesStatus, values: Vec<Option<f64>>) -> Self {
        Self {
            index,
            status,
            values,
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

    /// The values, oldest period first, `None` where the provider had no datapoint.
    ///
    /// Slot `i` covers the period starting at `range.start + i * period` for the query that
    /// produced it, so a caller that wants timestamps can recover them from the request it made.
    pub fn values(&self) -> &[Option<f64>] {
        &self.values
    }

    /// The most recent period's value, `None` if that period had no datapoint.
    ///
    /// Note the double meaning is deliberate: `None` from an empty series and `None` from a
    /// missing final datapoint are the same answer — "no reading for the latest period".
    pub fn latest(&self) -> Option<f64> {
        self.values.last().copied().flatten()
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
    pub fn new(series: Vec<MetricSeries>, cursor: Option<Cursor>) -> Self {
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
    /// a series whose every period is a gap.
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
    use super::*;

    #[test]
    fn the_latest_period_reads_through_to_its_value() {
        let series = MetricSeries::new(0, SeriesStatus::Complete, vec![Some(1.0), Some(2.0)]);

        assert_eq!(series.latest(), Some(2.0));
    }

    #[test]
    fn a_gap_in_the_latest_period_is_not_a_value() {
        let series = MetricSeries::new(0, SeriesStatus::Complete, vec![Some(1.0), None]);

        assert_eq!(series.latest(), None);
    }

    #[test]
    fn a_series_never_returned_is_distinguishable_from_one_that_is_all_gaps() {
        let page = MetricPage::new(
            vec![MetricSeries::new(
                0,
                SeriesStatus::Complete,
                vec![None, None],
            )],
            None,
        );

        assert!(page
            .series_at(0)
            .is_some_and(|s| s.values() == [None, None]));
        assert!(page.series_at(1).is_none());
    }
}
