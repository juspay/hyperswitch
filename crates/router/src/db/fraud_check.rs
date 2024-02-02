use diesel_models::fraud_check::{self as storage, FraudCheck, FraudCheckUpdate};
use error_stack::IntoReport;

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait FraudCheckInterface {
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<FraudCheck, errors::StorageError>;

    async fn update_fraud_check_response_with_attempt_id(
        &self,
        this: FraudCheck,
        fraud_check: FraudCheckUpdate,
    ) -> CustomResult<FraudCheck, errors::StorageError>;

    async fn find_fraud_check_by_payment_id(
        &self,
        payment_id: String,
        merchant_id: String,
    ) -> CustomResult<FraudCheck, errors::StorageError>;

    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        payment_id: String,
        merchant_id: String,
    ) -> CustomResult<Option<FraudCheck>, errors::StorageError>;
}

#[async_trait::async_trait]
impl FraudCheckInterface for Store {
        /// Asynchronously inserts a new fraud check response into the storage.
    /// 
    /// # Arguments
    /// 
    /// * `new` - A `storage::FraudCheckNew` object representing the new fraud check response to be inserted into the storage.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a `FraudCheck` if the insertion is successful, otherwise an `errors::StorageError`.
    /// 
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn).await.map_err(Into::into).into_report()
    }
        /// Updates the fraud check response with the provided attempt ID. It takes a FraudCheck object, a FraudCheckUpdate object, and returns a CustomResult containing the updated FraudCheck object or a StorageError if an error occurs during the update process.
    async fn update_fraud_check_response_with_attempt_id(
            &self,
            this: FraudCheck,
            fraud_check: FraudCheckUpdate,
        ) -> CustomResult<FraudCheck, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            this.update_with_attempt_id(&conn, fraud_check)
                .await
                .map_err(Into::into)
                .into_report()
        }

    /// Asynchronously finds a fraud check by payment ID and merchant ID.
    ///
    /// # Arguments
    ///
    /// * `payment_id` - A String representing the payment ID.
    /// * `merchant_id` - A String representing the merchant ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `FraudCheck` if successful, or a `StorageError` if an error occurred.
    ///
    async fn find_fraud_check_by_payment_id(
        &self,
        payment_id: String,
        merchant_id: String,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        FraudCheck::get_with_payment_id(&conn, payment_id, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds a fraud check by payment ID if it is present in the database.
    /// 
    /// # Arguments
    /// 
    /// * `payment_id` - A String representing the payment ID to search for
    /// * `merchant_id` - A String representing the merchant ID associated with the payment ID
    /// 
    /// # Returns
    /// 
    /// An `Option` containing the `FraudCheck` if it is present, or `None` if not found. Returns a `StorageError` if an error occurs during the storage operation.
    /// 
    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        payment_id: String,
        merchant_id: String,
    ) -> CustomResult<Option<FraudCheck>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        FraudCheck::get_with_payment_id_if_present(&conn, payment_id, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl FraudCheckInterface for MockDb {
        /// Inserts a new fraud check response into the storage.
    /// 
    /// # Arguments
    /// 
    /// * `_new` - A `FraudCheckNew` struct containing the new fraud check response.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<FraudCheck, errors::StorageError>` - A result indicating success or an error of type `StorageError`.
    /// 
    /// # Errors
    /// 
    /// Returns an error of type `StorageError::MockDbError` if the method encounters a mock database error.
    async fn insert_fraud_check_response(
            &self,
            _new: storage::FraudCheckNew,
        ) -> CustomResult<FraudCheck, errors::StorageError> {
            Err(errors::StorageError::MockDbError)?
        }
        /// Asynchronously updates the fraud check response with the attempt ID and returns the updated fraud check.
    ///
    /// # Arguments
    /// 
    /// * `self` - The reference to the current object.
    /// * `_this` - The original fraud check object.
    /// * `_fraud_check` - The fraud check update containing the attempt ID.
    ///
    /// # Returns
    ///
    /// * `CustomResult<FraudCheck, errors::StorageError>` - A result containing the updated fraud check or a storage error.
    ///
    async fn update_fraud_check_response_with_attempt_id(
        &self,
        _this: FraudCheck,
        _fraud_check: FraudCheckUpdate,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
        /// Asynchronously finds a fraud check by payment ID and merchant ID.
    ///
    /// # Arguments
    ///
    /// * `_payment_id` - A String representing the payment ID to search for.
    /// * `_merchant_id` - A String representing the merchant ID to search for.
    ///
    /// # Returns
    ///
    /// * A `CustomResult` containing a `FraudCheck` if successful, otherwise a `StorageError`.
    ///
    async fn find_fraud_check_by_payment_id(
        &self,
        _payment_id: String,
        _merchant_id: String,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a fraud check by payment ID if it is present in the database for the specified merchant.
    /// 
    /// # Arguments
    /// * `_payment_id` - A String representing the payment ID for which the fraud check is to be found.
    /// * `_merchant_id` - A String representing the merchant ID for which the fraud check is to be found.
    /// 
    /// # Returns
    /// * `CustomResult<Option<FraudCheck>, errors::StorageError>` - A custom result type containing an optional FraudCheck or a StorageError in case of failure.
    /// 
    /// This method attempts to find a fraud check in the database for the specified payment ID and merchant ID. If a fraud check is found, it is returned as an Option. If no fraud check is found, None is returned. If an error occurs during the database operation, a StorageError is returned.
    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        _payment_id: String,
        _merchant_id: String,
    ) -> CustomResult<Option<FraudCheck>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(feature = "kafka_events")]
#[async_trait::async_trait]
impl FraudCheckInterface for super::KafkaStore {
    async fn insert_fraud_check_response(
        &self,
        _new: storage::FraudCheckNew,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
    async fn update_fraud_check_response_with_attempt_id(
        &self,
        _this: FraudCheck,
        _fraud_check: FraudCheckUpdate,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_fraud_check_by_payment_id(
        &self,
        _payment_id: String,
        _merchant_id: String,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
