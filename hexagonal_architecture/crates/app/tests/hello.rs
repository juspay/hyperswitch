use actix_web::test::{call_and_read_body, TestRequest};

#[actix_web::test]
async fn test() {
    let service = hexagonal_architecture::mk_service().await;
    let request = TestRequest::get().uri("/").to_request();

    assert_eq!(&call_and_read_body(&service, request).await[..], b"hello :)");
}
