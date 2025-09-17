use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::revenue_recovery_data_backfill::{BackfillQuery, RevenueRecoveryDataBackfillForm};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, revenue_recovery_data_backfill},
    routes::AppState,
    services::{api, authentication as auth},
    types::{domain, storage},
};

#[instrument(skip_all, fields(flow = ?Flow::RecoveryDataBackfill))]
pub async fn revenue_recovery_data_backfill(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<BackfillQuery>,
    MultipartForm(form): MultipartForm<RevenueRecoveryDataBackfillForm>,
) -> HttpResponse {
    let flow = Flow::RecoveryDataBackfill;

    // Parse cutoff_time from query parameter
    let cutoff_datetime = match query
        .cutoff_time
        .as_ref()
        .map(|time_str| {
            time::PrimitiveDateTime::parse(
                time_str,
                &time::format_description::well_known::Iso8601::DEFAULT,
            )
        })
        .transpose()
    {
        Ok(datetime) => datetime,
        Err(err) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid datetime format: {}. Use ISO8601: 2024-01-15T10:30:00", err)
            }));
        }
    };

    let records = match form.validate_and_get_records_with_errors() {
        Ok(records) => records,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }));
        }
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        records,
        |state, _, records, _req| {
            revenue_recovery_data_backfill::revenue_recovery_data_backfill(
                state,
                records.records,
                cutoff_datetime,
            )
        },
        &auth::V2AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RecoveryDataBackfill))]
pub async fn revenue_recovery_data_backfill_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RecoveryDataBackfill;
    let connector_customer_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        connector_customer_id,
        |state, _: (), id, _| {
            revenue_recovery_data_backfill::unlock_connector_customer_status(state, id)
        },
        &auth::V2AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
