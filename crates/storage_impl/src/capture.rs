use common_utils::errors::CustomResult;
use diesel_models::{capture as storage, enums};
use error_stack::report;
use router_env::{instrument, tracing};
use sample::capture::CaptureInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> CaptureInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_capture(
        &self,
        capture: storage::CaptureNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Capture, errors::StorageError> {
        let db_call = || async {
            let conn = connection::pg_connection_write(self).await?;
            capture
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };
        db_call().await
    }

    #[instrument(skip_all)]
    async fn update_capture_with_capture_id(
        &self,
        this: storage::Capture,
        capture: storage::CaptureUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Capture, errors::StorageError> {
        let db_call = || async {
            let conn = connection::pg_connection_write(self).await?;
            this.update_with_capture_id(&conn, capture)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };
        db_call().await
    }

    #[instrument(skip_all)]
    async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        authorized_attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage::Capture>, errors::StorageError> {
        let db_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::Capture::find_all_by_merchant_id_payment_id_authorized_attempt_id(
                merchant_id,
                payment_id,
                authorized_attempt_id,
                &conn,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };
        db_call().await
    }
}
