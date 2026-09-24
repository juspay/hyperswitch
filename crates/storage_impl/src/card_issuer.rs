use common_utils::id_type;
pub use diesel_models::card_issuer::{
    CardIssuer, CardIssuerListItem, NewCardIssuer, UpdateCardIssuer,
};
use error_stack::report;
use hyperswitch_domain_models::card_issuer::CardIssuersInterface;
use router_env::{instrument, tracing};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    redis::cache::{self, CacheKind, CONFIG_CACHE},
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

const CARD_ISSUERS_LIST_CACHE_KEY: &str = "card_issuers_list";

#[async_trait::async_trait]
impl<T: DatabaseStore> CardIssuersInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_card_issuer(
        &self,
        new: NewCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let insert_call = || async {
            let conn = pg_connection_write(self).await?;
            new.insert(&conn)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        cache::publish_and_redact(
            self,
            CacheKind::Config(CARD_ISSUERS_LIST_CACHE_KEY.into()),
            insert_call,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: UpdateCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        let update_call = || async {
            let conn = pg_connection_write(self).await?;
            CardIssuer::update(&conn, id, update)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        cache::publish_and_redact(
            self,
            CacheKind::Config(CARD_ISSUERS_LIST_CACHE_KEY.into()),
            update_call,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn delete_card_issuer(
        &self,
        id: id_type::CardIssuerId,
    ) -> CustomResult<bool, StorageError> {
        let delete_call = || async {
            let conn = pg_connection_write(self).await?;
            CardIssuer::delete_by_id(&conn, id)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        cache::publish_and_redact(
            self,
            CacheKind::Config(CARD_ISSUERS_LIST_CACHE_KEY.into()),
            delete_call,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        limit: i64,
    ) -> CustomResult<Vec<CardIssuerListItem>, StorageError> {
        let fetch_func = || async {
            let conn = pg_connection_read(self).await?;
            CardIssuer::list_all(&conn, limit)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        cache::get_or_populate_in_memory(
            self,
            CARD_ISSUERS_LIST_CACHE_KEY,
            fetch_func,
            &CONFIG_CACHE,
        )
        .await
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
        self.router_store.insert_card_issuer(new).await
    }

    #[instrument(skip_all)]
    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: UpdateCardIssuer,
    ) -> CustomResult<CardIssuer, StorageError> {
        self.router_store.update_card_issuer(id, update).await
    }

    #[instrument(skip_all)]
    async fn delete_card_issuer(
        &self,
        id: id_type::CardIssuerId,
    ) -> CustomResult<bool, StorageError> {
        self.router_store.delete_card_issuer(id).await
    }

    #[instrument(skip_all)]
    async fn list_card_issuers(
        &self,
        limit: i64,
    ) -> CustomResult<Vec<CardIssuerListItem>, StorageError> {
        self.router_store.list_card_issuers(limit).await
    }

    #[instrument(skip_all)]
    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> CustomResult<Vec<CardIssuer>, StorageError> {
        self.router_store.get_card_issuers_by_ids(ids).await
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

    async fn delete_card_issuer(
        &self,
        id: id_type::CardIssuerId,
    ) -> CustomResult<bool, StorageError> {
        let mut card_issuers = self.card_issuers.lock().await;
        let card_issuer_index = card_issuers
            .iter()
            .position(|card_issuer| card_issuer.id == id)
            .ok_or(StorageError::ValueNotFound(format!(
                "No card issuer found for id = {id:?}"
            )))?;
        card_issuers.remove(card_issuer_index);
        Ok(true)
    }

    async fn list_card_issuers(
        &self,
        limit: i64,
    ) -> CustomResult<Vec<CardIssuerListItem>, StorageError> {
        let mut card_issuers_list = self
            .card_issuers
            .lock()
            .await
            .iter()
            .map(|card_issuer| CardIssuerListItem {
                id: card_issuer.id.clone(),
                issuer_name: card_issuer.issuer_name.clone(),
            })
            .collect::<Vec<_>>();
        card_issuers_list.sort_by(|a, b| a.issuer_name.cmp(&b.issuer_name));
        card_issuers_list.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(card_issuers_list)
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
