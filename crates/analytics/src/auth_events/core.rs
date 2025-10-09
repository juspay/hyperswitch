use std::collections::HashMap;

use api_models::analytics::{
    auth_events::{
        AuthEventDimensions, AuthEventMetrics, AuthEventMetricsBucketIdentifier,
        MetricsBucketResponse,
    },
    AuthEventFilterValue, AuthEventFiltersResponse, AuthEventMetricsResponse,
    AuthEventsAnalyticsMetadata, GetAuthEventFilterRequest, GetAuthEventMetricRequest,
};
use common_utils::types::TimeRange;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::{
    filters::{get_auth_events_filter_for_dimension, AuthEventFilterRow},
    sankey::{get_sankey_data, SankeyRow},
    AuthEventMetricsAccumulator,
};
use crate::{
    auth_events::AuthEventMetricAccumulator,
    enums::AuthInfo,
    errors::{AnalyticsError, AnalyticsResult},
    AnalyticsProvider,
};

#[instrument(skip_all)]
pub async fn get_metrics(
    pool: &AnalyticsProvider,
    auth: &AuthInfo,
    req: GetAuthEventMetricRequest,
) -> AnalyticsResult<AuthEventMetricsResponse<MetricsBucketResponse>> {
    let mut metrics_accumulator: HashMap<
        AuthEventMetricsBucketIdentifier,
        AuthEventMetricsAccumulator,
    > = HashMap::new();

    let mut set = tokio::task::JoinSet::new();
    for metric_type in req.metrics.iter().cloned() {
        let req = req.clone();
        let auth_scoped = auth.to_owned();
        let pool = pool.clone();
        set.spawn(async move {
            let data = pool
                .get_auth_event_metrics(
                    &metric_type,
                    &req.group_by_names.clone(),
                    &auth_scoped,
                    &req.filters,
                    req.time_series.map(|t| t.granularity),
                    &req.time_range,
                )
                .await
                .change_context(AnalyticsError::UnknownError);
            (metric_type, data)
        });
    }

    while let Some((metric, data)) = set
        .join_next()
        .await
        .transpose()
        .change_context(AnalyticsError::UnknownError)?
    {
        for (id, value) in data? {
            let metrics_builder = metrics_accumulator.entry(id).or_default();
            match metric {
                AuthEventMetrics::AuthenticationCount => metrics_builder
                    .authentication_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationAttemptCount => metrics_builder
                    .authentication_attempt_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationSuccessCount => metrics_builder
                    .authentication_success_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::ChallengeFlowCount => metrics_builder
                    .challenge_flow_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::ChallengeAttemptCount => metrics_builder
                    .challenge_attempt_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::ChallengeSuccessCount => metrics_builder
                    .challenge_success_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::FrictionlessFlowCount => metrics_builder
                    .frictionless_flow_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::FrictionlessSuccessCount => metrics_builder
                    .frictionless_success_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationErrorMessage => metrics_builder
                    .authentication_error_message
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationFunnel => metrics_builder
                    .authentication_funnel
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationExemptionApprovedCount => metrics_builder
                    .authentication_exemption_approved_count
                    .add_metrics_bucket(&value),
                AuthEventMetrics::AuthenticationExemptionRequestedCount => metrics_builder
                    .authentication_exemption_requested_count
                    .add_metrics_bucket(&value),
            }
        }
    }

    let mut total_error_message_count = 0;
    let query_data: Vec<MetricsBucketResponse> = metrics_accumulator
        .into_iter()
        .map(|(id, val)| {
            let collected_values = val.collect();
            if let Some(count) = collected_values.error_message_count {
                total_error_message_count += count;
            }
            MetricsBucketResponse {
                values: collected_values,
                dimensions: id,
            }
        })
        .collect();
    Ok(AuthEventMetricsResponse {
        query_data,
        meta_data: [AuthEventsAnalyticsMetadata {
            total_error_message_count: Some(total_error_message_count),
        }],
    })
}

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetAuthEventFilterRequest,
    auth: &AuthInfo,
) -> AnalyticsResult<AuthEventFiltersResponse> {
    let mut res = AuthEventFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(_pool) => {
                            Err(report!(AnalyticsError::UnknownError))
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                get_auth_events_filter_for_dimension(dim, auth, &req.time_range, pool)
                    .await
                    .map_err(|e| e.change_context(AnalyticsError::UnknownError))
            }
                    AnalyticsProvider::CombinedCkh(sqlx_pool, ckh_pool) | AnalyticsProvider::CombinedSqlx(sqlx_pool, ckh_pool) => {
                let ckh_result = get_auth_events_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    ckh_pool,
                )
                .await
                .map_err(|e| e.change_context(AnalyticsError::UnknownError));
                let sqlx_result = get_auth_events_filter_for_dimension(
                    dim,
                    auth,
                    &req.time_range,
                    sqlx_pool,
                )
                .await
                .map_err(|e| e.change_context(AnalyticsError::UnknownError));
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: AuthEventFilterRow| match dim {
            AuthEventDimensions::AuthenticationStatus => fil.authentication_status.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::TransactionStatus => fil.trans_status.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::AuthenticationType => fil.authentication_type.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::ErrorMessage => fil.error_message,
            AuthEventDimensions::AuthenticationConnector => fil.authentication_connector.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::MessageVersion => fil.message_version,
            AuthEventDimensions::AcsReferenceNumber => fil.acs_reference_number,
            AuthEventDimensions::Platform => fil.platform,
            AuthEventDimensions::Mcc => fil.mcc,
           AuthEventDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
            AuthEventDimensions::MerchantCountry => fil.merchant_country,
            AuthEventDimensions::BillingCountry => fil.billing_country,
            AuthEventDimensions::ShippingCountry => fil.shipping_country,
            AuthEventDimensions::IssuerCountry => fil.issuer_country,
            AuthEventDimensions::EarliestSupportedVersion => fil.earliest_supported_version,
            AuthEventDimensions::LatestSupportedVersion => fil.latest_supported_version,
            AuthEventDimensions::WhitelistDecision => fil.whitelist_decision.map(|i| i.to_string()),
            AuthEventDimensions::DeviceManufacturer => fil.device_manufacturer,
            AuthEventDimensions::DeviceType => fil.device_type,
            AuthEventDimensions::DeviceBrand => fil.device_brand,
            AuthEventDimensions::DeviceOs => fil.device_os,
            AuthEventDimensions::DeviceDisplay => fil.device_display,
            AuthEventDimensions::BrowserName => fil.browser_name,
            AuthEventDimensions::BrowserVersion => fil.browser_version,
            AuthEventDimensions::IssuerId => fil.issuer_id,
            AuthEventDimensions::SchemeName => fil.scheme_name,
            AuthEventDimensions::ExemptionRequested => fil.exemption_requested.map(|i| i.to_string()),
            AuthEventDimensions::ExemptionAccepted => fil.exemption_accepted.map(|i| i.to_string()),
        })
        .collect::<Vec<String>>();
        res.query_data.push(AuthEventFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}

#[instrument(skip_all)]
pub async fn get_sankey(
    pool: &AnalyticsProvider,
    auth: &AuthInfo,
    req: TimeRange,
) -> AnalyticsResult<Vec<SankeyRow>> {
    match pool {
        AnalyticsProvider::Sqlx(_) => Err(AnalyticsError::NotImplemented(
            "Sankey not implemented for sqlx",
        ))?,
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool) => {
            let sankey_rows = get_sankey_data(ckh_pool, auth, &req)
                .await
                .change_context(AnalyticsError::UnknownError)?;
            Ok(sankey_rows)
        }
    }
}
