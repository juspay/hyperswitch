use criterion::{criterion_group, criterion_main, Criterion};
use router::{configs::settings::Settings, routes::AppState};
use tokio::sync::oneshot;
use uuid::Uuid;

async fn create_service() -> AppState {
    let conf = Settings::new().unwrap();

    let tx: oneshot::Sender<()> = oneshot::channel().0;
    AppState::with_storage(conf, router::db::StorageImpl::Postgresql, tx).await
}

async fn insert_config(state: &AppState) {
    let config = storage_models::configs::ConfigNew {
        key: format!("bench-{}", Uuid::new_v4()),
        config: "Hi!".to_string(),
    };

    state.store.insert_config(config).await.unwrap();
}

fn crit_insert_merchant(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();
    let state = rt.block_on(create_service());

    c.bench_function("insert config", move |b| {
        b.to_async(&rt).iter(|| insert_config(&state));
    });
}

criterion_group!(db_bench, crit_insert_merchant);
criterion_main!(db_bench);
