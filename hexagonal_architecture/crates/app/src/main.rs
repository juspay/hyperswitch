mod config;
mod connector;
mod routes;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use config::Config;

    let proof = config::dotenv_with_proof().unwrap();
    let config = Config::new(&proof).unwrap();

    HttpServer::new(mk_app).bind(config.application_url)?.run().await
}

fn mk_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    use crate::connector::Stripe;

    let connector = Stripe {};
    App::new().service(routes::payments::mk_service(connector)).service(routes::mk_service())
}

#[cfg(test)]
async fn mk_service() -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = ServiceResponse<impl MessageBody>,
    Error = actix_web::Error,
> {
    actix_web::test::init_service(mk_app()).await
}
