#[cfg(feature = "olap")]
use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::{date_time, ext_traits::Encode};
#[cfg(feature = "olap")]
use data_models::payments::{
    payment_attempt::PaymentAttempt, payment_intent::PaymentIntentFetchConstraints,
};
use data_models::{
    errors::StorageError,
    payments::payment_intent::{
        PaymentIntent, PaymentIntentInterface, PaymentIntentNew, PaymentIntentUpdate,
    },
    MerchantStorageScheme,
};
#[cfg(feature = "olap")]
use diesel::{associations::HasTable, ExpressionMethods, JoinOnDsl, QueryDsl};
#[cfg(feature = "olap")]
use diesel_models::query::generics::db_metrics;
use diesel_models::{
    kv,
    payment_intent::{
        PaymentIntent as DieselPaymentIntent, PaymentIntentNew as DieselPaymentIntentNew,
        PaymentIntentUpdate as DieselPaymentIntentUpdate,
    },
};
#[cfg(feature = "olap")]
use diesel_models::{
    payment_attempt::PaymentAttempt as DieselPaymentAttempt,
    schema::{payment_attempt::dsl as pa_dsl, payment_intent::dsl as pi_dsl},
};
use error_stack::{IntoReport, ResultExt};
use redis_interface::HsetnxReply;
#[cfg(feature = "olap")]
use router_env::logger;
use router_env::{instrument, tracing};

