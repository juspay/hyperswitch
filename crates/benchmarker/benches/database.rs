use criterion::{criterion_group, criterion_main, Criterion};
use router::{configs::settings::Settings, routes::AppState};
use storage_models::merchant_account::MerchantAccountNew;
use tokio::sync::oneshot;
use uuid::Uuid;

async fn create_service() -> AppState {
    let conf = Settings::new().unwrap();

    let tx: oneshot::Sender<()> = oneshot::channel().0;
    AppState::with_storage(conf, router::db::StorageImpl::Postgresql, tx).await
}

async fn insert_merchant(state: &AppState) {
    let merchant_account = MerchantAccountNew {
        merchant_id: format!("config:benchmark_{}", Uuid::new_v4()),
        merchant_name: Some("123".to_string()),
        merchant_details: None,
        return_url: Some("123".to_string()),
        webhook_details: None,
        sub_merchants_enabled: Some(false),
        parent_merchant_id: None,
        enable_payment_response_hash: None,
        payment_response_hash_key: Some("123".to_string()),
        redirect_to_merchant_with_http_post: None,
        publishable_key: None,
        locker_id: None,
        metadata: None,
        routing_algorithm: None,
        primary_business_details: None::<i32>.into(),
        intent_fulfillment_time: None,
        frm_routing_algorithm: None,
    };

    state.store.insert_merchant(merchant_account).await.unwrap();
}

fn crit_insert_merchant(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();
    let state = rt.block_on(create_service());

    c.bench_function("insert merchant", move |b| {
        b.to_async(&rt).iter(|| insert_merchant(&state));
    });
}

criterion_group!(db_bench, crit_insert_merchant);
criterion_main!(db_bench);
