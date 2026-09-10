//! Reading metric datapoints from Amazon CloudWatch.
//!
//! Everything that knows about `aws_sdk_cloudwatch` lives here, including the rules CloudWatch
//! imposes that no other provider shares. The entry point is `GetMetricData`, not `DescribeAlarms`:
//! we own the thresholds, so CloudWatch is a source of numbers rather than an alarm estate.
//!
//! **Labels select, they do not aggregate.** A dimension combination *identifies* a CloudWatch
//! metric, so a query carrying no labels asks for the stream published with no dimensions at all —
//! not the sum or average of the dimensioned streams. A metric published per DB instance or per
//! target group therefore comes back empty when asked for without labels. Callers name the stream
//! they mean; Terraform publishes the dimension values for that purpose.
//!
//! **Query ids are ours.** `GetMetricData` requires ids matching `[a-z][a-zA-Z0-9_]*` and rejects
//! the whole batch if one is malformed, so ids are minted here from each query's position and
//! series come back keyed by that position. No caller can fail a batch by naming a metric
//! `rds-primary`.
//!
//! **Credentials** come from the default provider chain, as [`crate::aws_kms`] and
//! [`crate::file_storage::aws_s3`] use it; in the cluster that is the pod's IRSA role. Deliberately
//! not [`crate::email::ses`]'s assume-role-per-call. No proxy: the sandbox VPC reaches CloudWatch
//! through a `monitoring` interface endpoint.

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatch::{
    config::Region,
    types::{Dimension, Metric, MetricDataQuery, MetricDataResult, MetricStat, ScanBy, StatusCode},
    Client,
};
use common_utils::{ext_traits::ConfigExt, fp_utils::when};
use error_stack::{report, ResultExt};

use super::{
    Aggregation, Cursor, MetricPage, MetricQuery, MetricRequest, MetricSeries, MetricsError,
    MetricsProvider, MetricsResult, Period, SeriesStatus, TimeRange,
};

/// The prefix every minted query id carries, since CloudWatch requires a leading lowercase letter.
const QUERY_ID_PREFIX: &str = "m";

/// The most queries `GetMetricData` accepts in one call.
const MAX_QUERIES_PER_CALL: usize = 500;

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
/// The SDK client is built once and held; credentials refresh inside the provider chain.
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
        validate(request)?;

        let queries = request
            .queries
            .iter()
            .enumerate()
            .map(|(index, query)| metric_data_query(index, query))
            .collect::<Vec<_>>();

        let response = self
            .client
            .get_metric_data()
            .set_metric_data_queries(Some(queries))
            .start_time(to_aws_timestamp(request.range.start))
            .end_time(to_aws_timestamp(request.range.end))
            // Ascending so datapoints arrive oldest-first, the order `datapoints` promises.
            .scan_by(ScanBy::TimestampAscending)
            .set_next_token(cursor.map(|cursor| cursor.as_str().to_owned()))
            .send()
            .await
            .change_context(MetricsError::Transport)
            .attach_printable_lazy(|| {
                format!("Fetching {} metrics from CloudWatch", request.queries.len())
            })?;

        let series = response
            .metric_data_results()
            .iter()
            .map(|result| metric_series(result, request))
            .collect::<MetricsResult<Vec<_>>>()?;

        Ok(MetricPage::new(
            series,
            response.next_token().map(Cursor::new),
        ))
    }
}

/// Reject a request CloudWatch would refuse, before spending a round trip on it.
///
/// A rejected batch takes every metric in it down, so one bad period would blind a whole tick.
fn validate(request: &MetricRequest) -> MetricsResult<()> {
    validate_batch_size(request.queries.len())?;
    validate_range(request.range)?;
    request
        .queries
        .iter()
        .try_for_each(|query| validate_period(query.period))
}

/// `GetMetricData` takes between one and [`MAX_QUERIES_PER_CALL`] queries.
fn validate_batch_size(queries: usize) -> MetricsResult<()> {
    when(queries == 0, || {
        Err(report!(MetricsError::InvalidRequest))
            .attach_printable("A GetMetricData call needs at least one query")
    })?;

    when(queries > MAX_QUERIES_PER_CALL, || {
        Err(report!(MetricsError::InvalidRequest)).attach_printable_lazy(|| {
            format!("GetMetricData takes at most {MAX_QUERIES_PER_CALL} queries, got {queries}")
        })
    })
}

