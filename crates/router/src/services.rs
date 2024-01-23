pub mod api;
pub mod authentication;
pub mod authorization;
pub mod encryption;
#[cfg(feature = "olap")]
pub mod jwt;
pub mod kafka;
pub mod logger;
pub mod pm_auth;
#[cfg(feature = "recon")]
pub mod recon;

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "aws_kms")]
use data_models::errors::StorageError;
use data_models::errors::StorageResult;
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "aws_kms")]
use external_services::aws_kms::{self, decrypt::AwsKmsDecrypt};
#[cfg(not(feature = "aws_kms"))]
use masking::PeekInterface;
use masking::StrongSecret;
#[cfg(feature = "kv_store")]
use storage_impl::KVRouterStore;
use storage_impl::RouterStore;
use tokio::sync::oneshot;

pub use self::{api::*, encryption::*};
use crate::{configs::settings, consts, core::errors};

#[cfg(not(feature = "olap"))]
pub type StoreType = storage_impl::database::store::Store;
#[cfg(feature = "olap")]
pub type StoreType = storage_impl::database::store::ReplicaStore;

#[cfg(not(feature = "kv_store"))]
pub type Store = RouterStore<StoreType>;
#[cfg(feature = "kv_store")]
pub type Store = KVRouterStore<StoreType>;

pub async fn get_store(
    config: &settings::Settings,
    shut_down_signal: oneshot::Sender<()>,
    test_transaction: bool,
) -> StorageResult<Store> {
    #[cfg(feature = "aws_kms")]
    let aws_kms_client = aws_kms::get_aws_kms_client(&config.kms).await;

    #[cfg(feature = "aws_kms")]
    let master_config = config
        .master_database
        .clone()
        .decrypt_inner(aws_kms_client)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to decrypt master database config")?;
    #[cfg(not(feature = "aws_kms"))]
    let master_config = config.master_database.clone().into();

    #[cfg(all(feature = "olap", feature = "aws_kms"))]
    let replica_config = config
        .replica_database
        .clone()
        .decrypt_inner(aws_kms_client)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to decrypt replica database config")?;

    #[cfg(all(feature = "olap", not(feature = "aws_kms")))]
    let replica_config = config.replica_database.clone().into();

    let master_enc_key = get_master_enc_key(
        config,
        #[cfg(feature = "aws_kms")]
        aws_kms_client,
    )
    .await;
    #[cfg(not(feature = "olap"))]
    let conf = master_config;
    #[cfg(feature = "olap")]
    let conf = (master_config, replica_config);

    let store: RouterStore<StoreType> = if test_transaction {
        RouterStore::test_store(conf, &config.redis, master_enc_key).await?
    } else {
        RouterStore::from_config(
            conf,
            &config.redis,
            master_enc_key,
            shut_down_signal,
            consts::PUB_SUB_CHANNEL,
        )
        .await?
    };

    #[cfg(feature = "kv_store")]
    let store = KVRouterStore::from_store(
        store,
        config.drainer.stream_name.clone(),
        config.drainer.num_partitions,
        config.kv_config.ttl,
    );

    Ok(store)
}

#[allow(clippy::expect_used)]
async fn get_master_enc_key(
    conf: &crate::configs::settings::Settings,
    #[cfg(feature = "aws_kms")] aws_kms_client: &aws_kms::AwsKmsClient,
) -> StrongSecret<Vec<u8>> {
    #[cfg(feature = "aws_kms")]
    let master_enc_key = hex::decode(
        conf.secrets
            .master_enc_key
            .clone()
            .decrypt_inner(aws_kms_client)
            .await
            .expect("Failed to decrypt master enc key"),
    )
    .expect("Failed to decode from hex");

    #[cfg(not(feature = "aws_kms"))]
    let master_enc_key =
        hex::decode(conf.secrets.master_enc_key.peek()).expect("Failed to decode from hex");

    StrongSecret::new(master_enc_key)
}

#[inline]
pub fn generate_aes256_key() -> errors::CustomResult<[u8; 32], common_utils::errors::CryptoError> {
    use ring::rand::SecureRandom;

    let rng = ring::rand::SystemRandom::new();
    let mut key: [u8; 256 / 8] = [0_u8; 256 / 8];
    rng.fill(&mut key)
        .into_report()
        .change_context(common_utils::errors::CryptoError::EncodingFailed)?;
    Ok(key)
}
