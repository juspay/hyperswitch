//! Reading metric datapoints from Amazon CloudWatch.
//!
//! Everything that knows about `aws_sdk_cloudwatch` lives here, so
//! [`crate::metrics_service`] stays readable without the SDK in view. The entry point is
//! `GetMetricData`, not `DescribeAlarms`: we own the thresholds, so CloudWatch is a source of
//! numbers rather than an alarm estate to subscribe to.
//!
//! # Query ids are ours, not the caller's
//!
//! `GetMetricData` requires each query to carry a caller-supplied id matching
//! `[a-z][a-zA-Z0-9_]*`, and it rejects the **whole batch** if any one of them is malformed. That
//! is CloudWatch's constraint rather than a property of reading metrics, so it does not appear in
//! [`MetricsProvider`](super::MetricsProvider): ids are minted here from each query's position in
//! [`MetricRequest::queries`](super::MetricRequest::queries), and results are handed back keyed by
//! that same position. No caller can fail a batch by naming a metric `rds-primary`.
//!
//! # Credentials
//!
//! The default provider chain, exactly as [`crate::aws_kms`] and [`crate::file_storage::aws_s3`]
//! use it — in the cluster that resolves to the pod's IRSA role. Deliberately *not* the
//! [`crate::email::ses`] approach of assuming a role explicitly on every call: that pattern is a
//! known problem rather than a model to copy, and the sandbox VPC reaches CloudWatch through a
//! `monitoring` interface endpoint, so there is no proxy to route around either.

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatch::{
    config::Region,
    types::{Dimension, Metric, MetricDataQuery, MetricDataResult, MetricStat, ScanBy, StatusCode},
    Client,
};
use common_utils::ext_traits::ConfigExt;
use error_stack::{report, ResultExt};

use super::{
    Aggregation, Cursor, Datapoint, MetricPage, MetricQuery, MetricRequest, MetricSeries,
    MetricsError, MetricsProvider, MetricsResult, SeriesStatus,
};

/// The prefix every minted query id carries.
///
/// CloudWatch requires an id to start with a lowercase letter, which a bare index does not.
const QUERY_ID_PREFIX: &str = "m";

/// How to reach CloudWatch.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CloudWatchConfig {
    /// The AWS region whose metrics to read.
    pub region: String,
}

impl CloudWatchConfig {
    /// Validate the configuration at startup rather than at the first fetch.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.region.is_default_or_empty() {
            Err("cloudwatch.region must not be empty")
        } else {
            Ok(())
        }
    }
}

/// Reads metric datapoints from Amazon CloudWatch.
///
/// The SDK client is built once and held. Credentials refresh themselves inside the provider
/// chain, so there is no reason to rebuild it per call.
#[derive(Debug, Clone)]
pub struct CloudWatchMetrics {
    client: Client,
}

impl CloudWatchMetrics {
    /// Build a client, resolving the region and credential chain once.
    pub async fn create(config: &CloudWatchConfig) -> MetricsResult<Self> {
        config
            .validate()
            .map_err(|reason| report!(MetricsError::Configuration(reason)))?;

        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;

        Ok(Self {
            client: Client::new(&sdk_config),
        })
    }
}

#[async_trait::async_trait]
impl MetricsProvider for CloudWatchMetrics {
    async fn fetch(
        &self,
        request: &MetricRequest,
        cursor: Option<&Cursor>,
    ) -> MetricsResult<MetricPage> {
        let queries = request
            .queries()
            .iter()
            .enumerate()
            .map(|(index, query)| metric_data_query(index, query))
            .collect::<Vec<_>>();

        let response = self
            .client
            .get_metric_data()
            .set_metric_data_queries(Some(queries))
            .start_time(to_aws_timestamp(request.range().start()))
            .end_time(to_aws_timestamp(request.range().end()))
            // Ascending so datapoints arrive oldest-first, which is the order every caller reads
            // them in and the order `MetricSeries::datapoints` promises.
            .scan_by(ScanBy::TimestampAscending)
            .set_next_token(cursor.map(|cursor| cursor.as_str().to_owned()))
            .send()
            .await
            .change_context(MetricsError::Transport)
            .attach_printable_lazy(|| {
                format!(
                    "Fetching {} metrics from CloudWatch",
                    request.queries().len()
                )
            })?;

        let series = response
            .metric_data_results()
            .iter()
            .map(metric_series)
            .collect::<MetricsResult<Vec<_>>>()?;

        Ok(MetricPage::new(
            series,
            response.next_token().map(Cursor::new),
        ))
    }
}

/// The id a query at `index` is sent and reported under.
fn query_id(index: usize) -> String {
    format!("{QUERY_ID_PREFIX}{index}")
}

/// Recover the query position from an id [`query_id`] minted.
fn query_index(id: &str) -> Option<usize> {
    id.strip_prefix(QUERY_ID_PREFIX)?.parse().ok()
}

