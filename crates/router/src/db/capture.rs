use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as types, enums},
};

#[async_trait::async_trait]
pub trait CaptureInterface {
    async fn insert_capture(
        &self,
        capture: types::CaptureNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError>;

    async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        authorized_attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError>;

    async fn update_capture_with_capture_id(
        &self,
        this: types::Capture,
        capture: types::CaptureUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use error_stack::IntoReport;

    use super::CaptureInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{capture::*, enums},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
                /// Asynchronously inserts a new capture into the database based on the provided storage scheme.
        ///
        /// # Arguments
        ///
        /// * `capture` - The new capture to be inserted.
        /// * `_storage_scheme` - The storage scheme to be used for the insertion.
        ///
        /// # Returns
        ///
        /// * `CustomResult<Capture, errors::StorageError>` - A result indicating success with the inserted capture or an error of type `StorageError`.
        ///
        async fn insert_capture(
            &self,
            capture: CaptureNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                capture
                    .insert(&conn)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

                /// Asynchronously updates a capture with the given capture ID using the provided information and storage scheme.
        async fn update_capture_with_capture_id(
            &self,
            this: Capture,
            capture: CaptureUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                this.update_with_capture_id(&conn, capture)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

                /// Asynchronously finds all captures by the given merchant ID, payment ID, and authorized attempt ID using the specified storage scheme. Returns a vector of captures or a storage error.
        async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
            &self,
            merchant_id: &str,
            payment_id: &str,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                Capture::find_all_by_merchant_id_payment_id_authorized_attempt_id(
                    merchant_id,
                    payment_id,
                    authorized_attempt_id,
                    &conn,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            db_call().await
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::IntoReport;

    use super::CaptureInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{capture::*, enums},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
        async fn insert_capture(
            &self,
            capture: CaptureNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                capture
                    .insert(&conn)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

        async fn update_capture_with_capture_id(
            &self,
            this: Capture,
            capture: CaptureUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                this.update_with_capture_id(&conn, capture)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

        async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
            &self,
            merchant_id: &str,
            payment_id: &str,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                Capture::find_all_by_merchant_id_payment_id_authorized_attempt_id(
                    merchant_id,
                    payment_id,
                    authorized_attempt_id,
                    &conn,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            db_call().await
        }
    }
}

#[async_trait::async_trait]
impl CaptureInterface for MockDb {
        /// Asynchronously inserts a new capture into the storage and returns the inserted capture.
    ///
    /// # Arguments
    ///
    /// * `capture` - The new capture to be inserted
    /// * `_storage_scheme` - The storage scheme enumeration
    ///
    /// # Returns
    ///
    /// The inserted capture wrapped in a `CustomResult` or a `StorageError` if the operation fails.
    ///
    async fn insert_capture(
        &self,
        capture: types::CaptureNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        let mut captures = self.captures.lock().await;
        let capture = types::Capture {
            capture_id: capture.capture_id,
            payment_id: capture.payment_id,
            merchant_id: capture.merchant_id,
            status: capture.status,
            amount: capture.amount,
            currency: capture.currency,
            connector: capture.connector,
            error_message: capture.error_message,
            error_code: capture.error_code,
            error_reason: capture.error_reason,
            tax_amount: capture.tax_amount,
            created_at: capture.created_at,
            modified_at: capture.modified_at,
            authorized_attempt_id: capture.authorized_attempt_id,
            capture_sequence: capture.capture_sequence,
            connector_capture_id: capture.connector_capture_id,
            connector_response_reference_id: capture.connector_response_reference_id,
        };
        captures.push(capture.clone());
        Ok(capture)
    }

    #[instrument(skip_all)]
        /// Asynchronously updates a capture with the given capture ID using the provided capture data and storage scheme.
    /// 
    /// # Arguments
    /// * `this` - The capture to be updated.
    /// * `capture` - The updated capture data.
    /// * `storage_scheme` - The storage scheme to be used for the update.
    /// 
    /// # Returns
    /// The updated capture if successful, otherwise a `StorageError`.
    ///
    async fn update_capture_with_capture_id(
        &self,
        _this: types::Capture,
        _capture: types::CaptureUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
        /// Asynchronously finds all captures by the specified merchant ID, payment ID, authorized attempt ID, and storage scheme.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - The merchant ID to filter captures by.
    /// * `_payment_id` - The payment ID to filter captures by.
    /// * `_authorized_attempt_id` - The authorized attempt ID to filter captures by.
    /// * `_storage_scheme` - The storage scheme to use for the merchant.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing a vector of `Capture` objects if successful, or a `StorageError` if an error occurs.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if the function is implemented for `MockDb`.
    ///
    async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
        &self,
        _merchant_id: &str,
        _payment_id: &str,
        _authorized_attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
