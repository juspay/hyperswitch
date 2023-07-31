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
}

#[async_trait::async_trait]
impl FraudCheckInterface for Store {
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<FraudCheck, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn).await.map_err(Into::into).into_report()
    }
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
}

#[async_trait::async_trait]
impl FraudCheckInterface for MockDb {
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
