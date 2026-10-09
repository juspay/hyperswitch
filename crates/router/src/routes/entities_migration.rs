use actix_web::{web, HttpRequest, HttpResponse};
use api_models::entities_migration::EntitiesMigrationRequest;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, entities_migration},
    services::{api, authentication as auth},
};

pub async fn entities_migration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<EntitiesMigrationRequest>,
) -> HttpResponse {
    let flow = Flow::EntitiesMigration;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        &json_payload.into_inner(),
        |state, _, req, _| entities_migration::entities_migration(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
