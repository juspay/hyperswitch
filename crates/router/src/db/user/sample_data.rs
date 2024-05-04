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
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        _batch: Vec<PaymentIntentNew>,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn insert_payment_attempts_batch_for_sample_data(
        &self,
        _batch: Vec<PaymentAttemptBatchNew>,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn insert_refunds_batch_for_sample_data(
        &self,
        _batch: Vec<RefundNew>,
    ) -> CustomResult<Vec<Refund>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn delete_payment_intents_for_sample_data(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        Err(StorageError::MockDbError)?
    }
    async fn delete_payment_attempts_for_sample_data(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<PaymentAttempt>, StorageError> {
        Err(StorageError::MockDbError)?
    }
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
