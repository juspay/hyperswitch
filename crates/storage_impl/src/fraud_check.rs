use common_utils::errors::CustomResult;
use diesel_models::fraud_check as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::fraud_check::FraudCheckInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> FraudCheckInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_fraud_check_response(
        &self,
        new: storage::FraudCheckNew,
    ) -> CustomResult<storage::FraudCheck, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_fraud_check_response_with_attempt_id(
        &self,
        this: storage::FraudCheck,
        fraud_check: storage::FraudCheckUpdate,
    ) -> CustomResult<storage::FraudCheck, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update_with_attempt_id(&conn, fraud_check)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_fraud_check_by_payment_id(
        &self,
        payment_id: common_utils::id_type::PaymentId,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> CustomResult<storage::FraudCheck, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::FraudCheck::get_with_payment_id(&conn, payment_id, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_fraud_check_by_payment_id_if_present(
        &self,
        payment_id: common_utils::id_type::PaymentId,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> CustomResult<Option<storage::FraudCheck>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::FraudCheck::get_with_payment_id_if_present(&conn, payment_id, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
