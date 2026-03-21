use common_utils::id_type;
use error_stack::report;
use router_env::{instrument, tracing};
use storage_impl::MockDb;

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[async_trait::async_trait]
pub trait CardIssuersInterface {
    async fn insert_card_issuer(
        &self,
        new: storage::NewCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError>;

    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: storage::UpdateCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError>;

    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError>;

    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError>;
}

#[async_trait::async_trait]
impl CardIssuersInterface for Store {
    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: storage::NewCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: storage::UpdateCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::CardIssuer::update(&conn, id, update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::CardIssuer::list_filtered(&conn, query, limit.map(i64::from))
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::CardIssuer::find_by_ids(&conn, ids)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl CardIssuersInterface for MockDb {
    async fn insert_card_issuer(
        &self,
        _new: storage::NewCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_card_issuer(
        &self,
        _id: id_type::CardIssuerId,
        _update: storage::UpdateCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_card_issuers(
        &self,
        _query: Option<String>,
        _limit: Option<u8>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError> {
        Ok(vec![])
    }

    async fn get_card_issuers_by_ids(
        &self,
        _ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError> {
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl CardIssuersInterface for KafkaStore {
    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: storage::NewCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError> {
        self.diesel_store.insert_card_issuer(new).await
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: storage::UpdateCardIssuer,
    ) -> CustomResult<storage::CardIssuer, errors::StorageError> {
        self.diesel_store.update_card_issuer(id, update).await
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError> {
        self.diesel_store.list_card_issuers(query, limit).await
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<storage::CardIssuer>, errors::StorageError> {
        self.diesel_store.get_card_issuers_by_ids(ids).await
    }
}
