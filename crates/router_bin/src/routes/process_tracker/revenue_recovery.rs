use actix_web::{web, HttpRequest, HttpResponse};
use api_models::process_tracker::revenue_recovery as revenue_recovery_api;
use router_env::Flow;

use crate::{
    core::{api_locking, revenue_recovery},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

pub async fn revenue_recovery_pt_retrieve_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::GlobalPaymentId>,
) -> HttpResponse {
    let flow = Flow::RevenueRecoveryRetrieve;
    let id = path.into_inner();
    let payload = revenue_recovery_api::RevenueRecoveryId {
        revenue_recovery_id: id,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _: (), id, _| {
            revenue_recovery::retrieve_revenue_recovery_process_tracker(
                state,
                id.revenue_recovery_id,
            )
        },
        &auth::JWTAuth {
            permission: Permission::ProfileRevenueRecoveryRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
