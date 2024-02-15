use std::sync::atomic;

use router::{configs::settings::Settings, routes, services};

mod utils;

#[tokio::test]
#[should_panic]
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
