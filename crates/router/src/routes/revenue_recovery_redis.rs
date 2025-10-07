use actix_web::{web, HttpRequest, HttpResponse};
use api_models::revenue_recovery_data_backfill::GetRedisDataQuery;
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, revenue_recovery_data_backfill},
    routes::AppState,
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::RevenueRecoveryRedis))]
pub async fn get_revenue_recovery_redis_data(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<GetRedisDataQuery>,
) -> HttpResponse {
    let flow = Flow::RevenueRecoveryRedis;
    let connector_customer_id = path.into_inner();
    let key_type = &query.key_type;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _: (), _, _| {
            revenue_recovery_data_backfill::get_redis_data(state, &connector_customer_id, key_type)
        },
        &auth::V2AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
