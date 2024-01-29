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

#[cfg(any(feature = "kms", feature = "hashicorp-vault"))]
use data_models::errors::StorageError;
use data_models::errors::StorageResult;
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "hashicorp-vault")]
use external_services::hashicorp_vault::decrypt::VaultFetch;
#[cfg(feature = "kms")]
use external_services::kms::{self, decrypt::KmsDecrypt};
use masking::{PeekInterface, StrongSecret};
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
    #[cfg(feature = "kms")]
    let kms_client = kms::get_kms_client(&config.kms).await;

    #[cfg(feature = "hashicorp-vault")]
    let hc_client = external_services::hashicorp_vault::get_hashicorp_client(&config.hc_vault)
        .await
        .change_context(StorageError::InitializationError)?;

    let master_config = config.master_database.clone();

    #[cfg(feature = "hashicorp-vault")]
    let master_config = master_config
        .fetch_inner::<external_services::hashicorp_vault::Kv2>(hc_client)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to fetch data from hashicorp vault")?;

    #[cfg(feature = "kms")]
    let master_config = master_config
        .decrypt_inner(kms_client)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to decrypt master database config")?;

    #[cfg(feature = "olap")]
    let replica_config = config.replica_database.clone();

    #[cfg(all(feature = "olap", feature = "hashicorp-vault"))]
    let replica_config = replica_config
        .fetch_inner::<external_services::hashicorp_vault::Kv2>(hc_client)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to fetch data from hashicorp vault")?;

    #[cfg(all(feature = "olap", feature = "kms"))]
    let replica_config = replica_config
        .decrypt_inner(kms_client)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to decrypt replica database config")?;

    let master_enc_key = get_master_enc_key(
        config,
        #[cfg(feature = "kms")]
        kms_client,
        #[cfg(feature = "hashicorp-vault")]
        hc_client,
    )
    .await;
    #[cfg(not(feature = "olap"))]
    let conf = master_config.into();
    #[cfg(feature = "olap")]
    // this would get abstracted, for all cases
    #[allow(clippy::useless_conversion)]
    let conf = (master_config.into(), replica_config.into());

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
    #[cfg(feature = "kms")] kms_client: &kms::KmsClient,
    #[cfg(feature = "hashicorp-vault")]
    hc_client: &external_services::hashicorp_vault::HashiCorpVault,
) -> StrongSecret<Vec<u8>> {
    let master_enc_key = conf.secrets.master_enc_key.clone();

    #[cfg(feature = "hashicorp-vault")]
    let master_enc_key = master_enc_key
        .fetch_inner::<external_services::hashicorp_vault::Kv2>(hc_client)
        .await
        .expect("Failed to fetch master enc key");

    #[cfg(feature = "kms")]
    let master_enc_key = masking::Secret::<_, masking::WithType>::new(
        master_enc_key
            .decrypt_inner(kms_client)
            .await
            .expect("Failed to decrypt master enc key"),
    );

    let master_enc_key = hex::decode(master_enc_key.peek()).expect("Failed to decode from hex");

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
