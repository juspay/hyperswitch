//! Handlers for the mappers dictionary. The route tree that mounts them is in
//! [`crate::routes::app`].
//!
//! An entry is addressed by its `(name, key_)` in the path, for the reason a notification's
//! destination is: it keeps *which entry* answerable from an access log without anyone parsing a
//! body. Addressing it in a body instead was rejected for putting the entry's identity in a
//! different place on every route — and would have bought nothing, since a key that is not a bare
//! identifier survives percent-encoding, which `tests/config.rs` holds to.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::{dictionary::DictionaryUpsertRequest, UserName},
    },
    auth, services,
    state::AppState,
};

/// `GET /alerts/config/dictionary`.
pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::dictionary::list(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/config/dictionary/{name}/{key}`.
pub async fn read(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, key) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::dictionary::read(state, &name, &key).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/config/dictionary`.
pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<DictionaryUpsertRequest>,
) -> HttpResponse {
    // Read here and carried into the closure, which never sees the request — the same way the
    // notify handlers carry their destination. Reading it before `server_wrap` does not skip the
    // guard: a malformed header travels as a `Result` and is raised inside the closure, which runs
    // only after authentication has passed.
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::dictionary::upsert(state, payload, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `DELETE /alerts/config/dictionary/{name}/{key}`.
pub async fn retire(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, key) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::dictionary::retire(state, &name, &key).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}
