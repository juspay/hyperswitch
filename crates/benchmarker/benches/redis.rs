use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};
use redis_interface::RedisConnectionPool;
use router::{configs::settings::Settings, routes::AppState};
use tokio::sync::oneshot;
use uuid::Uuid;

async fn create_service() -> AppState {
    let conf = Settings::new().unwrap();

    let tx: oneshot::Sender<()> = oneshot::channel().0;
    AppState::with_storage(conf, router::db::StorageImpl::PostgresqlTest, tx).await
}

async fn set_key(redis_conn_pool: Arc<RedisConnectionPool>) {
    redis_conn_pool
        .set_key(
            &format!("benchmark_{}", Uuid::new_v4()),
            Uuid::new_v4().to_string(),
        )
        .await
        .unwrap();
}

fn crit_set_key(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();
    let state = rt.block_on(create_service());
    c.bench_function("set key redis", move |b| {
        let rc = state.store.get_redis_conn();
        b.to_async(&rt).iter(|| set_key(rc.clone()))
    });
}

criterion_group!(redis_bench, crit_set_key);
criterion_main!(redis_bench);
