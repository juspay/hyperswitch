use criterion::{black_box, criterion_group, criterion_main, Criterion};
use external_services::kms::{get_kms_client, KmsClient, KmsConfig};
use router::{configs::settings::Settings, routes::AppState};
use tokio::sync::oneshot;

async fn create_service() -> AppState {
    let conf = Settings::new().expect("Ohh! loading setting failed");

    let tx: oneshot::Sender<()> = oneshot::channel().0;
    AppState::with_storage(conf, router::db::StorageImpl::Postgresql, tx).await
}

async fn get_kms(kms_conf: &KmsConfig) -> &KmsClient {
    get_kms_client(&kms_conf).await
}

async fn decrypt_kms_enc_string(data: String, kms_client: &KmsClient) -> String {
    kms_client
        .decrypt(data)
        .await
        .expect("Ohh! decryption failed")
}

fn get_decryptable_data(state: &AppState) -> String {
    state.conf.secrets.admin_api_key.clone()
}

fn crit_decrypt(c: &mut Criterion) {
    let key_id = std::env::var("ROUTER__KMS__KEY_ID").expect("Failed to get key_id");
    let region = std::env::var("ROUTER__KMS__REGION").expect("Failed to get region");
    let kms_conf = KmsConfig { key_id, region };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Ohh! tokio runtime failed");
    let _guard = rt.enter();
    let state = rt.block_on(create_service());
    let data = get_decryptable_data(&state);
    let truth = state.conf.secrets.admin_api_key.clone();

    c.bench_function("kms decrypt", |b| {
        b.to_async(&rt).iter(|| async {
            let kms_client = black_box(get_kms(&kms_conf)).await;
            let output = black_box(decrypt_kms_enc_string(data.clone(), &kms_client)).await;
            output == truth
        });
    });
}

// criterion_group!(name = kms_bencher, crit_decrypt);
criterion_group!{
    name = kms_bencher;
    config = Criterion::default().sample_size(10);
    targets = crit_decrypt
}

criterion_main!(kms_bencher);
