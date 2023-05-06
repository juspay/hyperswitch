use router_env::{instrument, tracing};


#[instrument(skip_all)]
pub async fn invalidate() -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok().body("cache endpoint reached")
}