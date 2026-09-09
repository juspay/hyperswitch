use actix_multipart::form::MultipartFormConfig;
use actix_web::{web, Scope};

use crate::{
    errors::types::{ApiError, ApiErrorResponse},
    logger,
    routes::{config, health_check, notify},
    state::AppState,
};

pub struct Alerts;

impl Alerts {
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
            .service(config_scope())
    }
}

fn config_scope() -> Scope {
    web::scope("/config")
        .service(
            web::scope("/definitions")
                .service(
                    web::resource("")
                        .route(web::get().to(config::list_definitions))
                        .route(web::post().to(config::create_definition)),
                )
                .service(
                    web::resource("/{id}")
                        .route(web::get().to(config::read_definition))
                        .route(web::post().to(config::update_definition)),
                ),
        )
        .service(
            web::scope("/enablement")
                .service(web::resource("").route(web::get().to(config::list_enablements)))
                .service(
                    web::resource("/{name}/{product}")
                        .route(web::get().to(config::read_enablement))
                        .route(web::post().to(config::upsert_enablement)),
                ),
        )
}

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

pub struct Health;

impl Health {
    pub fn server(state: AppState) -> Scope {
        web::scope("health")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(health_check::health)))
            .service(web::resource("/ready").route(web::get().to(health_check::deep_health_check)))
    }
}
