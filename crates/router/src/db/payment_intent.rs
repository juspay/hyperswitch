use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::{
        api,
        storage::{self as types, enums},
    },
};

#[async_trait::async_trait]
pub trait PaymentIntentInterface {
    async fn update_payment_intent(
        &self,
        this: types::PaymentIntent,
        payment_intent: types::PaymentIntentUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentIntent, errors::StorageError>;

    async fn insert_payment_intent(
        &self,
        new: types::PaymentIntentNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentIntent, errors::StorageError>;

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentIntent, errors::StorageError>;

    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        pc: &api::PaymentListConstraints,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::PaymentIntent>, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::date_time;
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::{HsetnxReply, RedisEntryId};

    use super::PaymentIntentInterface;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::{
            api,
            storage::{enums, kv, payment_intent::*},
        },
        utils::storage_partitioning::KvStorePartition,
    };

    #[async_trait::async_trait]
    impl PaymentIntentInterface for Store {
        async fn insert_payment_intent(
            &self,
            new: PaymentIntentNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await;
                    new.insert(&conn).await.map_err(Into::into).into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", new.merchant_id, new.payment_id);
                    let created_intent = PaymentIntent {
                        id: 0i32,
                        payment_id: new.payment_id.clone(),
                        merchant_id: new.merchant_id.clone(),
                        status: new.status,
                        amount: new.amount,
                        currency: new.currency,
                        amount_captured: new.amount_captured,
                        customer_id: new.customer_id.clone(),
                        description: new.description.clone(),
                        return_url: new.return_url.clone(),
                        metadata: new.metadata.clone(),
                        connector_id: new.connector_id.clone(),
                        shipping_address_id: new.shipping_address_id.clone(),
                        billing_address_id: new.billing_address_id.clone(),
                        statement_descriptor_name: new.statement_descriptor_name.clone(),
                        statement_descriptor_suffix: new.statement_descriptor_suffix.clone(),
                        created_at: new.created_at.unwrap_or_else(date_time::now),
                        modified_at: new.created_at.unwrap_or_else(date_time::now),
                        last_synced: new.last_synced,
                        setup_future_usage: new.setup_future_usage,
                        off_session: new.off_session,
                        client_secret: new.client_secret.clone(),
                    };

                    match self
                        .redis_conn
                        .serialize_and_set_hash_field_if_not_exist(&key, "pi", &created_intent)
                        .await
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue(
                            format!("Payment Intent already exists for payment_id: {key}"),
                        ))
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => {
                            let redis_entry = kv::TypedSql {
                                op: kv::DBOperation::Insert {
                                    insertable: kv::Insertable::PaymentIntent(new),
                                },
                            };
                            let stream_name = self.get_drainer_stream_name(&PaymentIntent::shard_key(
                                crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                                    merchant_id: &created_intent.merchant_id,
                                    payment_id: &created_intent.payment_id,
                                },
                                self.config.drainer_num_partitions,
                            ));
                            self.redis_conn
                                .stream_append_entry(
                                    &stream_name,
                                    &RedisEntryId::AutoGeneratedID,
                                    redis_entry
                                        .to_field_value_pairs()
                                        .change_context(errors::StorageError::KVError)?,
                                )
                                .await
                                .change_context(errors::StorageError::KVError)?;
                            Ok(created_intent)
                        }
                        Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                    }
                }
            }
        }

        async fn update_payment_intent(
            &self,
            this: PaymentIntent,
            payment_intent: PaymentIntentUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await;
                    this.update(&conn, payment_intent)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", this.merchant_id, this.payment_id);

                    let updated_intent = payment_intent.clone().apply_changeset(this.clone());
                    // Check for database presence as well Maybe use a read replica here ?
                    let redis_value = serde_json::to_string(&updated_intent)
                        .into_report()
                        .change_context(errors::StorageError::KVError)?;
                    let updated_intent = self
                        .redis_conn
                        .set_hash_fields(&key, ("pi", &redis_value))
                        .await
                        .map(|_| updated_intent)
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::PaymentIntentUpdate(
                                kv::PaymentIntentUpdateMems {
                                    orig: this,
                                    update_data: payment_intent,
                                },
                            ),
                        },
                    };

                    let stream_name = self.get_drainer_stream_name(&PaymentIntent::shard_key(
                        crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                            merchant_id: &updated_intent.merchant_id,
                            payment_id: &updated_intent.payment_id,
                        },
                        self.config.drainer_num_partitions,
                    ));
                    self.redis_conn
                        .stream_append_entry(
                            &stream_name,
                            &RedisEntryId::AutoGeneratedID,
                            redis_entry
                                .to_field_value_pairs()
                                .change_context(errors::StorageError::KVError)?,
                        )
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(updated_intent)
                }
            }
        }

        async fn find_payment_intent_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await;
                    PaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", merchant_id, payment_id);
                    self.redis_conn
                        .get_hash_field_and_deserialize::<PaymentIntent>(
                            &key,
                            "pi",
                            "PaymentIntent",
                        )
                        .await
                        .map_err(|error| match error.current_context() {
                            errors::RedisError::NotFound => errors::StorageError::ValueNotFound(
                                format!("Payment Intent does not exist for {}", key),
                            )
                            .into(),
                            _ => error.change_context(errors::StorageError::KVError),
                        })
                    // Check for database presence as well Maybe use a read replica here ?
                }
            }
        }

        async fn filter_payment_intent_by_constraints(
            &self,
            merchant_id: &str,
            pc: &api::PaymentListConstraints,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await;
                    PaymentIntent::filter_by_constraints(&conn, merchant_id, pc)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
            }
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::IntoReport;

    use super::PaymentIntentInterface;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::{
            api,
            storage::{enums, payment_intent::*},
        },
    };

    #[async_trait::async_trait]
    impl PaymentIntentInterface for Store {
        async fn insert_payment_intent(
            &self,
            new: PaymentIntentNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await;
            new.insert(&conn).await.map_err(Into::into).into_report()
        }

        async fn update_payment_intent(
            &self,
            this: PaymentIntent,
            payment_intent: PaymentIntentUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await;
            this.update(&conn, payment_intent)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn find_payment_intent_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await;
            PaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn filter_payment_intent_by_constraints(
            &self,
            merchant_id: &str,
            pc: &api::PaymentListConstraints,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await;
            PaymentIntent::filter_by_constraints(&conn, merchant_id, pc)
                .await
                .map_err(Into::into)
                .into_report()
        }
    }
}

