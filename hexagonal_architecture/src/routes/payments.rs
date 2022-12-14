use actix_web::{web, Scope};

async fn create() -> &'static str {
    ""
}

async fn list() -> &'static str {
    ""
}

async fn confirm() -> &'static str {
    ""
}

pub fn mk_service() -> Scope {
    web::scope("/payments")
        .service(web::resource("/").route(web::post().to(create)))
        .service(web::resource("/list").route(web::get().to(list)))
        .service(web::resource("/confirm").route(web::post().to(confirm)))
}

#[cfg(test)]
mod tests {
    use actix_web::test::{self, TestRequest};

    #[actix_web::test]
    async fn payment_list() {
        let service = crate::mk_service().await;
        let request = TestRequest::get().uri("/list").to_request();

        assert_eq!(&test::call_and_read_body(&service, request).await[..], b"");
    }
}
