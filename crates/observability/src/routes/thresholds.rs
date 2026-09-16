//! Authenticated threshold override handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::thresholds::{ThresholdDeleteRequest, ThresholdUpsertRequest};
use error_stack::{report, ResultExt};
use serde::de::DeserializeOwned;

use crate::{
    auth, core,
    errors::{ObservabilityApiResult, ObservabilityError},
    services,
    state::AppState,
};

fn deserialize<T: DeserializeOwned>(payload: &[u8]) -> ObservabilityApiResult<T> {
    serde_json::from_slice(payload)
        .map_err(|_| report!(ObservabilityError::InvalidRequest))
        .attach_printable("The request body could not be parsed")
}

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::thresholds::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Bytes,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload,
        |state, payload| async move {
            core::thresholds::upsert(state, deserialize::<ThresholdUpsertRequest>(&payload)?).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Bytes,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload,
        |state, payload| async move {
            core::thresholds::delete(state, deserialize::<ThresholdDeleteRequest>(&payload)?).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
