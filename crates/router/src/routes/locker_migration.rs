use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, locker_migration},
    services::{api, authentication as auth},
};

pub async fn rust_locker_migration(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RustLockerMigration;
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        &merchant_id,
        |state, _, _, _| locker_migration::rust_locker_migration(state, &merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
