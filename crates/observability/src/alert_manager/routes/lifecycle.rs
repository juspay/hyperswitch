//! Handlers for alert lifecycle state and announcements. The route tree that mounts them is in
//! [`crate::routes::app`].
//!
//! Three handlers, not six. The channel is a path segment, resolved once into a
//! [`Channel`](crate::alert_manager::types::lifecycle::Channel) and passed down — the tables come
//! once per channel, but nothing that decides anything does.
//!
//! The channel is in the path for the reason a notify destination is: it keeps *which channel* a
//! request was for answerable from an access log, without anyone parsing a body, and it makes a
//! body naming one channel on a route serving the other unrepresentable.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::lifecycle::{AnnouncementRequest, Channel, LifecycleStateWriteRequest},
    },
    auth, services,
    state::AppState,
};

/// `GET /alerts/lifecycle/{channel}/state`.
pub async fn read_state(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    // Resolved here and carried into the closure, the same way the dictionary carries its user
    // name. An unknown channel travels as a `Result` and is raised inside the closure, which runs
    // only after authentication has passed.
    let channel = Channel::from_path(&path.into_inner());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::lifecycle::read_state(state, channel?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/lifecycle/{channel}/state`.
///
/// The whole state, applied in one transaction. `POST` rather than `PUT` for consistency with
/// every other write this service takes, even though this one really is a replacement.
pub async fn write_state(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<LifecycleStateWriteRequest>,
) -> HttpResponse {
    let channel = Channel::from_path(&path.into_inner());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::lifecycle::write_state(state, channel?, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/lifecycle/{channel}/announcements`.
///
/// An append. There is no route that removes one: a state row references an announcement
/// `ON DELETE CASCADE`, so removing an announcement would take the state pointing at it with it.
pub async fn record_announcement(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<AnnouncementRequest>,
) -> HttpResponse {
    let channel = Channel::from_path(&path.into_inner());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::lifecycle::record_announcement(state, channel?, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