/// Translate one query into the SDK's request shape.
fn metric_data_query(index: usize, query: &MetricQuery) -> MetricDataQuery {
    let dimensions = query
        .labels()
        .iter()
        .map(|(name, value)| Dimension::builder().name(name).value(value).build())
        .collect::<Vec<_>>();

    let metric = Metric::builder()
        .set_namespace(query.namespace().map(ToOwned::to_owned))
        .metric_name(query.name())
        .set_dimensions((!dimensions.is_empty()).then_some(dimensions))
        .build();

    MetricDataQuery::builder()
        .id(query_id(index))
        .metric_stat(
            MetricStat::builder()
                .metric(metric)
                .period(query.period().seconds())
                .stat(stat(query.aggregation()))
                .build(),
        )
        .build()
}

/// The name CloudWatch gives each aggregation on the wire.
fn stat(aggregation: Aggregation) -> &'static str {
    match aggregation {
        Aggregation::Average => "Average",
        Aggregation::Maximum => "Maximum",
        Aggregation::Minimum => "Minimum",
        Aggregation::Sum => "Sum",
    }
}

/// Translate one result back into a series.
fn metric_series(result: &MetricDataResult) -> MetricsResult<MetricSeries> {
    let index = result
        .id()
        .and_then(query_index)
        .ok_or_else(|| report!(MetricsError::MalformedResponse))
        .attach_printable_lazy(|| {
            format!(
                "CloudWatch reported a series under an id we never sent: {:?}",
                result.id()
            )
        })?;

    let timestamps = result.timestamps();
    let values = result.values();

    // CloudWatch documents these as always the same length. If they are not, pairing them off
    // would invent datapoints or silently drop them, and either is worse than refusing.
    if timestamps.len() != values.len() {
        return Err(report!(MetricsError::MalformedResponse)).attach_printable_lazy(|| {
            format!(
                "CloudWatch returned {} timestamps against {} values",
                timestamps.len(),
                values.len()
            )
        });
    }

    let datapoints = timestamps
        .iter()
        .zip(values)
        .map(|(timestamp, value)| Ok(Datapoint::new(from_aws_timestamp(timestamp)?, *value)))
        .collect::<MetricsResult<Vec<_>>>()?;

    Ok(MetricSeries::new(
        index,
        datapoints,
        series_status(result.status_code()),
    ))
}

/// Map CloudWatch's per-series status onto the provider-neutral one.
///
/// `Forbidden` and `InternalError` both become [`SeriesStatus::Failed`]: they differ in whose
/// fault it is, not in what the caller holds, and the interface does not carry blame.
fn series_status(status: Option<&StatusCode>) -> SeriesStatus {
    match status {
        Some(StatusCode::Complete) => SeriesStatus::Complete,
        Some(StatusCode::PartialData) => SeriesStatus::Partial,
        // A series with no status at all is not one we can call complete.
        _ => SeriesStatus::Failed,
    }
}

fn to_aws_timestamp(timestamp: time::OffsetDateTime) -> aws_smithy_types::DateTime {
    aws_smithy_types::DateTime::from_secs_and_nanos(
        timestamp.unix_timestamp(),
        timestamp.nanosecond(),
    )
}

