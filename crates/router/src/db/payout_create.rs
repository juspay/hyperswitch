use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PayoutCreateInterface {
    async fn find_payout_create_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError>;

    async fn find_payout_create_by_merchant_id_customer_id(
        &self,
        _merchant_id: &str,
        _customer_id: &str,
    ) -> CustomResult<Vec<storage::PayoutCreate>, errors::StorageError>;

    async fn update_payout_create_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutCreateUpdate,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError>;

    async fn insert_payout_create(
        &self,
        _payout: storage::PayoutCreateNew,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError>;
}

#[async_trait::async_trait]
impl PayoutCreateInterface for Store {
    async fn find_payout_create_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutCreate::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_payout_create_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage::PayoutCreate>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutCreate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_payout_create_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
        payout: storage::PayoutCreateUpdate,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PayoutCreate::update_by_merchant_id_payout_id(
            &conn,
            merchant_id,
            payout_id,
            payout,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn insert_payout_create(
        &self,
        payout: storage::PayoutCreateNew,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payout.insert(&conn).await.map_err(Into::into).into_report()
    }
}

#[async_trait::async_trait]
impl PayoutCreateInterface for MockDb {
    async fn find_payout_create_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payout_create_by_merchant_id_customer_id(
        &self,
        _merchant_id: &str,
        _customer_id: &str,
    ) -> CustomResult<Vec<storage::PayoutCreate>, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_payout_create_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutCreateUpdate,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_payout_create(
        &self,
        _payout: storage::PayoutCreateNew,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
