//! The whole route tree, in one place.
//!
//! Every route this service serves is visible in this file — mirroring
//! `crates/router/src/routes/app.rs`, where each area is a unit struct with a `server()` returning
//! its [`Scope`]. Handlers live in the sibling modules; only the shape of the tree lives here, so
//! "what does this service expose, and what guards it?" is one file, not a search.
//!
//! Exposed as `Scope` factories rather than as an assembled `App` so that the standalone binary
//! and a future in-router mount share one definition and cannot drift.

use actix_multipart::form::MultipartFormConfig;
use actix_web::{error::InternalError, web, HttpResponse, Scope};

use crate::{
    errors::types::{ApiError, ApiErrorResponse},
    logger,
    routes::{config, health_check, instances, lifecycle, mappers, notifications, notify},
    state::AppState,
};

const MAX_LIFECYCLE_STATE_BODY_BYTES: usize = 16 * 1024 * 1024;

/// The service's routes, all of them behind the internal API key.
pub struct Alerts;

impl Alerts {
    /// Build the guarded scope.
    ///
    /// Everything mounted here is authenticated. Anything that must be reachable without
    /// credentials belongs in [`Health`] instead — not as a path exception inside a guard, which
    /// is where auth bypasses come from.
    ///
    /// The path carries both facts: which channel, and which destination within it. The body is
    /// content only. See [`crate::types`] for why the destination is not a body field, and why a
    /// provider refusing a message comes back as a `200` rather than an error.
    pub fn server(state: AppState) -> Scope {
        let max_upload_bytes = state.conf.chat.get_inner().max_upload_bytes;

        web::scope("/alerts")
            .app_data(web::Data::new(state))
            .app_data(json_config())
            .app_data(path_config())
            .app_data(query_config())
            .app_data(multipart_config(max_upload_bytes))
            .service(
                web::scope("/chat")
                    .service(
                        web::resource("/notify/{destination}").route(web::post().to(notify::chat)),
                    )
                    .service(
                        web::resource("/upload/{destination}")
                            .route(web::post().to(notify::chat_upload)),
                    ),
            )
            .service(web::scope("/email").service(
                web::resource("/notify/{destination}").route(web::post().to(notify::email)),
            ))
            .service(AlertsConfig::server())
            .service(AlertsLifecycle::server())
            .service(AlertsInstances::server())
            .service(AlertsDimensions::server())
    }
}

pub struct AlertsConfig;

impl AlertsConfig {
    pub fn server() -> Scope {
        web::scope("/config")
            .service(
                web::scope("/definitions")
                    .service(
                        web::resource("")
                            .route(web::get().to(config::definition_list))
                            .route(web::post().to(config::definition_create)),
                    )
                    .service(
                        web::resource("/{id}")
                            .route(web::get().to(config::definition_retrieve))
                            .route(web::post().to(config::definition_update)),
                    ),
            )
            .service(
                web::scope("/mappers")
                    .service(
                        web::resource("")
                            .route(web::get().to(mappers::mapper_list))
                            .route(web::post().to(mappers::mapper_save)),
                    )
                    .service(
                        web::resource("/{name}/{key}")
                            .route(web::get().to(mappers::mapper_retrieve))
                            .route(web::delete().to(mappers::mapper_delete)),
                    ),
            )
            .service(
                web::scope("/enablement")
                    .service(web::resource("").route(web::get().to(config::enablement_list)))
                    .service(
                        web::resource("/{name}/{product}")
                            .route(web::get().to(config::enablement_retrieve))
                            .route(web::post().to(config::enablement_upsert)),
                    ),
            )
            .service(
                web::scope("/merchant-thresholds")
                    .service(
                        web::resource("")
                            .route(web::get().to(config::merchant_threshold_list))
                            .route(web::post().to(config::merchant_threshold_upsert)),
                    )
                    .service(
                        web::resource("/{id}")
                            .route(web::get().to(config::merchant_threshold_retrieve))
                            .route(web::post().to(config::merchant_threshold_update))
                            .route(web::delete().to(config::merchant_threshold_delete)),
                    ),
            )
            .service(
                web::scope("/notifications").service(
                    web::resource("/read")
                        .route(web::get().to(notifications::notification_watermark_retrieve))
                        .route(web::post().to(notifications::notification_watermark_upsert)),
                ),
            )
    }
}

