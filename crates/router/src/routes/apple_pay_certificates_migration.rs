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
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::ApplePayCertificatesMigration;
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        &merchant_id,
        |state, _, _, _| {
            apple_pay_certificates_migration::apple_pay_certificates_migration(state, &merchant_id)
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
