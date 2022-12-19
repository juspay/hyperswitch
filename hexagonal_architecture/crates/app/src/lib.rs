//!
//! Hexagonal architecture is a software design pattern that helps to separate the business logic from the input/output logic.
//! It does this by defining interfaces for each external interaction, which can then be injected into services that use them.
//! This allows for testing with mocks and interchangeable implementations.
//!

#![forbid(unsafe_code)]
#![forbid(non_ascii_idents)]
#![warn(missing_docs)]
#![warn(clippy::use_self)]
#![warn(rust_2018_idioms)]
#![warn(missing_debug_implementations)]

/// Config.
pub mod config;
/// Connector.
pub mod connector;
/// Extension traits.
pub mod ext_traits;
/// Routes.
pub mod routes;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::App;

/// Create application builder.
pub fn mk_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().service(routes::payments::mk_service()).service(routes::mk_service())
}

/// Initialize service from application builder instance.
pub async fn mk_service() -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = ServiceResponse<impl MessageBody>,
    Error = actix_web::Error,
> {
    // init_service is a method used to create a Service for testing.
    // It takes a regular App builder as an argument.
    actix_web::test::init_service(mk_app()).await
}