pub struct AlertsLifecycle;

impl AlertsLifecycle {
    pub fn server() -> Scope {
        web::scope("/lifecycle").service(
            web::scope("/{channel}")
                .service(
                    web::resource("/state")
                        .app_data(json_config().limit(MAX_LIFECYCLE_STATE_BODY_BYTES))
                        .route(web::get().to(lifecycle::lifecycle_state_retrieve))
                        .route(web::post().to(lifecycle::lifecycle_state_save)),
                )
                .service(
                    web::resource("/announcements")
                        .route(web::get().to(lifecycle::announcement_list))
                        .route(web::post().to(lifecycle::announcement_create)),
                )
                .service(
                    web::resource("/announcements/{id}")
                        .route(web::post().to(lifecycle::announcement_update)),
                ),
        )
    }
}

pub struct AlertsInstances;

impl AlertsInstances {
    pub fn server() -> Scope {
        web::scope("/instances").service(
            web::scope("/{channel}").service(
                web::resource("/{announcement_id}")
                    .route(web::get().to(instances::instances_retrieve))
                    .route(web::post().to(instances::instances_save)),
            ),
        )
    }
}

pub struct AlertsDimensions;

impl AlertsDimensions {
    pub fn server() -> Scope {
        web::scope("/dimensions").service(
            web::scope("/{channel}").service(
                web::resource("/{announcement_id}")
                    .route(web::get().to(instances::dimensions_retrieve))
                    .route(web::post().to(instances::dimensions_save)),
            ),
        )
    }
}

/// Make a malformed body render like every other error this service returns.
///
/// Without this, actix answers its own plain-text 400 and a caller has two error formats to parse
/// depending on how wrong it got things.
///
/// The parse failure itself goes to the log and not to the client, following
/// [`crate::services::server_wrap`]: serde's message quotes the offending part of the body, and a
/// body here carries merchant ids and payment volumes.
fn json_config() -> web::JsonConfig {
    web::JsonConfig::default().error_handler(|error, request| {
        logger::warn!(
            path = %request.path(),
            error = %error,
            "Request rejected: the body could not be parsed"
        );

        ApiErrorResponse::BadRequest(ApiError::new(
            "IR",
            4,
            "The request body could not be parsed",
        ))
        .into()
    })
}

fn path_config() -> web::PathConfig {
    web::PathConfig::default().error_handler(|error, request| {
        logger::warn!(
            path = %request.path(),
            error = %error,
            "Request rejected: a path segment could not be parsed"
        );

        InternalError::from_response(error, HttpResponse::NotFound().finish()).into()
    })
}

fn query_config() -> web::QueryConfig {
    web::QueryConfig::default().error_handler(|error, request| {
        logger::warn!(
            path = %request.path(),
            error = %error,
            "Request rejected: the query string could not be parsed"
        );

        ApiErrorResponse::BadRequest(ApiError::new(
            "IR",
            6,
            "The query string could not be parsed",
        ))
        .into()
    })
}

fn multipart_config(max_upload_bytes: usize) -> MultipartFormConfig {
    MultipartFormConfig::default()
        .total_limit(max_upload_bytes)
        .memory_limit(max_upload_bytes)
        .error_handler(|error, request| {
            logger::warn!(
                path = %request.path(),
                error = %error,
                "Request rejected: the multipart body could not be parsed"
            );
            ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                4,
                "The request body could not be parsed",
            ))
            .into()
        })
}

/// Liveness and database readiness, deliberately unauthenticated.
pub struct Health;

impl Health {
    /// Build the unguarded health scope.
    ///
    /// Separate from [`Alerts`] because probes do not carry credentials. Keeping it a distinct
    /// scope makes "unauthenticated" a structural property visible right here in the route tree,
    /// rather than a condition buried in a guard — so it stays true when someone adds `/healthz`,
    /// a trailing slash, or a route next to this one.
    pub fn server(state: AppState) -> Scope {
        web::scope("health")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(health_check::health)))
            .service(web::resource("/ready").route(web::get().to(health_check::deep_health_check)))
    }
}
