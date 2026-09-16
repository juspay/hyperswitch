//! Handlers for the per-merchant threshold override routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::merchant_thresholds::{
    MerchantThresholdsDeleteByFilterRequest, MerchantThresholdsDeleteRequest,
    MerchantThresholdsListRequest, MerchantThresholdsUpdateRequest,
    MerchantThresholdsUpsertRequest,
};

use crate::{auth, core, services, state::AppState};

/// `POST /alerts/alerts_manager/merchant_thresholds`.
pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MerchantThresholdsUpsertRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_manager::merchant_thresholds::upsert_merchant_threshold,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/alerts_manager/merchant_thresholds/list`.
pub async fn list(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MerchantThresholdsListRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_manager::merchant_thresholds::list_merchant_thresholds,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/alerts_manager/merchant_thresholds/update`.
pub async fn update(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MerchantThresholdsUpdateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_manager::merchant_thresholds::update_merchant_thresholds,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/alerts_manager/merchant_thresholds/delete`.
pub async fn delete_by_filter(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MerchantThresholdsDeleteByFilterRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_manager::merchant_thresholds::delete_merchant_thresholds_by_filter,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `DELETE /alerts/alerts_manager/merchant_thresholds/{id}`.
pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        MerchantThresholdsDeleteRequest {
            id: id.into_inner(),
        },
        core::alert_manager::merchant_thresholds::delete_merchant_threshold,
        &auth::InternalApiKeyAuth,
    )
    .await
}