fn from_aws_timestamp(
    timestamp: &aws_smithy_types::DateTime,
) -> MetricsResult<time::OffsetDateTime> {
    let nanos = i128::from(timestamp.secs())
        .checked_mul(1_000_000_000)
        .and_then(|seconds| seconds.checked_add(i128::from(timestamp.subsec_nanos())))
        .ok_or_else(|| report!(MetricsError::MalformedResponse))?;

    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .change_context(MetricsError::MalformedResponse)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use time::macros::datetime;

    use super::*;
    use crate::metrics_service::{Labels, Period};

    fn timestamp(datetime: time::OffsetDateTime) -> aws_smithy_types::DateTime {
        aws_smithy_types::DateTime::from_secs(datetime.unix_timestamp())
    }

    fn result_of(
        id: &str,
        timestamps: Vec<aws_smithy_types::DateTime>,
        values: Vec<f64>,
        status: StatusCode,
    ) -> MetricDataResult {
        MetricDataResult::builder()
            .id(id)
            .set_timestamps(Some(timestamps))
            .set_values(Some(values))
            .status_code(status)
            .build()
    }

    #[test]
    fn a_query_becomes_a_metric_stat_carrying_its_labels() {
        let query = MetricQuery::new("CPUUtilization", Period::FIVE_MINUTES, Aggregation::Average)
            .in_namespace("AWS/RDS")
            .with_labels(Labels::none().with("DBInstanceIdentifier", "sbx-hyp-rds-1"));

        let wire = metric_data_query(3, &query);

        assert_eq!(wire.id(), Some("m3"));
        let metric_stat = wire.metric_stat().unwrap();
        assert_eq!(metric_stat.period(), Some(300));
        assert_eq!(metric_stat.stat(), Some("Average"));

        let metric = metric_stat.metric().unwrap();
        assert_eq!(metric.namespace(), Some("AWS/RDS"));
        assert_eq!(metric.metric_name(), Some("CPUUtilization"));
        assert_eq!(metric.dimensions().len(), 1);
        assert_eq!(metric.dimensions()[0].name(), Some("DBInstanceIdentifier"));
        assert_eq!(metric.dimensions()[0].value(), Some("sbx-hyp-rds-1"));
    }

    #[test]
    fn a_query_with_no_labels_sends_no_dimensions_rather_than_an_empty_list() {
        let query = MetricQuery::new("RequestCount", Period::ONE_MINUTE, Aggregation::Sum);

        let wire = metric_data_query(0, &query);
        let metric = wire.metric_stat().unwrap().metric().unwrap();

        // `set_dimensions(None)` and `set_dimensions(Some(vec![]))` serialise differently, and an
        // empty dimension list is not the same request as an unqualified metric.
        assert!(metric.dimensions.is_none());
    }

    #[test]
    fn every_aggregation_has_the_name_cloudwatch_expects() {
        assert_eq!(stat(Aggregation::Average), "Average");
        assert_eq!(stat(Aggregation::Maximum), "Maximum");
        assert_eq!(stat(Aggregation::Minimum), "Minimum");
        assert_eq!(stat(Aggregation::Sum), "Sum");
    }

    #[test]
    fn minted_ids_survive_a_round_trip_and_reject_anything_else() {
        for index in [0, 1, 9, 75, 499] {
            assert_eq!(query_index(&query_id(index)), Some(index));
        }

        // Ids CloudWatch could invent for metric math, or that we simply never sent.
        for id in ["", "m", "e1", "m1_result", "ANOMALY", "-1"] {
            assert_eq!(query_index(id), None, "{id:?} should not resolve");
        }
    }

    #[test]
    fn a_zero_datapoint_survives_and_a_gap_stays_a_gap() {
        // 12:02 is absent from the response entirely; 12:01 aggregated to a real zero.
        let result = result_of(
            "m0",
            vec![
                timestamp(datetime!(2026-09-09 12:00:00 UTC)),
                timestamp(datetime!(2026-09-09 12:01:00 UTC)),
                timestamp(datetime!(2026-09-09 12:03:00 UTC)),
            ],
            vec![4.0, 0.0, 7.0],
            StatusCode::Complete,
        );

        let series = metric_series(&result).unwrap();

        assert_eq!(series.index(), 0);
        assert_eq!(series.datapoints().len(), 3);
        assert_eq!(
            series.window(datetime!(2026-09-09 12:04:00 UTC), Period::ONE_MINUTE, 4),
            vec![Some(4.0), Some(0.0), None, Some(7.0)]
        );
    }

    #[test]
    fn a_series_with_no_datapoints_reads_as_empty_rather_than_failing() {
        let result = result_of("m2", Vec::new(), Vec::new(), StatusCode::Complete);

        let series = metric_series(&result).unwrap();

        assert_eq!(series.index(), 2);
        assert!(series.datapoints().is_empty());
        assert_eq!(series.status(), SeriesStatus::Complete);
    }

    #[test]
    fn mismatched_timestamps_and_values_are_refused_rather_than_paired_off() {
        let result = result_of(
            "m0",
            vec![
                timestamp(datetime!(2026-09-09 12:00:00 UTC)),
                timestamp(datetime!(2026-09-09 12:01:00 UTC)),
            ],
            vec![1.0],
            StatusCode::Complete,
        );

        assert!(metric_series(&result).is_err());
    }

    #[test]
    fn a_series_reported_under_an_unknown_id_is_refused() {
        let result = result_of("e1", Vec::new(), Vec::new(), StatusCode::Complete);

        assert!(metric_series(&result).is_err());
    }

    #[test]
    fn a_forbidden_or_errored_series_arrives_as_data_rather_than_as_an_error() {
        // One metric the credentials cannot read must not blind the caller to the rest of the
        // batch, so this is a status on a series, not a failed call.
        for status in [StatusCode::Forbidden, StatusCode::InternalError] {
            let result = result_of("m5", Vec::new(), Vec::new(), status.clone());
            let series = metric_series(&result).unwrap();

            assert_eq!(series.status(), SeriesStatus::Failed, "{status:?}");
            assert_eq!(series.index(), 5);
        }

        let partial = result_of(
            "m6",
            vec![timestamp(datetime!(2026-09-09 12:00:00 UTC))],
            vec![1.0],
            StatusCode::PartialData,
        );
        assert_eq!(
            metric_series(&partial).unwrap().status(),
            SeriesStatus::Partial
        );
    }

    #[test]
    fn timestamps_survive_a_round_trip_through_the_sdk_representation() {
        let instant = datetime!(2026-09-09 12:34:56 UTC);

        assert_eq!(
            from_aws_timestamp(&to_aws_timestamp(instant)).unwrap(),
            instant
        );
    }

    #[test]
    fn an_empty_region_is_refused_before_a_client_is_built() {
        assert!(CloudWatchConfig::default().validate().is_err());
        assert!(CloudWatchConfig {
            region: "ap-south-1".to_owned(),
        }
        .validate()
        .is_ok());
    }
}
