#[cfg(feature = "olap")]
use api_models::payments::{AmountFilter, Order, SortBy, SortOn};
#[cfg(feature = "olap")]
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
#[cfg(feature = "olap")]
use common_utils::errors::ReportSwitchExt;
use common_utils::{
    ext_traits::{AsyncExt, Encode},
    types::keymanager::KeyManagerState,
};
#[cfg(feature = "olap")]
use diesel::{associations::HasTable, ExpressionMethods, JoinOnDsl, QueryDsl};
#[cfg(feature = "v1")]
use diesel_models::payment_intent::PaymentIntentUpdate as DieselPaymentIntentUpdate;
#[cfg(feature = "olap")]
use diesel_models::query::generics::db_metrics;
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
};
use error_stack::ResultExt;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::payments::{
    payment_attempt::PaymentAttempt, payment_intent::PaymentIntentFetchConstraints,
};
use hyperswitch_domain_models::{
    behaviour::Conversion,
    errors::StorageError,
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
    errors::RedisErrorExt,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_write},
    DatabaseStore, KVRouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for KVRouterStore<T> {
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
                todo!("Implement payment intent insert for kv")
            }
        }
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
                todo!()
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
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn: bb8::PooledConnection<
            '_,
            async_bb8_diesel::ConnectionManager<diesel::PgConnection>,
        > = pg_connection_read(self).await?;
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
                todo!()
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for crate::RouterStore<T> {
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
        let diesel_payment_intent_update =
            diesel_models::PaymentIntentUpdateInternal::from(payment_intent);

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
        use common_utils::errors::ReportSwitchExt;
        use futures::{future::try_join_all, FutureExt};

        let conn = connection::pg_connection_read(self).await.switch()?;
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
        let conn = connection::pg_connection_read(self).await.switch()?;
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

        let conn = connection::pg_connection_read(self).await.switch()?;
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

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PaymentIntentFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        let conn = connection::pg_connection_read(self).await.switch()?;
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
