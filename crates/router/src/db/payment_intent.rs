use super::MockDb;
#[cfg(feature = "olap")]
use crate::types::api;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as types, enums},
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

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        pc: &api::PaymentListConstraints,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api::TimeRange,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::PaymentIntent>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn apply_filters_on_payments_list(
        &self,
        merchant_id: &str,
        constraints: &api::PaymentListFilterConstraints,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<(types::PaymentIntent, types::PaymentAttempt)>, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::date_time;
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;

    use super::PaymentIntentInterface;
    #[cfg(feature = "olap")]
    use crate::types::api;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{enums, kv, payment_intent::*},
        utils::{self, db_utils, storage_partitioning},
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
                    let conn = connection::pg_connection_write(self).await?;
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
                        business_country: new.business_country,
                        business_label: new.business_label.clone(),
                        active_attempt_id: new.active_attempt_id.to_owned(),
                        order_details: new.order_details.clone(),
                        allowed_payment_method_types: new.allowed_payment_method_types.clone(),
                        connector_metadata: new.connector_metadata.clone(),
                        feature_metadata: new.feature_metadata.clone(),
                        attempt_count: new.attempt_count,
                    };

                    match self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .serialize_and_set_hash_field_if_not_exist(&key, "pi", &created_intent)
                        .await
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "payment_intent",
                            key: Some(key),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => {
                            let redis_entry = kv::TypedSql {
                                op: kv::DBOperation::Insert {
                                    insertable: kv::Insertable::PaymentIntent(new),
                                },
                            };
                            self.push_to_drainer_stream::<PaymentIntent>(
                                redis_entry,
                                storage_partitioning::PartitionKey::MerchantIdPaymentId {
                                    merchant_id: &created_intent.merchant_id,
                                    payment_id: &created_intent.payment_id,
                                },
                            )
                            .await?;
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
                    let conn = connection::pg_connection_write(self).await?;
                    this.update(&conn, payment_intent)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", this.merchant_id, this.payment_id);

                    let updated_intent = payment_intent.clone().apply_changeset(this.clone());
                    // Check for database presence as well Maybe use a read replica here ?

                    let redis_value =
                        utils::Encode::<PaymentIntent>::encode_to_string_of_json(&updated_intent)
                            .change_context(errors::StorageError::SerializationFailed)?;

                    let updated_intent = self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
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

                    self.push_to_drainer_stream::<PaymentIntent>(
                        redis_entry,
                        storage_partitioning::PartitionKey::MerchantIdPaymentId {
                            merchant_id: &updated_intent.merchant_id,
                            payment_id: &updated_intent.payment_id,
                        },
                    )
                    .await?;
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
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                PaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,

                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{merchant_id}_{payment_id}");
                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(&key, "pi", "PaymentIntent"),
                        database_call,
                    )
                    .await
                }
            }
        }

        #[cfg(feature = "olap")]
        async fn filter_payment_intent_by_constraints(
            &self,
            merchant_id: &str,
            pc: &api::PaymentListConstraints,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_read(self).await?;
                    PaymentIntent::filter_by_constraints(&conn, merchant_id, pc)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
            }
        }
        #[cfg(feature = "olap")]
        async fn filter_payment_intents_by_time_range_constraints(
            &self,
            merchant_id: &str,
            time_range: &api::TimeRange,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_read(self).await?;
                    PaymentIntent::filter_by_time_constraints(&conn, merchant_id, time_range)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
            }
        }

        #[cfg(feature = "olap")]
        async fn apply_filters_on_payments_list(
            &self,
            merchant_id: &str,
            constraints: &api::PaymentListFilterConstraints,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<(PaymentIntent, PaymentAttempt)>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_read(self).await?;
                    PaymentIntent::apply_filters_on_payments(&conn, merchant_id, constraints)
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
    use crate::types::storage::PaymentAttempt;
    use super::PaymentIntentInterface;
    #[cfg(feature = "olap")]
    use crate::types::api;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{enums, payment_intent::*},
    };

    #[async_trait::async_trait]
    impl PaymentIntentInterface for Store {
        async fn insert_payment_intent(
            &self,
            new: PaymentIntentNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            new.insert(&conn).await.map_err(Into::into).into_report()
        }

        async fn update_payment_intent(
            &self,
            this: PaymentIntent,
            payment_intent: PaymentIntentUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<PaymentIntent, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
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
            let conn = connection::pg_connection_read(self).await?;
            PaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        }

        #[cfg(feature = "olap")]
        async fn filter_payment_intent_by_constraints(
            &self,
            merchant_id: &str,
            pc: &api::PaymentListConstraints,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            PaymentIntent::filter_by_constraints(&conn, merchant_id, pc)
                .await
                .map_err(Into::into)
                .into_report()
        }
        #[cfg(feature = "olap")]
        async fn filter_payment_intents_by_time_range_constraints(
            &self,
            merchant_id: &str,
            time_range: &api::TimeRange,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<PaymentIntent>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            PaymentIntent::filter_by_time_constraints(&conn, merchant_id, time_range)
                .await
                .map_err(Into::into)
                .into_report()
        }

        #[cfg(feature = "olap")]
        async fn apply_filters_on_payments_list(
            &self,
            merchant_id: &str,
            constraints: &api::PaymentListFilterConstraints,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<(PaymentIntent, PaymentAttempt)>, errors::StorageError>
        {
            let conn = connection::pg_connection_read(self).await?;
            PaymentIntent::apply_filters_on_payments(&conn, merchant_id, constraints)
                .await
                .map_err(Into::into)
                .into_report()
        }
    }
}

#[async_trait::async_trait]
impl PaymentIntentInterface for MockDb {
    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        _merchant_id: &str,
        _pc: &api::PaymentListConstraints,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::PaymentIntent>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        _merchant_id: &str,
        _time_range: &api::TimeRange,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<types::PaymentIntent>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
    #[cfg(feature = "olap")]
    async fn apply_filters_on_payments_list(
        &self,
        _merchant_id: &str,
        _constraints: &api::PaymentListFilterConstraints,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<(types::PaymentIntent, types::PaymentAttempt)>, errors::StorageError>
    {
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
            business_country: new.business_country,
            business_label: new.business_label,
            active_attempt_id: new.active_attempt_id.to_owned(),
            order_details: new.order_details,
            allowed_payment_method_types: new.allowed_payment_method_types,
            connector_metadata: new.connector_metadata,
            feature_metadata: new.feature_metadata,
            attempt_count: new.attempt_count,
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
