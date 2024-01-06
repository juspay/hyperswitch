use std::collections::HashMap;

use api_models::analytics::{
    api_event::{
        ApiEventMetricsBucketIdentifier, ApiEventMetricsBucketValue, ApiLogsRequest,
        ApiMetricsBucketResponse,
    },
    AnalyticsMetadata, ApiEventFiltersResponse, GetApiEventFiltersRequest,
    GetApiEventMetricRequest, MetricsResponse,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::{IntoReport, ResultExt};
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};

use super::{
    events::{get_api_event, ApiLogsResult},
    metrics::ApiEventMetricRow,
};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    metrics,
    types::FiltersError,
    AnalyticsProvider,
};

#[instrument(skip_all)]
pub async fn api_events_core(
    pool: &AnalyticsProvider,
    req: ApiLogsRequest,
    merchant_id: String,
) -> AnalyticsResult<Vec<ApiLogsResult>> {
    let data = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "API Events not implemented for SQLX",
        ))
        .into_report()
        .attach_printable("SQL Analytics is not implemented for API Events"),
        AnalyticsProvider::Clickhouse(pool) => get_api_event(&merchant_id, req, pool).await,
        AnalyticsProvider::CombinedSqlx(_sqlx_pool, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_sqlx_pool, ckh_pool) => {
            get_api_event(&merchant_id, req, ckh_pool).await
        }
    }
    .switch()?;
    Ok(data)
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetApiEventFiltersRequest,
    merchant_id: String,
) -> AnalyticsResult<ApiEventFiltersResponse> {
    use api_models::analytics::{api_event::ApiEventDimensions, ApiEventFilterValue};

    use super::filters::get_api_event_filter_for_dimension;
    use crate::api_event::filters::ApiEventFilter;

    let mut res = ApiEventFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
            AnalyticsProvider::Sqlx(_pool) => Err(FiltersError::NotImplemented(
                "API Events not implemented for SQLX",
            ))
            .into_report()
            .attach_printable("SQL Analytics is not implemented for API Events"),
            AnalyticsProvider::Clickhouse(ckh_pool)
            | AnalyticsProvider::CombinedSqlx(_, ckh_pool)
            | AnalyticsProvider::CombinedCkh(_, ckh_pool) => {
                get_api_event_filter_for_dimension(dim, &merchant_id, &req.time_range, ckh_pool)
                    .await
            }
        }
        .switch()?
        .into_iter()
        .filter_map(|fil: ApiEventFilter| match dim {
            ApiEventDimensions::StatusCode => fil.status_code.map(|i| i.to_string()),
            ApiEventDimensions::FlowType => fil.flow_type,
            ApiEventDimensions::ApiFlow => fil.api_flow,
        })
        .collect::<Vec<String>>();
        res.query_data.push(ApiEventFilterValue {
            dimension: dim,
            values,
        })
    }

    Ok(res)
}

#[instrument(skip_all)]
pub async fn get_api_event_metrics(
    pool: &AnalyticsProvider,
    merchant_id: &str,
    req: GetApiEventMetricRequest,
) -> AnalyticsResult<MetricsResponse<ApiMetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<ApiEventMetricsBucketIdentifier, ApiEventMetricRow> =
        HashMap::new();

    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let pool = pool.clone();
        let task_span = tracing::debug_span!(
            "analytics_api_metrics_query",
            api_event_metric = metric_type.as_ref()
        );

        // TODO: lifetime issues with joinset,
        // can be optimized away if joinset lifetime requirements are relaxed
        let merchant_id_scoped = merchant_id.to_owned();
        set.spawn(
            async move {
                let data = pool
                    .get_api_event_metrics(
                        &metric_type,
                        &req.group_by_names.clone(),
                        &merchant_id_scoped,
                        &req.filters,
                        &req.time_series.map(|t| t.granularity),
                        &req.time_range,
                    )
                    .await
                    .change_context(AnalyticsError::UnknownError);
                (metric_type, data)
            }
            .instrument(task_span),
        );
    }

    while let Some((metric, data)) = set
        .join_next()
        .await
        .transpose()
        .into_report()
        .change_context(AnalyticsError::UnknownError)?
    {
        let data = data?;
        let attributes = &[
            metrics::request::add_attributes("metric_type", metric.to_string()),
            metrics::request::add_attributes("source", pool.to_string()),
        ];

        let value = u64::try_from(data.len());
        if let Ok(val) = value {
            metrics::BUCKETS_FETCHED.record(&metrics::CONTEXT, val, attributes);
            logger::debug!("Attributes: {:?}, Buckets fetched: {}", attributes, val);
        }
        for (id, value) in data {
            metrics_accumulator
                .entry(id)
                .and_modify(|data| {
                    data.api_count = data.api_count.or(value.api_count);
                    data.status_code_count = data.status_code_count.or(value.status_code_count);
                    data.latency = data.latency.or(value.latency);
                })
                .or_insert(value);
        }
    }

    let query_data: Vec<ApiMetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| ApiMetricsBucketResponse {
            values: ApiEventMetricsBucketValue {
                latency: val.latency,
                api_count: val.api_count,
                status_code_count: val.status_code_count,
            },
            dimensions: id,
        })
        .collect();

    Ok(MetricsResponse {
        query_data,
        meta_data: [AnalyticsMetadata {
            current_time_range: req.time_range,
        }],
    })
}
