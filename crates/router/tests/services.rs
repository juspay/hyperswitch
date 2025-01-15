use std::sync::{atomic, Arc};

use router::{configs::settings::Settings, routes, services};

mod utils;

#[tokio::test]
#[should_panic]
#[allow(clippy::unwrap_used)]
async fn get_redis_conn_failure() {
    // Arrange
    utils::setup().await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let app_state = Box::pin(routes::AppState::new(
        Settings::default(),
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();

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
#[allow(clippy::unwrap_used)]
async fn get_redis_conn_success() {
    // Arrange
    Box::pin(utils::setup()).await;
    let (tx, _) = tokio::sync::oneshot::channel();
    let app_state = Box::pin(routes::AppState::new(
        Settings::default(),
        tx,
        Box::new(services::MockApiClient),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();

    // Act
    let result = state.store.get_redis_conn();

    // Assert
    assert!(result.is_ok())
}
