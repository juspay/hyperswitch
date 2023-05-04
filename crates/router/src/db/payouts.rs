use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PayoutsInterface {
    async fn insert_payout_create(
        &self,
        pc: storage::PayoutCreateNew,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError>;

    async fn find_payout_create_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError>;

    async fn update_payout_create_by_payout_id(
        &self,
        payout_create: storage::PayoutCreate,
        updated_payout: storage::PayoutCreateUpdate,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError>;

    async fn insert_payouts(
        &self,
        p: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;

    async fn find_payout_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;
}

#[async_trait::async_trait]
impl PayoutsInterface for Store {
    async fn insert_payout_create(
        &self,
        pc: storage::PayoutCreateNew,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutCreateNew::insert(pc, &conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_payout_create_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutCreate::find_by_payout_id(&conn, payout_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_payout_create_by_payout_id(
        &self,
        payout_create: storage::PayoutCreate,
        updated_payout: storage::PayoutCreateUpdate,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutCreate::update(payout_create, &conn, updated_payout)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn insert_payouts(
        &self,
        p: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutsNew::insert(p, &conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_payout_by_payout_id(
        &self,
        payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Payouts::find_by_payout_id(&conn, payout_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl PayoutsInterface for MockDb {
    async fn insert_payout_create(
        &self,
        _pc: storage::PayoutCreateNew,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        // Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payout_create_by_payout_id(
        &self,
        _payout_id: &str,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        // Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_payout_create_by_payout_id(
        &self,
        _payout_create: storage::PayoutCreate,
        _updated_payout: storage::PayoutCreateUpdate,
    ) -> CustomResult<storage::PayoutCreate, errors::StorageError> {
        // Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_payouts(
        &self,
        _p: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payout_by_payout_id(
        &self,
        _payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
