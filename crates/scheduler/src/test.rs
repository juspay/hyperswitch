//! Regression test for the scheduler consumer's Redis consumer-group registration.
//!
//! Prior to the fix, `consumer_name` was generated fresh (a new UUID) on every poll
//! iteration inside `consumer_operations`. Since Redis registers a stream consumer
//! group entry as a side effect of `XREADGROUP` — even when the stream has no new
//! entries to deliver — and Redis never expires these entries on its own, this meant
//! the consumer group accumulated one new (permanently idle) consumer per poll for
//! the lifetime of the process.
//!
//! The fix moves `consumer_name` generation to `start_consumer`, so it is created
//! once per process and reused across every poll iteration instead.
//!
//! This test drives `utils::get_batches` — the function that actually issues the
//! `XREADGROUP` call — directly against a real Redis instance (same style as
//! `redis_interface::test`), simulating several poll iterations, and inspects the
//! consumer group via `XINFO CONSUMERS` to assert the registered consumer count
//! matches the fixed behavior (stable name -> count stays flat) while also
//! demonstrating the bug it replaces (fresh name per poll -> count grows).
//!
//! Run with: cargo test -p scheduler

use redis_interface::{
    RedisConnectionPool, RedisConnectionWithContext, RedisEntryId, RedisSettings,
};
use uuid::Uuid;

use crate::utils::get_batches;

async fn test_connection() -> RedisConnectionWithContext {
    let pool = RedisConnectionPool::new_without_event_emitter(&RedisSettings::default())
        .await
        .map(std::sync::Arc::new)
        .expect("failed to create redis connection pool");
    RedisConnectionWithContext::new_without_context(pool)
}

/// Returns the number of consumers currently registered on `group` for `stream`,
/// as reported by `XINFO CONSUMERS`.
async fn registered_consumer_count(
    conn: &RedisConnectionWithContext,
    stream_name: &str,
    group_name: &str,
) -> i64 {
    conn.evaluate_redis_script::<_, i64>(
        r#"return #redis.call("XINFO", "CONSUMERS", KEYS[1], ARGV[1])"#,
        vec![stream_name.to_string()],
        group_name.to_string(),
    )
    .await
    .expect("failed to inspect consumer group via XINFO CONSUMERS")
}

#[tokio::test]
async fn test_stable_consumer_name_does_not_grow_consumer_group() {
    let is_success = tokio::task::spawn_blocking(move || {
        futures::executor::block_on(async {
            let conn = test_connection().await;
            let uid = Uuid::new_v4();
            let stream_name = format!("test_scheduler_consumer_stable_{uid}");
            let group_name = format!("test_scheduler_group_stable_{uid}");

            conn.consumer_group_create(
                &stream_name.as_str().into(),
                &group_name,
                &RedisEntryId::AfterLastID,
            )
            .await
            .expect("failed to create consumer group");

            // Mirrors the fix: `consumer_name` is generated once (in `start_consumer`)
            // and passed unchanged into every poll iteration.
            let consumer_name = format!("consumer_{}", Uuid::new_v4());
            for _ in 0..5 {
                get_batches(&conn, &stream_name, &group_name, &consumer_name)
                    .await
                    .expect("failed to poll batches");
            }

            let consumer_count = registered_consumer_count(&conn, &stream_name, &group_name).await;

            consumer_count == 1
        })
    })
    .await
    .expect("Spawn block failure");

    assert!(is_success);
}

#[tokio::test]
async fn test_per_poll_consumer_name_grows_consumer_group() {
    let is_success = tokio::task::spawn_blocking(move || {
        futures::executor::block_on(async {
            let conn = test_connection().await;
            let uid = Uuid::new_v4();
            let stream_name = format!("test_scheduler_consumer_growth_{uid}");
            let group_name = format!("test_scheduler_group_growth_{uid}");

            conn.consumer_group_create(
                &stream_name.as_str().into(),
                &group_name,
                &RedisEntryId::AfterLastID,
            )
            .await
            .expect("failed to create consumer group");

            // Mirrors the pre-fix bug: a fresh `consumer_name` was generated inside
            // `consumer_operations` on every poll iteration.
            for _ in 0..5 {
                let consumer_name = format!("consumer_{}", Uuid::new_v4());
                get_batches(&conn, &stream_name, &group_name, &consumer_name)
                    .await
                    .expect("failed to poll batches");
            }

            let consumer_count = registered_consumer_count(&conn, &stream_name, &group_name).await;

            consumer_count == 5
        })
    })
    .await
    .expect("Spawn block failure");

    assert!(is_success);
}
