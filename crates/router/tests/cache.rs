#![allow(clippy::unwrap_used)]
use router::{configs::settings::Settings, routes, services};
use storage_impl::redis::cache;

mod utils;

#[actix_web::test]
async fn invalidate_existing_cache_success() {
    // Arrange
    Box::pin(utils::setup()).await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let state = Box::pin(routes::AppState::new(
        Settings::default(),
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;

    let cache_key = "cacheKey".to_string();
    let cache_key_value = "val".to_string();
    let _ = state
        .store
        .get_redis_conn()
        .unwrap()
        .set_key(&cache_key.clone(), cache_key_value.clone())
        .await;

    let api_key = ("api-key", "test_admin");
    let client = awc::Client::default();

    cache::CONFIG_CACHE
        .push(cache_key.clone(), cache_key_value.clone())
        .await;

    cache::ACCOUNTS_CACHE
        .push(cache_key.clone(), cache_key_value.clone())
        .await;

    // Act
    let mut response = client
        .post(format!(
            "http://127.0.0.1:8080/cache/invalidate/{cache_key}"
        ))
        .insert_header(api_key)
        .send()
        .await
        .unwrap();

    // Assert
    let response_body = response.body().await;
    println!("invalidate Cache: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::OK);
    assert!(cache::CONFIG_CACHE.get(&cache_key).await.is_none());
    assert!(cache::ACCOUNTS_CACHE.get(&cache_key).await.is_none());
}

#[actix_web::test]
async fn invalidate_non_existing_cache_success() {
    // Arrange
    Box::pin(utils::setup()).await;
    let cache_key = "cacheKey".to_string();
    let api_key = ("api-key", "test_admin");
    let client = awc::Client::default();

    // Act
    let mut response = client
        .post(format!(
            "http://127.0.0.1:8080/cache/invalidate/{cache_key}"
        ))
        .insert_header(api_key)
        .send()
        .await
        .unwrap();

    // Assert
    let response_body = response.body().await;
    println!("invalidate Cache: {response:?} : {response_body:?}");
    assert_eq!(response.status(), awc::http::StatusCode::NOT_FOUND);
}
