use common_utils::{id_type, pii::REDACTED};
use diesel_models::{kv, CardInfo, UpdateCardInfo};
use error_stack::{report, ResultExt};
use futures::future::try_join_all;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    cards_info::CardsInfoInterface,
    merchant_key_store::MerchantKeyStore,
};
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    diesel_error_to_data_error,
    errors::StorageError,
    kv_router_store::{FindResourceBy, InsertResourceParams, KVRouterStore, UpdateResourceParams},
    redis::kv_store::{decide_storage_scheme, KvStorePartition, Op, PartitionKey},
    store::enums::MerchantStorageScheme,
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, KeyManagerState, MockDb, RouterStore,
};

impl KvStorePartition for CardInfo {}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        let conn = pg_connection_read(self).await?;
        CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        data.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        CardInfo::update(&conn, card_iin, data)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        let conn = pg_connection_read(self).await?;
        CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        data.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        CardInfo::update(&conn, card_iin, data)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl CardsInfoInterface for MockDb {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        Ok(self
            .cards_info
            .lock()
            .await
            .iter()
            .find(|ci| ci.card_iin == card_iin)
            .cloned())
    }

    async fn add_card_info(&self, _data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_card_info(
        &self,
        _card_iin: String,
        _data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
