use common_utils::id_type;
pub use diesel_models::card_issuer::{CardIssuer, NewCardIssuer, UpdateCardIssuer};
use error_stack::report;
use hyperswitch_domain_models::card_issuer::CardIssuersInterface;
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> CardIssuersInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: NewCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: UpdateCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        CardIssuer::update(&conn, id, update)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        CardIssuer::list_filtered(&conn, query, limit.map(i64::from))
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        CardIssuer::find_by_ids(&conn, ids)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardIssuersInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: NewCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: UpdateCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        CardIssuer::update(&conn, id, update)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        CardIssuer::list_filtered(&conn, query, limit.map(i64::from))
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        CardIssuer::find_by_ids(&conn, ids)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl CardIssuersInterface for MockDb {
    type Error = StorageError;

    async fn insert_card_issuer(
        &self,
        _new: NewCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_card_issuer(
        &self,
        _id: id_type::CardIssuerId,
        _update: UpdateCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn list_card_issuers(
        &self,
        _query: Option<String>,
        _limit: Option<u8>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        Ok(vec![])
    }

    async fn get_card_issuers_by_ids(
        &self,
        _ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        Ok(vec![])
    }
}
