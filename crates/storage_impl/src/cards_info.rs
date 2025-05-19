pub use diesel_models::{CardInfo, UpdateCardInfo};
use error_stack::report;
use hyperswitch_domain_models::cards_info::CardsInfoInterface;
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    redis::kv_store::KvStorePartition,
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore,
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
