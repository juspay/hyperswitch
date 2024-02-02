use std::sync::atomic;

use router::{configs::settings::Settings, routes, services};

mod utils;

#[tokio::test]
#[should_panic]
/// Asynchronously performs the following steps:
/// 1. Calls `utils::setup()` to set up the utility functions.
/// 2. Creates a oneshot channel `tx` using `tokio::sync::oneshot::channel()`.
/// 3. Initializes the application state `state` using `routes::AppState::new()` with default settings, `tx`, and a mock API client.
/// 4. Retrieves the Redis connection from the application state and sets `is_redis_available` to `false` using `atomic::Ordering::SeqCst`.
/// 5. Retrieves the Redis connection from the application state again.
/// 
/// This method does not have explicit assertions, but it could potentially lead to a panic based on the `#[should_panic]` attribute.
async fn get_redis_conn_failure() {
    // Arrange
    utils::setup().await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let state = Box::pin(routes::AppState::new(
        Settings::default(),
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;

    let _ = state.store.get_redis_conn().map(|conn| {
        conn.is_redis_available
            .store(false, atomic::Ordering::SeqCst)
    });

    // Act
    let _ = state.store.get_redis_conn();

    // Assert
    // based on #[should_panic] attribute
}

#[tokio::test]
/// Asynchronously sets up the necessary utilities, creates a new application state,
/// and then attempts to retrieve a Redis connection from the application state.
/// Finally, it asserts that the result is a successful connection.
async fn get_redis_conn_success() {
    // Arrange
    Box::pin(utils::setup()).await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let state = Box::pin(routes::AppState::new(
        Settings::default(),
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;

    // Act
    let result = state.store.get_redis_conn();

    // Assert
    assert!(result.is_ok())
}
