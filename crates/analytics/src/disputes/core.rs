use api_models::analytics::{
    disputes::DisputeDimensions, DisputeFilterValue, DisputeFiltersResponse,
    GetDisputeFilterRequest,
};
use error_stack::ResultExt;

use super::filters::{get_dispute_filter_for_dimension, DisputeFilterRow};
use crate::{
    errors::{AnalyticsError, AnalyticsResult},
    AnalyticsProvider,
};

pub async fn get_filters(
    pool: &AnalyticsProvider,
    req: GetDisputeFilterRequest,
    merchant_id: &String,
) -> AnalyticsResult<DisputeFiltersResponse> {
    let mut res = DisputeFiltersResponse::default();
    for dim in req.group_by_names {
        let values = match pool {
                        AnalyticsProvider::Sqlx(pool) => {
                            get_dispute_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
            }
                        AnalyticsProvider::Clickhouse(pool) => {
                            get_dispute_filter_for_dimension(dim, merchant_id, &req.time_range, pool)
                    .await
            }
                    AnalyticsProvider::CombinedCkh(sqlx_pool, ckh_pool) => {
                let ckh_result = get_dispute_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_dispute_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres disputes analytics filters")
                    },
                    _ => {}
                };
                ckh_result
            }
                    AnalyticsProvider::CombinedSqlx(sqlx_pool, ckh_pool) => {
                let ckh_result = get_dispute_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    ckh_pool,
                )
                .await;
                let sqlx_result = get_dispute_filter_for_dimension(
                    dim,
                    merchant_id,
                    &req.time_range,
                    sqlx_pool,
                )
                .await;
                match (&sqlx_result, &ckh_result) {
                    (Ok(ref sqlx_res), Ok(ref ckh_res)) if sqlx_res != ckh_res => {
                        router_env::logger::error!(clickhouse_result=?ckh_res, postgres_result=?sqlx_res, "Mismatch between clickhouse & postgres disputes analytics filters")
                    },
                    _ => {}
                };
                sqlx_result
            }
        }
        .change_context(AnalyticsError::UnknownError)?
        .into_iter()
        .filter_map(|fil: DisputeFilterRow| match dim {
            DisputeDimensions::DisputeStatus => fil.dispute_status,
            DisputeDimensions::DisputeStage => fil.dispute_stage,
            DisputeDimensions::ConnectorStatus => fil.connector_status,
            DisputeDimensions::Connector => fil.connector,
        })
        .collect::<Vec<String>>();
        res.query_data.push(DisputeFilterValue {
            dimension: dim,
            values,
        })
    }
    Ok(res)
}
