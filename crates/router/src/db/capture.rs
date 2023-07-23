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

    async fn find_capture_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError>;

    async fn find_all_captures_by_authorized_attempt_id(
        &self,
        authorized_attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError>;

    async fn find_all_captures_by_authorized_attempt_ids(
        &self,
        authorized_attempt_id: Vec<String>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError>;

    async fn find_all_charged_captures_by_authorized_attempt_id(
        &self,
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
        async fn find_capture_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                Capture::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
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

        async fn find_all_captures_by_authorized_attempt_id(
            &self,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_by_authorized_attempt_id(&conn, authorized_attempt_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

        async fn find_all_captures_by_authorized_attempt_ids(
            &self,
            authorized_attempt_ids: Vec<String>,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_by_authorized_attempt_id_list(&conn, authorized_attempt_ids)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

        async fn find_all_charged_captures_by_authorized_attempt_id(
            &self,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_charged_by_authorized_attempt_id(authorized_attempt_id, &conn)
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
            let conn = connection::pg_connection_write(self).await?;
            capture
                .insert(&conn)
                .await
                .map_err(Into::into)
                .into_report()
        }
        async fn find_capture_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            Capture::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        }
        async fn update_capture_with_capture_id(
            &self,
            this: Capture,
            capture: CaptureUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            this.update_with_attempt_id(&conn, capture)
                .await
                .map_err(Into::into)
                .into_report()
        }
        async fn find_all_captures_by_authorized_attempt_id(
            &self,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_by_authorized_attempt_id(&conn, authorized_attempt_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

        async fn find_all_captures_by_authorized_attempt_ids(
            &self,
            authorized_attempt_ids: Vec<String>,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_by_authorized_attempt_id_list(&conn, authorized_attempt_ids)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            db_call().await
        }

        async fn find_all_charged_captures_by_authorized_attempt_id(
            &self,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_charged_by_authorized_attempt_id(authorized_attempt_id, &conn)
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
    async fn insert_capture(
        &self,
        capture: types::CaptureNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        let mut captures = self.captures.lock().await;
        #[allow(clippy::as_conversions)]
        let time = common_utils::date_time::now();

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
            created_at: capture.created_at.unwrap_or(time),
            modified_at: capture.modified_at.unwrap_or(time),
            authorized_attempt_id: capture.authorized_attempt_id,
            capture_sequence: capture.capture_sequence,
            connector_transaction_id: capture.connector_transaction_id,
        };
        captures.push(capture.clone());
        Ok(capture)
    }

    async fn find_capture_by_payment_id_merchant_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_capture_with_capture_id(
        &self,
        _this: types::Capture,
        _capture: types::CaptureUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
    async fn find_all_captures_by_authorized_attempt_id(
        &self,
        _authorized_attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
    async fn find_all_captures_by_authorized_attempt_ids(
        &self,
        _authorized_attempt_id: Vec<String>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_all_charged_captures_by_authorized_attempt_id(
        &self,
        _authorized_attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
