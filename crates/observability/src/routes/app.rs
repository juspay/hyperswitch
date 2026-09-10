use actix_multipart::form::MultipartFormConfig;
use actix_web::{web, Scope};

use crate::{
    alert_manager::routes::{config, dictionary, instances, lifecycle, notifications},
    errors::types::{ApiError, ApiErrorResponse},
    logger,
    routes::{health_check, notify},
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
            .service(lifecycle_scope())
            .service(instances_scope())
            .service(dimensions_scope())
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
            web::scope("/dictionary")
                .service(
                    web::resource("")
                        .route(web::get().to(dictionary::list))
                        .route(web::post().to(dictionary::upsert)),
                )
                .service(
                    web::resource("/{name}/{key}")
                        .route(web::get().to(dictionary::read))
                        .route(web::delete().to(dictionary::retire)),
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
        .service(
            web::scope("/notifications").service(
                web::resource("/read")
                    .route(web::get().to(notifications::read))
                    .route(web::post().to(notifications::mark_read)),
            ),
        )
}

fn lifecycle_scope() -> Scope {
    web::scope("/lifecycle").service(
        web::scope("/{channel}")
            .service(
                web::resource("/state")
                    .route(web::get().to(lifecycle::read_state))
                    .route(web::post().to(lifecycle::write_state)),
            )
            .service(
                web::resource("/announcements")
                    .route(web::post().to(lifecycle::record_announcement)),
            ),
    )
}

fn instances_scope() -> Scope {
    web::scope("/instances").service(
        web::scope("/{channel}").service(
            web::resource("/{announcement_id}")
                .route(web::get().to(instances::read_instances))
                .route(web::post().to(instances::write_instances)),
        ),
    )
}

fn dimensions_scope() -> Scope {
    web::scope("/dimensions").service(
        web::resource("/{announcement_id}")
            .route(web::get().to(instances::read_dimensions))
            .route(web::post().to(instances::write_dimensions)),
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
