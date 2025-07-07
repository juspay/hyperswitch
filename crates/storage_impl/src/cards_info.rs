// use diesel_models::{CardInfo, UpdateCardInfo};
use error_stack::report;
use hyperswitch_domain_models::cards_info::CardsInfoInterface;
pub use hyperswitch_domain_models::cards_info::{CardInfo, UpdateCardInfo};
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    redis::kv_store::KvStorePartition,
    utils::{pg_connection_read, pg_connection_write, ForeignFrom, ForeignInto},
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

impl KvStorePartition for CardInfo {}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        let conn = pg_connection_read(self).await?;
        diesel_models::CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(StorageError::from(error)))
            .map(|val: Option<diesel_models::CardInfo>| val.map(ForeignInto::foreign_into))
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        diesel_models::CardInfo::foreign_from(data)
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
            .map(ForeignInto::foreign_into)
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        diesel_models::CardInfo::update(&conn, card_iin, data.foreign_into())
            .await
            .map_err(|error| report!(StorageError::from(error)))
            .map(ForeignInto::foreign_into)
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn get_card_info(&self, card_iin: &str) -> CustomResult<Option<CardInfo>, StorageError> {
        let conn = pg_connection_read(self).await?;
        diesel_models::CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(StorageError::from(error)))
            .map(|val| val.map(ForeignInto::foreign_into))
    }
    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        diesel_models::CardInfo::foreign_from(data)
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
            .map(ForeignInto::foreign_into)
    }
    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, StorageError> {
        let conn = pg_connection_write(self).await?;
        diesel_models::CardInfo::update(&conn, card_iin, data.foreign_into())
            .await
            .map_err(|error| report!(StorageError::from(error)))
            .map(ForeignInto::foreign_into)
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
            .cloned()
            .map(ForeignInto::foreign_into))
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

impl ForeignFrom<diesel_models::CardInfo> for CardInfo {
    fn foreign_from(from: diesel_models::CardInfo) -> Self {
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

impl ForeignFrom<CardInfo> for diesel_models::CardInfo {
    fn foreign_from(from: CardInfo) -> Self {
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

impl ForeignFrom<diesel_models::UpdateCardInfo> for UpdateCardInfo {
    fn foreign_from(from: diesel_models::UpdateCardInfo) -> Self {
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

impl ForeignFrom<UpdateCardInfo> for diesel_models::UpdateCardInfo {
    fn foreign_from(from: UpdateCardInfo) -> Self {
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
