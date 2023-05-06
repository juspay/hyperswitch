use actix_web::web;
use router_env::{instrument, tracing};


#[instrument(skip_all)]
pub async fn invalidate(key: web::Path<String>) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok().body(format!("cache endpoint reached with `{}`", key))
}