use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait PayoutsInterface {
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;

    async fn update_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;

    async fn insert_payout(
        &self,
        _payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError>;
}

#[async_trait::async_trait]
impl PayoutsInterface for Store {
        /// Asynchronously finds a payout by its merchant ID and payout ID. 
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The ID of the merchant
    /// * `payout_id` - The ID of the payout
    ///
    /// # Returns
    ///
    /// A Result containing a storage::Payouts if the payout is found, or a storage error if the operation fails.
    ///
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Payouts::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates a payout by merchant ID and payout ID in the database.
    async fn update_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &str,
        payout_id: &str,
        payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Payouts::update_by_merchant_id_payout_id(&conn, merchant_id, payout_id, payout)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously inserts a new payout into the database and returns the inserted payout.
    ///
    /// # Arguments
    ///
    /// * `payout` - A `storage::PayoutsNew` object representing the payout to be inserted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `storage::Payouts` object if the insertion is successful, otherwise an `errors::StorageError`.
    ///
    async fn insert_payout(
        &self,
        payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payout.insert(&conn).await.map_err(Into::into).into_report()
    }
}

#[async_trait::async_trait]
impl PayoutsInterface for MockDb {
        /// Asynchronously finds a payout by its merchant ID and payout ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A string slice representing the merchant ID
    /// * `_payout_id` - A string slice representing the payout ID
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `Payouts` if successful, otherwise a `StorageError`
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if the function is called on `MockDb`
    ///
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously updates a payout by merchant ID and payout ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A reference to a string representing the merchant ID.
    /// * `_payout_id` - A reference to a string representing the payout ID.
    /// * `_payout` - A `PayoutsUpdate` struct containing the updated payout information.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the updated `Payouts` or a `StorageError` if the operation fails.
    ///
    async fn update_payout_by_merchant_id_payout_id(
        &self,
        _merchant_id: &str,
        _payout_id: &str,
        _payout: storage::PayoutsUpdate,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously inserts a new payout into the storage, returning the inserted payout or a storage error.
    async fn insert_payout(
        &self,
        _payout: storage::PayoutsNew,
    ) -> CustomResult<storage::Payouts, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
