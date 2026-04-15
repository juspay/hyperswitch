#[cfg(feature = "olap")]
use api_models::payments::{AmountFilter, Order, SortBy, SortOn};
#[cfg(feature = "olap")]
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
#[cfg(feature = "v2")]
use common_utils::fallback_reverse_lookup_not_found;
use common_utils::{
    ext_traits::{AsyncExt, Encode},
    types::keymanager::ToEncryptable,
};
#[cfg(feature = "olap")]
use diesel::{associations::HasTable, ExpressionMethods, JoinOnDsl, QueryDsl};
#[cfg(feature = "v1")]
use diesel_models::payment_intent::PaymentIntentUpdate as DieselPaymentIntentUpdate;
#[cfg(feature = "v2")]
use diesel_models::payment_intent::PaymentIntentUpdateInternal;
#[cfg(feature = "olap")]
use diesel_models::query::generics::db_metrics;
#[cfg(feature = "v2")]
use diesel_models::reverse_lookup::ReverseLookupNew;
#[cfg(all(feature = "v1", feature = "olap"))]
use diesel_models::schema::{
    payment_attempt::{self as payment_attempt_schema, dsl as pa_dsl},
    payment_intent::dsl as pi_dsl,
};
#[cfg(all(feature = "v2", feature = "olap"))]
use diesel_models::schema_v2::{
    payment_attempt::{self as payment_attempt_schema, dsl as pa_dsl},
    payment_intent::dsl as pi_dsl,
};
use diesel_models::{
    enums::MerchantStorageScheme,
    kv,
    payment_intent::{
        PaymentIntent as DieselPaymentIntent, PaymentIntentNew as DieselPaymentIntentNew,
    },
};
use error_stack::ResultExt;
#[cfg(all(feature = "v1", feature = "olap"))]
use futures::future::try_join_all;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdateInternal;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::payments::{
    payment_attempt::PaymentAttempt, payment_intent::PaymentIntentFetchConstraints,
};
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    payments::{
        payment_intent::{PaymentIntentInterface, PaymentIntentUpdate},
        EncryptedPaymentIntent, PaymentIntent,
    },
    RemoteStorageObject,
};
use redis_interface::HsetnxReply;
#[cfg(feature = "olap")]
use router_env::logger;
use router_env::{instrument, tracing};

