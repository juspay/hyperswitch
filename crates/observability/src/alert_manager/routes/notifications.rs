//! Handlers for the notification bell's read watermark. The route tree that mounts them is in
//! [`crate::routes::app`].
//!
//! Both routes address the same resource — one user's watermark — so both read the same header for
//! the same reason. See [`crate::alert_manager::types::UserName`] for why the user is not in the path.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{core, types::UserName},
    auth, services,
    state::AppState,
};

/// `GET /alerts/config/notifications/read`.
pub async fn read(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::read(state, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/config/notifications/read`.
///
/// No body. What to write is not a caller's choice — the row is stamped with this service's clock
/// — and the only other thing the write needs is the user, which is a header.
pub async fn mark_read(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::mark_read(state, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}
