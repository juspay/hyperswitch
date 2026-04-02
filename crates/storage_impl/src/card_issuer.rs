use common_utils::id_type;
use diesel_models::card_issuer as storage;
use error_stack::report;
use hyperswitch_domain_models::card_issuer::{
    CardIssuersInterface, CardIssuer as DomainCardIssuer, NewCardIssuer as DomainNewCardIssuer,
    UpdateCardIssuer as DomainUpdateCardIssuer,
};
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    utils::{pg_connection_read, pg_connection_write},
    transformers::ForeignFrom,
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

impl ForeignFrom<storage::CardIssuer> for DomainCardIssuer {
    fn foreign_from(from: storage::CardIssuer) -> Self {
        Self {
            id: from.id,
            issuer_name: from.issuer_name,
            created_at: from.created_at,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<DomainNewCardIssuer> for DomainCardIssuer {
    fn foreign_from(from: DomainNewCardIssuer) -> Self {
        Self {
            id: from.id,
            issuer_name: from.issuer_name,
            created_at: from.created_at,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<DomainCardIssuer> for storage::CardIssuer {
    fn foreign_from(from: DomainCardIssuer) -> Self {
        Self {
            id: from.id,
            issuer_name: from.issuer_name,
            created_at: from.created_at,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<DomainNewCardIssuer> for storage::CardIssuer {
    fn foreign_from(from: DomainNewCardIssuer) -> Self {
        Self {
            id: from.id,
            issuer_name: from.issuer_name,
            created_at: from.created_at,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<storage::NewCardIssuer> for DomainNewCardIssuer {
    fn foreign_from(from: storage::NewCardIssuer) -> Self {
        Self {
            id: from.id,
            issuer_name: from.issuer_name,
            created_at: from.created_at,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<DomainNewCardIssuer> for storage::NewCardIssuer {
    fn foreign_from(from: DomainNewCardIssuer) -> Self {
        Self {
            id: from.id,
            issuer_name: from.issuer_name,
            created_at: from.created_at,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<storage::UpdateCardIssuer> for DomainUpdateCardIssuer {
    fn foreign_from(from: storage::UpdateCardIssuer) -> Self {
        Self {
            issuer_name: from.issuer_name,
            last_modified_at: from.last_modified_at,
        }
    }
}

impl ForeignFrom<DomainUpdateCardIssuer> for storage::UpdateCardIssuer {
    fn foreign_from(from: DomainUpdateCardIssuer) -> Self {
        Self {
            issuer_name: from.issuer_name,
            last_modified_at: from.last_modified_at,
        }
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardIssuersInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: DomainNewCardIssuer,
    ) -> CustomResult<DomainCardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_new = storage::NewCardIssuer::foreign_from(new);
        let result = diesel_new.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardIssuer::foreign_from(result))
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: DomainUpdateCardIssuer,
    ) -> CustomResult<DomainCardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_update = storage::UpdateCardIssuer::foreign_from(update);
        let result = storage::CardIssuer::update(&conn, id, diesel_update)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardIssuer::foreign_from(result))
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<DomainCardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let results = storage::CardIssuer::list_filtered(&conn, query, limit.map(i64::from))
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(results
            .into_iter()
            .map(DomainCardIssuer::foreign_from)
            .collect())
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<DomainCardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let results = storage::CardIssuer::find_by_ids(&conn, ids)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(results
            .into_iter()
            .map(DomainCardIssuer::foreign_from)
            .collect())
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> CardIssuersInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: DomainNewCardIssuer,
    ) -> CustomResult<DomainCardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_new = storage::NewCardIssuer::foreign_from(new);
        let result = diesel_new.insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardIssuer::foreign_from(result))
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: DomainUpdateCardIssuer,
    ) -> CustomResult<DomainCardIssuer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_update = storage::UpdateCardIssuer::foreign_from(update);
        let result = storage::CardIssuer::update(&conn, id, diesel_update)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainCardIssuer::foreign_from(result))
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<DomainCardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let results = storage::CardIssuer::list_filtered(&conn, query, limit.map(i64::from))
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(results
            .into_iter()
            .map(DomainCardIssuer::foreign_from)
            .collect())
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<DomainCardIssuer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let results = storage::CardIssuer::find_by_ids(&conn, ids)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(results
            .into_iter()
            .map(DomainCardIssuer::foreign_from)
            .collect())
    }
}

#[async_trait::async_trait]
impl CardIssuersInterface for MockDb {
    type Error = StorageError;

    async fn insert_card_issuer(
        &self,
        new: DomainNewCardIssuer,
    ) -> CustomResult<DomainCardIssuer, StorageError> {
        let diesel_card_issuer = storage::CardIssuer::foreign_from(new.clone());
        self.card_issuers.lock().await.push(diesel_card_issuer);
        Ok(DomainCardIssuer::foreign_from(new))
    }

    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: DomainUpdateCardIssuer,
    ) -> CustomResult<DomainCardIssuer, StorageError> {
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
        Ok(DomainCardIssuer::foreign_from(card_issuer.clone()))
    }

    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> CustomResult<Vec<DomainCardIssuer>, StorageError> {
        let card_issuers = self.card_issuers.lock().await;
        let filtered: Vec<DomainCardIssuer> = card_issuers
            .iter()
            .filter(|ci| {
                query
                    .as_ref()
                    .is_none_or(|q| ci.issuer_name.contains(q.as_str()))
            })
            .take(limit.map_or(usize::MAX, usize::from))
            .map(|ci| DomainCardIssuer::foreign_from(ci.clone()))
            .collect();
        Ok(filtered)
    }

    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<DomainCardIssuer>, StorageError> {
        let card_issuers = self.card_issuers.lock().await;
        let filtered = card_issuers
            .iter()
            .filter(|ci| ids.contains(&ci.id))
            .map(|ci| DomainCardIssuer::foreign_from(ci.clone()))
            .collect();
        Ok(filtered)
    }
}
