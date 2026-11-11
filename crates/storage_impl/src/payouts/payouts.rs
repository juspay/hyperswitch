#[cfg(feature = "olap")]
use api_models::enums::PayoutConnectors;
use common_utils::{
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::Encode,
    types::keymanager::{Identifier, KeyManagerState},
};
#[cfg(feature = "olap")]
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
#[cfg(all(feature = "v1", feature = "olap"))]
use diesel::{JoinOnDsl, NullableExpressionMethods};
#[cfg(feature = "olap")]
use diesel_models::{
    address::Address as DieselAddress, customers::Customer as DieselCustomer,
    enums as storage_enums, query::generics::db_metrics, schema::payouts::dsl as po_dsl,
};
use diesel_models::{
    enums::MerchantStorageScheme,
    payouts::{
        Payouts as DieselPayouts, PayoutsNew as DieselPayoutsNew,
        PayoutsUpdate as DieselPayoutsUpdate,
    },
};
#[cfg(all(feature = "olap", feature = "v1"))]
use diesel_models::{
    payout_attempt::PayoutAttempt as DieselPayoutAttempt,
    schema::{address::dsl as add_dsl, customers::dsl as cust_dsl, payout_attempt::dsl as poa_dsl},
};
use error_stack::ResultExt;
#[cfg(feature = "olap")]
use hyperswitch_domain_models::payouts::PayoutFetchConstraints;
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    payouts::{
        payout_attempt::PayoutAttempt,
        payouts::{Payouts, PayoutsInterface, PayoutsNew, PayoutsUpdate},
    },
    type_encryption::{crypto_operation, CryptoOperation, EncryptedJsonType},
};
use hyperswitch_masking::{PeekInterface, Secret};
use redis_interface::HsetnxReply;
use router_env::instrument;

