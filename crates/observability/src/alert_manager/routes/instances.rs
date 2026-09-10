//! Handlers for per-merchant alert instances and their per-dimension breakdown. The route tree
//! that mounts them is in [`crate::routes::app`].
//!
//! Four handlers over three tables. The channel is a path segment resolved once into a
//! [`Channel`](crate::alert_manager::types::lifecycle::Channel) and passed down, exactly as the
//! lifecycle routes do — the instance table comes once per channel, but nothing that decides
//! anything does. The breakdown has no channel in its path because it has no `_xyne` twin; see
//! [`crate::alert_manager::types::instances`].
//!
//! The announcement is a path segment too, and it is the only way to address these rows. Both
//! tables reference `alerts_main` `ON DELETE CASCADE`, so the API keeps that relationship rather
//! than asking callers to fill the column themselves.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::{
            instances::{DimensionWriteRequest, InstanceWriteRequest},
            lifecycle::Channel,
        },
    },
    auth, services,
    state::AppState,
};

/// `GET /alerts/instances/{channel}/{announcement_id}`.
pub async fn read_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, uuid::Uuid)>,
) -> HttpResponse {
    // Resolved here and carried into the closure, the way the lifecycle routes carry theirs. An
    // unknown channel travels as a `Result` and is raised inside the closure, which runs only
    // after authentication has passed.
    let (channel, announcement) = path.into_inner();
    let channel = Channel::from_path(&channel);

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::instances::read_instances(state, channel?, announcement).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/instances/{channel}/{announcement_id}`.
///
/// Everything the announcement was about, replacing whatever it already carried. `POST` rather
/// than `PUT` for consistency with every other write this service takes.
pub async fn write_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, uuid::Uuid)>,
    payload: web::Json<InstanceWriteRequest>,
) -> HttpResponse {
    let (channel, announcement) = path.into_inner();
    let channel = Channel::from_path(&channel);

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::instances::write_instances(state, channel?, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/dimensions/{announcement_id}`.
pub async fn read_dimensions(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> HttpResponse {
    let announcement = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::instances::read_dimensions(state, announcement).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/dimensions/{announcement_id}`.
///
/// The whole breakdown, replacing whatever the announcement already carried. A breakdown wider
/// than the configured cap is stored cut down to it rather than refused — see
/// [`core::instances`] for why, and for where the cut is recorded.
pub async fn write_dimensions(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<DimensionWriteRequest>,
) -> HttpResponse {
    let announcement = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::instances::write_dimensions(state, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
