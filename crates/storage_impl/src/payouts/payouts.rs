#[cfg(feature = "olap")]
use api_models::enums::PayoutConnectors;
#[cfg(feature = "olap")]
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
#[cfg(feature = "olap")]
use common_utils::errors::ReportSwitchExt;
use common_utils::ext_traits::Encode;
#[cfg(all(
    feature = "olap",
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2")
))]
use diesel::JoinOnDsl;
#[cfg(feature = "olap")]
use diesel::{associations::HasTable, ExpressionMethods, NullableExpressionMethods, QueryDsl};
#[cfg(all(
    feature = "olap",
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2")
))]
use diesel_models::payout_attempt::PayoutAttempt as DieselPayoutAttempt;
#[cfg(all(
    feature = "olap",
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2")
))]
use diesel_models::schema::{
    address::dsl as add_dsl, customers::dsl as cust_dsl, payout_attempt::dsl as poa_dsl,
};
#[cfg(feature = "olap")]
use diesel_models::{
    address::Address as DieselAddress, customers::Customer as DieselCustomer,
    enums as storage_enums, query::generics::db_metrics, schema::payouts::dsl as po_dsl,
};
use diesel_models::{
    enums::MerchantStorageScheme,
    kv,
    payouts::{
        Payouts as DieselPayouts, PayoutsNew as DieselPayoutsNew,
        PayoutsUpdate as DieselPayoutsUpdate,
    },
};
use error_stack::ResultExt;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::payouts::PayoutFetchConstraints;
use hyperswitch_domain_models::{
    errors::StorageError,
    payouts::{
        payout_attempt::PayoutAttempt,
        payouts::{Payouts, PayoutsInterface, PayoutsNew, PayoutsUpdate},
    },
};
use redis_interface::HsetnxReply;
#[cfg(feature = "olap")]
use router_env::logger;
use router_env::{instrument, tracing};

