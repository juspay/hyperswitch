use error_stack::IntoReport;

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
        /// Asynchronously finds a payout attempt by merchant ID and payout ID.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `payout_id` - A reference to a string representing the payout ID.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a `storage::PayoutAttempt` if successful, or a `errors::StorageError` if an error occurred.
    /// 
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

        /// Updates a payout attempt by the merchant ID and payout ID using the provided PayoutAttemptUpdate data.
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

        /// Asynchronously inserts a new payout attempt into the database using the provided payout attempt model. 
    /// 
    /// # Arguments
    /// 
    /// * `payout` - The payout attempt model to be inserted into the database.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<storage::PayoutAttempt, errors::StorageError>` - A result containing either the newly inserted payout attempt or a storage error if the insertion fails.
    /// 
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
        /// Asynchronously finds a payout attempt by the given merchant ID and payout ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A reference to a string representing the merchant ID
    /// * `_payout_id` - A reference to a string representing the payout ID
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::PayoutAttempt` if successful, otherwise a `errors::StorageError`
    ///
    async fn find_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously updates a payout attempt by merchant ID and payout ID in the storage.
    async fn update_payout_attempt_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutAttemptUpdate,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously inserts a new payout attempt into the database.
    ///
    /// # Arguments
    ///
    /// * `_payout` - A `PayoutAttemptNew` struct representing the new payout attempt to be inserted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the inserted `PayoutAttempt` on success, or a `StorageError` on failure.
    ///
    /// # Errors
    ///
    /// If the function is called on a `MockDb`, it will always return a `MockDbError`.
    ///
    async fn insert_payout_attempt(
        &self,
        _payout: storage::PayoutAttemptNew,
    ) -> CustomResult<storage::PayoutAttempt, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
