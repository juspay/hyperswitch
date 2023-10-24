use api_models::analytics::{
    payments::PaymentDimensions, refunds::RefundDimensions, FilterValue, GetInfoResponse,
    GetPaymentFiltersRequest, GetRefundFilterRequest, PaymentFiltersResponse, RefundFilterValue,
    RefundFiltersResponse,
};

#[cfg(all(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
use error_stack::IntoReport;
use error_stack::ResultExt;
use hyperswitch_oss::{services::ApplicationResponse, types::domain};

#[cfg(all(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
use super::types::FiltersError;
use super::{
    errors::{self, AnalyticsError},
    payments::filters::{get_payment_filter_for_dimension, FilterRow},
    refunds::filters::{get_refund_filter_for_dimension, RefundFilterRow},
    types::AnalyticsDomain,
    utils, AnalyticsProvider,
};

pub type AnalyticsApiResponse<T> = errors::AnalyticsResult<ApplicationResponse<T>>;

pub async fn get_domain_info(domain: AnalyticsDomain) -> AnalyticsApiResponse<GetInfoResponse> {
    let info = match domain {
        AnalyticsDomain::Payments => GetInfoResponse {
            metrics: utils::get_payment_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_payment_dimensions(),
        },
        AnalyticsDomain::Refunds => GetInfoResponse {
            metrics: utils::get_refund_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_refund_dimensions(),
        },
    };
    Ok(ApplicationResponse::Json(info))
}

pub async fn payment_filters_core(
    pool: &AnalyticsProvider,
    req: GetPaymentFiltersRequest,
    merchant: domain::MerchantAccount,
) -> AnalyticsApiResponse<PaymentFiltersResponse> {
    let mut res = PaymentFiltersResponse::default();

    for dim in req.group_by_names {
        let values = match pool {
            #[cfg(feature = "sqlx_analytics")]
            AnalyticsProvider::Sqlx(pool) => {
                get_payment_filter_for_dimension(dim, &merchant.merchant_id, &req.time_range, pool)
                    .await
            }
            #[cfg(feature = "clickhouse_analytics")]
            AnalyticsProvider::Clickhouse(pool) => {
                get_payment_filter_for_dimension(dim, &merchant.merchant_id, &req.time_range, pool)
                    .await
            }
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            AnalyticsProvider::CombinedCkh(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    sqlx_poll,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            AnalyticsProvider::CombinedSqlx(sqlx_poll, ckh_pool) => {
                let ckh_result = get_payment_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_payment_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    sqlx_poll,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres payments analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: FilterRow| match dim {
            PaymentDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
            PaymentDimensions::PaymentStatus => fil.status.map(|i| i.as_ref().to_string()),
            PaymentDimensions::Connector => fil.connector,
            PaymentDimensions::AuthType => fil.authentication_type.map(|i| i.as_ref().to_string()),
            PaymentDimensions::PaymentMethod => fil.payment_method,
        })
        .collect::<Vec<String>>();
        res.query_data.push(FilterValue {
            dimension: dim,
            values,
        })
    }

    Ok(ApplicationResponse::Json(res))
}

pub async fn refund_filter_core(
    pool: &AnalyticsProvider,
    req: GetRefundFilterRequest,
    merchant: domain::MerchantAccount,
) -> AnalyticsApiResponse<RefundFiltersResponse> {
    let mut res = RefundFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
            #[cfg(feature = "sqlx_analytics")]
            AnalyticsProvider::Sqlx(pool) => {
                get_refund_filter_for_dimension(dim, &merchant.merchant_id, &req.time_range, pool)
                    .await
            }
            #[cfg(feature = "clickhouse_analytics")]
            AnalyticsProvider::Clickhouse(pool) => {
                get_refund_filter_for_dimension(dim, &merchant.merchant_id, &req.time_range, pool)
                    .await
            }
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            AnalyticsProvider::CombinedCkh(sqlx_pool, ckh_pool) => {
                let ckh_result = get_refund_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_refund_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
            #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
            AnalyticsProvider::CombinedSqlx(sqlx_pool, ckh_pool) => {
                let ckh_result = get_refund_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_refund_filter_for_dimension(
                    dim,
                    &merchant.merchant_id,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env_oss::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres refunds analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: RefundFilterRow| match dim {
            RefundDimensions::Currency => fil.currency.map(|i| i.as_ref().to_string()),
            RefundDimensions::RefundStatus => fil.refund_status.map(|i| i.as_ref().to_string()),
            RefundDimensions::Connector => fil.connector,
            RefundDimensions::RefundType => fil.refund_type.map(|i| i.as_ref().to_string()),
        })
        .collect::<Vec<String>>();
        res.query_data.push(RefundFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(ApplicationResponse::Json(res))
}