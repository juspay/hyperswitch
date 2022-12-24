// use actix_web::http::header;

pub fn cors() -> actix_cors::Cors {
    actix_cors::Cors::permissive() // FIXME : Never use in  production

    /*
    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
    .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE]);
    if CONFIG.profile == "debug" {         //   --------->>>  FIXME: It should be conditional
        cors.allowed_origin_fn(|origin, _req_head| {
            origin.as_bytes().starts_with(b"http://localhost")
        })
    } else {

    FIXME : I don't know what to put here
    .allowed_origin_fn(|origin, _req_head| origin.as_bytes().starts_with(b"http://localhost"))
    }
    */
}
