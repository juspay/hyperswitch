use actix_web::{
    get,
    Responder,
    HttpResponse,
    body::{MessageBody, BoxBody},
    web
};

use crate::{
    core::payments::flow::show_payments,
    routes::app::Config
};

#[get("/")]
pub async fn default () -> impl Responder {
    http_response(format!("{}","UP"))
}

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok()
        .body("health is good")
}

#[get("/payments")]
pub async fn show_payment(
    data: web::Data<Config>
) -> impl Responder {
    
    let conn = data.conn.get().expect("unable to get");
    let body = serde_json::to_string(&show_payments(&conn)).unwrap();
    HttpResponse::Ok()
        .body(body)
}

fn http_response<T: MessageBody + 'static>(
    response: T
) -> HttpResponse<BoxBody> {
    HttpResponse::Ok().body(response)
}
