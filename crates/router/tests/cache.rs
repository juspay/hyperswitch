use router::cache::{self};

mod utils;

#[actix_web::test]
async fn invalidate_in_memory_cache_success() {
    // Arrange
    utils::setup().await;

    let api_key = ("api-key", "test_admin");
    let client = awc::Client::default();
    let cache_key = "cacheKey".to_string();
    cache::CONFIG_CACHE.push(cache_key.clone(), "val".to_string()).await;

    // Act
    let mut response = client
    .post(format!("http://127.0.0.1:8080/cache/invalidate/{cache_key}"))
    .insert_header(api_key)
    .send()
    .await
    .unwrap();

    // Assert
    let response_body = response.body().await;
    println!("invalidate Cache: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);
    assert_eq!(cache::CONFIG_CACHE.get(&cache_key).is_some(), false);    
}