#[cfg(feature = "olap")]
use crate::connection;
#[cfg(all(
    feature = "olap",
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2")
))]
use crate::store::schema::{
    address::all_columns as addr_all_columns, customers::all_columns as cust_all_columns,
    payout_attempt::all_columns as poa_all_columns, payouts::all_columns as po_all_columns,
};
use crate::{
    diesel_error_to_data_error,
    errors::RedisErrorExt,
    kv_router_store::KVRouterStore,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_write},
    DataModelExt, DatabaseStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PayoutsInterface for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn insert_payout(
        &self,
        new: PayoutsNew,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayouts>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store.insert_payout(new, storage_scheme).await
            }
            MerchantStorageScheme::RedisKv => {
                let merchant_id = new.merchant_id.clone();
                let payout_id = new.payout_id.clone();
                let key = PartitionKey::MerchantIdPayoutId {
                    merchant_id: &merchant_id,
                    payout_id: &payout_id,
                };
                let key_str = key.to_string();
                let field = format!("po_{}", new.payout_id);
                let created_payout = Payouts {
                    payout_id: new.payout_id.clone(),
                    merchant_id: new.merchant_id.clone(),
                    customer_id: new.customer_id.clone(),
                    address_id: new.address_id.clone(),
                    payout_type: new.payout_type,
                    payout_method_id: new.payout_method_id.clone(),
                    amount: new.amount,
                    destination_currency: new.destination_currency,
                    source_currency: new.source_currency,
                    description: new.description.clone(),
                    recurring: new.recurring,
                    auto_fulfill: new.auto_fulfill,
                    return_url: new.return_url.clone(),
                    entity_type: new.entity_type,
                    metadata: new.metadata.clone(),
                    created_at: new.created_at,
                    last_modified_at: new.last_modified_at,
                    profile_id: new.profile_id.clone(),
                    status: new.status,
                    attempt_count: new.attempt_count,
                    confirm: new.confirm,
                    payout_link_id: new.payout_link_id.clone(),
                    client_secret: new.client_secret.clone(),
                    priority: new.priority,
                };

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: Box::new(kv::Insertable::Payouts(new.to_storage_model())),
                    },
                };

                match Box::pin(kv_wrapper::<DieselPayouts, _, _>(
                    self,
                    KvOperation::<DieselPayouts>::HSetNx(
                        &field,
                        &created_payout.clone().to_storage_model(),
                        redis_entry,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(StorageError::DuplicateValue {
                        entity: "payouts",
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(created_payout),
                    Err(error) => Err(error.change_context(StorageError::KVError)),
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn update_payout(
        &self,
        this: &Payouts,
        payout_update: PayoutsUpdate,
        payout_attempt: &PayoutAttempt,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let key = PartitionKey::MerchantIdPayoutId {
            merchant_id: &this.merchant_id,
            payout_id: &this.payout_id,
        };
        let field = format!("po_{}", this.payout_id);
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayouts>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, None),
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payout(this, payout_update, payout_attempt, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();

                let diesel_payout_update = payout_update.to_storage_model();
                let origin_diesel_payout = this.clone().to_storage_model();

                let diesel_payout = diesel_payout_update
                    .clone()
                    .apply_changeset(origin_diesel_payout.clone());
                // Check for database presence as well Maybe use a read replica here ?

                let redis_value = diesel_payout
                    .encode_to_string_of_json()
                    .change_context(StorageError::SerializationFailed)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: Box::new(kv::Updateable::PayoutsUpdate(kv::PayoutsUpdateMems {
                            orig: origin_diesel_payout,
                            update_data: diesel_payout_update,
                        })),
                    },
                };

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<DieselPayouts>::Hset((&field, redis_value), redis_entry),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(StorageError::KVError)?;

                Ok(Payouts::from_storage_model(diesel_payout))
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselPayouts::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(*er.current_context());
                    er.change_context(new_err)
                })
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayouts>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPayoutId {
                    merchant_id,
                    payout_id,
                };
                let field = format!("po_{payout_id}");
                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<DieselPayouts, _, _>(
                            self,
                            KvOperation::<DieselPayouts>::HGet(&field),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await
            }
        }
        .map(Payouts::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Option<Payouts>, StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselPayouts::find_optional_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(*er.current_context());
                    er.change_context(new_err)
                })
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayouts>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                let maybe_payouts = database_call().await?;
                Ok(maybe_payouts.and_then(|payout| {
                    if payout.payout_id == payout_id {
                        Some(payout)
                    } else {
                        None
                    }
                }))
            }
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPayoutId {
                    merchant_id,
                    payout_id,
                };
                let field = format!("po_{payout_id}");
                Box::pin(utils::try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<DieselPayouts, _, _>(
                            self,
                            KvOperation::<DieselPayouts>::HGet(&field),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                        .map(Some)
                    },
                    database_call,
                ))
                .await
            }
        }
        .map(|payout| payout.map(Payouts::from_storage_model))
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        self.router_store
            .filter_payouts_by_constraints(merchant_id, filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_and_attempts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<
        Vec<(
            Payouts,
            PayoutAttempt,
            Option<DieselCustomer>,
            Option<DieselAddress>,
        )>,
        StorageError,
    > {
        self.router_store
            .filter_payouts_and_attempts(merchant_id, filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    #[instrument[skip_all]]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        self.router_store
            .filter_payouts_by_time_range_constraints(merchant_id, time_range, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_filtered_payouts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        active_payout_ids: &[String],
        connector: Option<Vec<PayoutConnectors>>,
        currency: Option<Vec<storage_enums::Currency>>,
        status: Option<Vec<storage_enums::PayoutStatus>>,
        payout_method: Option<Vec<storage_enums::PayoutType>>,
    ) -> error_stack::Result<i64, StorageError> {
        self.router_store
            .get_total_count_of_filtered_payouts(
                merchant_id,
                active_payout_ids,
                connector,
                currency,
                status,
                payout_method,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PayoutFetchConstraints,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        self.router_store
            .filter_active_payout_ids_by_constraints(merchant_id, constraints)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PayoutsInterface for crate::RouterStore<T> {
    #[instrument(skip_all)]
    async fn insert_payout(
        &self,
        new: PayoutsNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let conn = pg_connection_write(self).await?;
        new.to_storage_model()
            .insert(&conn)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(Payouts::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn update_payout(
        &self,
        this: &Payouts,
        payout: PayoutsUpdate,
        _payout_attempt: &PayoutAttempt,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let conn = pg_connection_write(self).await?;
        this.clone()
            .to_storage_model()
            .update(&conn, payout.to_storage_model())
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(Payouts::from_storage_model)
    }

    #[instrument(skip_all)]
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPayouts::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
            .await
            .map(Payouts::from_storage_model)
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
    }

    #[instrument(skip_all)]
    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Option<Payouts>, StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPayouts::find_optional_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
            .await
            .map(|x| x.map(Payouts::from_storage_model))
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        let conn = connection::pg_connection_read(self).await.switch()?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);

        //[#350]: Replace this with Boxable Expression and pass it into generic filter
        // when https://github.com/rust-lang/rust/issues/52662 becomes stable
        let mut query = <DieselPayouts as HasTable>::table()
            .filter(po_dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(po_dsl::created_at.desc())
            .into_boxed();

        match filters {
            PayoutFetchConstraints::Single { payout_id } => {
                query = query.filter(po_dsl::payout_id.eq(payout_id.to_owned()));
            }
            PayoutFetchConstraints::List(params) => {
                if let Some(limit) = params.limit {
                    query = query.limit(limit.into());
                }

                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(po_dsl::customer_id.eq(customer_id.clone()));
                }
                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(po_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(po_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                starting_after_id,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, &params.ending_before_id) {
                    (Some(ending_at), _) => query.filter(po_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let ending_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                ending_before_id,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                query = query.offset(params.offset.into());

                if let Some(currency) = &params.currency {
                    query = query.filter(po_dsl::destination_currency.eq_any(currency.clone()));
                }

                if let Some(status) = &params.status {
                    query = query.filter(po_dsl::status.eq_any(status.clone()));
                }
            }
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        db_metrics::track_database_call::<<DieselPayouts as HasTable>::Table, _, _>(
            query.get_results_async::<DieselPayouts>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map(|payouts| {
            payouts
                .into_iter()
                .map(Payouts::from_storage_model)
                .collect::<Vec<Payouts>>()
        })
        .map_err(|er| {
            StorageError::DatabaseError(
                error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                    .attach_printable("Error filtering payout records"),
            )
            .into()
        })
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        feature = "olap",
        not(feature = "customer_v2")
    ))]
    #[instrument(skip_all)]
    async fn filter_payouts_and_attempts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<
        Vec<(
            Payouts,
            PayoutAttempt,
            Option<DieselCustomer>,
            Option<DieselAddress>,
        )>,
        StorageError,
    > {
        use common_utils::errors::ReportSwitchExt;

        let conn = connection::pg_connection_read(self).await.switch()?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPayouts::table()
            .inner_join(
                diesel_models::schema::payout_attempt::table
                    .on(poa_dsl::payout_id.eq(po_dsl::payout_id)),
            )
            .left_join(
                diesel_models::schema::customers::table
                    .on(cust_dsl::customer_id.nullable().eq(po_dsl::customer_id)),
            )
            .filter(cust_dsl::merchant_id.eq(merchant_id.to_owned()))
            .left_outer_join(
                diesel_models::schema::address::table
                    .on(add_dsl::address_id.nullable().eq(po_dsl::address_id)),
            )
            .filter(po_dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(po_dsl::created_at.desc())
            .into_boxed();

        query = match filters {
            PayoutFetchConstraints::Single { payout_id } => {
                query.filter(po_dsl::payout_id.eq(payout_id.to_owned()))
            }
            PayoutFetchConstraints::List(params) => {
                if let Some(limit) = params.limit {
                    query = query.limit(limit.into());
                }

                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(po_dsl::customer_id.eq(customer_id.clone()));
                }

                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(po_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match (params.starting_at, &params.starting_after_id) {
                    (Some(starting_at), _) => query.filter(po_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                starting_after_id,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, &params.ending_before_id) {
                    (Some(ending_at), _) => query.filter(po_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let ending_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                ending_before_id,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                query = query.offset(params.offset.into());

                if let Some(currency) = &params.currency {
                    query = query.filter(po_dsl::destination_currency.eq_any(currency.clone()));
                }

                let connectors = params
                    .connector
                    .as_ref()
                    .map(|c| c.iter().map(|c| c.to_string()).collect::<Vec<String>>());

                query = match connectors {
                    Some(connectors) => query.filter(poa_dsl::connector.eq_any(connectors)),
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(po_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                query = match &params.payout_method {
                    Some(payout_method) => {
                        query.filter(po_dsl::payout_type.eq_any(payout_method.clone()))
                    }
                    None => query,
                };

                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        query
            .select((
                po_all_columns,
                poa_all_columns,
                cust_all_columns.nullable(),
                addr_all_columns.nullable(),
            ))
            .get_results_async::<(
                DieselPayouts,
                DieselPayoutAttempt,
                Option<DieselCustomer>,
                Option<DieselAddress>,
            )>(conn)
            .await
            .map(|results| {
                results
                    .into_iter()
                    .map(|(pi, pa, c, add)| {
                        (
                            Payouts::from_storage_model(pi),
                            PayoutAttempt::from_storage_model(pa),
                            c,
                            add,
                        )
                    })
                    .collect()
            })
            .map_err(|er| {
                StorageError::DatabaseError(
                    error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                        .attach_printable("Error filtering payout records"),
                )
                .into()
            })
    }

    #[cfg(all(feature = "olap", feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn filter_payouts_and_attempts(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _filters: &PayoutFetchConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<
        Vec<(
            Payouts,
            PayoutAttempt,
            Option<DieselCustomer>,
            Option<DieselAddress>,
        )>,
        StorageError,
    > {
        todo!()
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        let payout_filters = (*time_range).into();
        self.filter_payouts_by_constraints(merchant_id, &payout_filters, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn get_total_count_of_filtered_payouts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        active_payout_ids: &[String],
        connector: Option<Vec<PayoutConnectors>>,
        currency: Option<Vec<storage_enums::Currency>>,
        status: Option<Vec<storage_enums::PayoutStatus>>,
        payout_type: Option<Vec<storage_enums::PayoutType>>,
    ) -> error_stack::Result<i64, StorageError> {
        let conn = self
            .db_store
            .get_replica_pool()
            .get()
            .await
            .change_context(StorageError::DatabaseConnectionError)?;
        let connector_strings = connector.as_ref().map(|connectors| {
            connectors
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
        });
        DieselPayouts::get_total_count_of_payouts(
            &conn,
            merchant_id,
            active_payout_ids,
            connector_strings,
            currency,
            status,
            payout_type,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        feature = "olap",
        not(feature = "customer_v2")
    ))]
    #[instrument(skip_all)]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PayoutFetchConstraints,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        let conn = connection::pg_connection_read(self).await.switch()?;
        let conn = async_bb8_diesel::Connection::as_async_conn(&conn);
        let mut query = DieselPayouts::table()
            .inner_join(
                diesel_models::schema::payout_attempt::table
                    .on(poa_dsl::payout_id.eq(po_dsl::payout_id)),
            )
            .left_join(
                diesel_models::schema::customers::table
                    .on(cust_dsl::customer_id.nullable().eq(po_dsl::customer_id)),
            )
            .select(po_dsl::payout_id)
            .filter(cust_dsl::merchant_id.eq(merchant_id.to_owned()))
            .filter(po_dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(po_dsl::created_at.desc())
            .into_boxed();

        query = match constraints {
            PayoutFetchConstraints::Single { payout_id } => {
                query.filter(po_dsl::payout_id.eq(payout_id.to_owned()))
            }
            PayoutFetchConstraints::List(params) => {
                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(po_dsl::customer_id.eq(customer_id.clone()));
                }
                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(po_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match params.starting_at {
                    Some(starting_at) => query.filter(po_dsl::created_at.ge(starting_at)),
                    None => query,
                };

                query = match params.ending_at {
                    Some(ending_at) => query.filter(po_dsl::created_at.le(ending_at)),
                    None => query,
                };

                query = match &params.currency {
                    Some(currency) => {
                        query.filter(po_dsl::destination_currency.eq_any(currency.clone()))
                    }
                    None => query,
                };

                query = match &params.status {
                    Some(status) => query.filter(po_dsl::status.eq_any(status.clone())),
                    None => query,
                };

                query
            }
        };

        logger::debug!(filter = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());

        db_metrics::track_database_call::<<DieselPayouts as HasTable>::Table, _, _>(
            query.get_results_async::<String>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|er| {
            StorageError::DatabaseError(
                error_stack::report!(diesel_models::errors::DatabaseError::from(er))
                    .attach_printable("Error filtering payout records"),
            )
            .into()
        })
    }

    #[cfg(all(feature = "olap", feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PayoutFetchConstraints,
    ) -> error_stack::Result<Vec<String>, StorageError> {
        todo!()
    }
}

impl DataModelExt for Payouts {
    type StorageModel = DieselPayouts;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPayouts {
            payout_id: self.payout_id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            address_id: self.address_id,
            payout_type: self.payout_type,
            payout_method_id: self.payout_method_id,
            amount: self.amount,
            destination_currency: self.destination_currency,
            source_currency: self.source_currency,
            description: self.description,
            recurring: self.recurring,
            auto_fulfill: self.auto_fulfill,
            return_url: self.return_url,
            entity_type: self.entity_type,
            metadata: self.metadata,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            status: self.status,
            attempt_count: self.attempt_count,
            confirm: self.confirm,
            payout_link_id: self.payout_link_id,
            client_secret: self.client_secret,
            priority: self.priority,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            payout_id: storage_model.payout_id,
            merchant_id: storage_model.merchant_id,
            customer_id: storage_model.customer_id,
            address_id: storage_model.address_id,
            payout_type: storage_model.payout_type,
            payout_method_id: storage_model.payout_method_id,
            amount: storage_model.amount,
            destination_currency: storage_model.destination_currency,
            source_currency: storage_model.source_currency,
            description: storage_model.description,
            recurring: storage_model.recurring,
            auto_fulfill: storage_model.auto_fulfill,
            return_url: storage_model.return_url,
            entity_type: storage_model.entity_type,
            metadata: storage_model.metadata,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
            profile_id: storage_model.profile_id,
            status: storage_model.status,
            attempt_count: storage_model.attempt_count,
            confirm: storage_model.confirm,
            payout_link_id: storage_model.payout_link_id,
            client_secret: storage_model.client_secret,
            priority: storage_model.priority,
        }
    }
}
impl DataModelExt for PayoutsNew {
    type StorageModel = DieselPayoutsNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPayoutsNew {
            payout_id: self.payout_id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            address_id: self.address_id,
            payout_type: self.payout_type,
            payout_method_id: self.payout_method_id,
            amount: self.amount,
            destination_currency: self.destination_currency,
            source_currency: self.source_currency,
            description: self.description,
            recurring: self.recurring,
            auto_fulfill: self.auto_fulfill,
            return_url: self.return_url,
            entity_type: self.entity_type,
            metadata: self.metadata,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            status: self.status,
            attempt_count: self.attempt_count,
            confirm: self.confirm,
            payout_link_id: self.payout_link_id,
            client_secret: self.client_secret,
            priority: self.priority,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            payout_id: storage_model.payout_id,
            merchant_id: storage_model.merchant_id,
            customer_id: storage_model.customer_id,
            address_id: storage_model.address_id,
            payout_type: storage_model.payout_type,
            payout_method_id: storage_model.payout_method_id,
            amount: storage_model.amount,
            destination_currency: storage_model.destination_currency,
            source_currency: storage_model.source_currency,
            description: storage_model.description,
            recurring: storage_model.recurring,
            auto_fulfill: storage_model.auto_fulfill,
            return_url: storage_model.return_url,
            entity_type: storage_model.entity_type,
            metadata: storage_model.metadata,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
            profile_id: storage_model.profile_id,
            status: storage_model.status,
            attempt_count: storage_model.attempt_count,
            confirm: storage_model.confirm,
            payout_link_id: storage_model.payout_link_id,
            client_secret: storage_model.client_secret,
            priority: storage_model.priority,
        }
    }
}
impl DataModelExt for PayoutsUpdate {
    type StorageModel = DieselPayoutsUpdate;
    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::Update {
                amount,
                destination_currency,
                source_currency,
                description,
                recurring,
                auto_fulfill,
                return_url,
                entity_type,
                metadata,
                profile_id,
                status,
                confirm,
                payout_type,
                address_id,
                customer_id,
            } => DieselPayoutsUpdate::Update {
                amount,
                destination_currency,
                source_currency,
                description,
                recurring,
                auto_fulfill,
                return_url,
                entity_type,
                metadata,
                profile_id,
                status,
                confirm,
                payout_type,
                address_id,
                customer_id,
            },
            Self::PayoutMethodIdUpdate { payout_method_id } => {
                DieselPayoutsUpdate::PayoutMethodIdUpdate { payout_method_id }
            }
            Self::RecurringUpdate { recurring } => {
                DieselPayoutsUpdate::RecurringUpdate { recurring }
            }
            Self::AttemptCountUpdate { attempt_count } => {
                DieselPayoutsUpdate::AttemptCountUpdate { attempt_count }
            }
            Self::StatusUpdate { status } => DieselPayoutsUpdate::StatusUpdate { status },
        }
    }

    #[allow(clippy::todo)]
    fn from_storage_model(_storage_model: Self::StorageModel) -> Self {
        todo!("Reverse map should no longer be needed")
    }
}
