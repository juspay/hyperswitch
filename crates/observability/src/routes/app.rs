//! The whole route tree, in one place.

use actix_multipart::form::MultipartFormConfig;
use actix_web::{web, Scope};

use crate::{
    errors::types::{ApiError, ApiErrorResponse},
    logger,
    routes::{health_check, notify},
    state::AppState,
};

/// The service's routes, all of them behind the internal API key.
pub struct Alerts;

impl Alerts {
    /// Build the guarded scope.
    pub fn server(state: AppState) -> Scope {
        let max_upload_bytes = state.conf.chat.get_inner().max_upload_bytes;

        web::scope("/alerts")
            .app_data(web::Data::new(state))
            .app_data(json_config())
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
    }
}

/// Make a malformed body render like every other error this service returns.
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

/// Liveness, deliberately unauthenticated.
pub struct Health;

impl Health {
    /// Build the unguarded health scope.
    pub fn server(state: AppState) -> Scope {
        web::scope("health")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(health_check::health)))
            .service(web::resource("/ready").route(web::get().to(health_check::deep_health_check)))
    }
}
