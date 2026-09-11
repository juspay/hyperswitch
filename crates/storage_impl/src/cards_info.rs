pub use diesel_models::{CardInfo, UpdateCardInfo};
use error_stack::report;
use hyperswitch_domain_models::cards_info::CardsInfoInterface;
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    redis::{cache, kv_store::KvStorePartition},
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

impl KvStorePartition for CardInfo {}

/// Namespace the IIN. The in-memory caches are separate objects, but
/// `get_or_populate_in_memory` writes through to redis under the bare key, where a
/// six-digit number on its own would be a collision waiting to happen.
fn cards_info_cache_key(card_iin: &str) -> String {
    format!("cards_info_{card_iin}")
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        // BIN data is reference data: read on every card payment, changed only by the
        // card-info admin endpoints. `None` is cached too -- an unrecognised BIN is the
        // common case for test cards and would otherwise query on every attempt.
        let find_by_iin = || async {
            let conn = pg_connection_read(self).await?;
            CardInfo::find_by_iin(&conn, card_iin)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        cache::get_or_populate_in_memory(
            self,
            &cards_info_cache_key(card_iin),
            find_by_iin,
            &cache::CARDS_INFO_CACHE,
        )
        .await
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        let cache_key = cards_info_cache_key(&data.card_iin);
        cache::publish_and_redact(self, cache::CacheKind::CardsInfo(cache_key.into()), || async {
            let conn = pg_connection_write(self).await?;
            data.insert(&conn)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        })
        .await
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        cache::publish_and_redact(
            self,
            cache::CacheKind::CardsInfo(cards_info_cache_key(&card_iin).into()),
            || async {
                let conn = pg_connection_write(self).await?;
                CardInfo::update(&conn, card_iin, data)
                    .await
                    .map_err(|error| report!(StorageError::from(error)))
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        // BIN data is reference data: read on every card payment, changed only by the
        // card-info admin endpoints. `None` is cached too -- an unrecognised BIN is the
        // common case for test cards and would otherwise query on every attempt.
        let find_by_iin = || async {
            let conn = pg_connection_read(self).await?;
            CardInfo::find_by_iin(&conn, card_iin)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        cache::get_or_populate_in_memory(
            self,
            &cards_info_cache_key(card_iin),
            find_by_iin,
            &cache::CARDS_INFO_CACHE,
        )
        .await
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        let cache_key = cards_info_cache_key(&data.card_iin);
        cache::publish_and_redact(self, cache::CacheKind::CardsInfo(cache_key.into()), || async {
            let conn = pg_connection_write(self).await?;
            data.insert(&conn)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        })
        .await
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        cache::publish_and_redact(
            self,
            cache::CacheKind::CardsInfo(cards_info_cache_key(&card_iin).into()),
            || async {
                let conn = pg_connection_write(self).await?;
                CardInfo::update(&conn, card_iin, data)
                    .await
                    .map_err(|error| report!(StorageError::from(error)))
            },
        )
        .await
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
