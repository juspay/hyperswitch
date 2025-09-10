use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::revenue_recovery_data_backfill::RevenueRecoveryDataBackfillForm;
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, revenue_recovery_data_backfill},
    routes::AppState,
    services::{api, authentication as auth},
    types::domain,
};

#[instrument(skip_all, fields(flow = ?Flow::RecoveryDataBackfill))]
pub async fn revenue_recovery_data_backfill(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<RevenueRecoveryDataBackfillForm>,
) -> HttpResponse {
    let flow = Flow::RecoveryDataBackfill;

    let records = match form.validate_and_get_records() {
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
            revenue_recovery_data_backfill::revenue_recovery_data_backfill(state, records)
        },
        &auth::V2AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
