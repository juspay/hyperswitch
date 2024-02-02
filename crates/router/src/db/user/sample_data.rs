use data_models::{
    errors::StorageError,
    payments::{payment_attempt::PaymentAttempt, payment_intent::PaymentIntentNew, PaymentIntent},
};
use diesel_models::{
    errors::DatabaseError,
    query::user::sample_data as sample_data_queries,
    refund::{Refund, RefundNew},
    user::sample_data::PaymentAttemptBatchNew,
};
use error_stack::{Report, ResultExt};
use storage_impl::DataModelExt;

use crate::{connection::pg_connection_write, core::errors::CustomResult, services::Store};

#[async_trait::async_trait]
pub trait BatchSampleDataInterface {
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        batch: Vec<PaymentIntentNew>,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError>;

    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        batch: Vec<PaymentAttemptBatchNew>,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError>;

    async fn insert_refunds_batch_for_sample_data(
        &self,
        batch: Vec<RefundNew>,
    ) -> CustomResult<Vec<Refund>, StorageError>;

    async fn delete_payment_intents_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError>;

    async fn delete_payment_attempts_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError>;

    async fn delete_refunds_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, StorageError>;
}

#[async_trait::async_trait]
impl BatchSampleDataInterface for Store {
        /// Inserts a batch of payment intents into the database for sample data, returning a list of the inserted payment intents.
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        batch: Vec<PaymentIntentNew>,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        let new_intents = batch.into_iter().map(|i| i.to_storage_model()).collect();
        sample_data_queries::insert_payment_intents(&conn, new_intents)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|v| {
                v.into_iter()
                    .map(PaymentIntent::from_storage_model)
                    .collect()
            })
    }

        /// Inserts a batch of new payment attempts into the database for sample data and returns a vector of payment attempts.
    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        batch: Vec<PaymentAttemptBatchNew>,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        sample_data_queries::insert_payment_attempts(&conn, batch)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|res| {
                res.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }
        /// Inserts a batch of refund records into the database for sample data.
    ///
    /// # Arguments
    ///
    /// * `batch` - A vector of RefundNew structs representing the batch of refund records to be inserted.
    ///
    /// # Returns
    ///
    /// * A CustomResult containing a vector of Refund structs if the operation is successful, otherwise a StorageError.
    ///
    async fn insert_refunds_batch_for_sample_data(
        &self,
        batch: Vec<RefundNew>,
    ) -> CustomResult<Vec<Refund>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        sample_data_queries::insert_refunds(&conn, batch)
            .await
            .map_err(diesel_error_to_data_error)
    }

        /// Asynchronously deletes payment intents for the specified merchant ID from the database and returns a vector of PaymentIntent objects.
    async fn delete_payment_intents_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_payment_intents(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|v| {
                v.into_iter()
                    .map(PaymentIntent::from_storage_model)
                    .collect()
            })
    }

        /// Asynchronously deletes payment attempts for a specific merchant from the database and returns the deleted payment attempts as a vector. Returns a CustomResult containing the vector of PaymentAttempt or a StorageError if an error occurs during the database operation.
    async fn delete_payment_attempts_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_payment_attempts(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|res| {
                res.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }
    async fn delete_refunds_for_sample_data(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_refunds(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
    }
}

#[async_trait::async_trait]
impl BatchSampleDataInterface for storage_impl::MockDb {
        /// Asynchronously inserts a batch of payment intents for sample data into the storage.
    ///
    /// # Arguments
    ///
    /// * `batch` - A vector of `PaymentIntentNew` objects to be inserted into the storage
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `PaymentIntent` objects if the insertion is successful, otherwise a `StorageError` indicating the failure
    ///
    /// # Errors
    ///
    /// This method always returns a `StorageError::MockDbError` to simulate a database error.
    ///
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        _batch: Vec<PaymentIntentNew>,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        Err(StorageError::MockDbError)?
    }

        /// Asynchronously inserts a batch of payment attempts for sample data into the storage.
    /// 
    /// # Arguments
    /// * `batch` - A vector of PaymentAttemptBatchNew objects to be inserted into the storage
    /// 
    /// # Returns
    /// A CustomResult containing a vector of PaymentAttempt objects if successful, or a StorageError if an error occurs
    /// 
    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        _batch: Vec<PaymentAttemptBatchNew>,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        Err(StorageError::MockDbError)?
    }

        /// Inserts a batch of refund records for sample data into the database.
    ///
    /// # Arguments
    ///
    /// * `batch` - A vector of RefundNew objects representing the batch of refund records to be inserted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `Refund` objects if the operation is successful, otherwise a `StorageError` is returned.
    ///
    async fn insert_refunds_batch_for_sample_data(
        &self,
        _batch: Vec<RefundNew>,
    ) -> CustomResult<Vec<Refund>, StorageError> {
        Err(StorageError::MockDbError)?
    }

        /// Asynchronously deletes payment intents for sample data associated with the provided merchant ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A reference to a string representing the merchant ID for which payment intents are to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `PaymentIntent` if successful, otherwise a `StorageError` if an error occurs during the storage operation.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if an error occurs during the deletion process.
    ///
    async fn delete_payment_intents_for_sample_data(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        Err(StorageError::MockDbError)?
    }

        /// Deletes payment attempts for sample data from the database for a specific merchant.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - The ID of the merchant for which payment attempts are to be deleted.
    ///
    /// # Returns
    ///
    /// * `CustomResult<Vec<PaymentAttempt>, StorageError>` - A result indicating success with a vector of deleted payment attempts, or an error if the deletion fails.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if the deletion operation encounters a mock database error.
    ///
    async fn delete_payment_attempts_for_sample_data(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        Err(StorageError::MockDbError)?
    }

        /// Deletes refunds for sample data associated with the given merchant ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A reference to a string representing the merchant ID for which refunds need to be deleted
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `Refund` if successful, otherwise a `StorageError`
    ///
    async fn delete_refunds_for_sample_data(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<Refund>, StorageError> {
        Err(StorageError::MockDbError)?
    }
}

// TODO: This error conversion is re-used from storage_impl and is not DRY when it should be
// Ideally the impl's here should be defined in that crate avoiding this re-definition
fn diesel_error_to_data_error(diesel_error: Report<DatabaseError>) -> Report<StorageError> {
    let new_err = match diesel_error.current_context() {
        DatabaseError::DatabaseConnectionError => StorageError::DatabaseConnectionError,
        DatabaseError::NotFound => StorageError::ValueNotFound("Value not found".to_string()),
        DatabaseError::UniqueViolation => StorageError::DuplicateValue {
            entity: "entity ",
            key: None,
        },
        err => StorageError::DatabaseError(error_stack::report!(*err)),
    };
    diesel_error.change_context(new_err)
}
