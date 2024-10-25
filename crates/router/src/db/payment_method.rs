use common_utils::{id_type, types::keymanager::KeyManagerState};
use diesel_models::payment_method::PaymentMethodUpdateInternal;
use error_stack::ResultExt;
use hyperswitch_domain_models::behaviour::{Conversion, ReverseConversion};

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::{
        domain,
        storage::{self as storage_types, enums::MerchantStorageScheme},
    },
};

#[async_trait::async_trait]
pub trait PaymentMethodInterface {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError>;

    // Need to fix this once we start moving to v2 for payment method
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method_list_by_global_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        id: &str,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError>;

    async fn insert_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        payment_method_update: storage_types::PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::{
        fallback_reverse_lookup_not_found, id_type, types::keymanager::KeyManagerState,
    };
    use diesel_models::{kv, PaymentMethodUpdateInternal};
    use error_stack::{report, ResultExt};
    use hyperswitch_domain_models::behaviour::{Conversion, ReverseConversion};
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{
        decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey,
    };

    use super::PaymentMethodInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::{
            domain,
            storage::{self as storage_types, enums::MerchantStorageScheme},
        },
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl PaymentMethodInterface for Store {
        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn find_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method_id: &str,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme =
                Box::pin(decide_storage_scheme::<_, storage_types::PaymentMethod>(
                    self,
                    storage_scheme,
                    Op::Find,
                ))
                .await;
            let get_pm = || async {
                match storage_scheme {
                    MerchantStorageScheme::PostgresOnly => database_call().await,
                    MerchantStorageScheme::RedisKv => {
                        let lookup_id = format!("payment_method_{}", payment_method_id);
                        let lookup = fallback_reverse_lookup_not_found!(
                            self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                                .await,
                            database_call().await
                        );

                        let key = PartitionKey::CombinationKey {
                            combination: &lookup.pk_id,
                        };

                        Box::pin(db_utils::try_redis_get_else_try_database_get(
                            async {
                                Box::pin(kv_wrapper(
                                    self,
                                    KvOperation::<storage_types::PaymentMethod>::HGet(
                                        &lookup.sk_id,
                                    ),
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
            };

            get_pm()
                .await?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        #[instrument(skip_all)]
        async fn find_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method_id: &id_type::GlobalPaymentMethodId,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::PaymentMethod::find_by_id(&conn, payment_method_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme =
                Box::pin(decide_storage_scheme::<_, storage_types::PaymentMethod>(
                    self,
                    storage_scheme,
                    Op::Find,
                ))
                .await;
            let get_pm = || async {
                match storage_scheme {
                    MerchantStorageScheme::PostgresOnly => database_call().await,
                    MerchantStorageScheme::RedisKv => {
                        let lookup_id =
                            format!("payment_method_{}", payment_method_id.get_string_repr());
                        let lookup = fallback_reverse_lookup_not_found!(
                            self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                                .await,
                            database_call().await
                        );

                        let key = PartitionKey::CombinationKey {
                            combination: &lookup.pk_id,
                        };

                        Box::pin(db_utils::try_redis_get_else_try_database_get(
                            async {
                                kv_wrapper(
                                    self,
                                    KvOperation::<storage_types::PaymentMethod>::HGet(
                                        &lookup.sk_id,
                                    ),
                                    key,
                                )
                                .await?
                                .try_into_hget()
                            },
                            database_call,
                        ))
                        .await
                    }
                }
            };

            get_pm()
                .await?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn find_payment_method_by_locker_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            locker_id: &str,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::PaymentMethod::find_by_locker_id(&conn, locker_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme =
                Box::pin(decide_storage_scheme::<_, storage_types::PaymentMethod>(
                    self,
                    storage_scheme,
                    Op::Find,
                ))
                .await;
            let get_pm = || async {
                match storage_scheme {
                    MerchantStorageScheme::PostgresOnly => database_call().await,
                    MerchantStorageScheme::RedisKv => {
                        let lookup_id = format!("payment_method_locker_{}", locker_id);
                        let lookup = fallback_reverse_lookup_not_found!(
                            self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                                .await,
                            database_call().await
                        );

                        let key = PartitionKey::CombinationKey {
                            combination: &lookup.pk_id,
                        };

                        Box::pin(db_utils::try_redis_get_else_try_database_get(
                            async {
                                Box::pin(kv_wrapper(
                                    self,
                                    KvOperation::<storage_types::PaymentMethod>::HGet(
                                        &lookup.sk_id,
                                    ),
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
            };

            get_pm()
                .await?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        // not supported in kv
        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn get_payment_method_count_by_customer_id_merchant_id_status(
            &self,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            status: common_enums::PaymentMethodStatus,
        ) -> CustomResult<i64, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::get_count_by_customer_id_merchant_id_status(
                &conn,
                customer_id,
                merchant_id,
                status,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        #[instrument(skip_all)]
        async fn insert_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method_new = payment_method
                .construct_new()
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            let conn = connection::pg_connection_write(self).await?;
            payment_method_new
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn insert_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let storage_scheme =
                Box::pin(decide_storage_scheme::<_, storage_types::PaymentMethod>(
                    self,
                    storage_scheme,
                    Op::Insert,
                ))
                .await;

            let mut payment_method_new = payment_method
                .construct_new()
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            payment_method_new.update_storage_scheme(storage_scheme);
            let pm = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    payment_method_new
                        .insert(&conn)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                MerchantStorageScheme::RedisKv => {
                    let merchant_id = payment_method_new.merchant_id.clone();
                    let customer_id = payment_method_new.customer_id.clone();

                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id: &merchant_id,
                        customer_id: &customer_id,
                    };
                    let key_str = key.to_string();
                    let field = format!("payment_method_id_{}", payment_method_new.get_id());

                    let reverse_lookup_entry = |v: String| diesel_models::ReverseLookupNew {
                        sk_id: field.clone(),
                        pk_id: key_str.clone(),
                        lookup_id: v,
                        source: "payment_method".to_string(),
                        updated_by: storage_scheme.to_string(),
                    };

                    let lookup_id1 = format!("payment_method_{}", payment_method_new.get_id());
                    let mut reverse_lookups = vec![lookup_id1];
                    if let Some(locker_id) = &payment_method_new.locker_id {
                        reverse_lookups.push(format!("payment_method_locker_{}", locker_id))
                    }

                    let results = reverse_lookups.into_iter().map(|v| {
                        self.insert_reverse_lookup(reverse_lookup_entry(v), storage_scheme)
                    });

                    futures::future::try_join_all(results).await?;

                    let storage_payment_method = (&payment_method_new).into();

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: Box::new(kv::Insertable::PaymentMethod(payment_method_new)),
                        },
                    };

                    match Box::pin(kv_wrapper::<diesel_models::PaymentMethod, _, _>(
                        self,
                        KvOperation::<diesel_models::PaymentMethod>::HSetNx(
                            &field,
                            &storage_payment_method,
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "payment_method",
                            key: Some(storage_payment_method.get_id().clone()),
                        }
                        .into()),
                        Ok(HsetnxReply::KeySet) => Ok(storage_payment_method),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }?;

            pm.convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn update_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            payment_method_update: storage_types::PaymentMethodUpdate,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method = Conversion::convert(payment_method)
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            let merchant_id = payment_method.merchant_id.clone();
            let customer_id = payment_method.customer_id.clone();
            let key = PartitionKey::MerchantIdCustomerId {
                merchant_id: &merchant_id,
                customer_id: &customer_id,
            };
            let field = format!("payment_method_id_{}", payment_method.get_id());
            let storage_scheme =
                Box::pin(decide_storage_scheme::<_, storage_types::PaymentMethod>(
                    self,
                    storage_scheme,
                    Op::Update(key.clone(), &field, payment_method.updated_by.as_deref()),
                ))
                .await;
            let pm = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    payment_method
                        .update_with_payment_method_id(
                            &conn,
                            payment_method_update.convert_to_payment_method_update(storage_scheme),
                        )
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                MerchantStorageScheme::RedisKv => {
                    let key_str = key.to_string();

                    let p_update: PaymentMethodUpdateInternal =
                        payment_method_update.convert_to_payment_method_update(storage_scheme);
                    let updated_payment_method =
                        p_update.clone().apply_changeset(payment_method.clone());

                    let redis_value = serde_json::to_string(&updated_payment_method)
                        .change_context(errors::StorageError::SerializationFailed)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: Box::new(kv::Updateable::PaymentMethodUpdate(Box::new(
                                kv::PaymentMethodUpdateMems {
                                    orig: payment_method,
                                    update_data: p_update,
                                },
                            ))),
                        },
                    };

                    Box::pin(kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::<diesel_models::PaymentMethod>::Hset(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_payment_method)
                }
            }?;

            pm.convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        #[instrument(skip_all)]
        async fn update_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            payment_method_update: storage_types::PaymentMethodUpdate,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method = Conversion::convert(payment_method)
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            let conn = connection::pg_connection_write(self).await?;
            payment_method
                .update_with_id(&conn, payment_method_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_list(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            limit: Option<i64>,
        ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let payment_methods = storage_types::PaymentMethod::find_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
                limit,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

            let pm_futures = payment_methods
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_payment_methods = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_payment_methods)
        }

        // Need to fix this once we start moving to v2 for payment method
        #[cfg(all(
            feature = "v2",
            feature = "customer_v2",
            feature = "payment_methods_v2"
        ))]
        async fn find_payment_method_list_by_global_id(
            &self,
            _state: &KeyManagerState,
            _key_store: &domain::MerchantKeyStore,
            _id: &str,
            _limit: Option<i64>,
        ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
            todo!()
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_status(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            status: common_enums::PaymentMethodStatus,
            limit: Option<i64>,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::PaymentMethod::find_by_customer_id_merchant_id_status(
                    &conn,
                    customer_id,
                    merchant_id,
                    status,
                    limit,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };

            let payment_methods = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };

                    let pattern = "payment_method_id_*";

                    let redis_fut = async {
                        let kv_result = Box::pin(kv_wrapper::<storage_types::PaymentMethod, _, _>(
                            self,
                            KvOperation::<storage_types::PaymentMethod>::Scan(pattern),
                            key,
                        ))
                        .await?
                        .try_into_scan();
                        kv_result.map(|payment_methods| {
                            payment_methods
                                .into_iter()
                                .filter(|pm| pm.status == status)
                                .collect()
                        })
                    };

                    Box::pin(db_utils::find_all_combined_kv_database(
                        redis_fut,
                        database_call,
                        limit,
                    ))
                    .await
                }
            }?;

            let pm_futures = payment_methods
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_payment_methods = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_payment_methods)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        async fn delete_payment_method_by_merchant_id_payment_method_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            merchant_id: &id_type::MerchantId,
            payment_method_id: &str,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::PaymentMethod::delete_by_merchant_id_payment_method_id(
                &conn,
                merchant_id,
                payment_method_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }

        // Soft delete, Check if KV stuff is needed here
        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        async fn delete_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method = Conversion::convert(payment_method)
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            let conn = connection::pg_connection_write(self).await?;
            let payment_method_update = storage_types::PaymentMethodUpdate::StatusUpdate {
                status: Some(common_enums::PaymentMethodStatus::Inactive),
            };
            payment_method
                .update_with_id(&conn, payment_method_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        // Check if KV stuff is needed here
        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        async fn find_payment_method_by_fingerprint_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            fingerprint_id: &str,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_fingerprint_id(&conn, fingerprint_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use common_utils::{id_type, types::keymanager::KeyManagerState};
    use error_stack::{report, ResultExt};
    use hyperswitch_domain_models::behaviour::{Conversion, ReverseConversion};
    use router_env::{instrument, tracing};

    use super::PaymentMethodInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::{
            domain,
            storage::{self as storage_types, enums::MerchantStorageScheme},
        },
    };

    #[async_trait::async_trait]
    impl PaymentMethodInterface for Store {
        #[instrument(skip_all)]
        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        async fn find_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method_id: &str,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        async fn find_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method_id: &id_type::GlobalPaymentMethodId,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_id(&conn, payment_method_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn find_payment_method_by_locker_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            locker_id: &str,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_locker_id(&conn, locker_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn get_payment_method_count_by_customer_id_merchant_id_status(
            &self,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            status: common_enums::PaymentMethodStatus,
        ) -> CustomResult<i64, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::get_count_by_customer_id_merchant_id_status(
                &conn,
                customer_id,
                merchant_id,
                status,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn insert_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method_new = payment_method
                .construct_new()
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            let conn = connection::pg_connection_write(self).await?;
            payment_method_new
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn update_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            payment_method_update: storage_types::PaymentMethodUpdate,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method = Conversion::convert(payment_method)
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            let conn = connection::pg_connection_write(self).await?;
            payment_method
                .update_with_payment_method_id(&conn, payment_method_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        #[instrument(skip_all)]
        async fn update_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
            payment_method_update: storage_types::PaymentMethodUpdate,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method = payment_method
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            let conn = connection::pg_connection_write(self).await?;
            payment_method
                .update_with_id(&conn, payment_method_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_list(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            limit: Option<i64>,
        ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let payment_methods = storage_types::PaymentMethod::find_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
                limit,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

            let pm_futures = payment_methods
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_payment_methods = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_payment_methods)
        }

        // Need to fix this once we move to payment method for customer
        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn find_payment_method_list_by_global_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            id: &str,
            limit: Option<i64>,
        ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_global_id(&conn, id, limit)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_status(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            status: common_enums::PaymentMethodStatus,
            limit: Option<i64>,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let payment_methods =
                storage_types::PaymentMethod::find_by_customer_id_merchant_id_status(
                    &conn,
                    customer_id,
                    merchant_id,
                    status,
                    limit,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

            let pm_futures = payment_methods
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_payment_methods = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_payment_methods)
        }

        #[cfg(all(
            any(feature = "v1", feature = "v2"),
            not(feature = "payment_methods_v2")
        ))]
        async fn delete_payment_method_by_merchant_id_payment_method_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            merchant_id: &id_type::MerchantId,
            payment_method_id: &str,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::PaymentMethod::delete_by_merchant_id_payment_method_id(
                &conn,
                merchant_id,
                payment_method_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        async fn delete_payment_method(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            payment_method: domain::PaymentMethod,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let payment_method = Conversion::convert(payment_method)
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            let conn = connection::pg_connection_write(self).await?;
            let payment_method_update = storage_types::PaymentMethodUpdate::StatusUpdate {
                status: Some(common_enums::PaymentMethodStatus::Inactive),
            };
            payment_method
                .update_with_id(&conn, payment_method_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
        async fn find_payment_method_by_fingerprint_id(
            &self,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            fingerprint_id: &str,
        ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_fingerprint_id(&conn, fingerprint_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for MockDb {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.get_id() == payment_method_id)
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.get_id() == payment_method_id)
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        locker_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.locker_id == Some(locker_id.to_string()))
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let count = payment_methods
            .iter()
            .filter(|pm| {
                pm.customer_id == *customer_id
                    && pm.merchant_id == *merchant_id
                    && pm.status == status
            })
            .count();
        i64::try_from(count).change_context(errors::StorageError::MockDbError)
    }

    async fn insert_payment_method(
        &self,
        _state: &KeyManagerState,
        _key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;

        let pm = Conversion::convert(payment_method.clone())
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        payment_methods.push(pm);
        Ok(payment_method)
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_methods_found: Vec<storage_types::PaymentMethod> = payment_methods
            .iter()
            .filter(|pm| pm.customer_id == *customer_id && pm.merchant_id == *merchant_id)
            .cloned()
            .collect();

        if payment_methods_found.is_empty() {
            Err(
                errors::StorageError::ValueNotFound("cannot find payment method".to_string())
                    .into(),
            )
        } else {
            let pm_futures = payment_methods_found
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_payment_methods = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_payment_methods)
        }
    }

    // Need to fix this once we complete v2 payment method
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method_list_by_global_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        _id: &str,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
        todo!()
    }

    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_methods_found: Vec<storage_types::PaymentMethod> = payment_methods
            .iter()
            .filter(|pm| {
                pm.customer_id == *customer_id
                    && pm.merchant_id == *merchant_id
                    && pm.status == status
            })
            .cloned()
            .collect();

        if payment_methods_found.is_empty() {
            Err(
                errors::StorageError::ValueNotFound("cannot find payment methods".to_string())
                    .into(),
            )
        } else {
            let pm_futures = payment_methods_found
                .into_iter()
                .map(|pm| async {
                    pm.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .collect::<Vec<_>>();

            let domain_payment_methods = futures::future::try_join_all(pm_futures).await?;

            Ok(domain_payment_methods)
        }
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;
        match payment_methods
            .iter()
            .position(|pm| pm.merchant_id == *merchant_id && pm.get_id() == payment_method_id)
        {
            Some(index) => {
                let deleted_payment_method = payment_methods.remove(index);
                Ok(deleted_payment_method
                    .convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?)
            }
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to delete".to_string(),
            )
            .into()),
        }
    }

    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        payment_method_update: storage_types::PaymentMethodUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let pm_update_res = self
            .payment_methods
            .lock()
            .await
            .iter_mut()
            .find(|pm| pm.get_id() == payment_method.get_id())
            .map(|pm| {
                let payment_method_updated =
                    PaymentMethodUpdateInternal::from(payment_method_update)
                        .create_payment_method(pm.clone());
                *pm = payment_method_updated.clone();
                payment_method_updated
            });

        match pm_update_res {
            Some(result) => Ok(result
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to update".to_string(),
            )
            .into()),
        }
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let payment_method_update = storage_types::PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
        };

        let pm_update_res = self
            .payment_methods
            .lock()
            .await
            .iter_mut()
            .find(|pm| pm.get_id() == payment_method.get_id())
            .map(|pm| {
                let payment_method_updated =
                    PaymentMethodUpdateInternal::from(payment_method_update)
                        .create_payment_method(pm.clone());
                *pm = payment_method_updated.clone();
                payment_method_updated
            });

        match pm_update_res {
            Some(result) => Ok(result
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to update".to_string(),
            )
            .into()),
        }
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.locker_fingerprint_id == Some(fingerprint_id.to_string()))
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }
}
