use error_stack::IntoReport;
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PayoutAttemptInterface {
    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError>;

    async fn update_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutAttemptUpdate,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError>;

    async fn insert_payout_attempt(
        &self,
        _payout: storage::PayoutAttemptNew,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError>;
}

#[async_trait::async_trait]
impl PayoutAttemptInterface for Store {
    #[instrument(skip_all)]
    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PayoutAttempt::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn update_payout_attempt_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
        payout: storage::PayoutAttemptUpdate,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PayoutAttempt::update_by_merchant_id_payout_id(
            &conn,
            merchant_id,
            payout_id,
            payout,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
    async fn insert_payout_attempt(
        &self,
        payout: storage::PayoutAttemptNew,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payout.insert(&conn).await.map_err(Into::into).into_report()
    }
}

#[async_trait::async_trait]
impl PayoutAttemptInterface for MockDb {
    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutAttemptUpdate,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_payout_attempt(
        &self,
        _payout: storage::PayoutAttemptNew,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
