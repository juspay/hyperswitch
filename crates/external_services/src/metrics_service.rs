//! Reading metric datapoints from a monitoring provider.
//!
//! A [`MetricsProvider`] answers one question: what values did these metrics take over this
//! window? It holds no thresholds and no notion that a number is "bad", so the same provider can
//! serve an alarm evaluator, a dashboard or a one-off investigation.
//!
//! The vocabulary is nobody's in particular. Amazon says *dimensions*, *statistics* and
//! *namespaces*; Google and Prometheus say *labels*. These types use the names that survive
//! translation, and provider-specific rules — which periods are legal, how big a batch may be —
//! are enforced by each provider rather than baked in here.
//!
//! Pagination is the caller's: [`MetricsProvider::fetch`] returns one [`MetricPage`], and a page
//! carrying a [`Cursor`] means there is more. Nothing loops on the caller's behalf. A single
//! series can span pages, so a caller that paginates concatenates datapoints under the same
//! [`index`](MetricSeries::index) — in practice one page suffices, since what fills a page is
//! datapoint count rather than elapsed time.

/// Reading metrics from Amazon CloudWatch.
pub mod aws_cloudwatch;
mod error;
mod query;
mod response;

pub use self::{
    error::{MetricsError, MetricsResult},
    query::{Aggregation, Labels, MetricQuery, MetricRequest, Period, TimeRange},
    response::{Cursor, MetricPage, MetricSeries, SeriesStatus},
};

/// Reads metric datapoints from a monitoring provider.
///
/// Object-safe on purpose, so a caller resolving its provider from configuration can hold an
/// `Arc<dyn MetricsProvider>`. Resist adding an associated type: it is what stops
/// [`crate::email::EmailClient`] being usable as a trait object.
#[async_trait::async_trait]
pub trait MetricsProvider: Send + Sync + std::fmt::Debug {
    /// Fetch one page of datapoints for `request`.
    ///
    /// Pass `None` as `cursor` for the first page, then the previous page's
    /// [`MetricPage::cursor`]. A page whose cursor is `None` is the last. The same `request` must
    /// be passed for every page of one traversal.
    async fn fetch(
        &self,
        request: &MetricRequest,
        cursor: Option<&Cursor>,
    ) -> MetricsResult<MetricPage>;
}
