use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, apple_pay_certificates_migration},
    services::{api, authentication as auth},
};

pub async fn apple_pay_certificates_migration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<
        api_models::apple_pay_certificates_migration::ApplePayCertificatesMigrationRequest,
    >,
) -> HttpResponse {
    let flow = Flow::ApplePayCertificatesMigration;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        &json_payload.into_inner(),
        |state, _, req, _| {
            apple_pay_certificates_migration::apple_pay_certificates_migration(state, req)
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
