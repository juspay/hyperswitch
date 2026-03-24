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
        new: NewCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let card_issuer = CardIssuer {
            id: new.id,
            issuer_name: new.issuer_name,
            created_at: new.created_at,
            last_modified_at: new.last_modified_at,
        };
        self.card_issuers.lock().await.push(card_issuer.clone());
        Ok(card_issuer)
    }

    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: UpdateCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let mut card_issuers = self.card_issuers.lock().await;
        let card_issuer =
            card_issuers
                .iter_mut()
                .find(|ci| ci.id == id)
                .ok_or(StorageError::ValueNotFound(format!(
                    "No card issuer found for id = {id:?}"
                )))?;
        card_issuer.issuer_name = update.issuer_name;
        card_issuer.last_modified_at = update.last_modified_at;
        Ok(card_issuer.clone())
    }

    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        let card_issuers = self.card_issuers.lock().await;
        let filtered: Vec<CardIssuer> = card_issuers
            .iter()
            .filter(|ci| {
                query
                    .as_ref()
                    .is_none_or(|q| ci.issuer_name.contains(q.as_str()))
            })
            .take(limit.map_or(usize::MAX, usize::from))
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        let card_issuers = self.card_issuers.lock().await;
        let filtered = card_issuers
            .iter()
            .filter(|ci| ids.contains(&ci.id))
            .cloned()
            .collect();
        Ok(filtered)
    }
}
