//! Handlers for the merchant alert delivery switch routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::merchants_alert_external_config::{
    MerchantsAlertExternalConfigCreateRequest, MerchantsAlertExternalConfigKey,
    MerchantsAlertExternalConfigListRequest, MerchantsAlertExternalConfigUpdateRequest,
};

use crate::{auth, core, services, state::AppState};

/// `POST /alerts/alerts_manager/external_config`.
pub async fn create(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MerchantsAlertExternalConfigCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_manager::merchants_alert_external_config::create_merchant_alert_external_config,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/alerts_manager/external_config/list`.
pub async fn list(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MerchantsAlertExternalConfigListRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_manager::merchants_alert_external_config::list_merchant_alert_external_configs,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/alerts_manager/external_config/{name}/{product}`.
pub async fn retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<MerchantsAlertExternalConfigKey>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        path.into_inner(),
        core::alert_manager::merchants_alert_external_config::retrieve_merchant_alert_external_config,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/alerts_manager/external_config/{name}/{product}`.
pub async fn update(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<MerchantsAlertExternalConfigKey>,
    payload: web::Json<MerchantsAlertExternalConfigUpdateRequest>,
) -> HttpResponse {
    let MerchantsAlertExternalConfigKey { name, product } = path.into_inner();
    let mut payload = payload.into_inner();
    payload.name = name;
    payload.product = product;

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload,
        core::alert_manager::merchants_alert_external_config::update_merchant_alert_external_config,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `DELETE /alerts/alerts_manager/external_config/{name}/{product}`.
pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<MerchantsAlertExternalConfigKey>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        path.into_inner(),
        core::alert_manager::merchants_alert_external_config::delete_merchant_alert_external_config,
        &auth::InternalApiKeyAuth,
    )
    .await
}