#[async_trait::async_trait]
impl PaymentIntentInterface for MockDb {
    async fn filter_payment_intent_by_constraints(
        &self,
        _merchant_id: &str,
        _pc: &api::PaymentListConstraints,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::PaymentIntent>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_payment_intent(
        &self,
        new: types::PaymentIntentNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentIntent, errors::StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        let time = common_utils::date_time::now();
        let payment_intent = types::PaymentIntent {
            #[allow(clippy::as_conversions)]
            id: payment_intents.len() as i32,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            status: new.status,
            amount: new.amount,
            currency: new.currency,
            amount_captured: new.amount_captured,
            customer_id: new.customer_id,
            description: new.description,
            return_url: new.return_url,
            metadata: new.metadata,
            connector_id: new.connector_id,
            shipping_address_id: new.shipping_address_id,
            billing_address_id: new.billing_address_id,
            statement_descriptor_name: new.statement_descriptor_name,
            statement_descriptor_suffix: new.statement_descriptor_suffix,
            created_at: new.created_at.unwrap_or(time),
            modified_at: new.modified_at.unwrap_or(time),
            last_synced: new.last_synced,
            setup_future_usage: new.setup_future_usage,
            off_session: new.off_session,
            client_secret: new.client_secret,
        };
        payment_intents.push(payment_intent.clone());
        Ok(payment_intent)
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_payment_intent(
        &self,
        this: types::PaymentIntent,
        update: types::PaymentIntentUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentIntent, errors::StorageError> {
        let mut payment_intents = self.payment_intents.lock().await;
        let payment_intent = payment_intents
            .iter_mut()
            .find(|item| item.id == this.id)
            .unwrap();
        *payment_intent = update.apply_changeset(this);
        Ok(payment_intent.clone())
    }

    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<types::PaymentIntent, errors::StorageError> {
        let payment_intents = self.payment_intents.lock().await;

        Ok(payment_intents
            .iter()
            .find(|payment_intent| {
                payment_intent.payment_id == payment_id && payment_intent.merchant_id == merchant_id
            })
            .cloned()
            .unwrap())
    }
}
