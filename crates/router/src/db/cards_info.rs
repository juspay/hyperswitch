use error_stack::IntoReport;

use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::storage::cards_info::CardInfo,
};

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    async fn get_card_info(
        &self,
        _card_bin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError>;
}

#[async_trait::async_trait]
impl CardsInfoInterface for Store {
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl CardsInfoInterface for MockDb {
    async fn get_card_info(
        &self,
        _card_bin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
