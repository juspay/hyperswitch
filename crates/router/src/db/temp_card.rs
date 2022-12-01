use super::MockDb;
#[cfg(feature = "diesel")]
use crate::connection::pg_connection;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{TempCard, TempCardNew},
};

#[async_trait::async_trait]
pub trait TempCardInterface {
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

#[cfg(feature = "diesel")]
#[async_trait::async_trait]
impl TempCardInterface for super::Store {
    async fn insert_temp_card(
        &self,
        address: TempCardNew,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        address.insert_diesel(&conn).await
    }

    async fn find_tempcard_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> CustomResult<Option<TempCard>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        TempCard::find_by_transaction_id(&conn, transaction_id).await
    }

    async fn insert_tempcard_with_token(
        &self,
        card: TempCard,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        TempCard::insert_with_token(card, &conn).await
    }

    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        TempCard::find_by_token(&conn, token).await
    }
}

#[cfg(feature = "sqlx")]
#[async_trait::async_trait]
impl TempCardInterface for super::Sqlx {
    #[allow(clippy::panic)]
    async fn insert_temp_card(
        &self,
        address: TempCardNew,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let val = address.insert::<TempCard>(&self.pool, "temp_card").await;

        match val {
            Ok(val) => Ok(val),
            Err(err) => {
                panic!("{err}");
            }
        }
    }

    async fn find_tempcard_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> CustomResult<Option<TempCard>, errors::StorageError> {
        todo!()
    }

    async fn insert_tempcard_with_token(
        &self,
        card: TempCard,
    ) -> CustomResult<TempCard, errors::StorageError> {
        todo!()
    }

    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<TempCard, errors::StorageError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl TempCardInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_temp_card(
        &self,
        insert: TempCardNew,
    ) -> CustomResult<TempCard, errors::StorageError> {
        let mut cards = self.temp_cards().await;
        let card = TempCard {
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
        transaction_id: &str,
    ) -> CustomResult<Option<TempCard>, errors::StorageError> {
        todo!()
    }

    async fn insert_tempcard_with_token(
        &self,
        card: TempCard,
    ) -> CustomResult<TempCard, errors::StorageError> {
        todo!()
    }

    async fn find_tempcard_by_token(
        &self,
        token: &i32,
    ) -> CustomResult<TempCard, errors::StorageError> {
        todo!()
    }
}