use crate::{
    redis::kv_store::{PartitionKey, RedisConnInterface},
    utils::{pg_connection_read, pg_connection_write},
    DataModelExt, DatabaseStore, KVRouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for KVRouterStore<T> {
    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payment_intent(new, storage_scheme)
                    .await
            }

            MerchantStorageScheme::RedisKv => {
                let key = format!("{}_{}", new.merchant_id, new.payment_id);
                let field = format!("pi_{}", new.payment_id);
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
                    profile_id: new.profile_id.clone(),
                    merchant_decision: new.merchant_decision.clone(),
                    payment_confirm_source: new.payment_confirm_source,
                };

                match self
                    .get_redis_conn()
                    .change_context(StorageError::DatabaseConnectionError)?
                    .serialize_and_set_hash_field_if_not_exist(&key, &field, &created_intent)
                    .await
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(StorageError::DuplicateValue {
                        entity: "payment_intent",
                        key: Some(key),
                    })
                    .into_report(),
                    Ok(HsetnxReply::KeySet) => {
                        let redis_entry = kv::TypedSql {
                            op: kv::DBOperation::Insert {
                                insertable: kv::Insertable::PaymentIntent(new.to_storage_model()),
                            },
                        };
                        self.push_to_drainer_stream::<DieselPaymentIntent>(
                            redis_entry,
                            PartitionKey::MerchantIdPaymentId {
                                merchant_id: &created_intent.merchant_id,
                                payment_id: &created_intent.payment_id,
                            },
                        )
                        .await
                        .change_context(StorageError::KVError)?;
                        Ok(created_intent)
                    }
                    Err(error) => Err(error.change_context(StorageError::KVError)),
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payment_intent(this, payment_intent, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key = format!("{}_{}", this.merchant_id, this.payment_id);
                let field = format!("pi_{}", this.payment_id);

                let updated_intent = payment_intent.clone().apply_changeset(this.clone());
                // Check for database presence as well Maybe use a read replica here ?

                let redis_value =
                    Encode::<PaymentIntent>::encode_to_string_of_json(&updated_intent)
                        .change_context(StorageError::SerializationFailed)?;

                let updated_intent = self
                    .get_redis_conn()
                    .change_context(StorageError::DatabaseConnectionError)?
                    .set_hash_fields(&key, (&field, &redis_value))
                    .await
                    .map(|_| updated_intent)
                    .change_context(StorageError::KVError)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: kv::Updateable::PaymentIntentUpdate(
                            kv::PaymentIntentUpdateMems {
                                orig: this.to_storage_model(),
                                update_data: payment_intent.to_storage_model(),
                            },
                        ),
                    },
                };

                self.push_to_drainer_stream::<DieselPaymentIntent>(
                    redis_entry,
                    PartitionKey::MerchantIdPaymentId {
                        merchant_id: &updated_intent.merchant_id,
                        payment_id: &updated_intent.payment_id,
                    },
                )
                .await
                .change_context(StorageError::KVError)?;
                Ok(updated_intent)
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let database_call = || async {
            self.router_store
                .find_payment_intent_by_payment_id_merchant_id(
                    payment_id,
                    merchant_id,
                    storage_scheme,
                )
                .await
        };
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,

            MerchantStorageScheme::RedisKv => {
                let key = format!("{merchant_id}_{payment_id}");
                let field = format!("pi_{payment_id}");
                crate::utils::try_redis_get_else_try_database_get(
                    self.get_redis_conn()
                        .change_context(StorageError::DatabaseConnectionError)?
                        .get_hash_field_and_deserialize(&key, &field, "PaymentIntent"),
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
        filters: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .filter_payment_intent_by_constraints(merchant_id, filters, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => Err(StorageError::KVError.into()),
        }
    }
    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .filter_payment_intents_by_time_range_constraints(
                        merchant_id,
                        time_range,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => Err(StorageError::KVError.into()),
        }
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        filters: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .get_filtered_payment_intents_attempt(merchant_id, filters, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => Err(StorageError::KVError.into()),
        }
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .get_filtered_active_attempt_ids_for_total_count(
                        merchant_id,
                        constraints,
                        storage_scheme,
                    )
                    .await
            }

            MerchantStorageScheme::RedisKv => Err(StorageError::KVError.into()),
        }
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for crate::RouterStore<T> {
    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        new.to_storage_model()
            .insert(&conn)
            .await
            .map_err(|er| {
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentIntent::from_storage_model)
    }

    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        this.to_storage_model()
            .update(&conn, payment_intent.to_storage_model())
            .await
            .map_err(|er| {
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentIntent::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
            .await
            .map(PaymentIntent::from_storage_model)
            .map_err(|er| {
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        merchant_id: &str,
        filters: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        let conn = self.get_replica_pool();

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
            payment_intents
                .into_iter()
                .map(PaymentIntent::from_storage_model)
                .collect::<Vec<PaymentIntent>>()
        })
        .into_report()
        .map_err(|er| {
            let new_err = StorageError::DatabaseError(format!("{er:?}"));
            er.change_context(new_err)
        })
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        time_range: &api_models::payments::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        // TODO: Remove this redundant function
        let payment_filters = (*time_range).into();
        self.filter_payment_intent_by_constraints(merchant_id, &payment_filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_payment_intents_attempt(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        let conn = self.get_replica_pool();

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
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(pi_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                query = query.offset(params.offset.into());

                if let Some(currency) = &params.currency {
                    query = query.filter(pi_dsl::currency.eq_any(currency.clone()));
                }

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

                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        query
            .get_results_async::<(DieselPaymentIntent, DieselPaymentAttempt)>(conn)
            .await
            .map(|results| {
                results
                    .into_iter()
                    .map(|(pi, pa)| {
                        (
                            PaymentIntent::from_storage_model(pi),
                            PaymentAttempt::from_storage_model(pa),
                        )
                    })
                    .collect()
            })
            .into_report()
            .map_err(|er| {
                let new_er = StorageError::DatabaseError(format!("{er:?}"));
                er.change_context(new_er)
            })
            .attach_printable("Error filtering payment records")
    }

    #[cfg(feature = "olap")]
    async fn get_filtered_active_attempt_ids_for_total_count(
        &self,
        merchant_id: &str,
        constraints: &PaymentIntentFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        let conn = self.get_replica_pool();

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
        .into_report()
        .map_err(|er| {
            let new_err = StorageError::DatabaseError(format!("{er:?}"));
            er.change_context(new_err)
        })
        .attach_printable_lazy(|| "Error filtering records by predicate")
    }
}

impl DataModelExt for PaymentIntentNew {
    type StorageModel = DieselPaymentIntentNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentIntentNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
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
            active_attempt_id: self.active_attempt_id,
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_confirm_source: self.payment_confirm_source,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            amount_captured: storage_model.amount_captured,
            customer_id: storage_model.customer_id,
            description: storage_model.description,
            return_url: storage_model.return_url,
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
            active_attempt_id: storage_model.active_attempt_id,
            business_country: storage_model.business_country,
            business_label: storage_model.business_label,
            order_details: storage_model.order_details,
            allowed_payment_method_types: storage_model.allowed_payment_method_types,
            connector_metadata: storage_model.connector_metadata,
            feature_metadata: storage_model.feature_metadata,
            attempt_count: storage_model.attempt_count,
            profile_id: storage_model.profile_id,
            merchant_decision: storage_model.merchant_decision,
            payment_confirm_source: storage_model.payment_confirm_source,
        }
    }
}

impl DataModelExt for PaymentIntent {
    type StorageModel = DieselPaymentIntent;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentIntent {
            id: self.id,
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
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
            active_attempt_id: self.active_attempt_id,
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
            profile_id: self.profile_id,
            merchant_decision: self.merchant_decision,
            payment_confirm_source: self.payment_confirm_source,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            amount_captured: storage_model.amount_captured,
            customer_id: storage_model.customer_id,
            description: storage_model.description,
            return_url: storage_model.return_url,
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
            active_attempt_id: storage_model.active_attempt_id,
            business_country: storage_model.business_country,
            business_label: storage_model.business_label,
            order_details: storage_model.order_details,
            allowed_payment_method_types: storage_model.allowed_payment_method_types,
            connector_metadata: storage_model.connector_metadata,
            feature_metadata: storage_model.feature_metadata,
            attempt_count: storage_model.attempt_count,
            profile_id: storage_model.profile_id,
            merchant_decision: storage_model.merchant_decision,
            payment_confirm_source: storage_model.payment_confirm_source,
        }
    }
}

impl DataModelExt for PaymentIntentUpdate {
    type StorageModel = DieselPaymentIntentUpdate;

    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::ResponseUpdate {
                status,
                amount_captured,
                return_url,
            } => DieselPaymentIntentUpdate::ResponseUpdate {
                status,
                amount_captured,
                return_url,
            },
            Self::MetadataUpdate { metadata } => {
                DieselPaymentIntentUpdate::MetadataUpdate { metadata }
            }
            Self::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => DieselPaymentIntentUpdate::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            },
            Self::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            } => DieselPaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            },
            Self::PGStatusUpdate { status } => DieselPaymentIntentUpdate::PGStatusUpdate { status },
            Self::Update {
                amount,
                currency,
                setup_future_usage,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                return_url,
                business_country,
                business_label,
                description,
                statement_descriptor_name,
                statement_descriptor_suffix,
                order_details,
                metadata,
                payment_confirm_source,
            } => DieselPaymentIntentUpdate::Update {
                amount,
                currency,
                setup_future_usage,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                return_url,
                business_country,
                business_label,
                description,
                statement_descriptor_name,
                statement_descriptor_suffix,
                order_details,
                metadata,
                payment_confirm_source,
            },
            Self::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
            } => DieselPaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
            },
            Self::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
            } => DieselPaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
            },
            Self::ApproveUpdate { merchant_decision } => {
                DieselPaymentIntentUpdate::ApproveUpdate { merchant_decision }
            }
            Self::RejectUpdate {
                status,
                merchant_decision,
            } => DieselPaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
            },
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        match storage_model {
            DieselPaymentIntentUpdate::ResponseUpdate {
                status,
                amount_captured,
                return_url,
            } => Self::ResponseUpdate {
                status,
                amount_captured,
                return_url,
            },
            DieselPaymentIntentUpdate::MetadataUpdate { metadata } => {
                Self::MetadataUpdate { metadata }
            }
            DieselPaymentIntentUpdate::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            } => Self::ReturnUrlUpdate {
                return_url,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
            },
            DieselPaymentIntentUpdate::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            } => Self::MerchantStatusUpdate {
                status,
                shipping_address_id,
                billing_address_id,
            },
            DieselPaymentIntentUpdate::PGStatusUpdate { status } => Self::PGStatusUpdate { status },
            DieselPaymentIntentUpdate::Update {
                amount,
                currency,
                setup_future_usage,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                return_url,
                business_country,
                business_label,
                description,
                statement_descriptor_name,
                statement_descriptor_suffix,
                order_details,
                metadata,
                payment_confirm_source,
            } => Self::Update {
                amount,
                currency,
                setup_future_usage,
                status,
                customer_id,
                shipping_address_id,
                billing_address_id,
                return_url,
                business_country,
                business_label,
                description,
                statement_descriptor_name,
                statement_descriptor_suffix,
                order_details,
                metadata,
                payment_confirm_source,
            },
            DieselPaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
            } => Self::PaymentAttemptAndAttemptCountUpdate {
                active_attempt_id,
                attempt_count,
            },
            DieselPaymentIntentUpdate::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
            } => Self::StatusAndAttemptUpdate {
                status,
                active_attempt_id,
                attempt_count,
            },
            DieselPaymentIntentUpdate::ApproveUpdate { merchant_decision } => {
                Self::ApproveUpdate { merchant_decision }
            }
            DieselPaymentIntentUpdate::RejectUpdate {
                status,
                merchant_decision,
            } => Self::RejectUpdate {
                status,
                merchant_decision,
            },
        }
    }
}
