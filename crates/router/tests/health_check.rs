mod utils;

#[actix_web::test]
async fn health_check() {
    utils::setup().await;
    let client = awc::Client::default();

    let response = client
        .get("http://127.0.0.1:8080/health")
        .send()
        .await
        .unwrap();
    println!("{:?}", response);
    assert_eq!(response.status(), awc::http::StatusCode::OK)
}

#[actix_web::test]
async fn health_check_root() {
    utils::setup().await;
    let client = awc::Client::default();

    let response = client.get("http://127.0.0.1:8080/").send().await.unwrap();
    println!("{:?}", response);
    // assert_eq!(response.status(), awc::http::StatusCode::OK)
}
