#[cfg(feature = "olap")]
use api_models::payments::AmountFilter;
#[cfg(feature = "olap")]
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
#[cfg(feature = "olap")]
use common_utils::errors::ReportSwitchExt;
use common_utils::ext_traits::{AsyncExt, Encode};
#[cfg(feature = "olap")]
use diesel::{associations::HasTable, ExpressionMethods, JoinOnDsl, QueryDsl};
use diesel_models::{
    enums::MerchantStorageScheme,
    kv,
    payment_attempt::PaymentAttempt as DieselPaymentAttempt,
    payment_intent::{
        PaymentIntent as DieselPaymentIntent, PaymentIntentUpdate as DieselPaymentIntentUpdate,
    },
};
#[cfg(feature = "olap")]
use diesel_models::{
    query::generics::db_metrics,
    schema::{payment_attempt::dsl as pa_dsl, payment_intent::dsl as pi_dsl},
};
use error_stack::ResultExt;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints;
use hyperswitch_domain_models::{
    behaviour::Conversion,
    errors::StorageError,
    merchant_key_store::MerchantKeyStore,
    payments::{
        payment_attempt::PaymentAttempt,
        payment_intent::{PaymentIntentInterface, PaymentIntentUpdate},
        PaymentIntent,
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
    diesel_error_to_data_error,
    errors::RedisErrorExt,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_write},
    DataModelExt, DatabaseStore, KVRouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for KVRouterStore<T> {
    async fn insert_payment_intent(
        &self,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let merchant_id = payment_intent.merchant_id.clone();
        let payment_id = payment_intent.payment_id.clone();
        let field = format!("pi_{}", payment_intent.payment_id);
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &merchant_id,
            payment_id: &payment_id,
        };
        let storage_scheme =
            decide_storage_scheme::<_, DieselPaymentIntent>(self, storage_scheme, Op::Insert).await;
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
                        insertable: kv::Insertable::PaymentIntent(new_payment_intent),
                    },
                };

                let diesel_payment_intent = payment_intent
                    .clone()
                    .convert()
                    .await
                    .change_context(StorageError::EncryptionError)?;

                match kv_wrapper::<DieselPaymentIntent, _, _>(
                    self,
                    KvOperation::<DieselPaymentIntent>::HSetNx(
                        &field,
                        &diesel_payment_intent,
                        redis_entry,
                    ),
                    key,
                )
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

    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent_update: PaymentIntentUpdate,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let merchant_id = this.merchant_id.clone();
        let payment_id = this.payment_id.clone();
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &merchant_id,
            payment_id: &payment_id,
        };
        let field = format!("pi_{}", this.payment_id);
        let storage_scheme = decide_storage_scheme::<_, DieselPaymentIntent>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, Some(&this.updated_by)),
        )
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
                        updatable: kv::Updateable::PaymentIntentUpdate(
                            kv::PaymentIntentUpdateMems {
                                orig: origin_diesel_intent,
                                update_data: diesel_intent_update,
                            },
                        ),
                    },
                };

                kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<DieselPaymentIntent>::Hset((&field, redis_value), redis_entry),
                    key,
                )
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(StorageError::KVError)?;

                let payment_intent =
                    PaymentIntent::convert_back(diesel_intent, merchant_key_store.key.get_inner())
                        .await
                        .change_context(StorageError::DecryptionError)?;

                Ok(payment_intent)
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselPaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(er.current_context());
                    er.change_context(new_err)
                })
        };
        let storage_scheme =
            decide_storage_scheme::<_, DieselPaymentIntent>(self, storage_scheme, Op::Find).await;
        let diesel_payment_intent = match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,

            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id,
                    payment_id,
                };
                let field = format!("pi_{payment_id}");
                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper::<DieselPaymentIntent, _, _>(
                            self,
                            KvOperation::<DieselPaymentIntent>::HGet(&field),
                            key,
                        )
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await
            }
        }?;

        PaymentIntent::convert_back(diesel_payment_intent, merchant_key_store.key.get_inner())
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn get_active_payment_attempt(
        &self,
        payment: &mut PaymentIntent,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, StorageError> {
        match payment.active_attempt.clone() {
            RemoteStorageObject::ForeignID(attempt_id) => {
                let conn = pg_connection_read(self).await?;

                let pa = DieselPaymentAttempt::find_by_merchant_id_attempt_id(
                    &conn,
                    payment.merchant_id.as_str(),
                    attempt_id.as_str(),
                )
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(er.current_context());
                    er.change_context(new_err)
                })
                .map(PaymentAttempt::from_storage_model)?;
                payment.active_attempt = RemoteStorageObject::Object(pa.clone());
                Ok(pa)
            }
            RemoteStorageObject::Object(pa) => Ok(pa.clone()),
        }
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        self.router_store
            .filter_payment_intent_by_constraints(
                merchant_id,
                filters,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        self.router_store
            .filter_payment_intents_by_time_range_constraints(
                merchant_id,
                time_range,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        filters: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        self.router_store
            .get_filtered_payment_intents_attempt(
                merchant_id,
                filters,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
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
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for crate::RouterStore<T> {
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
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })?;

        PaymentIntent::convert_back(diesel_payment_intent, merchant_key_store.key.get_inner())
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
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
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })?;

        PaymentIntent::convert_back(diesel_payment_intent, merchant_key_store.key.get_inner())
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .async_and_then(|diesel_payment_intent| async {
                PaymentIntent::convert_back(
                    diesel_payment_intent,
                    merchant_key_store.key.get_inner(),
                )
                .await
                .change_context(StorageError::DecryptionError)
            })
            .await
    }

    #[instrument(skip_all)]
    async fn get_active_payment_attempt(
        &self,
        payment: &mut PaymentIntent,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, StorageError> {
        match &payment.active_attempt {
            RemoteStorageObject::ForeignID(attempt_id) => {
                let conn = pg_connection_read(self).await?;

                let pa = DieselPaymentAttempt::find_by_merchant_id_attempt_id(
                    &conn,
                    payment.merchant_id.as_str(),
                    attempt_id.as_str(),
                )
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(er.current_context());
                    er.change_context(new_err)
                })
                .map(PaymentAttempt::from_storage_model)?;
                payment.active_attempt = RemoteStorageObject::Object(pa.clone());
                Ok(pa)
            }
            RemoteStorageObject::Object(pa) => Ok(pa.clone()),
        }
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
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
                    query = query.filter(pi_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payment_intent_by_payment_id_merchant_id(
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
                    diesel_payment_intent,
                    merchant_key_store.key.get_inner(),
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

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        // TODO: Remove this redundant function
        let payment_filters = (*time_range).into();
        self.filter_payment_intent_by_constraints(
            merchant_id,
            &payment_filters,
            merchant_key_store,
            storage_scheme,
        )
        .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        use futures::{future::try_join_all, FutureExt};

        let conn = connection::pg_connection_read(self).await.switch()?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPaymentIntent::table()
            .inner_join(
                diesel_models::schema::payment_attempt::table
                    .on(pa_dsl::attempt_id.eq(pi_dsl::active_attempt_id)),
            )
            .filter(pi_dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(pi_dsl::created_at.desc())
            .into_boxed();

        query = match constraints {
            PaymentIntentFetchConstraints::Single { payment_intent_id } => {
                query.filter(pi_dsl::payment_id.eq(payment_intent_id.to_owned()))
            }
            PaymentIntentFetchConstraints::List(params) => {
                if let Some(limit) = params.limit {
                    query = query.limit(limit.into());
                }

                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(pi_dsl::customer_id.eq(customer_id.clone()));
                }

                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(pi_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(pi_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payment_intent_by_payment_id_merchant_id(
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

                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        query
            .get_results_async::<(DieselPaymentIntent, DieselPaymentAttempt)>(conn)
            .await
            .map(|results| {
                try_join_all(results.into_iter().map(|(pi, pa)| {
                    PaymentIntent::convert_back(pi, merchant_key_store.key.get_inner()).map(
                        |payment_intent| {
                            payment_intent.map(|payment_intent| {
                                (payment_intent, PaymentAttempt::from_storage_model(pa))
                            })
                        },
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

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
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
