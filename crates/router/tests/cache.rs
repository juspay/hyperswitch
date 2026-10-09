use std::sync::Arc;

use router::{configs::settings::Settings, routes, services};
use storage_impl::redis::cache::CacheKey;

mod utils;

#[actix_web::test]
async fn invalidate_existing_cache_success() {
    // Arrange
    Box::pin(utils::setup()).await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let app_state = Box::pin(routes::AppState::new(
        Settings::default(),
        tx,
        Box::new(services::MockApiClient),
        env!("CARGO_PKG_NAME"),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();
    let cache_key = "cacheKey".to_string();
    let cache_key_value = "val".to_string();
    let _ = state
        .store
        .get_redis_conn()
        .unwrap()
        .set_key(&cache_key.clone().into(), cache_key_value.clone())
        .await;

    let api_key = ("api-key", "test_admin");
    let client = awc::Client::default();

    state
        .store
        .caches()
        .config
        .push(
            CacheKey {
                key: cache_key.clone(),
                prefix: String::default(),
            },
            cache_key_value.clone(),
        )
        .await;

    state
        .store
        .caches()
        .accounts
        .push(
            CacheKey {
                key: cache_key.clone(),
                prefix: String::default(),
            },
            cache_key_value.clone(),
        )
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
    assert!(state
        .store
        .caches()
        .config
        .get_val::<String>(CacheKey {
            key: cache_key.clone(),
            prefix: String::default()
        })
        .await
        .is_none());
    assert!(state
        .store
        .caches()
        .accounts
        .get_val::<String>(CacheKey {
            key: cache_key,
            prefix: String::default()
        })
        .await
        .is_none());
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
