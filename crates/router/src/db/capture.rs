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
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
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
    use error_stack::{report, ResultExt};
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::{
        redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
        utils::find_all_combined_kv_database,
        KvSupportedEntity,
    };

    use super::CaptureInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        services::Store,
        types::storage::{capture::*, enums},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
        #[instrument(skip_all)]
        async fn insert_capture(
            &self,
            capture: CaptureNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let storage_scheme = Box::pin(decide_storage_scheme::<_, Capture>(
                self,
                storage_scheme,
                Op::Insert,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    capture
                        .insert(&conn)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let created_capture = Capture {
                        capture_id: capture.capture_id.clone(),
                        payment_id: capture.payment_id.clone(),
                        merchant_id: capture.merchant_id.clone(),
                        status: capture.status,
                        amount: capture.amount,
                        currency: capture.currency,
                        connector: capture.connector.clone(),
                        error_message: capture.error_message.clone(),
                        error_code: capture.error_code.clone(),
                        error_reason: capture.error_reason.clone(),
                        tax_amount: capture.tax_amount,
                        created_at: capture.created_at,
                        modified_at: capture.created_at,
                        authorized_attempt_id: capture.authorized_attempt_id.clone(),
                        connector_capture_id: capture.connector_capture_id.clone(),
                        capture_sequence: capture.capture_sequence,
                        connector_response_reference_id: capture
                            .connector_response_reference_id
                            .clone(),
                        processor_capture_data: capture.processor_capture_data.clone(),
                        connector_capture_data: capture.connector_capture_data.clone(),
                    };

                    let key = created_capture.get_partition_key();
                    let key_str = key.to_string();
                    let field = created_capture.get_hash_field_key();

                    let mut query_gen_conn = connection::pg_connection_write(self).await?;
                    let drainer_query = capture
                        .generate_drainer_insert_query(&mut query_gen_conn)
                        .await
                        .change_context(errors::StorageError::KVError)
                        .attach_printable("Failed to generate capture insert query")?;

                    match Box::pin(kv_wrapper::<Capture, _, _>(
                        self,
                        KvOperation::<Capture>::HSetNx(&field, &created_capture, drainer_query),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "capture",
                            key: Some(created_capture.capture_id),
                        }
                        .into()),
                        Ok(HsetnxReply::KeySet) => Ok(created_capture),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        #[instrument(skip_all)]
        async fn update_capture_with_capture_id(
            &self,
            this: Capture,
            capture: CaptureUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let key = this.get_partition_key();
            let field = this.get_hash_field_key();
            let storage_scheme = Box::pin(decide_storage_scheme::<_, Capture>(
                self,
                storage_scheme,
                Op::Update(key.clone(), &field, None),
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    this.update_with_capture_id(&conn, capture)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key_str = key.to_string();
                    let capture_id = this.capture_id.clone();
                    let updated_capture =
                        CaptureUpdateInternal::from(capture.clone()).apply_changeset(this.clone());

                    let redis_value = serde_json::to_string(&updated_capture)
                        .change_context(errors::StorageError::SerializationFailed)?;

                    let mut query_gen_conn = connection::pg_connection_write(self).await?;
                    let drainer_query = capture
                        .generate_drainer_update_query(&mut query_gen_conn, capture_id)
                        .await
                        .change_context(errors::StorageError::KVError)
                        .attach_printable("Failed to generate capture update query")?;

                    Box::pin(kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<Capture>((&field, redis_value), drainer_query),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_capture)
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            payment_id: &common_utils::id_type::PaymentId,
            authorized_attempt_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<Capture>, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                Capture::find_all_by_merchant_id_payment_id_authorized_attempt_id(
                    merchant_id,
                    payment_id,
                    authorized_attempt_id,
                    &conn,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, Capture>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdPaymentId {
                        merchant_id,
                        payment_id,
                    };
                    let pattern = format!("pa_{authorized_attempt_id}_capture_*");

                    let redis_fut = async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<Capture>::Scan(&pattern),
                            key,
                        ))
                        .await?
                        .try_into_scan()
                    };

                    let mut captures = Box::pin(find_all_combined_kv_database(
                        redis_fut,
                        database_call,
                        None,
                    ))
                    .await?;

                    captures.sort_by_key(|c| c.capture_sequence);

                    Ok(captures)
                }
            }
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::report;
    use router_env::{instrument, tracing};

    use super::CaptureInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{capture::*, enums},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
        #[instrument(skip_all)]
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
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };
            db_call().await
        }

        #[instrument(skip_all)]
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
                .map_err(|error| report!(errors::StorageError::from(error)))
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
            processor_capture_data: capture.processor_capture_data,
            // Below fields are deprecated. Please add any new fields above this line.
            connector_capture_data: None,
        };
        captures.push(capture.clone());
        Ok(capture)
    }

    #[instrument(skip_all)]
    async fn update_capture_with_capture_id(
        &self,
        _this: types::Capture,
        _capture: types::CaptureUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
    async fn find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _payment_id: &common_utils::id_type::PaymentId,
        _authorized_attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::Capture>, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
