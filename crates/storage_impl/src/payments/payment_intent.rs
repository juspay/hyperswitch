#[cfg(feature = "olap")]
use api_models::payments::{AmountFilter, Order, SortBy, SortOn};
#[cfg(feature = "olap")]
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
#[cfg(feature = "v2")]
use common_utils::{errors::ParsingError, fallback_reverse_lookup_not_found};
use common_utils::{
    ext_traits::{AsyncExt, Encode},
    types::keymanager::KeyManagerState,
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
    enums::MerchantStorageScheme, kv, payment_intent::PaymentIntent as DieselPaymentIntent,
    PaymentIntentNew as DieselPaymentIntentNew,
};
use error_stack::ResultExt;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::payments::{
    payment_attempt::PaymentAttempt, payment_intent::PaymentIntentFetchConstraints,
};
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    payments::{
        payment_intent::{PaymentIntentInterface, PaymentIntentUpdate},
        PaymentIntent,
    },
};
use redis_interface::HsetnxReply;
#[cfg(feature = "olap")]
use router_env::logger;
use router_env::{instrument, tracing};

#[cfg(feature = "olap")]
use crate::connection;
use crate::{
    diesel_error_to_data_error,
    errors::{RedisErrorExt, StorageError},
    kv_router_store::KVRouterStore,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_write},
    DatabaseStore,
};
#[cfg(feature = "v2")]
use crate::{errors, lookup::ReverseLookupInterface, utils::ForeignTryFrom};
use common_utils::encryption::Encryption;
use common_utils::errors::CustomResult;
use common_utils::errors::ValidationError;
use common_utils::type_name;
use common_utils::types::keymanager;
use hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdateFields;
use hyperswitch_domain_models::payments::AmountDetails;
use hyperswitch_domain_models::payments::EncryptedPaymentIntent;
use hyperswitch_domain_models::type_encryption::crypto_operation;
use hyperswitch_domain_models::type_encryption::CryptoOperation;
use masking::Secret;
use common_utils::types::CreatedBy;

use masking::PeekInterface;
use masking::ExposeInterface;
use common_utils::types::keymanager::ToEncryptable;
use common_utils::ext_traits::ValueExt;

