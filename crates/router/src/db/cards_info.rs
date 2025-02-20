use error_stack::report;
use router_env::{instrument, tracing};

use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::storage::cards_info::{CardInfo, UpdateCardInfo},
};

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    async fn get_card_info(
        &self,
        _card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError>;
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, errors::StorageError>;
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, errors::StorageError>;
}

#[async_trait::async_trait]
impl CardsInfoInterface for Store {
    #[instrument(skip_all)]
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        data.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        CardInfo::update(&conn, card_iin, data)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl CardsInfoInterface for MockDb {
    #[instrument(skip_all)]
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
        Ok(self
            .cards_info
            .lock()
            .await
            .iter()
            .find(|ci| ci.card_iin == card_iin)
            .cloned())
    }

    async fn add_card_info(&self, _data: CardInfo) -> CustomResult<CardInfo, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_card_info(
        &self,
        _card_iin: String,
        _data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