#[cfg(feature = "olap")]
use crate::connection;
#[cfg(all(feature = "olap", feature = "v1"))]
use crate::store::schema::{
    address::all_columns as addr_all_columns, customers::all_columns as cust_all_columns,
    payout_attempt::all_columns as poa_all_columns, payouts::all_columns as po_all_columns,
};
use crate::{
    behaviour::{Conversion, ReverseConversion},
    diesel_error_to_data_error,
    errors::{RedisErrorExt, StorageError},
    kv_router_store::KVRouterStore,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{self, pg_connection_read, pg_connection_read_replica, pg_connection_write},
    DataModelExt, DatabaseStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PayoutsInterface for KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_payout(
        &self,
        new: PayoutsNew,
        merchant_key_store: &MerchantKeyStore,
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
                self.router_store
                    .insert_payout(new, merchant_key_store, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let merchant_id = new.merchant_id.clone();
                let payout_id = new.payout_id.clone();
                let key = PartitionKey::MerchantIdPayoutId {
                    merchant_id: &merchant_id,
                    payout_id: &payout_id,
                };
                let key_str = key.to_string();
                let field = format!("po_{}", new.payout_id.get_string_repr());
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
                    funds_transfer_type: new.funds_transfer_type,
                    sender_details: new.sender_details.clone(),
                    created_at: new.created_at,
                    last_modified_at: new.last_modified_at,
                    profile_id: new.profile_id.clone(),
                    status: new.status,
                    attempt_count: new.attempt_count,
                    confirm: new.confirm,
                    payout_link_id: new.payout_link_id.clone(),
                    client_secret: new.client_secret.clone(),
                    priority: new.priority,
                    organization_id: new.organization_id.clone(),
                    processor_merchant_id: new.processor_merchant_id.clone(),
                    created_by: new.created_by.clone(),
                    billing_descriptor: new.billing_descriptor.clone(),
                };

                let mut query_gen_conn = pg_connection_write(self).await?;
                let drainer_query = new
                    .to_storage_model()
                    .generate_drainer_insert_query(&mut query_gen_conn)
                    .await
                    .change_context(StorageError::KVError)
                    .attach_printable("Failed to generate payouts insert query")?;

                match Box::pin(kv_wrapper::<DieselPayouts, _, _>(
                    self,
                    KvOperation::<DieselPayouts>::HSetNx(
                        &field,
                        &created_payout.clone().to_storage_model(),
                        drainer_query,
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
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let key = PartitionKey::MerchantIdPayoutId {
            merchant_id: &this.merchant_id,
            payout_id: &this.payout_id,
        };
        let field = format!("po_{}", this.payout_id.get_string_repr());
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPayouts>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, None),
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payout(
                        this,
                        payout_update,
                        payout_attempt,
                        merchant_key_store,
                        storage_scheme,
                    )
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

                let mut query_gen_conn = pg_connection_write(self).await?;
                let drainer_query = diesel_payout_update
                    .clone()
                    .generate_drainer_update_query(
                        &mut query_gen_conn,
                        this.payout_id.clone(),
                        this.merchant_id.clone(),
                    )
                    .await
                    .change_context(StorageError::KVError)
                    .attach_printable("Failed to generate payouts update query")?;

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<DieselPayouts>::Hset((&field, redis_value), drainer_query),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(StorageError::KVError)?;

                diesel_payout
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
        merchant_key_store: &MerchantKeyStore,
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
        let storage_model = match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPayoutId {
                    merchant_id,
                    payout_id,
                };
                let field = format!("po_{}", payout_id.get_string_repr());
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
        }?;
        storage_model
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                Identifier::Merchant(merchant_key_store.merchant_id.clone()),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
        merchant_key_store: &MerchantKeyStore,
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
        let maybe_payouts = match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                let maybe_payouts = database_call().await?;
                maybe_payouts.filter(|payout| &payout.payout_id == payout_id)
            }
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPayoutId {
                    merchant_id,
                    payout_id,
                };
                let field = format!("po_{}", payout_id.get_string_repr());
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
                .await?
            }
        };
        match maybe_payouts {
            Some(storage_model) => Ok(Some(
                storage_model
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?,
            )),
            None => Ok(None),
        }
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        self.router_store
            .filter_payouts_by_constraints(merchant_id, filters, merchant_key_store, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_and_attempts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
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
            .filter_payouts_and_attempts(merchant_id, filters, merchant_key_store, storage_scheme)
            .await
    }

    #[cfg(feature = "olap")]
    #[instrument[skip_all]]
    async fn filter_payouts_by_time_range_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        time_range: &common_utils::types::TimeRange,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        self.router_store
            .filter_payouts_by_time_range_constraints(
                merchant_id,
                time_range,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_filtered_payouts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        active_payout_ids: &[common_utils::id_type::PayoutId],
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        connector: Option<Vec<PayoutConnectors>>,
        currency: Option<Vec<storage_enums::Currency>>,
        status: Option<Vec<storage_enums::PayoutStatus>>,
        payout_method: Option<Vec<storage_enums::PayoutType>>,
    ) -> error_stack::Result<i64, StorageError> {
        self.router_store
            .get_total_count_of_filtered_payouts(
                merchant_id,
                active_payout_ids,
                profile_id_list,
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
    ) -> error_stack::Result<Vec<common_utils::id_type::PayoutId>, StorageError> {
        self.router_store
            .filter_active_payout_ids_by_constraints(merchant_id, constraints)
            .await
    }

    #[cfg(feature = "olap")]
    async fn get_payout_intent_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::PayoutStatus, i64)>, StorageError> {
        self.router_store
            .get_payout_intent_status_with_count(merchant_id, profile_id_list, time_range)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PayoutsInterface for crate::RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_payout(
        &self,
        new: PayoutsNew,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let conn = pg_connection_write(self).await?;
        let storage_model = new.to_storage_model().insert(&conn).await.map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })?;
        storage_model
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                Identifier::Merchant(merchant_key_store.merchant_id.clone()),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_payout(
        &self,
        this: &Payouts,
        payout: PayoutsUpdate,
        _payout_attempt: &PayoutAttempt,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let conn = pg_connection_write(self).await?;
        let storage_model = this
            .clone()
            .to_storage_model()
            .update(&conn, payout.to_storage_model())
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })?;
        storage_model
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                Identifier::Merchant(merchant_key_store.merchant_id.clone()),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Payouts, StorageError> {
        let conn = pg_connection_read(self).await?;
        let storage_model =
            DieselPayouts::find_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(*er.current_context());
                    er.change_context(new_err)
                })?;
        storage_model
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                Identifier::Merchant(merchant_key_store.merchant_id.clone()),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_optional_payout_by_merchant_id_payout_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payout_id: &common_utils::id_type::PayoutId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Option<Payouts>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let maybe_payouts =
            DieselPayouts::find_optional_by_merchant_id_payout_id(&conn, merchant_id, payout_id)
                .await
                .map_err(|er| {
                    let new_err = diesel_error_to_data_error(*er.current_context());
                    er.change_context(new_err)
                })?;
        match maybe_payouts {
            Some(storage_model) => Ok(Some(
                storage_model
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?,
            )),
            None => Ok(None),
        }
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn filter_payouts_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        //[#350]: Replace this with Boxable Expression and pass it into generic filter
        // when https://github.com/rust-lang/rust/issues/52662 becomes stable
        let mut query = diesel_models::boxed_list_query!(
            DieselPayouts,
            scope = po_dsl::merchant_id.eq(merchant_id.to_owned()),
            order = po_dsl::created_at.desc()
        );

        match filters {
            PayoutFetchConstraints::Single { payout_id } => {
                query = query.filter(po_dsl::payout_id.eq(payout_id.to_owned()));
            }
            PayoutFetchConstraints::List(params) => {
                if let Some(customer_id) = &params.customer_id {
                    query = query.filter(po_dsl::customer_id.eq(customer_id.clone()));
                }
                if let Some(profile_id) = &params.profile_id {
                    query = query.filter(po_dsl::profile_id.eq(profile_id.clone()));
                }

                query = match (params.starting_at, params.starting_after_id.as_ref()) {
                    (Some(starting_at), _) => query.filter(po_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let starting_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                starting_after_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, params.ending_before_id.as_ref()) {
                    (Some(ending_at), _) => query.filter(po_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        // TODO: Fetch partial columns for this query since we only need some columns
                        let ending_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                ending_before_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                if let Some(currency) = &params.currency {
                    query = query.filter(po_dsl::destination_currency.eq_any(currency.clone()));
                }

                if let Some(status) = &params.status {
                    query = query.filter(po_dsl::status.eq_any(status.clone()));
                }

                query = diesel_models::list::apply_pagination(query, params.limit, params.offset);
            }
        }

        let storage_models = conn
            .track_get_results::<<DieselPayouts as HasTable>::Table, _, DieselPayouts>(
                db_metrics::DatabaseOperation::Filter,
                query,
            )
            .await
            .map_err(|er| {
                error_stack::report!(StorageError::from(er))
                    .attach_printable("Error filtering payout records")
            })?;

        let mut payouts = Vec::with_capacity(storage_models.len());
        for storage_model in storage_models.into_iter() {
            payouts.push(
                storage_model
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?,
            );
        }
        Ok(payouts)
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn filter_payouts_and_attempts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        filters: &PayoutFetchConstraints,
        merchant_key_store: &MerchantKeyStore,
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
        let conn = connection::pg_connection_read(self).await?;
        let mut query = diesel_models::list::into_boxed_list(
            DieselPayouts::table()
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
                .order(po_dsl::created_at.desc()),
        );

        query = match filters {
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

                if let Some(merchant_order_reference_id_filter) =
                    &params.merchant_order_reference_id
                {
                    query = query.filter(
                        poa_dsl::merchant_order_reference_id
                            .eq(merchant_order_reference_id_filter.clone()),
                    );
                }

                query = match (params.starting_at, params.starting_after_id.as_ref()) {
                    (Some(starting_at), _) => query.filter(po_dsl::created_at.ge(starting_at)),
                    (None, Some(starting_after_id)) => {
                        let starting_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                starting_after_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.ge(starting_at))
                    }
                    (None, None) => query,
                };

                query = match (params.ending_at, params.ending_before_id.as_ref()) {
                    (Some(ending_at), _) => query.filter(po_dsl::created_at.le(ending_at)),
                    (None, Some(ending_before_id)) => {
                        let ending_at = self
                            .find_payout_by_merchant_id_payout_id(
                                merchant_id,
                                ending_before_id,
                                merchant_key_store,
                                storage_scheme,
                            )
                            .await?
                            .created_at;
                        query.filter(po_dsl::created_at.le(ending_at))
                    }
                    (None, None) => query,
                };

                if let Some(currency) = &params.currency {
                    query = query.filter(po_dsl::destination_currency.eq_any(currency.clone()));
                }

                let connectors = params
                    .connector
                    .as_ref()
                    .map(|c| c.iter().map(|c| c.to_string()).collect::<Vec<String>>());

                query = match connectors {
                    Some(conn_filters) if !conn_filters.is_empty() => {
                        query.filter(poa_dsl::connector.eq_any(conn_filters))
                    }
                    _ => query,
                };

                query = match &params.status {
                    Some(status_filters) if !status_filters.is_empty() => {
                        query.filter(po_dsl::status.eq_any(status_filters.clone()))
                    }
                    _ => query,
                };

                query = match &params.payout_method {
                    Some(payout_method) if !payout_method.is_empty() => {
                        query.filter(po_dsl::payout_type.eq_any(payout_method.clone()))
                    }
                    _ => query,
                };

                diesel_models::list::apply_pagination(query, params.limit, params.offset)
            }
        };

        let results = conn
            .track_get_results::<<DieselPayouts as HasTable>::Table, _, (
                DieselPayouts,
                DieselPayoutAttempt,
                Option<DieselCustomer>,
                Option<DieselAddress>,
            )>(
                db_metrics::DatabaseOperation::Filter,
                query.select((
                    po_all_columns,
                    poa_all_columns,
                    cust_all_columns.nullable(),
                    addr_all_columns.nullable(),
                )),
            )
            .await
            .map_err(|er| {
                error_stack::report!(StorageError::from(er))
                    .attach_printable("Error filtering payout records")
            })?;

        let mut payouts_and_attempts = Vec::with_capacity(results.len());
        for (pi, pa, c, add) in results.into_iter() {
            payouts_and_attempts.push((
                pi.convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                )
                .await
                .change_context(StorageError::DecryptionError)?,
                PayoutAttempt::from_storage_model(pa),
                c,
                add,
            ));
        }
        Ok(payouts_and_attempts)
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    #[instrument(skip_all)]
    async fn filter_payouts_and_attempts(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _filters: &PayoutFetchConstraints,
        _merchant_key_store: &MerchantKeyStore,
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
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<Payouts>, StorageError> {
        let payout_filters = (*time_range).into();
        self.filter_payouts_by_constraints(
            merchant_id,
            &payout_filters,
            merchant_key_store,
            storage_scheme,
        )
        .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn get_total_count_of_filtered_payouts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        active_payout_ids: &[common_utils::id_type::PayoutId],
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        connector: Option<Vec<PayoutConnectors>>,
        currency: Option<Vec<storage_enums::Currency>>,
        status: Option<Vec<storage_enums::PayoutStatus>>,
        payout_type: Option<Vec<storage_enums::PayoutType>>,
    ) -> error_stack::Result<i64, StorageError> {
        let conn = pg_connection_read_replica(self).await?;
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
            profile_id_list,
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

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        constraints: &PayoutFetchConstraints,
    ) -> error_stack::Result<Vec<common_utils::id_type::PayoutId>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let mut query = diesel_models::list::into_boxed_list(
            DieselPayouts::table()
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
                .order(po_dsl::created_at.desc()),
        );

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

        conn.track_get_results::<<DieselPayouts as HasTable>::Table, _, String>(
            db_metrics::DatabaseOperation::Filter,
            query,
        )
        .await
        .map_err(|er| {
            error_stack::report!(StorageError::from(er))
                .attach_printable("Error filtering payout records")
        })?
        .into_iter()
        .map(|s| {
            common_utils::id_type::PayoutId::try_from(std::borrow::Cow::Owned(s))
                .change_context(StorageError::DeserializationFailed)
                .attach_printable("Failed to deserialize PayoutId from database string")
        })
        .collect::<error_stack::Result<Vec<_>, _>>()
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    #[instrument(skip_all)]
    async fn filter_active_payout_ids_by_constraints(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _constraints: &PayoutFetchConstraints,
    ) -> error_stack::Result<Vec<common_utils::id_type::PayoutId>, StorageError> {
        todo!()
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn get_payout_intent_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> error_stack::Result<Vec<(common_enums::PayoutStatus, i64)>, StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let mut query = diesel_models::list::into_boxed_list(
            <DieselPayouts as HasTable>::table()
                .group_by(po_dsl::status)
                .select((po_dsl::status, diesel::dsl::count_star()))
                .filter(po_dsl::merchant_id.eq(merchant_id.to_owned())),
        );

        if let Some(profile_id) = profile_id_list {
            query = query.filter(po_dsl::profile_id.eq_any(profile_id));
        }

        query = query.filter(po_dsl::created_at.ge(time_range.start_time));

        query = match time_range.end_time {
            Some(ending_at) => query.filter(po_dsl::created_at.le(ending_at)),
            None => query,
        };

        conn.track_get_results::<<DieselPayouts as HasTable>::Table, _, (common_enums::PayoutStatus, i64)>(
            db_metrics::DatabaseOperation::Filter,
            query,
        )
        .await
        .map_err(|er| {
            error_stack::report!(StorageError::from(er))
                .attach_printable("Error filtering payout records")
        })
    }
}