#[cfg(feature = "olap")]
use crate::connection;
use crate::{
    behaviour::Conversion,
    diesel_error_to_data_error,
    errors::{RedisErrorExt, StorageError},
    kv_router_store::KVRouterStore,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_write},
    DatabaseStore,
};
#[cfg(feature = "v2")]
use crate::{errors, lookup::ReverseLookupInterface};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[cfg(feature = "v1")]
    async fn insert_payment_intent(
        &self,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let processor_merchant_id = payment_intent.processor_merchant_id.clone();
        let payment_id = payment_intent.get_id().to_owned();
        let field = payment_intent.get_id().get_hash_key_for_kv_store();
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &processor_merchant_id,
            payment_id: &payment_id,
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentIntent>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payment_intent(payment_intent, merchant_key_store, storage_scheme)
                    .await
            }

            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();
                let new_payment_intent = payment_intent
                    .clone()
                    .construct_new()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: Box::new(kv::Insertable::PaymentIntent(Box::new(
                            new_payment_intent,
                        ))),
                    },
                };

                let diesel_payment_intent = payment_intent
                    .clone()
                    .convert()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                match Box::pin(kv_wrapper::<DieselPaymentIntent, _, _>(
                    self,
                    KvOperation::<DieselPaymentIntent>::HSetNx(
                        &field,
                        &diesel_payment_intent,
                        redis_entry,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(StorageError::DuplicateValue {
                        entity: "payment_intent",
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(payment_intent),
                    Err(error) => Err(error.change_context(StorageError::KVError)),
                }
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn insert_payment_intent(
        &self,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payment_intent(payment_intent, merchant_key_store, storage_scheme)
                    .await
            }

            MerchantStorageScheme::RedisKv => {
                let id = payment_intent.id.clone();
                let key = PartitionKey::GlobalPaymentId { id: &id };
                let field = format!("pi_{}", id.get_string_repr());
                let key_str = key.to_string();

                let new_payment_intent = payment_intent
                    .clone()
                    .construct_new()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: Box::new(kv::Insertable::PaymentIntent(Box::new(
                            new_payment_intent,
                        ))),
                    },
                };

                let diesel_payment_intent = payment_intent
                    .clone()
                    .convert()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                if let Some(merchant_reference_id) = &payment_intent.merchant_reference_id {
                    let reverse_lookup = ReverseLookupNew {
                        lookup_id: format!(
                            "pi_merchant_reference_{}_{}",
                            payment_intent.profile_id.get_string_repr(),
                            merchant_reference_id.get_string_repr()
                        ),
                        pk_id: key_str.clone(),
                        sk_id: field.clone(),
                        source: "payment_intent".to_string(),
                        updated_by: storage_scheme.to_string(),
                    };
                    self.insert_reverse_lookup(reverse_lookup, storage_scheme)
                        .await?;
                }

                match Box::pin(kv_wrapper::<DieselPaymentIntent, _, _>(
                    self,
                    KvOperation::<DieselPaymentIntent>::HSetNx(
                        &field,
                        &diesel_payment_intent,
                        redis_entry,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(StorageError::DuplicateValue {
                        entity: "payment_intent",
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(payment_intent),
                    Err(error) => Err(error.change_context(StorageError::KVError)),
                }
            }
        }
    }

    #[cfg(all(feature = "v2", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, Option<PaymentAttempt>)>, StorageError> {
        self.router_store
            .get_filtered_payment_intents_attempt(
                merchant_id,
                constraints,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent_update: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let processor_merchant_id = this.processor_merchant_id.clone();
        let payment_id = this.get_id().to_owned();
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &processor_merchant_id,
            payment_id: &payment_id,
        };
        let field = format!("pi_{}", this.get_id().get_string_repr());
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentIntent>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, Some(&this.updated_by)),
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payment_intent(
                        this,
                        payment_intent_update,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();

                let diesel_intent_update =
                    DieselPaymentIntentUpdate::foreign_from(payment_intent_update);
                let origin_diesel_intent = this
                    .convert()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                let diesel_intent = diesel_intent_update
                    .clone()
                    .apply_changeset(origin_diesel_intent.clone());
                // Check for database presence as well Maybe use a read replica here ?

                let redis_value = diesel_intent
                    .encode_to_string_of_json()
                    .change_context(StorageError::SerializationFailed)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: Box::new(kv::Updateable::PaymentIntentUpdate(Box::new(
                            kv::PaymentIntentUpdateMems {
                                orig: origin_diesel_intent,
                                update_data: diesel_intent_update,
                            },
                        ))),
                    },
                };

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<DieselPaymentIntent>::Hset((&field, redis_value), redis_entry),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(StorageError::KVError)?;

                let payment_intent = PaymentIntent::convert_back(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    diesel_intent,
                    merchant_key_store.key.get_inner(),
                    processor_merchant_id.into(),
                )
                .await
                .change_context(StorageError::DecryptionError)?;

                Ok(payment_intent)
            }
        }
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent_update: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payment_intent(
                        this,
                        payment_intent_update,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let id = this.id.clone();
                let merchant_id = this.merchant_id.clone();
                let key = PartitionKey::GlobalPaymentId { id: &id };
                let field = format!("pi_{}", id.get_string_repr());
                let key_str = key.to_string();

                let diesel_intent_update =
                    PaymentIntentUpdateInternal::try_from(payment_intent_update)
                        .change_context(StorageError::DeserializationFailed)?;
                let origin_diesel_intent = this
                    .convert()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                let diesel_intent = diesel_intent_update
                    .clone()
                    .apply_changeset(origin_diesel_intent.clone());

                let redis_value = diesel_intent
                    .encode_to_string_of_json()
                    .change_context(StorageError::SerializationFailed)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: Box::new(kv::Updateable::PaymentIntentUpdate(Box::new(
                            kv::PaymentIntentUpdateMems {
                                orig: origin_diesel_intent,
                                update_data: diesel_intent_update,
                            },
                        ))),
                    },
                };

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<DieselPaymentIntent>::Hset((&field, redis_value), redis_entry),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(StorageError::KVError)?;

                let payment_intent = PaymentIntent::convert_back(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    diesel_intent,
                    merchant_key_store.key.get_inner(),
                    merchant_id.into(),
                )
                .await
                .change_context(StorageError::DecryptionError)?;

                Ok(payment_intent)
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_processor_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselPaymentIntent::find_by_payment_id_processor_merchant_id(
                &conn,
                payment_id,
                processor_merchant_id,
            )
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentIntent>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        let diesel_payment_intent = match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,

            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id: processor_merchant_id,
                    payment_id,
                };
                let field = payment_id.get_hash_key_for_kv_store();
                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<DieselPaymentIntent, _, _>(
                            self,
                            KvOperation::<DieselPaymentIntent>::HGet(&field),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await
            }
        }?;

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            processor_merchant_id.to_owned().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_id(
        &self,
        id: &common_utils::id_type::GlobalPaymentId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentIntent>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;

        let database_call = || async {
            let conn: bb8::PooledConnection<
                '_,
                async_bb8_diesel::ConnectionManager<diesel::PgConnection>,
            > = pg_connection_read(self).await?;

            DieselPaymentIntent::find_by_global_id(&conn, id)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(*er.current_context());
                    er.change_context(new_err)
                })
        };

        let diesel_payment_intent = match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::GlobalPaymentId { id };
                let field = format!("pi_{}", id.get_string_repr());

                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<DieselPaymentIntent, _, _>(
                            self,
                            KvOperation::<DieselPaymentIntent>::HGet(&field),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await
            }
        }?;

        let merchant_id = diesel_payment_intent.merchant_id.clone();

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_id.into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intent_by_constraints(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        self.router_store
            .filter_payment_intent_by_constraints(
                processor_merchant_id,
                filters,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        self.router_store
            .filter_payment_intents_by_time_range_constraints(
                processor_merchant_id,
                time_range,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_intent_status_with_count(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::IntentStatus, i64)>, StorageError> {
        self.router_store
            .get_intent_status_with_count(processor_merchant_id, profile_id_list, time_range)
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_payment_intents_attempt(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        self.router_store
            .get_filtered_payment_intents_attempt(
                processor_merchant_id,
                filters,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        self.router_store
            .get_filtered_active_attempt_ids_for_total_count(
                processor_merchant_id,
                constraints,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v2", feature = "olap"))]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Option<String>>, StorageError> {
        self.router_store
            .get_filtered_active_attempt_ids_for_total_count(
                merchant_id,
                constraints,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_intent_by_merchant_reference_id_profile_id(
        &self,
        merchant_reference_id: &common_utils::id_type::PaymentReferenceId,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: &MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_intent_by_merchant_reference_id_profile_id(
                        merchant_reference_id,
                        profile_id,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let lookup_id = format!(
                    "pi_merchant_reference_{}_{}",
                    profile_id.get_string_repr(),
                    merchant_reference_id.get_string_repr()
                );

                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, *storage_scheme)
                        .await,
                    self.router_store
                        .find_payment_intent_by_merchant_reference_id_profile_id(
                            merchant_reference_id,
                            profile_id,
                            merchant_key_store,
                            storage_scheme,
                        )
                        .await
                );

                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };

                let database_call = || async {
                    let conn = pg_connection_read(self).await?;
                    DieselPaymentIntent::find_by_merchant_reference_id_profile_id(
                        &conn,
                        merchant_reference_id,
                        profile_id,
                    )
                    .await
                    .map_err(|er| {
                        let new_err = diesel_error_to_data_error(*er.current_context());
                        er.change_context(new_err)
                    })
                };

                let diesel_payment_intent = Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<DieselPaymentIntent, _, _>(
                            self,
                            KvOperation::<DieselPaymentIntent>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await?;

                let merchant_id = diesel_payment_intent.merchant_id.clone();

                PaymentIntent::convert_back(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    diesel_payment_intent,
                    merchant_key_store.key.get_inner(),
                    merchant_id.into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for crate::RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_payment_intent(
        &self,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_payment_intent = payment_intent
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })?;

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_payment_intent_update = DieselPaymentIntentUpdate::foreign_from(payment_intent);

        let diesel_payment_intent = this
            .convert()
            .await
            .change_context(StorageError::EncryptionError)?
            .update(&conn, diesel_payment_intent_update)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })?;

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_payment_intent_update = PaymentIntentUpdateInternal::try_from(payment_intent)
            .change_context(StorageError::DeserializationFailed)?;
        let diesel_payment_intent = this
            .convert()
            .await
            .change_context(StorageError::EncryptionError)?
            .update(&conn, diesel_payment_intent_update)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })?;

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_processor_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentIntent::find_by_payment_id_processor_merchant_id(
            &conn,
            payment_id,
            processor_merchant_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .async_and_then(|diesel_payment_intent| async {
            PaymentIntent::convert_back(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                diesel_payment_intent,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
        })
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_id(
        &self,
        id: &common_utils::id_type::GlobalPaymentId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_read(self).await?;
        let diesel_payment_intent = DieselPaymentIntent::find_by_global_id(&conn, id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })?;

        let merchant_id = diesel_payment_intent.merchant_id.clone();

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_id.to_owned().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_merchant_reference_id_profile_id(
        &self,
        merchant_reference_id: &common_utils::id_type::PaymentReferenceId,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: &MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_read(self).await?;
        let diesel_payment_intent = DieselPaymentIntent::find_by_merchant_reference_id_profile_id(
            &conn,
            merchant_reference_id,
            profile_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })?;
        let merchant_id = diesel_payment_intent.merchant_id.clone();

        PaymentIntent::convert_back(
            self.get_keymanager_state()
                .attach_printable("Missing KeyManagerState")?,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_id.to_owned().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn filter_payment_intent_by_constraints(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        use futures::{future::try_join_all, FutureExt};

        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);

        //[#350]: Replace this with Boxable Expression and pass it into generic filter
        // when https://github.com/rust-lang/rust/issues/52662 becomes stable
        let mut query = <DieselPaymentIntent as HasTable>::table()
            .filter(pi_dsl::processor_merchant_id.eq(processor_merchant_id.to_owned()))
            .order(pi_dsl::created_at.desc())
            .into_boxed();

        match filters {
            PaymentIntentFetchConstraints::Single { payment_intent_id } => {
                query = query.filter(pi_dsl::payment_id.eq(payment_intent_id.to_owned()));
            }
            PaymentIntentFetchConstraints::List(params) => {
                if let Some(limit) = params.limit {
                    query = query.limit(limit.into());
                }

                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(pi_dsl::customer_id.eq(customer_id.clone()));
                }
                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(pi_dsl::profile_id.eq_any(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payment_intent_by_payment_id_processor_merchant_id(
                                starting_after_id,
                                processor_merchant_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, &params.ending_before_id) {
                    (Some(ending_at), _) => query.filter(pi_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let ending_at = self
                            .find_payment_intent_by_payment_id_processor_merchant_id(
                                ending_before_id,
                                processor_merchant_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                query = query.offset(params.offset.into());

                query = match &params.currency {
                    Some(currency) => query.filter(pi_dsl::currency.eq_any(currency.clone())),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                if let Some(currency) = &params.currency {
                    query = query.filter(pi_dsl::currency.eq_any(currency.clone()));
                }

                if let Some(status) = &params.status {
                    query = query.filter(pi_dsl::status.eq_any(status.clone()));
                }
            }
        }
        let keymanager_state = self
            .get_keymanager_state()
            .attach_printable("Missing KeyManagerState")?;
        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());
        db_metrics::track_database_call::<<DieselPaymentIntent as HasTable>::Table, _, _>(
            query.get_results_async::<DieselPaymentIntent>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map(|payment_intents| {
            try_join_all(payment_intents.into_iter().map(|diesel_payment_intent| {
                PaymentIntent::convert_back(
                    keymanager_state,
                    diesel_payment_intent,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
            }))
            .map(|join_result| join_result.change_context(StorageError::DecryptionError))
        })
        .map_err(|er| {
            StorageError::DatabaseError(
                error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                    .attach_printable("Error filtering payment records"),
            )
        })?
        .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        // TODO: Remove this redundant function
        let payment_filters = (*time_range).into();
        self.filter_payment_intent_by_constraints(
            processor_merchant_id,
            &payment_filters,
            merchant_key_store,
            storage_scheme,
        )
        .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn get_intent_status_with_count(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::IntentStatus, i64)>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);

        let mut query = <DieselPaymentIntent as HasTable>::table()
            .group_by(pi_dsl::status)
            .select((pi_dsl::status, diesel::dsl::count_star()))
            .filter(pi_dsl::processor_merchant_id.eq(processor_merchant_id.to_owned()))
            .into_boxed();

        if let Some(profile_id) = profile_id_list {
            query = query.filter(pi_dsl::profile_id.eq_any(profile_id));
        }

        query = query.filter(pi_dsl::created_at.ge(time_range.start_time));

        query = match time_range.end_time {
            Some(ending_at) => query.filter(pi_dsl::created_at.le(ending_at)),
            None => query,
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        db_metrics::track_database_call::<<DieselPaymentIntent as HasTable>::Table, _, _>(
            query.get_results_async::<(common_enums::IntentStatus, i64)>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|er| {
            StorageError::DatabaseError(
                error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                    .attach_printable("Error filtering payment records"),
            )
            .into()
        })
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_payment_intents_attempt(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .filter(pi_dsl::processor_merchant_id.eq(processor_merchant_id.to_owned()))
            .inner_join(
                payment_attempt_schema::table.on(pa_dsl::attempt_id.eq(pi_dsl::active_attempt_id)),
            )
            .filter(pa_dsl::processor_merchant_id.eq(processor_merchant_id.to_owned())) // Ensure merchant_ids match, as different merchants can share payment/attempt IDs.
            .into_boxed();

        query = match constraints {
            PaymentIntentFetchConstraints::Single { payment_intent_id } => {
                query.filter(pi_dsl::payment_id.eq(payment_intent_id.to_owned()))
            }
            PaymentIntentFetchConstraints::List(params) => {
                query = match params.order {
                    Order {
                        on: SortOn::Amount,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::amount.asc()),
                    Order {
                        on: SortOn::Amount,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::amount.desc()),
                    Order {
                        on: SortOn::Created,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::created_at.asc()),
                    Order {
                        on: SortOn::Created,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::created_at.desc()),
                    Order {
                        on: SortOn::Modified,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::modified_at.asc()),
                    Order {
                        on: SortOn::Modified,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::modified_at.desc()),
                    Order {
                        on: SortOn::AttemptCount,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::attempt_count.asc()),
                    Order {
                        on: SortOn::AttemptCount,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::attempt_count.desc()),
                };

                if let Some(limit) = params.limit {
                    query = query.limit(limit.into());
                }

                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(pi_dsl::customer_id.eq(customer_id.clone()));
                }

                if let Some(merchant_order_reference_id) = &params.merchant_order_reference_id {
                    query = query.filter(
                        pi_dsl::merchant_order_reference_id.eq(merchant_order_reference_id.clone()),
                    )
                }

                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(pi_dsl::profile_id.eq_any(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payment_intent_by_payment_id_processor_merchant_id(
                                starting_after_id,
                                processor_merchant_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, &params.ending_before_id) {
                    (Some(ending_at), _) => query.filter(pi_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let ending_at = self
                            .find_payment_intent_by_payment_id_processor_merchant_id(
                                ending_before_id,
                                processor_merchant_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                query = query.offset(params.offset.into());

                query = match params.amount_filter {
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.between(start, end)),
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: None,
                    }) => query.filter(pi_dsl::amount.ge(start)),
                    Some(AmountFilter {
                        start_amount: None,
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.le(end)),
                    _ => query,
                };

                query = match &params.currency {
                    Some(currency) => query.filter(pi_dsl::currency.eq_any(currency.clone())),
                    None => query,
                };

                let connectors = params
                    .connector
                    .as_ref()
                    .map(|c| c.iter().map(|c| c.to_string()).collect::<Vec<String>>());

                query = match connectors {
                    Some(connectors) => query.filter(pa_dsl::connector.eq_any(connectors)),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                query = match &params.payment_method {
                    Some(payment_method) => {
                        query.filter(pa_dsl::payment_method.eq_any(payment_method.clone()))
                    }
                    None => query,
                };

                query = match &params.payment_method_type {
                    Some(payment_method_type) => query
                        .filter(pa_dsl::payment_method_type.eq_any(payment_method_type.clone())),
                    None => query,
                };

                query = match &params.authentication_type {
                    Some(authentication_type) => query
                        .filter(pa_dsl::authentication_type.eq_any(authentication_type.clone())),
                    None => query,
                };

                query = match &params.merchant_connector_id {
                    Some(merchant_connector_id) => query.filter(
                        pa_dsl::merchant_connector_id.eq_any(merchant_connector_id.clone()),
                    ),
                    None => query,
                };

                if let Some(card_network) = &params.card_network {
                    query = query.filter(pa_dsl::card_network.eq_any(card_network.clone()));
                }

                if let Some(card_discovery) = &params.card_discovery {
                    query = query.filter(pa_dsl::card_discovery.eq_any(card_discovery.clone()));
                }

                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());
        let keymanager_state = self
            .get_keymanager_state()
            .attach_printable("Missing KeyManagerState")?;

        query
            .get_results_async::<(
                DieselPaymentIntent,
                diesel_models::payment_attempt::PaymentAttempt,
            )>(conn)
            .await
            .async_map(|results| {
                try_join_all(results.into_iter().map(|(pi, pa)| async {
                    let payment_intent = PaymentIntent::convert_back(
                        keymanager_state,
                        pi,
                        merchant_key_store.key.get_inner(),
                        processor_merchant_id.to_owned().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?;

                    let payment_attempt = PaymentAttempt::convert_back(
                        keymanager_state,
                        pa,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?;

                    Ok((payment_intent, payment_attempt))
                }))
            })
            .await
            .map_err(|er| {
                StorageError::DatabaseError(
                    error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                        .attach_printable("Error filtering payment records"),
                )
            })?
    }

    #[cfg(all(feature = "v2", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, Option<PaymentAttempt>)>, StorageError> {
        use diesel::NullableExpressionMethods as _;
        use futures::{future::try_join_all, FutureExt};

        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
            .left_join(
                payment_attempt_schema::table
                    .on(pi_dsl::active_attempt_id.eq(pa_dsl::id.nullable())),
            )
            // Filtering on merchant_id for payment_attempt is not required for v2 as payment_attempt_ids are globally unique
            .into_boxed();

        query = match constraints {
            PaymentIntentFetchConstraints::List(params) => {
                query = match params.order {
                    Order {
                        on: SortOn::Amount,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::amount.asc()),
                    Order {
                        on: SortOn::Amount,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::amount.desc()),
                    Order {
                        on: SortOn::Created,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::created_at.asc()),
                    Order {
                        on: SortOn::Created,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::created_at.desc()),
                    Order {
                        on: SortOn::Modified,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::modified_at.asc()),
                    Order {
                        on: SortOn::Modified,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::modified_at.desc()),
                    Order {
                        on: SortOn::AttemptCount,
                        by: SortBy::Asc,
                    } => query.order(pi_dsl::attempt_count.asc()),
                    Order {
                        on: SortOn::AttemptCount,
                        by: SortBy::Desc,
                    } => query.order(pi_dsl::attempt_count.desc()),
                };

                if let Some(limit) = params.limit {
                    query = query.limit(limit.into());
                }

                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(pi_dsl::customer_id.eq(customer_id.clone()));
                }

                if let Some(merchant_order_reference_id) = &params.merchant_order_reference_id {
                    query = query.filter(
                        pi_dsl::merchant_reference_id.eq(merchant_order_reference_id.clone()),
                    )
                }

                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(pi_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payment_intent_by_id(
                                starting_after_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, &params.ending_before_id) {
                    (Some(ending_at), _) => query.filter(pi_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let ending_at = self
                            .find_payment_intent_by_id(
                                ending_before_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                query = query.offset(params.offset.into());

                query = match params.amount_filter {
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.between(start, end)),
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: None,
                    }) => query.filter(pi_dsl::amount.ge(start)),
                    Some(AmountFilter {
                        start_amount: None,
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.le(end)),
                    _ => query,
                };

                query = match &params.currency {
                    Some(currency) => query.filter(pi_dsl::currency.eq_any(currency.clone())),
                    None => query,
                };

                query = match &params.connector {
                    Some(connector) => query.filter(pa_dsl::connector.eq_any(connector.clone())),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                query = match &params.payment_method_type {
                    Some(payment_method_type) => query
                        .filter(pa_dsl::payment_method_type_v2.eq_any(payment_method_type.clone())),
                    None => query,
                };

                query = match &params.payment_method_subtype {
                    Some(payment_method_subtype) => query.filter(
                        pa_dsl::payment_method_subtype.eq_any(payment_method_subtype.clone()),
                    ),
                    None => query,
                };

                query = match &params.authentication_type {
                    Some(authentication_type) => query
                        .filter(pa_dsl::authentication_type.eq_any(authentication_type.clone())),
                    None => query,
                };

                query = match &params.merchant_connector_id {
                    Some(merchant_connector_id) => query.filter(
                        pa_dsl::merchant_connector_id.eq_any(merchant_connector_id.clone()),
                    ),
                    None => query,
                };

                if let Some(card_network) = &params.card_network {
                    query = query.filter(pa_dsl::card_network.eq_any(card_network.clone()));
                }

                if let Some(payment_id) = &params.payment_id {
                    query = query.filter(pi_dsl::id.eq(payment_id.clone()));
                }

                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());
        let keymanager_state = self
            .get_keymanager_state()
            .attach_printable("Missing KeyManagerState")?;

        query
            .get_results_async::<(
                DieselPaymentIntent,
                Option<diesel_models::payment_attempt::PaymentAttempt>,
            )>(conn)
            .await
            .change_context(StorageError::DecryptionError)
            .async_and_then(|output| async {
                try_join_all(output.into_iter().map(
                    |(pi, pa): (_, Option<diesel_models::payment_attempt::PaymentAttempt>)| async {
                        let payment_intent = PaymentIntent::convert_back(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
                            pi,
                            merchant_key_store.key.get_inner(),
                            merchant_id.to_owned().into(),
                        );
                        let payment_attempt = pa
                            .async_map(|val| {
                                PaymentAttempt::convert_back(
                                    keymanager_state,
                                    val,
                                    merchant_key_store.key.get_inner(),
                                    merchant_id.to_owned().into(),
                                )
                            })
                            .map(|val| val.transpose());

                        let output = futures::try_join!(payment_intent, payment_attempt);
                        output.change_context(StorageError::DecryptionError)
                    },
                ))
                .await
            })
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[cfg(all(feature = "v2", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Option<String>>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .select(pi_dsl::active_attempt_id)
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(pi_dsl::created_at.desc())
            .into_boxed();

        query = match constraints {
            PaymentIntentFetchConstraints::List(params) => {
                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(pi_dsl::customer_id.eq(customer_id.clone()));
                }
                if let Some(merchant_order_reference_id) = &params.merchant_order_reference_id {
                    query = query.filter(
                        pi_dsl::merchant_reference_id.eq(merchant_order_reference_id.clone()),
                    )
                }
                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(pi_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match params.starting_at {
                    Some(starting_at) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    None => query,
                };

                query = match params.ending_at {
                    Some(ending_at) => query.filter(pi_dsl::created_at.le(ending_at)),
                    None => query,
                };

                query = match params.amount_filter {
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.between(start, end)),
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: None,
                    }) => query.filter(pi_dsl::amount.ge(start)),
                    Some(AmountFilter {
                        start_amount: None,
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.le(end)),
                    _ => query,
                };

                query = match &params.currency {
                    Some(currency) => query.filter(pi_dsl::currency.eq_any(currency.clone())),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                if let Some(payment_id) = &params.payment_id {
                    query = query.filter(pi_dsl::id.eq(payment_id.clone()));
                }

                query
            }
        };

        db_metrics::track_database_call::<<DieselPaymentIntent as HasTable>::Table, _, _>(
            query.get_results_async::<Option<String>>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|er| {
            StorageError::DatabaseError(
                error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                    .attach_printable("Error filtering payment records"),
            )
            .into()
        })
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .select(pi_dsl::active_attempt_id)
            .filter(pi_dsl::processor_merchant_id.eq(processor_merchant_id.to_owned()))
            .order(pi_dsl::created_at.desc())
            .into_boxed();

        query = match constraints {
            PaymentIntentFetchConstraints::Single { payment_intent_id } => {
                query.filter(pi_dsl::payment_id.eq(payment_intent_id.to_owned()))
            }
            PaymentIntentFetchConstraints::List(params) => {
                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(pi_dsl::customer_id.eq(customer_id.clone()));
                }
                if let Some(merchant_order_reference_id) = &params.merchant_order_reference_id {
                    query = query.filter(
                        pi_dsl::merchant_order_reference_id.eq(merchant_order_reference_id.clone()),
                    )
                }
                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(pi_dsl::profile_id.eq_any(profile_id.clone()));
                }

                query = match params.starting_at {
                    Some(starting_at) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    None => query,
                };

                query = match params.ending_at {
                    Some(ending_at) => query.filter(pi_dsl::created_at.le(ending_at)),
                    None => query,
                };

                query = match params.amount_filter {
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.between(start, end)),
                    Some(AmountFilter {
                        start_amount: Some(start),
                        end_amount: None,
                    }) => query.filter(pi_dsl::amount.ge(start)),
                    Some(AmountFilter {
                        start_amount: None,
                        end_amount: Some(end),
                    }) => query.filter(pi_dsl::amount.le(end)),
                    _ => query,
                };

                query = match &params.currency {
                    Some(currency) => query.filter(pi_dsl::currency.eq_any(currency.clone())),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                query
            }
        };

        db_metrics::track_database_call::<<DieselPaymentIntent as HasTable>::Table, _, _>(
            query.get_results_async::<String>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|er| {
            StorageError::DatabaseError(
                error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                    .attach_printable("Error filtering payment records"),
            )
            .into()
        })
    }
}

#[cfg(feature = "v2")]
use common_utils::errors::ParsingError;
#[cfg(feature = "v2")]
use common_utils::ext_traits::ValueExt;
use common_utils::{
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    type_name,
    types::{
        keymanager::{self, KeyManagerState},
        CreatedBy,
    },
};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
#[cfg(feature = "v2")]
use hyperswitch_masking::ExposeInterface;
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::ForeignFrom;

#[cfg(feature = "v1")]
impl ForeignFrom<PaymentIntentUpdate> for PaymentIntentUpdateInternal {
    fn foreign_from(payment_intent_update: PaymentIntentUpdate) -> Self {
        match payment_intent_update {
            PaymentIntentUpdate::MetadataUpdate {
                metadata,
                updated_by,
                feature_metadata,
            } => Self {
                metadata,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                feature_metadata,
                ..Default::default()
            },
            PaymentIntentUpdate::Update(value) => Self {
                amount: Some(value.amount),
                currency: Some(value.currency),
                setup_future_usage: value.setup_future_usage,
                status: Some(value.status),
                customer_id: value.customer_id,
                shipping_address_id: value.shipping_address_id,
                billing_address_id: value.billing_address_id,
                return_url: value.return_url,
                business_country: value.business_country,
                business_label: value.business_label,
                description: value.description,
                statement_descriptor_name: value.statement_descriptor_name,
                statement_descriptor_suffix: value.statement_descriptor_suffix,
                order_details: value.order_details,
                metadata: value.metadata,
                payment_confirm_source: value.payment_confirm_source,
                updated_by: value.updated_by,
                session_expiry: value.session_expiry,
                fingerprint_id: value.fingerprint_id,
                request_external_three_ds_authentication: value
                    .request_external_three_ds_authentication,
                frm_metadata: value.frm_metadata,
                customer_details: value.customer_details,
                billing_details: value.billing_details,
                merchant_order_reference_id: value.merchant_order_reference_id,
                shipping_details: value.shipping_details,
                is_payment_processor_token_flow: value.is_payment_processor_token_flow,
                tax_details: value.tax_details,
                tax_status: value.tax_status,
                discount_amount: value.discount_amount,
                order_date: value.order_date,
                shipping_amount_tax: value.shipping_amount_tax,
                duty_amount: value.duty_amount,
                installment_options: value.installment_options,
                ..Default::default()
            },
            PaymentIntentUpdate::PaymentCreateUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details,
                updated_by,
            } => Self {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::PGStatusUpdate {
                status,
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
            } => Self {
                status: Some(status),
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
                ..Default::default()
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
                updated_by,
            } => Self {
                status: Some(status),
                shipping_address_id,
                billing_address_id,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::ResponseUpdate {
                // amount,
                // currency,
                status,
                amount_captured,
                fingerprint_id,
                // customer_id,
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
            } => Self {
                // amount,
                // currency: Some(currency),
                status: Some(status),
                amount_captured,
                fingerprint_id,
                // customer_id,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
                ..Default::default()
            },
            PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self {
                active_attempt_id: Some(active_attempt_id),
                attempt_count: Some(attempt_count),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self {
                status: Some(status),
                active_attempt_id: Some(active_attempt_id),
                attempt_count: Some(attempt_count),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::ApproveUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self {
                status: Some(status),
                merchant_decision,
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self {
                status: Some(status),
                merchant_decision,
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::SurchargeApplicableUpdate {
                surcharge_applicable,
                updated_by,
            } => Self {
                surcharge_applicable: Some(surcharge_applicable),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate { amount } => Self {
                amount: Some(amount),
                ..Default::default()
            },
            PaymentIntentUpdate::AuthorizationCountUpdate {
                authorization_count,
            } => Self {
                authorization_count: Some(authorization_count),
                ..Default::default()
            },
            PaymentIntentUpdate::CompleteAuthorizeUpdate {
                shipping_address_id,
            } => Self {
                shipping_address_id,
                ..Default::default()
            },
            PaymentIntentUpdate::ManualUpdate { status, updated_by } => Self {
                status,
                modified_at: Some(common_utils::date_time::now()),
                updated_by,
                ..Default::default()
            },
            PaymentIntentUpdate::SessionResponseUpdate {
                tax_details,
                shipping_address_id,
                updated_by,
                shipping_details,
            } => Self {
                tax_details: Some(tax_details),
                shipping_address_id,
                updated_by,
                shipping_details,
                ..Default::default()
            },
            PaymentIntentUpdate::StateMetadataUpdate {
                state_metadata,
                updated_by,
            } => Self {
                state_metadata: Some(state_metadata),
                updated_by,
                amount: None,
                currency: None,
                status: None,
                amount_captured: None,
                customer_id: None,
                return_url: None,
                setup_future_usage: None,
                off_session: None,
                metadata: None,
                billing_address_id: None,
                shipping_address_id: None,
                modified_at: None,
                active_attempt_id: None,
                business_country: None,
                business_label: None,
                description: None,
                statement_descriptor_name: None,
                statement_descriptor_suffix: None,
                order_details: None,
                attempt_count: None,
                merchant_decision: None,
                payment_confirm_source: None,
                surcharge_applicable: None,
                incremental_authorization_allowed: None,
                authorization_count: None,
                fingerprint_id: None,
                session_expiry: None,
                request_external_three_ds_authentication: None,
                frm_metadata: None,
                customer_details: None,
                billing_details: None,
                merchant_order_reference_id: None,
                shipping_details: None,
                is_payment_processor_token_flow: None,
                tax_details: None,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
                payment_channel: None,
                feature_metadata: None,
                tax_status: None,
                discount_amount: None,
                order_date: None,
                shipping_amount_tax: None,
                duty_amount: None,
                enable_partial_authorization: None,
                enable_overcapture: None,
                shipping_cost: None,
                installment_options: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<PaymentIntentUpdate> for DieselPaymentIntentUpdate {
    fn foreign_from(value: PaymentIntentUpdate) -> Self {
        match value {
            PaymentIntentUpdate::ResponseUpdate {
                status,
                amount_captured,
                fingerprint_id,
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
            } => Self::ResponseUpdate {
                status,
                amount_captured,
                fingerprint_id,
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
            },
            PaymentIntentUpdate::MetadataUpdate {
                metadata,
                updated_by,
                feature_metadata,
            } => Self::MetadataUpdate {
                metadata,
                updated_by,
                feature_metadata,
            },
            PaymentIntentUpdate::StateMetadataUpdate {
                state_metadata,
                updated_by,
            } => Self::StateMetadataUpdate {
                state_metadata,
                updated_by,
            },
            PaymentIntentUpdate::Update(value) => {
                Self::Update(Box::new(diesel_models::PaymentIntentUpdateFields {
                    amount: value.amount,
                    currency: value.currency,
                    setup_future_usage: value.setup_future_usage,
                    status: value.status,
                    customer_id: value.customer_id,
                    shipping_address_id: value.shipping_address_id,
                    billing_address_id: value.billing_address_id,
                    return_url: value.return_url,
                    business_country: value.business_country,
                    business_label: value.business_label,
                    description: value.description,
                    statement_descriptor_name: value.statement_descriptor_name,
                    statement_descriptor_suffix: value.statement_descriptor_suffix,
                    order_details: value.order_details,
                    metadata: value.metadata,
                    payment_confirm_source: value.payment_confirm_source,
                    updated_by: value.updated_by,
                    session_expiry: value.session_expiry,
                    fingerprint_id: value.fingerprint_id,
                    request_external_three_ds_authentication: value
                        .request_external_three_ds_authentication,
                    frm_metadata: value.frm_metadata,
                    customer_details: value.customer_details.map(Encryption::from),
                    billing_details: value.billing_details.map(Encryption::from),
                    merchant_order_reference_id: value.merchant_order_reference_id,
                    shipping_details: value.shipping_details.map(Encryption::from),
                    is_payment_processor_token_flow: value.is_payment_processor_token_flow,
                    tax_details: value.tax_details,
                    force_3ds_challenge: value.force_3ds_challenge,
                    is_iframe_redirection_enabled: value.is_iframe_redirection_enabled,
                    payment_channel: value.payment_channel,
                    feature_metadata: value.feature_metadata,
                    tax_status: value.tax_status,
                    discount_amount: value.discount_amount,
                    order_date: value.order_date,
                    shipping_amount_tax: value.shipping_amount_tax,
                    duty_amount: value.duty_amount,
                    enable_partial_authorization: value.enable_partial_authorization,
                    enable_overcapture: value.enable_overcapture,
                    shipping_cost: value.shipping_cost,
                    installment_options: value
                        .installment_options
                        .map(common_types::payments::InstallmentOptions),
                }))
            }
            PaymentIntentUpdate::PaymentCreateUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details,
                updated_by,
            } => Self::PaymentCreateUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                customer_details: customer_details.map(Encryption::from),
                updated_by,
            },
            PaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
                updated_by,
            } => Self::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
                updated_by,
            },
            PaymentIntentUpdate::PGStatusUpdate {
                status,
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
            } => Self::PGStatusUpdate {
                status,
                updated_by,
                incremental_authorization_allowed,
                feature_metadata,
            },
            PaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
                updated_by,
            },
            PaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
                updated_by,
            } => Self::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
                updated_by,
            },
            PaymentIntentUpdate::ApproveUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self::ApproveUpdate {
                status,
                merchant_decision,
                updated_by,
            },
            PaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
                updated_by,
            } => Self::RejectUpdate {
                status,
                merchant_decision,
                updated_by,
            },
            PaymentIntentUpdate::SurchargeApplicableUpdate {
                surcharge_applicable,
                updated_by,
            } => Self::SurchargeApplicableUpdate {
                surcharge_applicable: Some(surcharge_applicable),
                updated_by,
            },
            PaymentIntentUpdate::IncrementalAuthorizationAmountUpdate { amount } => {
                Self::IncrementalAuthorizationAmountUpdate { amount }
            }
            PaymentIntentUpdate::AuthorizationCountUpdate {
                authorization_count,
            } => Self::AuthorizationCountUpdate {
                authorization_count,
            },
            PaymentIntentUpdate::CompleteAuthorizeUpdate {
                shipping_address_id,
            } => Self::CompleteAuthorizeUpdate {
                shipping_address_id,
            },
            PaymentIntentUpdate::ManualUpdate { status, updated_by } => {
                Self::ManualUpdate { status, updated_by }
            }
            PaymentIntentUpdate::SessionResponseUpdate {
                tax_details,
                shipping_address_id,
                updated_by,
                shipping_details,
            } => Self::SessionResponseUpdate {
                tax_details,
                shipping_address_id,
                updated_by,
                shipping_details: shipping_details.map(Encryption::from),
            },
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<PaymentIntentUpdateInternal> for diesel_models::PaymentIntentUpdateInternal {
    fn foreign_from(value: PaymentIntentUpdateInternal) -> Self {
        let modified_at = common_utils::date_time::now();
        let PaymentIntentUpdateInternal {
            amount,
            currency,
            status,
            amount_captured,
            customer_id,
            return_url,
            setup_future_usage,
            off_session,
            metadata,
            billing_address_id,
            shipping_address_id,
            modified_at: _,
            active_attempt_id,
            business_country,
            business_label,
            description,
            statement_descriptor_name,
            statement_descriptor_suffix,
            order_details,
            attempt_count,
            merchant_decision,
            payment_confirm_source,
            updated_by,
            surcharge_applicable,
            incremental_authorization_allowed,
            authorization_count,
            session_expiry,
            fingerprint_id,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details,
            billing_details,
            merchant_order_reference_id,
            shipping_details,
            is_payment_processor_token_flow,
            tax_details,
            force_3ds_challenge,
            is_iframe_redirection_enabled,
            payment_channel,
            feature_metadata,
            tax_status,
            discount_amount,
            order_date,
            shipping_amount_tax,
            duty_amount,
            enable_partial_authorization,
            enable_overcapture,
            shipping_cost,
            state_metadata,
            installment_options,
        } = value;
        Self {
            amount,
            currency,
            status,
            amount_captured,
            customer_id,
            return_url: None, // deprecated
            setup_future_usage,
            off_session,
            metadata,
            billing_address_id,
            shipping_address_id,
            modified_at,
            active_attempt_id,
            business_country,
            business_label,
            description,
            statement_descriptor_name,
            statement_descriptor_suffix,
            order_details,
            attempt_count,
            merchant_decision,
            payment_confirm_source,
            updated_by,
            surcharge_applicable,
            incremental_authorization_allowed,
            authorization_count,
            session_expiry,
            fingerprint_id,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details: customer_details.map(Encryption::from),
            billing_details: billing_details.map(Encryption::from),
            merchant_order_reference_id,
            shipping_details: shipping_details.map(Encryption::from),
            is_payment_processor_token_flow,
            tax_details,
            force_3ds_challenge,
            is_iframe_redirection_enabled,
            extended_return_url: return_url,
            payment_channel,
            feature_metadata,
            tax_status,
            discount_amount,
            order_date,
            shipping_amount_tax,
            duty_amount,
            enable_partial_authorization,
            enable_overcapture,
            shipping_cost,
            state_metadata,
            installment_options: installment_options
                .map(common_types::payments::InstallmentOptions),
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for PaymentIntent {
    type DstType = DieselPaymentIntent;
    type NewDstType = DieselPaymentIntentNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let Self {
            merchant_id,
            amount_details,
            status,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            statement_descriptor,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage,
            active_attempt_id,
            active_attempt_id_type,
            active_attempts_group_id,
            order_details,
            allowed_payment_method_types,
            connector_metadata,
            feature_metadata,
            attempt_count,
            profile_id,
            payment_link_id,
            frm_merchant_decision,
            updated_by,
            request_incremental_authorization,
            split_txns_enabled,
            authorization_count,
            session_expiry,
            request_external_three_ds_authentication,
            frm_metadata,
            customer_details,
            merchant_reference_id,
            billing_address,
            shipping_address,
            capture_method,
            id,
            authentication_type,
            prerouting_algorithm,
            organization_id,
            enable_payment_link,
            apply_mit_exemption,
            customer_present,
            routing_algorithm_id,
            payment_link_config,
            split_payments,
            force_3ds_challenge,
            force_3ds_challenge_trigger,
            processor_merchant_id,
            created_by,
            is_iframe_redirection_enabled,
            is_payment_id_from_merchant,
            enable_partial_authorization,
        } = self;
        Ok(DieselPaymentIntent {
            skip_external_tax_calculation: Some(amount_details.get_external_tax_action_as_bool()),
            surcharge_applicable: Some(amount_details.get_surcharge_action_as_bool()),
            merchant_id,
            status,
            amount: amount_details.order_amount,
            currency: amount_details.currency,
            amount_captured,
            customer_id,
            description,
            return_url,
            metadata,
            statement_descriptor,
            created_at,
            modified_at,
            last_synced,
            setup_future_usage: Some(setup_future_usage),
            active_attempt_id,
            active_attempt_id_type: Some(active_attempt_id_type),
            active_attempts_group_id,
            order_details: order_details.map(|order_details| {
                order_details
                    .into_iter()
                    .map(|order_detail| Secret::new(order_detail.expose()))
                    .collect::<Vec<_>>()
            }),
            allowed_payment_method_types: allowed_payment_method_types
                .map(|allowed_payment_method_types| {
                    allowed_payment_method_types
                        .encode_to_value()
                        .change_context(ValidationError::InvalidValue {
                            message: "Failed to serialize allowed_payment_method_types".to_string(),
                        })
                })
                .transpose()?
                .map(Secret::new),
            connector_metadata: connector_metadata
                .map(|cm| {
                    cm.encode_to_value()
                        .change_context(ValidationError::InvalidValue {
                            message: "Failed to serialize connector_metadata".to_string(),
                        })
                })
                .transpose()?
                .map(Secret::new),
            feature_metadata,
            attempt_count,
            profile_id,
            frm_merchant_decision,
            payment_link_id,
            updated_by,

            request_incremental_authorization: Some(request_incremental_authorization),
            split_txns_enabled: Some(split_txns_enabled),
            authorization_count,
            session_expiry,
            request_external_three_ds_authentication: Some(
                request_external_three_ds_authentication.as_bool(),
            ),
            frm_metadata,
            customer_details: customer_details.map(Encryption::from),
            billing_address: billing_address.map(Encryption::from),
            shipping_address: shipping_address.map(Encryption::from),
            capture_method: Some(capture_method),
            id,
            authentication_type,
            prerouting_algorithm: prerouting_algorithm
                .map(|prerouting_algorithm| {
                    prerouting_algorithm.encode_to_value().change_context(
                        ValidationError::InvalidValue {
                            message: "Failed to serialize prerouting_algorithm".to_string(),
                        },
                    )
                })
                .transpose()?,
            merchant_reference_id,
            surcharge_amount: amount_details.surcharge_amount,
            tax_on_surcharge: amount_details.tax_on_surcharge,
            organization_id,
            shipping_cost: amount_details.shipping_cost,
            tax_details: amount_details.tax_details,
            enable_payment_link: Some(enable_payment_link.as_bool()),
            apply_mit_exemption: Some(apply_mit_exemption.as_bool()),
            customer_present: Some(customer_present.as_bool()),
            payment_link_config,
            routing_algorithm_id,
            psd2_sca_exemption_type: None,
            request_extended_authorization: None,
            platform_merchant_id: None,
            split_payments,
            force_3ds_challenge,
            force_3ds_challenge_trigger,
            processor_merchant_id: Some(processor_merchant_id),
            created_by: created_by.map(|created_by| created_by.to_string()),
            is_iframe_redirection_enabled,
            is_payment_id_from_merchant,
            payment_channel: None,
            tax_status: None,
            discount_amount: None,
            shipping_amount_tax: None,
            duty_amount: None,
            order_date: None,
            enable_partial_authorization: Some(enable_partial_authorization),
            enable_overcapture: None,
            mit_category: None,
            billing_descriptor: None,
            tokenization: None,
            partner_merchant_identifier_details: None,
            state_metadata: None,
            installment_options: None,
        })
    }
    async fn convert_back(
        state: &KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentIntent::to_encryptable(
                    EncryptedPaymentIntent {
                        billing_address: storage_model.billing_address,
                        shipping_address: storage_model.shipping_address,
                        customer_details: storage_model.customer_details,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentIntent::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let amount_details = hyperswitch_domain_models::payments::AmountDetails {
                order_amount: storage_model.amount,
                currency: storage_model.currency,
                surcharge_amount: storage_model.surcharge_amount,
                tax_on_surcharge: storage_model.tax_on_surcharge,
                shipping_cost: storage_model.shipping_cost,
                tax_details: storage_model.tax_details,
                skip_external_tax_calculation: common_enums::TaxCalculationOverride::from(
                    storage_model.skip_external_tax_calculation,
                ),
                skip_surcharge_calculation: common_enums::SurchargeCalculationOverride::from(
                    storage_model.surcharge_applicable,
                ),
                amount_captured: storage_model.amount_captured,
            };

            let billing_address = data
                .billing_address
                .map(|billing| {
                    billing.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            let shipping_address = data
                .shipping_address
                .map(|shipping| {
                    shipping.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;
            let allowed_payment_method_types = storage_model
                .allowed_payment_method_types
                .map(|allowed_payment_method_types| {
                    allowed_payment_method_types.parse_value("Vec<PaymentMethodType>")
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)?;
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                merchant_id: storage_model.merchant_id.clone(),
                status: storage_model.status,
                amount_details,
                amount_captured: storage_model.amount_captured,
                customer_id: storage_model.customer_id,
                description: storage_model.description,
                return_url: storage_model.return_url,
                metadata: storage_model.metadata,
                statement_descriptor: storage_model.statement_descriptor,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                setup_future_usage: storage_model.setup_future_usage.unwrap_or_default(),
                active_attempt_id: storage_model.active_attempt_id,
                active_attempt_id_type: storage_model.active_attempt_id_type.unwrap_or_default(),
                active_attempts_group_id: storage_model.active_attempts_group_id,
                order_details: storage_model.order_details.map(|order_details| {
                    order_details
                        .into_iter()
                        .map(|order_detail| Secret::new(order_detail.expose()))
                        .collect::<Vec<_>>()
                }),
                allowed_payment_method_types,
                connector_metadata: storage_model
                    .connector_metadata
                    .map(|cm| cm.parse_value("ConnectorMetadata"))
                    .transpose()
                    .change_context(common_utils::errors::CryptoError::DecodingFailed)
                    .attach_printable("Failed to deserialize connector_metadata")?,
                feature_metadata: storage_model.feature_metadata,
                attempt_count: storage_model.attempt_count,
                profile_id: storage_model.profile_id,
                frm_merchant_decision: storage_model.frm_merchant_decision,
                payment_link_id: storage_model.payment_link_id,
                updated_by: storage_model.updated_by,
                request_incremental_authorization: storage_model
                    .request_incremental_authorization
                    .unwrap_or_default(),
                split_txns_enabled: storage_model.split_txns_enabled.unwrap_or_default(),
                authorization_count: storage_model.authorization_count,
                session_expiry: storage_model.session_expiry,
                request_external_three_ds_authentication: storage_model
                    .request_external_three_ds_authentication
                    .into(),
                frm_metadata: storage_model.frm_metadata,
                customer_details: data.customer_details,
                billing_address,
                shipping_address,
                capture_method: storage_model.capture_method.unwrap_or_default(),
                id: storage_model.id,
                merchant_reference_id: storage_model.merchant_reference_id,
                organization_id: storage_model.organization_id,
                authentication_type: storage_model.authentication_type,
                prerouting_algorithm: storage_model
                    .prerouting_algorithm
                    .map(|prerouting_algorithm_value| {
                        prerouting_algorithm_value
                            .parse_value("PaymentRoutingInfo")
                            .change_context(common_utils::errors::CryptoError::DecodingFailed)
                    })
                    .transpose()?,
                enable_payment_link: storage_model.enable_payment_link.into(),
                apply_mit_exemption: storage_model.apply_mit_exemption.into(),
                customer_present: storage_model.customer_present.into(),
                payment_link_config: storage_model.payment_link_config,
                routing_algorithm_id: storage_model.routing_algorithm_id,
                split_payments: storage_model.split_payments,
                force_3ds_challenge: storage_model.force_3ds_challenge,
                force_3ds_challenge_trigger: storage_model.force_3ds_challenge_trigger,
                processor_merchant_id: storage_model
                    .processor_merchant_id
                    .unwrap_or(storage_model.merchant_id),
                created_by: storage_model
                    .created_by
                    .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
                is_iframe_redirection_enabled: storage_model.is_iframe_redirection_enabled,
                is_payment_id_from_merchant: storage_model.is_payment_id_from_merchant,
                enable_partial_authorization: storage_model
                    .enable_partial_authorization
                    .unwrap_or(false.into()),
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment intent".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let amount_details = self.amount_details;

        Ok(DieselPaymentIntentNew {
            surcharge_applicable: Some(amount_details.get_surcharge_action_as_bool()),
            skip_external_tax_calculation: Some(amount_details.get_external_tax_action_as_bool()),
            merchant_id: self.merchant_id,
            status: self.status,
            amount: amount_details.order_amount,
            currency: amount_details.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
            metadata: self.metadata,
            statement_descriptor: self.statement_descriptor,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: Some(self.setup_future_usage),
            active_attempt_id: self.active_attempt_id,
            order_details: self.order_details,
            allowed_payment_method_types: self
                .allowed_payment_method_types
                .map(|allowed_payment_method_types| {
                    allowed_payment_method_types
                        .encode_to_value()
                        .change_context(ValidationError::InvalidValue {
                            message: "Failed to serialize allowed_payment_method_types".to_string(),
                        })
                })
                .transpose()?
                .map(Secret::new),
            connector_metadata: self
                .connector_metadata
                .map(|cm| {
                    cm.encode_to_value()
                        .change_context(ValidationError::InvalidValue {
                            message: "Failed to serialize connector_metadata".to_string(),
                        })
                })
                .transpose()?
                .map(Secret::new),
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            frm_merchant_decision: self.frm_merchant_decision,
            payment_link_id: self.payment_link_id,
            updated_by: self.updated_by,

            request_incremental_authorization: Some(self.request_incremental_authorization),
            split_txns_enabled: Some(self.split_txns_enabled),
            authorization_count: self.authorization_count,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: Some(
                self.request_external_three_ds_authentication.as_bool(),
            ),
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_address: self.billing_address.map(Encryption::from),
            shipping_address: self.shipping_address.map(Encryption::from),
            capture_method: Some(self.capture_method),
            id: self.id,
            merchant_reference_id: self.merchant_reference_id,
            authentication_type: self.authentication_type,
            prerouting_algorithm: self
                .prerouting_algorithm
                .map(|prerouting_algorithm| {
                    prerouting_algorithm.encode_to_value().change_context(
                        ValidationError::InvalidValue {
                            message: "Failed to serialize prerouting_algorithm".to_string(),
                        },
                    )
                })
                .transpose()?,
            surcharge_amount: amount_details.surcharge_amount,
            tax_on_surcharge: amount_details.tax_on_surcharge,
            organization_id: self.organization_id,
            shipping_cost: amount_details.shipping_cost,
            tax_details: amount_details.tax_details,
            enable_payment_link: Some(self.enable_payment_link.as_bool()),
            apply_mit_exemption: Some(self.apply_mit_exemption.as_bool()),
            platform_merchant_id: None,
            force_3ds_challenge: self.force_3ds_challenge,
            force_3ds_challenge_trigger: self.force_3ds_challenge_trigger,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            routing_algorithm_id: self.routing_algorithm_id,
            is_payment_id_from_merchant: self.is_payment_id_from_merchant,
            payment_channel: None,
            tax_status: None,
            discount_amount: None,
            mit_category: None,
            shipping_amount_tax: None,
            duty_amount: None,
            order_date: None,
            enable_partial_authorization: Some(self.enable_partial_authorization),
            tokenization: None,
            active_attempt_id_type: Some(self.active_attempt_id_type),
            active_attempts_group_id: self.active_attempts_group_id,
            state_metadata: None,
            installment_options: None,
        })
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for PaymentIntent {
    type DstType = DieselPaymentIntent;
    type NewDstType = DieselPaymentIntentNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(DieselPaymentIntent {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: None, // deprecated
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.get_id(),
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_link_id: self.payment_link_id,
            payment_confirm_source: self.payment_confirm_source,
            updated_by: self.updated_by,
            surcharge_applicable: self.surcharge_applicable,
            request_incremental_authorization: self.request_incremental_authorization,
            incremental_authorization_allowed: self.incremental_authorization_allowed,
            authorization_count: self.authorization_count,
            fingerprint_id: self.fingerprint_id,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: self.request_external_three_ds_authentication,
            charges: None,
            split_payments: self.split_payments,
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_details: self.billing_details.map(Encryption::from),
            merchant_order_reference_id: self.merchant_order_reference_id,
            shipping_details: self.shipping_details.map(Encryption::from),
            is_payment_processor_token_flow: self.is_payment_processor_token_flow,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            tax_details: self.tax_details,
            skip_external_tax_calculation: self.skip_external_tax_calculation,
            request_extended_authorization: self.request_extended_authorization,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            platform_merchant_id: None,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            force_3ds_challenge: self.force_3ds_challenge,
            force_3ds_challenge_trigger: self.force_3ds_challenge_trigger,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            extended_return_url: self.return_url,
            is_payment_id_from_merchant: self.is_payment_id_from_merchant,
            payment_channel: self.payment_channel,
            tax_status: self.tax_status,
            discount_amount: self.discount_amount,
            order_date: self.order_date,
            shipping_amount_tax: self.shipping_amount_tax,
            duty_amount: self.duty_amount,
            enable_partial_authorization: self.enable_partial_authorization,
            enable_overcapture: self.enable_overcapture,
            mit_category: self.mit_category,
            billing_descriptor: self.billing_descriptor,
            tokenization: self.tokenization,
            partner_merchant_identifier_details: self.partner_merchant_identifier_details,
            state_metadata: self.state_metadata,
            installment_options: self
                .installment_options
                .map(common_types::payments::InstallmentOptions),
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentIntent::to_encryptable(
                    EncryptedPaymentIntent {
                        billing_details: storage_model.billing_details,
                        shipping_details: storage_model.shipping_details,
                        customer_details: storage_model.customer_details,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentIntent::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                payment_id: storage_model.payment_id,
                merchant_id: storage_model.merchant_id.clone(),
                status: storage_model.status,
                amount: storage_model.amount,
                currency: storage_model.currency,
                amount_captured: storage_model.amount_captured,
                customer_id: storage_model.customer_id,
                description: storage_model.description,
                return_url: storage_model
                    .extended_return_url
                    .or(storage_model.return_url), // fallback to legacy
                metadata: storage_model.metadata,
                connector_id: storage_model.connector_id,
                shipping_address_id: storage_model.shipping_address_id,
                billing_address_id: storage_model.billing_address_id,
                statement_descriptor_name: storage_model.statement_descriptor_name,
                statement_descriptor_suffix: storage_model.statement_descriptor_suffix,
                created_at: storage_model.created_at,
                modified_at: storage_model.modified_at,
                last_synced: storage_model.last_synced,
                setup_future_usage: storage_model.setup_future_usage,
                off_session: storage_model.off_session,
                client_secret: storage_model.client_secret,
                active_attempt: RemoteStorageObject::ForeignID(storage_model.active_attempt_id),
                business_country: storage_model.business_country,
                business_label: storage_model.business_label,
                order_details: storage_model.order_details,
                allowed_payment_method_types: storage_model.allowed_payment_method_types,
                connector_metadata: storage_model.connector_metadata,
                feature_metadata: storage_model.feature_metadata,
                attempt_count: storage_model.attempt_count,
                profile_id: storage_model.profile_id,
                merchant_decision: storage_model.merchant_decision,
                payment_link_id: storage_model.payment_link_id,
                payment_confirm_source: storage_model.payment_confirm_source,
                updated_by: storage_model.updated_by,
                surcharge_applicable: storage_model.surcharge_applicable,
                request_incremental_authorization: storage_model.request_incremental_authorization,
                incremental_authorization_allowed: storage_model.incremental_authorization_allowed,
                authorization_count: storage_model.authorization_count,
                fingerprint_id: storage_model.fingerprint_id,
                session_expiry: storage_model.session_expiry,
                request_external_three_ds_authentication: storage_model
                    .request_external_three_ds_authentication,
                split_payments: storage_model.split_payments,
                frm_metadata: storage_model.frm_metadata,
                shipping_cost: storage_model.shipping_cost,
                tax_details: storage_model.tax_details,
                customer_details: data.customer_details,
                billing_details: data.billing_details,
                merchant_order_reference_id: storage_model.merchant_order_reference_id,
                shipping_details: data.shipping_details,
                is_payment_processor_token_flow: storage_model.is_payment_processor_token_flow,
                organization_id: storage_model.organization_id,
                skip_external_tax_calculation: storage_model.skip_external_tax_calculation,
                request_extended_authorization: storage_model.request_extended_authorization,
                psd2_sca_exemption_type: storage_model.psd2_sca_exemption_type,
                processor_merchant_id: storage_model
                    .processor_merchant_id
                    .unwrap_or(storage_model.merchant_id),
                created_by: storage_model
                    .created_by
                    .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
                force_3ds_challenge: storage_model.force_3ds_challenge,
                force_3ds_challenge_trigger: storage_model.force_3ds_challenge_trigger,
                is_iframe_redirection_enabled: storage_model.is_iframe_redirection_enabled,
                is_payment_id_from_merchant: storage_model.is_payment_id_from_merchant,
                payment_channel: storage_model.payment_channel,
                tax_status: storage_model.tax_status,
                discount_amount: storage_model.discount_amount,
                shipping_amount_tax: storage_model.shipping_amount_tax,
                duty_amount: storage_model.duty_amount,
                order_date: storage_model.order_date,
                enable_partial_authorization: storage_model.enable_partial_authorization,
                enable_overcapture: storage_model.enable_overcapture,
                mit_category: storage_model.mit_category,
                billing_descriptor: storage_model.billing_descriptor,
                tokenization: storage_model.tokenization,
                partner_merchant_identifier_details: storage_model
                    .partner_merchant_identifier_details,
                state_metadata: storage_model.state_metadata,
                installment_options: storage_model.installment_options.map(|o| o.0),
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment intent".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(DieselPaymentIntentNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: None, // deprecated
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt.get_id(),
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_link_id: self.payment_link_id,
            payment_confirm_source: self.payment_confirm_source,
            updated_by: self.updated_by,
            surcharge_applicable: self.surcharge_applicable,
            request_incremental_authorization: self.request_incremental_authorization,
            incremental_authorization_allowed: self.incremental_authorization_allowed,
            authorization_count: self.authorization_count,
            fingerprint_id: self.fingerprint_id,
            session_expiry: self.session_expiry,
            request_external_three_ds_authentication: self.request_external_three_ds_authentication,
            charges: None,
            split_payments: self.split_payments,
            frm_metadata: self.frm_metadata,
            customer_details: self.customer_details.map(Encryption::from),
            billing_details: self.billing_details.map(Encryption::from),
            merchant_order_reference_id: self.merchant_order_reference_id,
            shipping_details: self.shipping_details.map(Encryption::from),
            is_payment_processor_token_flow: self.is_payment_processor_token_flow,
            organization_id: self.organization_id,
            shipping_cost: self.shipping_cost,
            tax_details: self.tax_details,
            skip_external_tax_calculation: self.skip_external_tax_calculation,
            request_extended_authorization: self.request_extended_authorization,
            psd2_sca_exemption_type: self.psd2_sca_exemption_type,
            platform_merchant_id: None,
            processor_merchant_id: Some(self.processor_merchant_id),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            force_3ds_challenge: self.force_3ds_challenge,
            force_3ds_challenge_trigger: self.force_3ds_challenge_trigger,
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            extended_return_url: self.return_url,
            is_payment_id_from_merchant: self.is_payment_id_from_merchant,
            payment_channel: self.payment_channel,
            tax_status: self.tax_status,
            discount_amount: self.discount_amount,
            order_date: self.order_date,
            shipping_amount_tax: self.shipping_amount_tax,
            duty_amount: self.duty_amount,
            enable_partial_authorization: self.enable_partial_authorization,
            enable_overcapture: self.enable_overcapture,
            mit_category: self.mit_category,
            billing_descriptor: self.billing_descriptor,
            tokenization: self.tokenization,
            partner_merchant_identifier_details: self.partner_merchant_identifier_details,
            state_metadata: self.state_metadata,
            installment_options: self
                .installment_options
                .map(common_types::payments::InstallmentOptions),
        })
    }
}
