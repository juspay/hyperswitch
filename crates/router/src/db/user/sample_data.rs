use diesel_models::{
    errors::DatabaseError,
    query::user::sample_data as sample_data_queries,
    refund::{Refund, RefundNew},
    user::sample_data::PaymentAttemptBatchNew,
};
use error_stack::{Report, ResultExt};
use futures::{future::try_join_all, FutureExt};
use hyperswitch_domain_models::{
    behaviour::Conversion,
    errors::StorageError,
    merchant_key_store::MerchantKeyStore,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use storage_impl::DataModelExt;

use crate::{connection::pg_connection_write, core::errors::CustomResult, services::Store};

#[async_trait::async_trait]
pub trait BatchSampleDataInterface {
    async fn insert_payment_intents_batch_for_sample_data(
        &self,
        batch: Vec<PaymentIntent>,
        key_store: &MerchantKeyStore,
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
        key_store: &MerchantKeyStore,
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
        batch: Vec<PaymentIntent>,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        let new_intents = try_join_all(batch.into_iter().map(|payment_intent| async {
            payment_intent
                .construct_new()
                .await
                .change_context(StorageError::EncryptionError)
        }))
        .await?;

        sample_data_queries::insert_payment_intents(&conn, new_intents)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|v| {
                try_join_all(v.into_iter().map(|payment_intent| {
                    PaymentIntent::convert_back(payment_intent, key_store.key.get_inner())
                }))
                .map(|join_result| join_result.change_context(StorageError::DecryptionError))
            })?
            .await
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
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<PaymentIntent>, StorageError> {
        let conn = pg_connection_write(self)
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        sample_data_queries::delete_payment_intents(&conn, merchant_id)
            .await
            .map_err(diesel_error_to_data_error)
            .map(|v| {
                try_join_all(v.into_iter().map(|payment_intent| {
                    PaymentIntent::convert_back(payment_intent, key_store.key.get_inner())
                }))
                .map(|join_result| join_result.change_context(StorageError::DecryptionError))
            })?
            .await
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
        _batch: Vec<PaymentIntent>,
        _key_store: &MerchantKeyStore,
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
        _key_store: &MerchantKeyStore,
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
