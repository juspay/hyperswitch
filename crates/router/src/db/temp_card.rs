use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait TempCardInterface {
    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<storage::TempCard, errors::StorageError>;

    async fn insert_temp_card(
        &self,
        address: storage::TempCardNew,
    ) -> CustomResult<storage::TempCard, errors::StorageError>;

    async fn find_tempcard_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> CustomResult<Option<storage::TempCard>, errors::StorageError>;

    async fn insert_tempcard_with_token(
        &self,
        new: storage::TempCard,
    ) -> CustomResult<storage::TempCard, errors::StorageError>;
}

#[async_trait::async_trait]
impl TempCardInterface for Store {
    async fn insert_temp_card(
        &self,
        address: storage::TempCardNew,
    ) -> CustomResult<storage::TempCard, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        address
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_tempcard_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> CustomResult<Option<storage::TempCard>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::TempCard::find_by_transaction_id(&conn, transaction_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn insert_tempcard_with_token(
        &self,
        card: storage::TempCard,
    ) -> CustomResult<storage::TempCard, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::TempCard::insert_with_token(card, &conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<storage::TempCard, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::TempCard::find_by_token(&conn, token)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl TempCardInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_temp_card(
        &self,
        insert: storage::TempCardNew,
    ) -> CustomResult<storage::TempCard, errors::StorageError> {
        let mut cards = self.temp_cards.lock().await;
        let card = storage::TempCard {
            #[allow(clippy::as_conversions)]
            id: cards.len() as i32,
            date_created: insert.date_created,
            txn_id: insert.txn_id,
            card_info: insert.card_info,
        };
        cards.push(card.clone());
        Ok(card)
    }

    async fn find_tempcard_by_transaction_id(
        &self,
        _transaction_id: &str,
    ) -> CustomResult<Option<storage::TempCard>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_tempcard_with_token(
        &self,
        _card: storage::TempCard,
    ) -> CustomResult<storage::TempCard, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_tempcard_by_token(
        &self,
        _token: &i32,
    ) -> CustomResult<storage::TempCard, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
