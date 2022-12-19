use actix_web::web::{self, Data, Json};
use actix_web::{HttpRequest, Scope};
use router_core::{payments, types};

use crate::connector::Stripe;
use crate::ext_traits::HttpRequestExt;

async fn create(req: HttpRequest, Json(payment): Json<types::NewPayment>) -> Json<types::Payment> {
    Json(payments::create(req.payments(), req.connector(), payment).await)
}

async fn list(req: HttpRequest) -> Json<Vec<types::Payment>> {
    Json(payments::list(req.payments()).await)
}

async fn confirm(req: HttpRequest, Json(payment_id): Json<u64>) -> Json<types::Verify> {
    Json(payments::confirm(req.payments(), req.connector(), payment_id).await)
}

/// Creates scope for `/payments` path prefix.
pub fn mk_service() -> Scope {
    let connector = Stripe {};
    let payments = memory_adapter::InMemoryPayments::default();

    web::scope("/payments")
        .app_data(Data::new(payments))
        .app_data(Data::new(connector))
        .service(web::resource("").route(web::post().to(create)))
        .service(web::resource("/list").route(web::get().to(list)))
        .service(web::resource("/confirm").route(web::post().to(confirm)))
}
