use std::sync::atomic;

use router::{configs::settings::Settings, routes};

mod utils;

#[tokio::test]
#[should_panic]
async fn get_redis_conn_failure() {
    // Arrange
    utils::setup().await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let state = routes::AppState::new(Settings::default(), tx).await;

    state
        .store
        .get_redis_conn()
        .expect("")
        .is_redis_available
        .store(false, atomic::Ordering::SeqCst);

    // Act
    state.store.get_redis_conn().expect("");

    // Assert
    // based on #[should_panic] attribute
}

#[tokio::test]
async fn get_redis_conn_success() {
    // Arrange
    utils::setup().await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let state = routes::AppState::new(Settings::default(), tx).await;

    // Act
    let result = state.store.get_redis_conn();

    // Assert
    assert!(result.is_ok())
}
