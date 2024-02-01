use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, locker_migration},
    services::{api, authentication as auth},
};

/// Handles the Rust Locker Migration API endpoint.
///
/// This method takes in the Appstate, HttpRequest, and merchant ID as parameters and returns an HttpResponse. It creates a flow for Rust Locker Migration, retrieves the merchant ID from the path, and then uses the api::server_wrap function to wrap the locker_migration::rust_locker_migration function call with the necessary state, request, merchant ID, authentication, and locking action parameters. The function returns the result of the wrapped function call as an asynchronous response.
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
        |state, _, _| locker_migration::rust_locker_migration(state, &merchant_id),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