/// The window has to move forwards.
fn validate_range(range: TimeRange) -> MetricsResult<()> {
    when(range.start >= range.end, || {
        Err(report!(MetricsError::InvalidRequest))
            .attach_printable("A GetMetricData time range must start before it ends")
    })
}

/// CloudWatch takes high-resolution periods of 1, 5, 10, 20 or 30 seconds, or any positive
/// multiple of 60.
fn validate_period(period: Period) -> MetricsResult<()> {
    let seconds = period.seconds();
    let usable = matches!(seconds, 1 | 5 | 10 | 20 | 30) || (seconds > 0 && seconds % 60 == 0);

    when(!usable, || {
        Err(report!(MetricsError::InvalidRequest)).attach_printable_lazy(|| {
            format!(
                "{seconds} is not a CloudWatch period: use 1, 5, 10, 20, 30, or a multiple of 60"
            )
        })
    })
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
        .labels
        .iter()
        .map(|(name, value)| Dimension::builder().name(name).value(value).build())
        .collect::<Vec<_>>();

    let metric = Metric::builder()
        .set_namespace(query.namespace.clone())
        .metric_name(&query.name)
        // `None` and `Some(vec![])` serialise differently, and an empty dimension list is not the
        // same request as an unqualified metric.
        .set_dimensions((!dimensions.is_empty()).then_some(dimensions))
        .build();

    MetricDataQuery::builder()
        .id(query_id(index))
        .metric_stat(
            MetricStat::builder()
                .metric(metric)
                .period(query.period.seconds())
                .stat(stat(query.aggregation))
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

/// Translate one result back into a series, laid out on the grid its query asked for.
///
/// The timestamps are the only place a gap is visible — CloudWatch's `Values` for a metric that
/// stopped reporting is an unbroken run of readings, with nothing in it to say a period is missing.
/// Spending them here, against the range and period from the request, is what turns that into a
/// `None` the caller cannot read past.
fn metric_series(
    result: &MetricDataResult,
    request: &MetricRequest,
) -> MetricsResult<MetricSeries> {
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

    let query = request
        .queries
        .get(index)
        .ok_or_else(|| report!(MetricsError::MalformedResponse))
        .attach_printable_lazy(|| {
            format!("CloudWatch reported a series for query {index}, which was never sent")
        })?;

    let timestamps = result.timestamps();
    let values = result.values();

    validate_pairing(timestamps.len(), values.len())?;

    let mut slots = vec![None; slot_count(request.range, query.period)];

    timestamps
        .iter()
        .zip(values)
        .map(|(timestamp, value)| Ok((from_aws_timestamp(timestamp)?, *value)))
        .collect::<MetricsResult<Vec<_>>>()?
        .into_iter()
        .filter_map(|(timestamp, value)| {
            slot_of(timestamp, request.range.start, query.period).map(|slot| (slot, value))
        })
        .for_each(|(slot, value)| {
            // A later datapoint sharing a slot wins, since they arrive oldest-first.
            if let Some(bucket) = slots.get_mut(slot) {
                *bucket = Some(value);
            }
        });

    Ok(MetricSeries::new(
        index,
        series_status(result.status_code()),
        slots,
    ))
}

/// CloudWatch documents a series' timestamps and values as always the same length.
///
/// Pairing them off when they are not would invent datapoints or drop them, and either is worse
/// than refusing.
fn validate_pairing(timestamps: usize, values: usize) -> MetricsResult<()> {
    when(timestamps != values, || {
        Err(report!(MetricsError::MalformedResponse)).attach_printable_lazy(|| {
            format!("CloudWatch returned {timestamps} timestamps against {values} values")
        })
    })
}

/// Which slot of the grid a timestamp falls in, or `None` if it falls outside.
///
/// Integer division floors, so a datapoint lands in the slot *containing* it rather than having to
/// sit exactly on a boundary.
fn slot_of(
    timestamp: time::OffsetDateTime,
    start: time::OffsetDateTime,
    period: Period,
) -> Option<usize> {
    let offset = timestamp.unix_timestamp() - start.unix_timestamp();

    Some(period.seconds_i64())
        .filter(|seconds| *seconds > 0 && offset >= 0)
        .map(|seconds| offset / seconds)
        .and_then(|slot| usize::try_from(slot).ok())
}

/// How many periods of `period` the window covers.
///
/// Rounded up, so a range that is not a whole number of periods still gets a slot for its trailing
/// part rather than dropping it. `TimeRange::ending_at` always produces exact multiples, so this
/// only matters for a hand-built range.
fn slot_count(range: TimeRange, period: Period) -> usize {
    let span = range.end.unix_timestamp() - range.start.unix_timestamp();

    Some(period.seconds_i64())
        .filter(|seconds| *seconds > 0)
        .map(|seconds| span.div_euclid(seconds) + i64::from(span.rem_euclid(seconds) > 0))
        .and_then(|slots| usize::try_from(slots).ok())
        .unwrap_or_default()
}

/// Map CloudWatch's per-series status onto the provider-neutral one.
///
/// `Forbidden` and `InternalError` both become [`SeriesStatus::Failed`]: they differ in whose fault
/// it is, not in what the caller holds.
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
    use crate::metrics_service::{Labels, TimeRange};

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

    fn query(period: Period) -> MetricQuery {
        MetricQuery {
            namespace: Some("AWS/RDS".to_owned()),
            name: "CPUUtilization".to_owned(),
            labels: Labels::default(),
            period,
            aggregation: Aggregation::Average,
        }
    }

    fn request(queries: Vec<MetricQuery>) -> MetricRequest {
        MetricRequest {
            range: TimeRange::ending_at(datetime!(2026-09-09 12:00:00 UTC), Period::ONE_MINUTE, 3),
            queries,
        }
    }

    #[test]
    fn a_query_becomes_a_metric_stat_carrying_its_labels() {
        let query = MetricQuery {
            namespace: Some("AWS/RDS".to_owned()),
            name: "CPUUtilization".to_owned(),
            labels: [("DBInstanceIdentifier", "sbx-hyp-rds-1")]
                .into_iter()
                .collect(),
            period: Period::FIVE_MINUTES,
            aggregation: Aggregation::Average,
        };

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
        let wire = metric_data_query(0, &query(Period::ONE_MINUTE));
        let metric = wire.metric_stat().unwrap().metric().unwrap();

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
    fn only_periods_cloudwatch_accepts_get_past_validation() {
        for seconds in [1, 5, 10, 20, 30, 60, 300, 3600, 86400] {
            assert!(
                validate(&request(vec![query(Period::from_seconds(seconds))])).is_ok(),
                "{seconds} should be accepted"
            );
        }

        for seconds in [0, -60, 2, 45, 61, 359] {
            assert!(
                validate(&request(vec![query(Period::from_seconds(seconds))])).is_err(),
                "{seconds} should be rejected"
            );
        }
    }

    #[test]
    fn a_batch_that_cloudwatch_would_refuse_never_reaches_the_network() {
        assert!(validate(&request(Vec::new())).is_err());

        let too_many = vec![query(Period::ONE_MINUTE); MAX_QUERIES_PER_CALL + 1];
        assert!(validate(&request(too_many)).is_err());

        let instant = datetime!(2026-09-09 12:00:00 UTC);
        let backwards = MetricRequest {
            range: TimeRange {
                start: instant,
                end: instant,
            },
            queries: vec![query(Period::ONE_MINUTE)],
        };
        assert!(validate(&backwards).is_err());
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

    /// A request whose window is the four minutes before 12:04, for one query.
    fn four_minutes() -> MetricRequest {
        MetricRequest {
            range: TimeRange::ending_at(datetime!(2026-09-09 12:04:00 UTC), Period::ONE_MINUTE, 4),
            queries: vec![query(Period::ONE_MINUTE)],
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

        let series = metric_series(&result, &four_minutes()).unwrap();

        assert_eq!(series.index(), 0);
        assert_eq!(series.values(), [Some(4.0), Some(0.0), None, Some(7.0)]);
    }

    #[test]
    fn identical_values_still_reveal_the_gap_the_timestamps_describe() {
        // The values alone are an unbroken run of 1.0 — only the missing 12:02 timestamp says a
        // period had no reading at all, which is the whole reason timestamps are spent here.
        let result = result_of(
            "m0",
            vec![
                timestamp(datetime!(2026-09-09 12:00:00 UTC)),
                timestamp(datetime!(2026-09-09 12:01:00 UTC)),
                timestamp(datetime!(2026-09-09 12:03:00 UTC)),
            ],
            vec![1.0, 1.0, 1.0],
            StatusCode::Complete,
        );

        let series = metric_series(&result, &four_minutes()).unwrap();

        assert_eq!(series.values(), [Some(1.0), Some(1.0), None, Some(1.0)]);
    }

    #[test]
    fn an_unaligned_datapoint_lands_in_the_period_containing_it() {
        let result = result_of(
            "m0",
            vec![timestamp(datetime!(2026-09-09 12:00:37 UTC))],
            vec![9.0],
            StatusCode::Complete,
        );

        let series = metric_series(&result, &four_minutes()).unwrap();

        assert_eq!(series.values(), [Some(9.0), None, None, None]);
    }

    #[test]
    fn a_series_with_no_datapoints_is_all_gaps_rather_than_empty() {
        let series = metric_series(
            &result_of("m0", Vec::new(), Vec::new(), StatusCode::Complete),
            &four_minutes(),
        )
        .unwrap();

        // Four periods were asked for, so four gaps come back — not an empty list, which would
        // let a caller conclude there was nothing to evaluate.
        assert_eq!(series.values(), [None, None, None, None]);
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

        assert!(metric_series(&result, &four_minutes()).is_err());
    }

    #[test]
    fn a_series_reported_under_an_id_we_never_sent_is_refused() {
        // An id CloudWatch could invent for metric math.
        let invented = result_of("e1", Vec::new(), Vec::new(), StatusCode::Complete);
        assert!(metric_series(&invented, &four_minutes()).is_err());

        // A well-formed id for a query that was not in this request.
        let out_of_range = result_of("m7", Vec::new(), Vec::new(), StatusCode::Complete);
        assert!(metric_series(&out_of_range, &four_minutes()).is_err());
    }

    #[test]
    fn a_forbidden_or_errored_series_arrives_as_data_rather_than_as_an_error() {
        // One metric the credentials cannot read must not blind the caller to the rest of the
        // batch, so this is a status on a series, not a failed call.
        for status in [StatusCode::Forbidden, StatusCode::InternalError] {
            let result = result_of("m0", Vec::new(), Vec::new(), status.clone());
            let series = metric_series(&result, &four_minutes()).unwrap();

            assert_eq!(series.status(), SeriesStatus::Failed, "{status:?}");
            assert_eq!(series.index(), 0);
        }

        let partial = result_of(
            "m0",
            vec![timestamp(datetime!(2026-09-09 12:00:00 UTC))],
            vec![1.0],
            StatusCode::PartialData,
        );
        assert_eq!(
            metric_series(&partial, &four_minutes()).unwrap().status(),
            SeriesStatus::Partial
        );
    }

    #[test]
    fn a_slot_count_rounds_up_so_a_trailing_partial_period_is_not_dropped() {
        let exact = TimeRange::ending_at(datetime!(2026-09-09 12:04:00 UTC), Period::ONE_MINUTE, 4);
        assert_eq!(slot_count(exact, Period::ONE_MINUTE), 4);

        let ragged = TimeRange {
            start: datetime!(2026-09-09 12:00:00 UTC),
            end: datetime!(2026-09-09 12:04:30 UTC),
        };
        assert_eq!(slot_count(ragged, Period::ONE_MINUTE), 5);

        assert_eq!(slot_count(exact, Period::from_seconds(0)), 0);
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