#[async_trait::async_trait]
impl Conversion for Payouts {
    type DstType = DieselPayouts;
    type NewDstType = DieselPayoutsNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(self.to_storage_model())
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let sender_details = crypto_operation::<
            EncryptedJsonType<api_models::sender::SenderDetails>,
            hyperswitch_masking::WithType,
        >(
            state,
            common_utils::type_name!(DieselPayouts),
            CryptoOperation::DecryptOptional(other.sender_details.clone()),
            key_manager_identifier,
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_optionaloperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payouts sender_details".to_string(),
        })?;

        let mut payouts = Self::from_storage_model(other);
        payouts.sender_details = sender_details;
        Ok(payouts)
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(DieselPayoutsNew {
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
            funds_transfer_type: self.funds_transfer_type,
            sender_details: self.sender_details.map(Encryption::from),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            status: self.status,
            attempt_count: self.attempt_count,
            confirm: self.confirm,
            payout_link_id: self.payout_link_id,
            client_secret: self.client_secret,
            priority: self.priority,
            organization_id: self.organization_id,
            processor_merchant_id: self.processor_merchant_id,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            billing_descriptor: self.billing_descriptor,
        })
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
            funds_transfer_type: self.funds_transfer_type,
            sender_details: self.sender_details.map(Encryption::from),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            status: self.status,
            attempt_count: self.attempt_count,
            confirm: self.confirm,
            payout_link_id: self.payout_link_id,
            client_secret: self.client_secret,
            priority: self.priority,
            organization_id: self.organization_id,
            processor_merchant_id: self.processor_merchant_id,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            billing_descriptor: self.billing_descriptor,
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
            funds_transfer_type: storage_model.funds_transfer_type,
            // `sender_details` holds ciphertext in storage; it is populated by the
            // async decryption step in the `PayoutsInterface` implementations.
            sender_details: None,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
            profile_id: storage_model.profile_id,
            status: storage_model.status,
            attempt_count: storage_model.attempt_count,
            confirm: storage_model.confirm,
            payout_link_id: storage_model.payout_link_id,
            client_secret: storage_model.client_secret,
            priority: storage_model.priority,
            organization_id: storage_model.organization_id,
            processor_merchant_id: storage_model.processor_merchant_id,
            created_by: storage_model
                .created_by
                .and_then(|created_by| created_by.parse::<common_utils::types::CreatedBy>().ok()),
            billing_descriptor: storage_model.billing_descriptor,
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
            funds_transfer_type: self.funds_transfer_type,
            sender_details: self.sender_details.map(Encryption::from),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            profile_id: self.profile_id,
            status: self.status,
            attempt_count: self.attempt_count,
            confirm: self.confirm,
            payout_link_id: self.payout_link_id,
            client_secret: self.client_secret,
            priority: self.priority,
            organization_id: self.organization_id,
            processor_merchant_id: self.processor_merchant_id,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            billing_descriptor: self.billing_descriptor,
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
            funds_transfer_type: storage_model.funds_transfer_type,
            // `sender_details` holds ciphertext in storage; it is populated by the
            // async decryption step in the `PayoutsInterface` implementations.
            sender_details: None,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
            profile_id: storage_model.profile_id,
            status: storage_model.status,
            attempt_count: storage_model.attempt_count,
            confirm: storage_model.confirm,
            payout_link_id: storage_model.payout_link_id,
            client_secret: storage_model.client_secret,
            priority: storage_model.priority,
            organization_id: storage_model.organization_id,
            processor_merchant_id: storage_model.processor_merchant_id,
            created_by: storage_model
                .created_by
                .and_then(|created_by| created_by.parse::<common_utils::types::CreatedBy>().ok()),
            billing_descriptor: storage_model.billing_descriptor,
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
                funds_transfer_type,
                sender_details,
                profile_id,
                status,
                confirm,
                payout_type,
                address_id,
                customer_id,
                billing_descriptor,
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
                funds_transfer_type,
                sender_details,
                profile_id,
                status,
                confirm,
                payout_type,
                address_id,
                customer_id,
                billing_descriptor,
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
            Self::StatusAndMetadataUpdate { status, metadata } => {
                DieselPayoutsUpdate::StatusAndMetadataUpdate { status, metadata }
            }
            Self::ManualUpdate { status } => DieselPayoutsUpdate::ManualUpdate { status },
        }
    }

    #[allow(clippy::todo)]
    fn from_storage_model(_storage_model: Self::StorageModel) -> Self {
        todo!("Reverse map should no longer be needed")
    }
}
