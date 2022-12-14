use actix_web::{web, Scope};

pub mod payments;

pub fn mk_service() -> Scope {
    web::scope("").service(web::resource("/").to(hello))
}

async fn hello() -> &'static str {
    "hello :)"
}

#[cfg(test)]
mod tests {
    use actix_web::test::{self, TestRequest};

    #[actix_web::test]
    async fn hello() {
        let service = crate::mk_service().await;
        let request = TestRequest::get().uri("/").to_request();

        assert_eq!(&test::call_and_read_body(&service, request).await[..], b"hello :)");
    }
}
