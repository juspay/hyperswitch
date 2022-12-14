use actix_web::web::{self, Data, Json};
use actix_web::Scope;
use router_core::connector::Connector;
use router_core::types;

async fn create(
    connector: Data<impl Connector>,
    Json(payment): Json<types::NewPayment>,
) -> Json<types::Payment> {
    dbg!(1);
    Json(connector.create_payment(payment).await)
}

async fn list() -> &'static str {
    "list"
}

async fn confirm(connector: Data<impl Connector>, Json(payment_id): Json<u64>) -> String {
    format!("{:?}", connector.verify_payment(payment_id).await)
}

pub fn mk_service<C: Connector + 'static>(connector: C) -> Scope {
    web::scope("/payments")
        .app_data(web::Data::new(connector))
        .service(web::resource("").route(web::post().to::<_, (Data<C>, _)>(create)))
        .service(web::resource("/list").route(web::get().to(list)))
        .service(web::resource("/confirm").route(web::post().to::<_, (Data<C>, _)>(confirm)))
}

#[cfg(test)]
mod tests {
    use actix_web::test::{self, TestRequest};
    use router_core::types::{NewPayment, Payment};
    use router_core::BigDecimal;

    #[actix_web::test]
    async fn payment_list() {
        let service = crate::mk_service().await;
        let request = TestRequest::get().uri("/list").to_request();

        assert_eq!(&test::call_and_read_body(&service, request).await[..], b"");
    }

    #[actix_web::test]
    async fn payment_create() {
        let service = crate::mk_service().await;

        let amount = BigDecimal::from(5);
        let request = TestRequest::post()
            .uri("/payments")
            .set_json(NewPayment { amount: amount.clone() })
            .to_request();

        let payment: Payment = test::call_and_read_body_json(&service, request).await;
        assert_eq!(payment.amount, amount);
    }
}
