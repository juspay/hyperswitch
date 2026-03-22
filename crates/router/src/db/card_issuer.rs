use common_utils::id_type;
use hyperswitch_domain_models::card_issuer::CardIssuersInterface;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    types::storage,
};

#[async_trait::async_trait]
impl CardIssuersInterface for KafkaStore {
    type Error = errors::StorageError;

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
