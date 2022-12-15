use actix_web::web::{self, Data, Json};
use actix_web::Scope;
use router_core::connector::Connector;
use router_core::types;

async fn create(
    connector: Data<impl Connector>,
    Json(payment): Json<types::NewPayment>,
) -> Json<types::Payment> {
    Json(connector.create_payment(payment).await)
}

async fn list() -> Json<Vec<types::Payment>> {
    Json(vec![])
}

async fn confirm(
    connector: Data<impl Connector>,
    Json(payment_id): Json<u64>,
) -> Json<types::Verify> {
    Json(connector.verify_payment(payment_id).await)
}

pub fn mk_service<C: Connector>(connector: C) -> Scope {
    web::scope("/payments")
        .app_data(web::Data::new(connector))
        .service(web::resource("").route(web::post().to::<_, (Data<C>, _)>(create)))
        .service(web::resource("/list").route(web::get().to(list)))
        .service(web::resource("/confirm").route(web::post().to::<_, (Data<C>, _)>(confirm)))
}

#[cfg(test)]
mod tests {
    use actix_web::test::{self, TestRequest};
    use actix_web::web::{Data, Json};
    use router_core::connector::MockConnector;
    use router_core::types::{NewPayment, Payment, Verify};
    use router_core::BigDecimal;

    #[actix_web::test]
    async fn payment_list() {
        let service = crate::mk_service().await;
        let request = TestRequest::get().uri("/payments/list").to_request();

        assert_eq!(
            &test::call_and_read_body_json::<_, _, Vec<Payment>>(&service, request).await[..],
            Vec::new()
        );
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

    #[actix_web::test]
    async fn payment_create2() {
        let mut connector = MockConnector::new();

        connector.expect_create_payment().return_once(|new| Payment { id: 42, amount: new.amount });
        connector.expect_verify_payment().return_once(|_| Verify::Ok);

        let connector = Data::new(connector);

        let json = Json(NewPayment { amount: BigDecimal::from(15) });
        let payment = super::create(connector.clone(), json).await;

        assert_eq!(payment.id, 42);
        assert_eq!(payment.amount, BigDecimal::from(15));

        let confirm = super::confirm(connector.clone(), Json(payment.id)).await;
        assert_eq!(*confirm, Verify::Ok);
    }
}
