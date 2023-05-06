mod utils;

#[actix_web::test]
async fn invalidate_cache_success() {
    utils::setup().await;

    //let key = "cachekey".to_string();
    let client = awc::Client::default();
    let cache_key = "cacheKey";
    let mut response = client
    .get(format!("http://127.0.0.1:8080/cache/invalidate/{cache_key}"))
    .send()
    .await
    .unwrap();

    let response_body = response.body().await;
    println!("invalidate Cache: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);
}
