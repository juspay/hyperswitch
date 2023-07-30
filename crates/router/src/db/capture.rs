use diesel_models::payment_attempt::PaymentAttemptUpdate;
use error_stack::ResultExt;

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    db::payment_attempt::PaymentAttemptInterface,
    types::storage::{self as types, enums, errors as storage_errors},
    utils,
};

#[async_trait::async_trait]
pub trait CaptureInterface {
    async fn insert_capture(
        &self,
        capture: types::CaptureNew,
        parent_attempt: types::PaymentAttempt,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(types::Capture, types::PaymentAttempt), errors::StorageError>;

    async fn find_all_captures_by_authorized_attempt_id(
        &self,
        authorized_attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError>;

    async fn find_all_charged_captures_by_authorized_attempt_id(
        &self,
        authorized_attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError>;

    async fn update_capture_and_attempt_with_capture_id(
        &self,
        this: types::Capture,
        parent_attempt: types::PaymentAttempt,
        intent: &types::PaymentIntent,
        capture: types::CaptureUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(types::Capture, types::PaymentAttempt), errors::StorageError>;

    fn validate_attempt_and_capture(
        &self,
        parent_attempt: &types::PaymentAttempt,
        authorized_attempt_id: &String,
    ) -> CustomResult<(), errors::StorageError> {
        utils::when(&parent_attempt.attempt_id != authorized_attempt_id, || {
            Err(
                errors::StorageError::DatabaseError(storage_errors::DatabaseError::Others.into())
                    .into(),
            )
            .attach_printable("authorized_attempt_id of capture did not match parent attempt_id")
        })
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use error_stack::IntoReport;

    use super::CaptureInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        db::payment_attempt::PaymentAttemptInterface,
        services::Store,
        types::storage::{self, capture::*, enums},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
        async fn insert_capture(
            &self,
            capture: CaptureNew,
            parent_attempt: storage::PaymentAttempt,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<(Capture, storage::PaymentAttempt), errors::StorageError> {
            self.validate_attempt_and_capture(&parent_attempt, &capture.authorized_attempt_id)?;
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                let new_capture = capture
                    .insert(&conn)
                    .await
                    .map_err(Into::into)
                    .into_report()?;
                let previous_count = parent_attempt.multiple_capture_count.unwrap_or_default();
                let updated_attempt = self
                    .update_payment_attempt_with_attempt_id(
                        parent_attempt,
                        storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                            status: None,
                            multiple_capture_count: Some(previous_count + 1),
                        },
                        storage_scheme,
                    )
                    .await?;
                Ok((new_capture, updated_attempt))
            };
            db_call().await
        }

        async fn update_capture_and_attempt_with_capture_id(
            &self,
            this: Capture,
            parent_attempt: storage::PaymentAttempt,
            intent: &storage::PaymentIntent,
            capture: CaptureUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<(Capture, storage::PaymentAttempt), errors::StorageError> {
            self.validate_attempt_and_capture(&parent_attempt, &this.authorized_attempt_id)?;
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                let previous_capture_status = this.status;
                let updated_capture = this
                    .update_with_capture_id(&conn, capture)
                    .await
                    .map_err(Into::into)
                    .into_report()?;
                let attempt_update = if updated_capture.status != previous_capture_status {
                    //if capture status is updated, lets update attempt status accordingly
                    match updated_capture.status {
                        enums::CaptureStatus::Charged => {
                            let total_amount_captured =
                                intent.amount_captured.unwrap_or_default() + updated_capture.amount;
                            let authorized_amount = parent_attempt.amount;
                            Some(
                                storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                                    status: Some(if total_amount_captured < authorized_amount {
                                        enums::AttemptStatus::PartialCharged
                                    } else {
                                        enums::AttemptStatus::Charged
                                    }),
                                    multiple_capture_count: None,
                                },
                            )
                        }
                        api_models::enums::CaptureStatus::Pending => Some(
                            storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                                status: Some(enums::AttemptStatus::CaptureInitiated),
                                multiple_capture_count: None,
                            },
                        ),
                        //for rest of the cases, don't update payment_attempt
                        api_models::enums::CaptureStatus::Started
                        | api_models::enums::CaptureStatus::Failure => None,
                    }
                } else {
                    None
                };
                let updated_attempt = match attempt_update {
                    Some(payment_attempt_update) => {
                        self.update_payment_attempt_with_attempt_id(
                            parent_attempt,
                            payment_attempt_update,
                            storage_scheme,
                        )
                        .await?
                    }
                    None => parent_attempt,
                };
                Ok((updated_capture, updated_attempt))
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
                Capture::find_all_by_authorized_attempt_id(authorized_attempt_id, &conn)
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
        db::payment_attempt::PaymentAttemptInterface,
        services::Store,
        types::storage::{self, capture::*, enums},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
        async fn insert_capture(
            &self,
            capture: CaptureNew,
            parent_attempt: storage::PaymentAttempt,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<(Capture, storage::PaymentAttempt), errors::StorageError> {
            self.validate_attempt_and_capture(&parent_attempt, &capture.authorized_attempt_id)?;
            let conn = connection::pg_connection_write(self).await?;
            let new_capture = capture
                .insert(&conn)
                .await
                .map_err(Into::into)
                .into_report()?;
            let previous_count = parent_attempt.multiple_capture_count.unwrap_or_default();
            let updated_attempt = self
                .update_payment_attempt_with_attempt_id(
                    parent_attempt,
                    storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                        status: None,
                        multiple_capture_count: Some(previous_count + 1),
                    },
                    storage_scheme,
                )
                .await?;
            Ok((new_capture, updated_attempt))
        }
        async fn update_capture_and_attempt_with_capture_id(
            &self,
            this: Capture,
            parent_attempt: storage::PaymentAttempt,
            intent: &storage::PaymentIntent,
            capture: CaptureUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<(Capture, storage::PaymentAttempt), errors::StorageError> {
            self.validate_attempt_and_capture(&parent_attempt, &this.authorized_attempt_id)?;
            let conn = connection::pg_connection_write(self).await?;
            let previous_capture_status = this.status;
            let updated_capture = this
                .update_with_capture_id(&conn, capture)
                .await
                .map_err(Into::into)
                .into_report()?;
            let attempt_update = if updated_capture.status != previous_capture_status {
                //if capture status is updated, lets update attempt status accordingly
                match updated_capture.status {
                    enums::CaptureStatus::Charged => {
                        let total_amount_captured =
                            intent.amount_captured.unwrap_or_default() + updated_capture.amount;
                        let authorized_amount = parent_attempt.amount;
                        Some(
                            storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                                status: Some(if total_amount_captured < authorized_amount {
                                    enums::AttemptStatus::PartialCharged
                                } else {
                                    enums::AttemptStatus::Charged
                                }),
                                multiple_capture_count: None,
                            },
                        )
                    }
                    api_models::enums::CaptureStatus::Pending => Some(
                        storage::PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                            status: Some(enums::AttemptStatus::CaptureInitiated),
                            multiple_capture_count: None,
                        },
                    ),
                    //for rest of the cases, don't update payment_attempt
                    api_models::enums::CaptureStatus::Started
                    | api_models::enums::CaptureStatus::Failure => None,
                }
            } else {
                None
            };
            let updated_attempt = match attempt_update {
                Some(payment_attempt_update) => {
                    self.update_payment_attempt_with_attempt_id(
                        parent_attempt,
                        payment_attempt_update,
                        storage_scheme,
                    )
                    .await?
                }
                None => parent_attempt,
            };
            Ok((updated_capture, updated_attempt))
        }

        async fn find_all_captures_by_authorized_attempt_id(
            &self,
            authorized_attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let db_call = || async {
                let conn = connection::pg_connection_write(self).await?;
                Capture::find_all_by_authorized_attempt_id(authorized_attempt_id, &conn)
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
        parent_attempt: types::PaymentAttempt,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(types::Capture, types::PaymentAttempt), errors::StorageError> {
        self.validate_attempt_and_capture(&parent_attempt, &capture.authorized_attempt_id)?;
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
            connector_transaction_id: capture.connector_transaction_id,
        };
        let previous_count = parent_attempt.multiple_capture_count.unwrap_or_default();
        let updated_attempt = self
            .update_payment_attempt_with_attempt_id(
                parent_attempt,
                PaymentAttemptUpdate::MultipleCaptureResponseUpdate {
                    status: None,
                    multiple_capture_count: Some(previous_count + 1),
                },
                storage_scheme,
            )
            .await?;
        captures.push(capture.clone());
        Ok((capture, updated_attempt))
    }

    async fn update_capture_and_attempt_with_capture_id(
        &self,
        _this: types::Capture,
        _parent_attempt: types::PaymentAttempt,
        _intent: &types::PaymentIntent,
        _capture: types::CaptureUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(types::Capture, types::PaymentAttempt), errors::StorageError> {
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

    async fn find_all_charged_captures_by_authorized_attempt_id(
        &self,
        _authorized_attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
