// use actix_web::http::header;

use crate::configs::settings;

pub fn cors(config: settings::CorsSettings) -> actix_cors::Cors {
    let allowed_methods = config.allowed_methods.iter().map(|s| s.as_str());

    let mut cors = actix_cors::Cors::default()
        .allowed_methods(allowed_methods)
        .max_age(config.max_age);

    if config.wildcard_origin {
        cors = cors.allow_any_origin()
    } else {
        for origin in &config.origins {
            cors = cors.allowed_origin(origin);
        }
    }

    cors
}
