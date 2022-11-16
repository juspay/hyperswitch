use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{TempCard, TempCardNew},
};

#[async_trait::async_trait]
pub trait ITempCard {
    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<TempCard, errors::StorageError>;

    async fn insert_temp_card(
        &self,
        address: TempCardNew,
    ) -> CustomResult<TempCard, errors::StorageError>;

    async fn find_tempcard_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> CustomResult<Option<TempCard>, errors::StorageError>;

    async fn insert_tempcard_with_token(
        &self,
        new: TempCard,
    ) -> CustomResult<TempCard, errors::StorageError>;
}

#[async_trait::async_trait]
impl ITempCard for Store {
    async fn insert_temp_card(
        &self,
        address: TempCardNew,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        address.insert(&conn).await
    }

    async fn find_tempcard_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> CustomResult<Option<TempCard>, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        TempCard::find_by_transaction_id(&conn, transaction_id).await
    }

    async fn insert_tempcard_with_token(
        &self,
        card: TempCard,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        TempCard::insert_with_token(card, &conn).await
    }

    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let conn = pg_connection(&self.pg_pool.conn).await;
        TempCard::find_by_token(&conn, token).await
    }
}
