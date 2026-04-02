use diesel_models::cards_info as storage;
use error_stack::report;
use hyperswitch_domain_models::cards_info::{
    CardInfo as DomainCardInfo, CardsInfoInterface, UpdateCardInfo as DomainUpdateCardInfo,
};
pub use hyperswitch_domain_models::cards_info::CardInfo;
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    redis::kv_store::KvStorePartition,
    utils::{pg_connection_read, pg_connection_write},
    transformers::ForeignFrom,
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

impl ForeignFrom<storage::CardInfo> for DomainCardInfo {
    fn foreign_from(from: storage::CardInfo) -> Self {
        Self {
            card_iin: from.card_iin,
            card_issuer: from.card_issuer,
            card_network: from.card_network,
            card_type: from.card_type,
            card_subtype: from.card_subtype,
            card_issuing_country: from.card_issuing_country,
            bank_code_id: from.bank_code_id,
            bank_code: from.bank_code,
            country_code: from.country_code,
            date_created: from.date_created,
            last_updated: from.last_updated,
            last_updated_provider: from.last_updated_provider,
        }
    }
}

impl ForeignFrom<DomainCardInfo> for storage::CardInfo {
    fn foreign_from(from: DomainCardInfo) -> Self {
        Self {
            card_iin: from.card_iin,
            card_issuer: from.card_issuer,
            card_network: from.card_network,
            card_type: from.card_type,
            card_subtype: from.card_subtype,
            card_issuing_country: from.card_issuing_country,
            bank_code_id: from.bank_code_id,
            bank_code: from.bank_code,
            country_code: from.country_code,
            date_created: from.date_created,
            last_updated: from.last_updated,
            last_updated_provider: from.last_updated_provider,
        }
    }
}

impl ForeignFrom<storage::UpdateCardInfo> for DomainUpdateCardInfo {
    fn foreign_from(from: storage::UpdateCardInfo) -> Self {
        Self {
            card_issuer: from.card_issuer,
            card_network: from.card_network,
            card_type: from.card_type,
            card_subtype: from.card_subtype,
            card_issuing_country: from.card_issuing_country,
            bank_code_id: from.bank_code_id,
            bank_code: from.bank_code,
            country_code: from.country_code,
            last_updated: from.last_updated,
            last_updated_provider: from.last_updated_provider,
        }
    }
}

impl ForeignFrom<DomainUpdateCardInfo> for storage::UpdateCardInfo {
    fn foreign_from(from: DomainUpdateCardInfo) -> Self {
        Self {
            card_issuer: from.card_issuer,
            card_network: from.card_network,
            card_type: from.card_type,
            card_subtype: from.card_subtype,
            card_issuing_country: from.card_issuing_country,
            bank_code_id: from.bank_code_id,
            bank_code: from.bank_code,
            country_code: from.country_code,
            last_updated: from.last_updated,
            last_updated_provider: from.last_updated_provider,
        }
    }
}

impl KvStorePartition for storage::CardInfo {}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<DomainCardInfo>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let result = storage::CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(result.map(DomainCardInfo::foreign_from))
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: DomainCardInfo) -> CustomResult<DomainCardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_data = storage::CardInfo::foreign_from(data);
        let result = diesel_data.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardInfo::foreign_from(result))
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: DomainUpdateCardInfo,
    ) -> CustomResult<DomainCardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_update = storage::UpdateCardInfo::foreign_from(data);
        let result = storage::CardInfo::update(&conn, card_iin, diesel_update)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardInfo::foreign_from(result))
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<DomainCardInfo>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let result = storage::CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(result.map(DomainCardInfo::foreign_from))
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: DomainCardInfo) -> CustomResult<DomainCardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_data = storage::CardInfo::foreign_from(data);
        let result = diesel_data.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardInfo::foreign_from(result))
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: DomainUpdateCardInfo,
    ) -> CustomResult<DomainCardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_update = storage::UpdateCardInfo::foreign_from(data);
        let result = storage::CardInfo::update(&conn, card_iin, diesel_update)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardInfo::foreign_from(result))
    }
}

#[async_trait::async_trait]
impl CardsInfoInterface for MockDb {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<DomainCardInfo>, StorageError> {
        Ok(self
            .cards_info
            .lock()
            .await
            .iter()
            .find(|ci| ci.card_iin == card_iin)
            .map(|ci| DomainCardInfo::foreign_from(ci.clone())))
    }

    async fn add_card_info(&self, _data: DomainCardInfo) -> CustomResult<DomainCardInfo, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_card_info(
        &self,
        _card_iin: String,
        _data: DomainUpdateCardInfo,
    ) -> CustomResult<DomainCardInfo, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
