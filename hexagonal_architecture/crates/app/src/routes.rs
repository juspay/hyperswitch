use actix_web::{web, Scope};

/// Payments routes.
pub mod payments;

/// Creates scope for common path prefix.
pub fn mk_service() -> Scope {
    web::scope("").service(web::resource("/").to(hello))
}

async fn hello() -> &'static str {
    "hello :)"
}
