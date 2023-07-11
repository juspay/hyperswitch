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

    async fn update_capture_with_attempt_id(
        &self,
        this: types::Capture,
        capture: types::CaptureUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;

    use super::CaptureInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage::{capture::*, enums, kv, ReverseLookupNew},
        utils::{self, db_utils, storage_partitioning},
    };

    #[async_trait::async_trait]
    impl CaptureInterface for Store {
        async fn insert_capture(
            &self,
            capture: CaptureNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    capture
                        .insert(&conn)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", capture.merchant_id, capture.payment_id);

                    let created_capture = Capture {
                        id: Default::default(),
                        payment_id: capture.payment_id.clone(),
                        merchant_id: capture.merchant_id.clone(),
                        attempt_id: capture.attempt_id.clone(),
                        status: capture.status,
                        amount: capture.amount,
                        currency: capture.currency,
                        connector: capture.connector.clone(),
                        error_message: capture.error_message.clone(),
                        error_code: capture.error_code.clone(),
                        error_reason: capture.error_reason.clone(),
                        tax_amount: capture.tax_amount,
                        created_at: capture
                            .created_at
                            .unwrap_or_else(common_utils::date_time::now),
                        modified_at: capture
                            .modified_at
                            .unwrap_or_else(common_utils::date_time::now),
                        authorized_attempt_id: capture.authorized_attempt_id.clone(),
                        capture_sequence: capture.capture_sequence,
                    };

                    let field = format!("capture_{}", created_capture.attempt_id);
                    match self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .serialize_and_set_hash_field_if_not_exist(&key, &field, &created_capture)
                        .await
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "capture",
                            key: Some(key),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => {
                            let conn = connection::pg_connection_write(self).await?;

                            //Reverse lookup for attempt_id
                            ReverseLookupNew {
                                lookup_id: format!(
                                    "{}_{}",
                                    &created_capture.merchant_id, &created_capture.attempt_id,
                                ),
                                pk_id: key,
                                sk_id: field,
                                source: "capture".to_string(),
                            }
                            .insert(&conn)
                            .await
                            .map_err(Into::<errors::StorageError>::into)
                            .into_report()?;

                            let redis_entry = kv::TypedSql {
                                op: kv::DBOperation::Insert {
                                    insertable: kv::Insertable::Capture(Box::new(capture)),
                                },
                            };
                            self.push_to_drainer_stream::<Capture>(
                                redis_entry,
                                crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                                    merchant_id: &created_capture.merchant_id,
                                    payment_id: &created_capture.payment_id,
                                }
                            )
                            .await?;
                            Ok(created_capture)
                        }
                        Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                    }
                }
            }
        }
        async fn find_capture_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                Capture::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("{payment_id}_{merchant_id}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;
                    let key = &lookup.pk_id;

                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "Capture"),
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn update_capture_with_attempt_id(
            &self,
            this: Capture,
            capture: CaptureUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Capture, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    this.update_with_attempt_id(&conn, capture)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", this.merchant_id, this.payment_id);

                    let updated_capture = capture.clone().apply_changeset(this.clone());
                    // Check for database presence as well Maybe use a read replica here ?

                    let redis_value =
                        utils::Encode::<Capture>::encode_to_string_of_json(&updated_capture)
                            .change_context(errors::StorageError::SerializationFailed)?;

                    let updated_capture = self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_hash_fields(&key, ("pi", &redis_value))
                        .await
                        .map(|_| updated_capture)
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::CaptureUpdate(Box::new(
                                kv::CaptureUpdateMems {
                                    orig: this,
                                    update_data: capture,
                                },
                            )),
                        },
                    };

                    self.push_to_drainer_stream::<Capture>(
                        redis_entry,
                        storage_partitioning::PartitionKey::MerchantIdPaymentId {
                            merchant_id: &updated_capture.merchant_id,
                            payment_id: &updated_capture.payment_id,
                        },
                    )
                    .await?;
                    Ok(updated_capture)
                }
            }
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
        async fn update_capture_with_attempt_id(
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
        let id = captures.len() as i32;
        let time = common_utils::date_time::now();

        let capture = types::Capture {
            id,
            payment_id: capture.payment_id,
            merchant_id: capture.merchant_id,
            attempt_id: capture.attempt_id,
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

    async fn update_capture_with_attempt_id(
        &self,
        _this: types::Capture,
        _capture: types::CaptureUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::Capture, errors::StorageError> {
        //Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
