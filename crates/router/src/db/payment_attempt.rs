use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as types, enums},
};

#[async_trait::async_trait]
pub trait PaymentAttemptInterface {
    async fn insert_payment_attempt(
        &self,
        payment_attempt: types::PaymentAttemptNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;

    async fn update_payment_attempt(
        &self,
        this: types::PaymentAttempt,
        payment_attempt: types::PaymentAttemptUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
        connector_txn_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_merchant_id_attempt_id(
        &self,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::IntoReport;

    use super::PaymentAttemptInterface;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{enums, payment_attempt::*},
    };

    #[async_trait::async_trait]
    impl PaymentAttemptInterface for Store {
        async fn insert_payment_attempt(
            &self,
            payment_attempt: PaymentAttemptNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            payment_attempt
                .insert(&conn)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn update_payment_attempt(
            &self,
            this: PaymentAttempt,
            payment_attempt: PaymentAttemptUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            this.update(&conn, payment_attempt)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn find_payment_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            PaymentAttempt::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
            &self,
            connector_transaction_id: &str,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            PaymentAttempt::find_by_connector_transaction_id_payment_id_merchant_id(
                &conn,
                connector_transaction_id,
                payment_id,
                merchant_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            PaymentAttempt::find_last_successful_attempt_by_payment_id_merchant_id(
                &conn,
                payment_id,
                merchant_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        async fn find_payment_attempt_by_merchant_id_connector_txn_id(
            &self,
            merchant_id: &str,
            connector_txn_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            PaymentAttempt::find_by_merchant_id_connector_txn_id(
                &conn,
                merchant_id,
                connector_txn_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        async fn find_payment_attempt_by_merchant_id_attempt_id(
            &self,
            merchant_id: &str,
            attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;

            PaymentAttempt::find_by_merchant_id_attempt_id(&conn, merchant_id, attempt_id)
                .await
                .map_err(Into::into)
                .into_report()
        }
    }
}

#[async_trait::async_trait]
impl PaymentAttemptInterface for MockDb {
    async fn find_payment_attempt_by_merchant_id_attempt_id(
        &self,
        _merchant_id: &str,
        _attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        _merchant_id: &str,
        _connector_txn_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: types::PaymentAttemptNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        let mut payment_attempts = self.payment_attempts.lock().await;
        #[allow(clippy::as_conversions)]
        let id = payment_attempts.len() as i32;
        let time = common_utils::date_time::now();

        let payment_attempt = types::PaymentAttempt {
            id,
            payment_id: payment_attempt.payment_id,
            merchant_id: payment_attempt.merchant_id,
            attempt_id: payment_attempt.attempt_id,
            status: payment_attempt.status,
            amount: payment_attempt.amount,
            currency: payment_attempt.currency,
            save_to_locker: payment_attempt.save_to_locker,
            connector: payment_attempt.connector,
            error_message: payment_attempt.error_message,
            offer_amount: payment_attempt.offer_amount,
            surcharge_amount: payment_attempt.surcharge_amount,
            tax_amount: payment_attempt.tax_amount,
            payment_method_id: payment_attempt.payment_method_id,
            payment_method: payment_attempt.payment_method,
            connector_transaction_id: payment_attempt.connector_transaction_id,
            capture_method: payment_attempt.capture_method,
            capture_on: payment_attempt.capture_on,
            confirm: payment_attempt.confirm,
            authentication_type: payment_attempt.authentication_type,
            created_at: payment_attempt.created_at.unwrap_or(time),
            modified_at: payment_attempt.modified_at.unwrap_or(time),
            last_synced: payment_attempt.last_synced,
            cancellation_reason: payment_attempt.cancellation_reason,
            amount_to_capture: payment_attempt.amount_to_capture,
            mandate_id: None,
            browser_info: None,
            payment_token: None,
            error_code: payment_attempt.error_code,
            connector_metadata: None,
            payment_experience: payment_attempt.payment_experience,
            payment_method_type: payment_attempt.payment_method_type,
            payment_method_data: payment_attempt.payment_method_data,
        };
        payment_attempts.push(payment_attempt.clone());
        Ok(payment_attempt)
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_attempt(
        &self,
        this: types::PaymentAttempt,
        payment_attempt: types::PaymentAttemptUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        let mut payment_attempts = self.payment_attempts.lock().await;

        let item = payment_attempts
            .iter_mut()
            .find(|item| item.id == this.id)
            .unwrap();

        *item = payment_attempt.apply_changeset(this);

        Ok(item.clone())
    }

    async fn find_payment_attempt_by_payment_id_merchant_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        _connector_transaction_id: &str,
        _payment_id: &str,
        _merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentAttempt, errors::StorageError> {
        let payment_attempts = self.payment_attempts.lock().await;

        Ok(payment_attempts
            .iter()
            .find(|payment_attempt| {
                payment_attempt.payment_id == payment_id
                    && payment_attempt.merchant_id == merchant_id
            })
            .cloned()
            .unwrap())
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::date_time;
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;

    use super::PaymentAttemptInterface;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage::{enums, kv, payment_attempt::*, ReverseLookupNew},
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl PaymentAttemptInterface for Store {
        async fn insert_payment_attempt(
            &self,
            payment_attempt: PaymentAttemptNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await?;
                    payment_attempt
                        .insert(&conn)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!(
                        "{}_{}",
                        payment_attempt.merchant_id, payment_attempt.payment_id
                    );

                    let created_attempt = PaymentAttempt {
                        id: Default::default(),
                        payment_id: payment_attempt.payment_id.clone(),
                        merchant_id: payment_attempt.merchant_id.clone(),
                        attempt_id: payment_attempt.attempt_id.clone(),
                        status: payment_attempt.status,
                        amount: payment_attempt.amount,
                        currency: payment_attempt.currency,
                        save_to_locker: payment_attempt.save_to_locker,
                        connector: payment_attempt.connector.clone(),
                        error_message: payment_attempt.error_message.clone(),
                        offer_amount: payment_attempt.offer_amount,
                        surcharge_amount: payment_attempt.surcharge_amount,
                        tax_amount: payment_attempt.tax_amount,
                        payment_method_id: payment_attempt.payment_method_id.clone(),
                        payment_method: payment_attempt.payment_method,
                        connector_transaction_id: payment_attempt.connector_transaction_id.clone(),
                        capture_method: payment_attempt.capture_method,
                        capture_on: payment_attempt.capture_on,
                        confirm: payment_attempt.confirm,
                        authentication_type: payment_attempt.authentication_type,
                        created_at: payment_attempt.created_at.unwrap_or_else(date_time::now),
                        modified_at: payment_attempt.created_at.unwrap_or_else(date_time::now),
                        last_synced: payment_attempt.last_synced,
                        amount_to_capture: payment_attempt.amount_to_capture,
                        cancellation_reason: payment_attempt.cancellation_reason.clone(),
                        mandate_id: payment_attempt.mandate_id.clone(),
                        browser_info: payment_attempt.browser_info.clone(),
                        payment_token: payment_attempt.payment_token.clone(),
                        error_code: payment_attempt.error_code.clone(),
                        connector_metadata: payment_attempt.connector_metadata.clone(),
                        payment_experience: payment_attempt.payment_experience.clone(),
                        payment_method_type: payment_attempt.payment_method_type.clone(),
                        payment_method_data: payment_attempt.payment_method_data.clone(),
                    };

                    let field = format!("pa_{}", created_attempt.attempt_id);
                    match self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .serialize_and_set_hash_field_if_not_exist(&key, &field, &created_attempt)
                        .await
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "payment attempt",
                            key: Some(key),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => {
                            let conn = pg_connection(&self.master_pool).await?;

                            //Reverse lookup for attempt_id
                            ReverseLookupNew {
                                lookup_id: format!(
                                    "{}_{}",
                                    &created_attempt.merchant_id,
                                    // [#439]: Change this to `attempt_id`
                                    &created_attempt.payment_id,
                                ),
                                pk_id: key,
                                sk_id: field,
                                source: "payment_attempt".to_string(),
                            }
                            .insert(&conn)
                            .await
                            .map_err(Into::<errors::StorageError>::into)
                            .into_report()?;

                            let redis_entry = kv::TypedSql {
                                op: kv::DBOperation::Insert {
                                    insertable: kv::Insertable::PaymentAttempt(payment_attempt),
                                },
                            };
                            self.push_to_drainer_stream::<PaymentAttempt>(
                                redis_entry,
                                crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                                    merchant_id: &created_attempt.merchant_id,
                                    payment_id: &created_attempt.payment_id,
                                }
                            )
                            .await?;
                            Ok(created_attempt)
                        }
                        Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                    }
                }
            }
        }

        async fn update_payment_attempt(
            &self,
            this: PaymentAttempt,
            payment_attempt: PaymentAttemptUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await?;
                    this.update(&conn, payment_attempt)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", this.merchant_id, this.payment_id);
                    let old_connector_transaction_id = &this.connector_transaction_id;
                    let updated_attempt = payment_attempt.clone().apply_changeset(this.clone());
                    // Check for database presence as well Maybe use a read replica here ?
                    let redis_value = serde_json::to_string(&updated_attempt)
                        .into_report()
                        .change_context(errors::StorageError::KVError)?;
                    let field = format!("pa_{}", updated_attempt.attempt_id);
                    let updated_attempt = self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_hash_fields(&key, (&field, &redis_value))
                        .await
                        .map(|_| updated_attempt)
                        .change_context(errors::StorageError::KVError)?;

                    let conn = pg_connection(&self.master_pool).await?;
                    // Reverse lookup for connector_transaction_id
                    if let (None, Some(connector_transaction_id)) = (
                        old_connector_transaction_id,
                        &updated_attempt.connector_transaction_id,
                    ) {
                        let field = format!("pa_{}", updated_attempt.attempt_id);
                        ReverseLookupNew {
                            lookup_id: format!(
                                "{}_{}",
                                &updated_attempt.merchant_id, connector_transaction_id
                            ),
                            pk_id: key.clone(),
                            sk_id: field.clone(),
                            source: "payment_attempt".to_string(),
                        }
                        .insert(&conn)
                        .await
                        .map_err(Into::<errors::StorageError>::into)
                        .into_report()?;
                    }

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::PaymentAttemptUpdate(
                                kv::PaymentAttemptUpdateMems {
                                    orig: this,
                                    update_data: payment_attempt,
                                },
                            ),
                        },
                    };
                    self.push_to_drainer_stream::<PaymentAttempt>(
                        redis_entry,
                        crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                            merchant_id: &updated_attempt.merchant_id,
                            payment_id: &updated_attempt.payment_id,
                        },
                    )
                    .await?;
                    Ok(updated_attempt)
                }
            }
        }

        async fn find_payment_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
                PaymentAttempt::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    // [#439]: get the attempt_id from payment_intent
                    let key = format!("{merchant_id}_{payment_id}");
                    let lookup = self.get_lookup_by_lookup_id(&key).await?;

                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(
                                &lookup.pk_id,
                                &lookup.sk_id,
                                "PaymentAttempt",
                            ),
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
            &self,
            connector_transaction_id: &str,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
                PaymentAttempt::find_by_connector_transaction_id_payment_id_merchant_id(
                    &conn,
                    connector_transaction_id,
                    payment_id,
                    merchant_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    // We assume that PaymentAttempt <=> PaymentIntent is a one-to-one relation for now
                    let lookup_id = format!("{merchant_id}_{connector_transaction_id}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;
                    let key = &lookup.pk_id;

                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "PaymentAttempt"),
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            self.find_payment_attempt_by_payment_id_merchant_id(
                payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
            .and_then(|attempt| match attempt.status {
                enums::AttemptStatus::Charged => Ok(attempt),
                _ => Err(errors::StorageError::ValueNotFound(format!(
                    "Successful payment attempt does not exist for {payment_id}_{merchant_id}"
                )))
                .into_report(),
            })
        }

        async fn find_payment_attempt_by_merchant_id_connector_txn_id(
            &self,
            merchant_id: &str,
            connector_txn_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
                PaymentAttempt::find_by_merchant_id_connector_txn_id(
                    &conn,
                    merchant_id,
                    connector_txn_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,

                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("{merchant_id}_{connector_txn_id}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;

                    let key = &lookup.pk_id;
                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "PaymentAttempt"),
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn find_payment_attempt_by_merchant_id_attempt_id(
            &self,
            merchant_id: &str,
            attempt_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
                PaymentAttempt::find_by_merchant_id_attempt_id(&conn, merchant_id, attempt_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,

                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("{merchant_id}_{attempt_id}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;
                    let key = &lookup.pk_id;
                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "PaymentAttempt"),
                        database_call,
                    )
                    .await
                }
            }
        }
    }
}
