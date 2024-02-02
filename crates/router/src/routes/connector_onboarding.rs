use actix_web::{web, HttpRequest, HttpResponse};
use api_models::connector_onboarding as api_types;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, connector_onboarding as core},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

/// Asynchronously handles the request to retrieve an action URL by taking in the app state, HTTP request, and JSON payload. It creates a flow of type `GetActionUrl`, extracts the request payload from the JSON payload, and wraps the server API call using the `api::server_wrap` function. It awaits the result of the wrapped API call and returns the HTTP response.
pub async fn get_action_url(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<api_types::ActionUrlRequest>,
) -> HttpResponse {
    let flow = Flow::GetActionUrl;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        core::get_action_url,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously handles the syncing of onboarding status by taking in the application state, HTTP request,
/// and JSON payload, and returning an HTTP response. It creates a flow for syncing onboarding status, extracts
/// the request payload, and calls the server_wrap function to handle the synchronization process, using the
/// provided application state, HTTP request, request payload, onboarding status synchronization function,
/// JWT authentication with MerchantAccountWrite permission, and a lock action of NotApplicable. It returns
/// the resulting HTTP response.
pub async fn sync_onboarding_status(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<api_types::OnboardingSyncRequest>,
) -> HttpResponse {
    let flow = Flow::SyncOnboardingStatus;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        core::sync_onboarding_status,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously handles a POST request to reset the tracking ID. This method extracts the AppState, HttpRequest, and request payload from the parameters, creates a Flow enum instance for the reset tracking ID operation, and then calls the server_wrap function from the api module with the necessary parameters to process the request. The server_wrap function wraps the core reset_tracking_id function with authentication, permission checking, and locking mechanisms before awaiting the result and returning the HttpResponse.
pub async fn reset_tracking_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<api_types::ResetTrackingIdRequest>,
) -> HttpResponse {
    let flow = Flow::ResetTrackingId;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        core::reset_tracking_id,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
