use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, revenue_recovery::api as revenue_recovery_api},
    routes::AppState,
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::RevenueRecoveryCancel))]
pub async fn revenue_recovery_cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::PaymentReferenceId>,
) -> impl Responder {
    let flow = Flow::RevenueRecoveryCancel;
    let merchant_reference_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            revenue_recovery_api::cancel_revenue_recovery_core(
                state,
                auth.platform,
                auth.profile,
                merchant_reference_id.clone(),
            )
        },
        &auth::V2ApiKeyAuth {
            allow_connected_scope_operation: false,
            allow_platform_self_operation: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