use crate::behaviour::Conversion;

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[cfg(feature = "v1")]
    async fn insert_payment_intent(
        &self,
        state: &KeyManagerState,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let merchant_id = payment_intent.merchant_id.clone();
        let payment_id = payment_intent.get_id().to_owned();
        let field = payment_intent.get_id().get_hash_key_for_kv_store();
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &merchant_id,
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
                    .insert_payment_intent(
                        state,
                        payment_intent,
                        merchant_key_store,
                        storage_scheme,
                    )
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
        state: &KeyManagerState,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payment_intent(
                        state,
                        payment_intent,
                        merchant_key_store,
                        storage_scheme,
                    )
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
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, Option<PaymentAttempt>)>, StorageError> {
        self.router_store
            .get_filtered_payment_intents_attempt(
                state,
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
        state: &KeyManagerState,
        this: PaymentIntent,
        payment_intent_update: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let merchant_id = this.merchant_id.clone();
        let payment_id = this.get_id().to_owned();
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &merchant_id,
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
                        state,
                        this,
                        payment_intent_update,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();

                let diesel_intent_update = DieselPaymentIntentUpdate::from(payment_intent_update);
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
                    state,
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

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        state: &KeyManagerState,
        this: PaymentIntent,
        payment_intent_update: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payment_intent(
                        state,
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
                    PaymentIntentUpdateInternal::foreign_try_from(payment_intent_update)
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
                    state,
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
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        state: &KeyManagerState,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselPaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
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
                    merchant_id,
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
            state,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_id.to_owned().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_id(
        &self,
        state: &KeyManagerState,
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
            state,
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
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        self.router_store
            .filter_payment_intent_by_constraints(
                state,
                merchant_id,
                filters,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        self.router_store
            .filter_payment_intents_by_time_range_constraints(
                state,
                merchant_id,
                time_range,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_intent_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::IntentStatus, i64)>, StorageError> {
        self.router_store
            .get_intent_status_with_count(merchant_id, profile_id_list, time_range)
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_payment_intents_attempt(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        self.router_store
            .get_filtered_payment_intents_attempt(
                state,
                merchant_id,
                filters,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        self.router_store
            .get_filtered_active_attempt_ids_for_total_count(
                merchant_id,
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
        state: &KeyManagerState,
        merchant_reference_id: &common_utils::id_type::PaymentReferenceId,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: &MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_intent_by_merchant_reference_id_profile_id(
                        state,
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
                            state,
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
                    state,
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
        state: &KeyManagerState,
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
            state,
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
        state: &KeyManagerState,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_payment_intent_update = DieselPaymentIntentUpdate::from(payment_intent);

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
            state,
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
        state: &KeyManagerState,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        let diesel_payment_intent_update = PaymentIntentUpdateInternal::foreign_try_from(payment_intent)
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
            state,
            diesel_payment_intent,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        state: &KeyManagerState,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .async_and_then(|diesel_payment_intent| async {
                PaymentIntent::convert_back(
                    state,
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
        state: &KeyManagerState,
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
            state,
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
        state: &KeyManagerState,
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
            state,
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
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
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
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
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
                            .find_payment_intent_by_payment_id_merchant_id(
                                state,
                                starting_after_id,
                                merchant_id,
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
                            .find_payment_intent_by_payment_id_merchant_id(
                                state,
                                ending_before_id,
                                merchant_id,
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

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        db_metrics::track_database_call::<<DieselPaymentIntent as HasTable>::Table, _, _>(
            query.get_results_async::<DieselPaymentIntent>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map(|payment_intents| {
            try_join_all(payment_intents.into_iter().map(|diesel_payment_intent| {
                PaymentIntent::convert_back(
                    state,
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
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        // TODO: Remove this redundant function
        let payment_filters = (*time_range).into();
        self.filter_payment_intent_by_constraints(
            state,
            merchant_id,
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
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::IntentStatus, i64)>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);

        let mut query = <DieselPaymentIntent as HasTable>::table()
            .group_by(pi_dsl::status)
            .select((pi_dsl::status, diesel::dsl::count_star()))
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
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
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        use futures::{future::try_join_all, FutureExt};

        use crate::DataModelExt;

        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
            .inner_join(
                payment_attempt_schema::table.on(pa_dsl::attempt_id.eq(pi_dsl::active_attempt_id)),
            )
            .filter(pa_dsl::merchant_id.eq(merchant_id.to_owned())) // Ensure merchant_ids match, as different merchants can share payment/attempt IDs.
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
                            .find_payment_intent_by_payment_id_merchant_id(
                                state,
                                starting_after_id,
                                merchant_id,
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
                            .find_payment_intent_by_payment_id_merchant_id(
                                state,
                                ending_before_id,
                                merchant_id,
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

        query
            .get_results_async::<(
                DieselPaymentIntent,
                diesel_models::payment_attempt::PaymentAttempt,
            )>(conn)
            .await
            .map(|results| {
                try_join_all(results.into_iter().map(|(pi, pa)| {
                    PaymentIntent::convert_back(
                        state,
                        pi,
                        merchant_key_store.key.get_inner(),
                        merchant_id.to_owned().into(),
                    )
                    .map(|payment_intent| {
                        payment_intent.map(|payment_intent| {
                            (payment_intent, PaymentAttempt::from_storage_model(pa))
                        })
                    })
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

    #[cfg(all(feature = "v2", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_payment_intents_attempt(
        &self,
        state: &KeyManagerState,
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
            PaymentIntentFetchConstraints::Single { payment_intent_id } => {
                query.filter(pi_dsl::id.eq(payment_intent_id.to_owned()))
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
                                state,
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
                                state,
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
                    Some(currency) => query.filter(pi_dsl::currency.eq(*currency)),
                    None => query,
                };

                query = match &params.connector {
                    Some(connector) => query.filter(pa_dsl::connector.eq(*connector)),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq(*status)),
                    None => query,
                };

                query = match &params.payment_method_type {
                    Some(payment_method_type) => {
                        query.filter(pa_dsl::payment_method_type_v2.eq(*payment_method_type))
                    }
                    None => query,
                };

                query = match &params.payment_method_subtype {
                    Some(payment_method_subtype) => {
                        query.filter(pa_dsl::payment_method_subtype.eq(*payment_method_subtype))
                    }
                    None => query,
                };

                query = match &params.authentication_type {
                    Some(authentication_type) => {
                        query.filter(pa_dsl::authentication_type.eq(*authentication_type))
                    }
                    None => query,
                };

                query = match &params.merchant_connector_id {
                    Some(merchant_connector_id) => query
                        .filter(pa_dsl::merchant_connector_id.eq(merchant_connector_id.clone())),
                    None => query,
                };

                if let Some(card_network) = &params.card_network {
                    query = query.filter(pa_dsl::card_network.eq(card_network.clone()));
                }
                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

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
                            state,
                            pi,
                            merchant_key_store.key.get_inner(),
                            merchant_id.to_owned().into(),
                        );
                        let payment_attempt = pa
                            .async_map(|val| {
                                PaymentAttempt::convert_back(
                                    state,
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
            PaymentIntentFetchConstraints::Single { payment_intent_id } => {
                query.filter(pi_dsl::id.eq(payment_intent_id.to_owned()))
            }
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
                    Some(currency) => query.filter(pi_dsl::currency.eq(*currency)),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(pi_dsl::status.eq(*status)),
                    None => query,
                };

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
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .select(pi_dsl::active_attempt_id)
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
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

// This conversion is used in the `update_payment_intent` function
#[cfg(feature = "v2")]
impl ForeignTryFrom<PaymentIntentUpdate> for diesel_models::PaymentIntentUpdateInternal {
    type Error = error_stack::Report<ParsingError>;
    fn foreign_try_from(payment_intent_update: PaymentIntentUpdate) -> Result<Self, Self::Error> {
        match payment_intent_update {
            PaymentIntentUpdate::ConfirmIntent {
                status,
                active_attempt_id,
                updated_by,
            } => Ok(Self {
                status: Some(status),
                active_attempt_id: Some(active_attempt_id),
                prerouting_algorithm: None,
                modified_at: common_utils::date_time::now(),
                amount: None,
                amount_captured: None,
                currency: None,
                shipping_cost: None,
                tax_details: None,
                skip_external_tax_calculation: None,
                surcharge_applicable: None,
                surcharge_amount: None,
                tax_on_surcharge: None,
                routing_algorithm_id: None,
                capture_method: None,
                authentication_type: None,
                billing_address: None,
                shipping_address: None,
                customer_present: None,
                description: None,
                return_url: None,
                setup_future_usage: None,
                apply_mit_exemption: None,
                statement_descriptor: None,
                order_details: None,
                allowed_payment_method_types: None,
                metadata: None,
                connector_metadata: None,
                feature_metadata: None,
                payment_link_config: None,
                request_incremental_authorization: None,
                session_expiry: None,
                frm_metadata: None,
                request_external_three_ds_authentication: None,
                updated_by,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
            }),

            PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status,
                updated_by,
                amount_captured,
                feature_metadata,
            } => Ok(Self {
                status: Some(status),
                active_attempt_id: None,
                prerouting_algorithm: None,
                modified_at: common_utils::date_time::now(),
                amount_captured,
                amount: None,
                currency: None,
                shipping_cost: None,
                tax_details: None,
                skip_external_tax_calculation: None,
                surcharge_applicable: None,
                surcharge_amount: None,
                tax_on_surcharge: None,
                routing_algorithm_id: None,
                capture_method: None,
                authentication_type: None,
                billing_address: None,
                shipping_address: None,
                customer_present: None,
                description: None,
                return_url: None,
                setup_future_usage: None,
                apply_mit_exemption: None,
                statement_descriptor: None,
                order_details: None,
                allowed_payment_method_types: None,
                metadata: None,
                connector_metadata: None,
                feature_metadata: feature_metadata.map(|val| *val),
                payment_link_config: None,
                request_incremental_authorization: None,
                session_expiry: None,
                frm_metadata: None,
                request_external_three_ds_authentication: None,
                updated_by,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
            }),
            PaymentIntentUpdate::SyncUpdate {
                status,
                amount_captured,
                updated_by,
            } => Ok(Self {
                status: Some(status),
                active_attempt_id: None,
                prerouting_algorithm: None,
                modified_at: common_utils::date_time::now(),
                amount: None,
                currency: None,
                amount_captured,
                shipping_cost: None,
                tax_details: None,
                skip_external_tax_calculation: None,
                surcharge_applicable: None,
                surcharge_amount: None,
                tax_on_surcharge: None,
                routing_algorithm_id: None,
                capture_method: None,
                authentication_type: None,
                billing_address: None,
                shipping_address: None,
                customer_present: None,
                description: None,
                return_url: None,
                setup_future_usage: None,
                apply_mit_exemption: None,
                statement_descriptor: None,
                order_details: None,
                allowed_payment_method_types: None,
                metadata: None,
                connector_metadata: None,
                feature_metadata: None,
                payment_link_config: None,
                request_incremental_authorization: None,
                session_expiry: None,
                frm_metadata: None,
                request_external_three_ds_authentication: None,
                updated_by,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
            }),
            PaymentIntentUpdate::CaptureUpdate {
                status,
                amount_captured,
                updated_by,
            } => Ok(Self {
                status: Some(status),
                amount_captured,
                active_attempt_id: None,
                prerouting_algorithm: None,
                modified_at: common_utils::date_time::now(),
                amount: None,
                currency: None,
                shipping_cost: None,
                tax_details: None,
                skip_external_tax_calculation: None,
                surcharge_applicable: None,
                surcharge_amount: None,
                tax_on_surcharge: None,
                routing_algorithm_id: None,
                capture_method: None,
                authentication_type: None,
                billing_address: None,
                shipping_address: None,
                customer_present: None,
                description: None,
                return_url: None,
                setup_future_usage: None,
                apply_mit_exemption: None,
                statement_descriptor: None,
                order_details: None,
                allowed_payment_method_types: None,
                metadata: None,
                connector_metadata: None,
                feature_metadata: None,
                payment_link_config: None,
                request_incremental_authorization: None,
                session_expiry: None,
                frm_metadata: None,
                request_external_three_ds_authentication: None,
                updated_by,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
            }),
            PaymentIntentUpdate::SessionIntentUpdate {
                prerouting_algorithm,
                updated_by,
            } => Ok(Self {
                status: None,
                active_attempt_id: None,
                modified_at: common_utils::date_time::now(),
                amount_captured: None,
                prerouting_algorithm: Some(
                    prerouting_algorithm
                        .encode_to_value()
                        .attach_printable("Failed to Serialize prerouting_algorithm")?,
                ),
                amount: None,
                currency: None,
                shipping_cost: None,
                tax_details: None,
                skip_external_tax_calculation: None,
                surcharge_applicable: None,
                surcharge_amount: None,
                tax_on_surcharge: None,
                routing_algorithm_id: None,
                capture_method: None,
                authentication_type: None,
                billing_address: None,
                shipping_address: None,
                customer_present: None,
                description: None,
                return_url: None,
                setup_future_usage: None,
                apply_mit_exemption: None,
                statement_descriptor: None,
                order_details: None,
                allowed_payment_method_types: None,
                metadata: None,
                connector_metadata: None,
                feature_metadata: None,
                payment_link_config: None,
                request_incremental_authorization: None,
                session_expiry: None,
                frm_metadata: None,
                request_external_three_ds_authentication: None,
                updated_by,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
            }),
            PaymentIntentUpdate::UpdateIntent(boxed_intent) => {
                let PaymentIntentUpdateFields {
                    amount,
                    currency,
                    shipping_cost,
                    tax_details,
                    skip_external_tax_calculation,
                    skip_surcharge_calculation,
                    surcharge_amount,
                    tax_on_surcharge,
                    routing_algorithm_id,
                    capture_method,
                    authentication_type,
                    billing_address,
                    shipping_address,
                    customer_present,
                    description,
                    return_url,
                    setup_future_usage,
                    apply_mit_exemption,
                    statement_descriptor,
                    order_details,
                    allowed_payment_method_types,
                    metadata,
                    connector_metadata,
                    feature_metadata,
                    payment_link_config,
                    request_incremental_authorization,
                    session_expiry,
                    frm_metadata,
                    request_external_three_ds_authentication,
                    active_attempt_id,
                    updated_by,
                    force_3ds_challenge,
                    is_iframe_redirection_enabled,
                } = *boxed_intent;
                Ok(Self {
                    status: None,
                    active_attempt_id,
                    prerouting_algorithm: None,
                    modified_at: common_utils::date_time::now(),
                    amount_captured: None,
                    amount,
                    currency,
                    shipping_cost,
                    tax_details,
                    skip_external_tax_calculation: skip_external_tax_calculation
                        .map(|val| val.as_bool()),
                    surcharge_applicable: skip_surcharge_calculation.map(|val| val.as_bool()),
                    surcharge_amount,
                    tax_on_surcharge,
                    routing_algorithm_id,
                    capture_method,
                    authentication_type,
                    billing_address: billing_address.map(Encryption::from),
                    shipping_address: shipping_address.map(Encryption::from),
                    customer_present: customer_present.map(|val| val.as_bool()),
                    description,
                    return_url,
                    setup_future_usage,
                    apply_mit_exemption: apply_mit_exemption.map(|val| val.as_bool()),
                    statement_descriptor,
                    order_details,
                    allowed_payment_method_types: allowed_payment_method_types
                        .map(|allowed_payment_method_types| {
                            allowed_payment_method_types.encode_to_value()
                        })
                        .and_then(|r| r.ok().map(Secret::new)),
                    metadata,
                    connector_metadata,
                    feature_metadata,
                    payment_link_config,
                    request_incremental_authorization,
                    session_expiry,
                    frm_metadata,
                    request_external_three_ds_authentication:
                        request_external_three_ds_authentication.map(|val| val.as_bool()),

                    updated_by,
                    force_3ds_challenge,
                    is_iframe_redirection_enabled,
                })
            }
            PaymentIntentUpdate::RecordUpdate {
                status,
                feature_metadata,
                updated_by,
                active_attempt_id,
            } => Ok(Self {
                status: Some(status),
                amount_captured: None,
                active_attempt_id: Some(active_attempt_id),
                modified_at: common_utils::date_time::now(),
                amount: None,
                currency: None,
                shipping_cost: None,
                tax_details: None,
                skip_external_tax_calculation: None,
                surcharge_applicable: None,
                surcharge_amount: None,
                tax_on_surcharge: None,
                routing_algorithm_id: None,
                capture_method: None,
                authentication_type: None,
                billing_address: None,
                shipping_address: None,
                customer_present: None,
                description: None,
                return_url: None,
                setup_future_usage: None,
                apply_mit_exemption: None,
                statement_descriptor: None,
                order_details: None,
                allowed_payment_method_types: None,
                metadata: None,
                connector_metadata: None,
                feature_metadata: *feature_metadata,
                payment_link_config: None,
                request_incremental_authorization: None,
                prerouting_algorithm: None,
                session_expiry: None,
                frm_metadata: None,
                request_external_three_ds_authentication: None,
                updated_by,
                force_3ds_challenge: None,
                is_iframe_redirection_enabled: None,
            }),
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
            connector_metadata,
            feature_metadata,
            attempt_count,
            profile_id,
            frm_merchant_decision,
            payment_link_id,
            updated_by,

            request_incremental_authorization: Some(request_incremental_authorization),
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
            created_by: created_by.map(|cb| cb.to_string()),
            is_iframe_redirection_enabled,
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

            let amount_details = AmountDetails {
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
                order_details: storage_model.order_details.map(|order_details| {
                    order_details
                        .into_iter()
                        .map(|order_detail| Secret::new(order_detail.expose()))
                        .collect::<Vec<_>>()
                }),
                allowed_payment_method_types,
                connector_metadata: storage_model.connector_metadata,
                feature_metadata: storage_model.feature_metadata,
                attempt_count: storage_model.attempt_count,
                profile_id: storage_model.profile_id,
                frm_merchant_decision: storage_model.frm_merchant_decision,
                payment_link_id: storage_model.payment_link_id,
                updated_by: storage_model.updated_by,
                request_incremental_authorization: storage_model
                    .request_incremental_authorization
                    .unwrap_or_default(),
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
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            frm_merchant_decision: self.frm_merchant_decision,
            payment_link_id: self.payment_link_id,
            updated_by: self.updated_by,

            request_incremental_authorization: Some(self.request_incremental_authorization),
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
            created_by: self.created_by.map(|cb| cb.to_string()),
            is_iframe_redirection_enabled: self.is_iframe_redirection_enabled,
            routing_algorithm_id: self.routing_algorithm_id,
        })
    }
}
