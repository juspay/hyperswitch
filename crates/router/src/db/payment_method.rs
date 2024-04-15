use diesel_models::payment_method::PaymentMethodUpdateInternal;
use error_stack::ResultExt;

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as storage_types, enums::MerchantStorageScheme},
};

#[async_trait::async_trait]
pub trait PaymentMethodInterface {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError>;

    async fn find_payment_method_by_locker_id(
        &self,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
        limit: Option<i64>,
    ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError>;

    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError>;

    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError>;

    async fn insert_payment_method(
        &self,
        payment_method_new: storage_types::PaymentMethodNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError>;

    async fn update_payment_method(
        &self,
        payment_method: storage_types::PaymentMethod,
        payment_method_update: storage_types::PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError>;

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::fallback_reverse_lookup_not_found;
    use diesel_models::{kv, PaymentMethodUpdateInternal};
    use error_stack::{report, ResultExt};
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation, PartitionKey};

    use super::PaymentMethodInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage::{self as storage_types, enums::MerchantStorageScheme},
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl PaymentMethodInterface for Store {
        #[instrument(skip_all)]
        async fn find_payment_method(
            &self,
            payment_method_id: &str,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };

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
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::PaymentMethod>::HGet(&lookup.sk_id),
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
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_locker_id(
            &self,
            locker_id: &str,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::PaymentMethod::find_by_locker_id(&conn, locker_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };

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
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::PaymentMethod>::HGet(&lookup.sk_id),
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
        }
        // not supported in kv
        #[instrument(skip_all)]
        async fn get_payment_method_count_by_customer_id_merchant_id_status(
            &self,
            customer_id: &str,
            merchant_id: &str,
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
            payment_method_new: storage_types::PaymentMethodNew,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            match storage_scheme {
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
                    let field =
                        format!("payment_method_id_{}", payment_method_new.payment_method_id);

                    let reverse_lookup_entry = |v: String| diesel_models::ReverseLookupNew {
                        sk_id: field.clone(),
                        pk_id: key_str.clone(),
                        lookup_id: v,
                        source: "payment_method".to_string(),
                        updated_by: storage_scheme.to_string(),
                    };

                    let lookup_id1 =
                        format!("payment_method_{}", &payment_method_new.payment_method_id);
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
                            insertable: kv::Insertable::PaymentMethod(payment_method_new),
                        },
                    };

                    match kv_wrapper::<diesel_models::PaymentMethod, _, _>(
                        self,
                        KvOperation::<diesel_models::PaymentMethod>::HSetNx(
                            &field,
                            &storage_payment_method,
                            redis_entry,
                        ),
                        key,
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "payment_method",
                            key: Some(storage_payment_method.payment_method_id),
                        }
                        .into()),
                        Ok(HsetnxReply::KeySet) => Ok(storage_payment_method),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        #[instrument(skip_all)]
        async fn update_payment_method(
            &self,
            payment_method: storage_types::PaymentMethod,
            payment_method_update: storage_types::PaymentMethodUpdate,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    payment_method
                        .update_with_payment_method_id(&conn, payment_method_update.into())
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                MerchantStorageScheme::RedisKv => {
                    let merchant_id = payment_method.merchant_id.clone();
                    let customer_id = payment_method.customer_id.clone();
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id: &merchant_id,
                        customer_id: &customer_id,
                    };
                    let key_str = key.to_string();
                    let field = format!("payment_method_id_{}", payment_method.payment_method_id);

                    let p_update: PaymentMethodUpdateInternal = payment_method_update.into();
                    let updated_payment_method =
                        p_update.clone().apply_changeset(payment_method.clone());

                    let redis_value = serde_json::to_string(&updated_payment_method)
                        .change_context(errors::StorageError::SerializationFailed)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::PaymentMethodUpdate(
                                kv::PaymentMethodUpdateMems {
                                    orig: payment_method,
                                    update_data: p_update,
                                },
                            ),
                        },
                    };

                    kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::<diesel_models::PaymentMethod>::Hset(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_payment_method)
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_list(
            &self,
            customer_id: &str,
            merchant_id: &str,
            limit: Option<i64>,
        ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
                limit,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_status(
            &self,
            customer_id: &str,
            merchant_id: &str,
            status: common_enums::PaymentMethodStatus,
            limit: Option<i64>,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
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

            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };

                    let pattern = "payment_method_id_*";

                    let redis_fut = async {
                        let kv_result = kv_wrapper::<storage_types::PaymentMethod, _, _>(
                            self,
                            KvOperation::<storage_types::PaymentMethod>::Scan(pattern),
                            key,
                        )
                        .await?
                        .try_into_scan();
                        kv_result.map(|payment_methods| {
                            payment_methods
                                .into_iter()
                                .filter(|pm| pm.status == status)
                                .collect()
                        })
                    };

                    Box::pin(db_utils::find_all_redis_database(
                        redis_fut,
                        database_call,
                        limit,
                    ))
                    .await
                }
            }
        }

        async fn delete_payment_method_by_merchant_id_payment_method_id(
            &self,
            merchant_id: &str,
            payment_method_id: &str,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::PaymentMethod::delete_by_merchant_id_payment_method_id(
                &conn,
                merchant_id,
                payment_method_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::report;
    use router_env::{instrument, tracing};

    use super::PaymentMethodInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{self as storage_types, enums::MerchantStorageScheme},
    };
    #[async_trait::async_trait]
    impl PaymentMethodInterface for Store {
        #[instrument(skip_all)]
        async fn find_payment_method(
            &self,
            payment_method_id: &str,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_locker_id(
            &self,
            locker_id: &str,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_locker_id(&conn, locker_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn get_payment_method_count_by_customer_id_merchant_id_status(
            &self,
            customer_id: &str,
            merchant_id: &str,
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
            payment_method_new: storage_types::PaymentMethodNew,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            payment_method_new
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn update_payment_method(
            &self,
            payment_method: storage_types::PaymentMethod,
            payment_method_update: storage_types::PaymentMethodUpdate,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            payment_method
                .update_with_payment_method_id(&conn, payment_method_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_list(
            &self,
            customer_id: &str,
            merchant_id: &str,
            limit: Option<i64>,
        ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
                limit,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_payment_method_by_customer_id_merchant_id_status(
            &self,
            customer_id: &str,
            merchant_id: &str,
            status: common_enums::PaymentMethodStatus,
            limit: Option<i64>,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::PaymentMethod::find_by_customer_id_merchant_id_status(
                &conn,
                customer_id,
                merchant_id,
                status,
                limit,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        async fn delete_payment_method_by_merchant_id_payment_method_id(
            &self,
            merchant_id: &str,
            payment_method_id: &str,
        ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::PaymentMethod::delete_by_merchant_id_payment_method_id(
                &conn,
                merchant_id,
                payment_method_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for MockDb {
    async fn find_payment_method(
        &self,
        payment_method_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.payment_method_id == payment_method_id)
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

    async fn find_payment_method_by_locker_id(
        &self,
        locker_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_method = payment_methods
            .iter()
            .find(|pm| pm.locker_id == Some(locker_id.to_string()))
            .cloned();

        match payment_method {
            Some(pm) => Ok(pm),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method".to_string(),
            )
            .into()),
        }
    }

    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let count = payment_methods
            .iter()
            .filter(|pm| {
                pm.customer_id == customer_id
                    && pm.merchant_id == merchant_id
                    && pm.status == status
            })
            .count();
        i64::try_from(count).change_context(errors::StorageError::MockDbError)
    }

    async fn insert_payment_method(
        &self,
        payment_method_new: storage_types::PaymentMethodNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;

        let payment_method = storage_types::PaymentMethod {
            id: i32::try_from(payment_methods.len())
                .change_context(errors::StorageError::MockDbError)?,
            customer_id: payment_method_new.customer_id,
            merchant_id: payment_method_new.merchant_id,
            payment_method_id: payment_method_new.payment_method_id,
            locker_id: payment_method_new.locker_id,
            accepted_currency: payment_method_new.accepted_currency,
            scheme: payment_method_new.scheme,
            token: payment_method_new.token,
            cardholder_name: payment_method_new.cardholder_name,
            issuer_name: payment_method_new.issuer_name,
            issuer_country: payment_method_new.issuer_country,
            payer_country: payment_method_new.payer_country,
            is_stored: payment_method_new.is_stored,
            swift_code: payment_method_new.swift_code,
            direct_debit_token: payment_method_new.direct_debit_token,
            created_at: payment_method_new.created_at,
            last_modified: payment_method_new.last_modified,
            payment_method: payment_method_new.payment_method,
            payment_method_type: payment_method_new.payment_method_type,
            payment_method_issuer: payment_method_new.payment_method_issuer,
            payment_method_issuer_code: payment_method_new.payment_method_issuer_code,
            metadata: payment_method_new.metadata,
            payment_method_data: payment_method_new.payment_method_data,
            last_used_at: payment_method_new.last_used_at,
            connector_mandate_details: payment_method_new.connector_mandate_details,
            customer_acceptance: payment_method_new.customer_acceptance,
            status: payment_method_new.status,
            network_transaction_id: payment_method_new.network_transaction_id,
        };
        payment_methods.push(payment_method.clone());
        Ok(payment_method)
    }

    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        customer_id: &str,
        merchant_id: &str,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_methods_found: Vec<storage_types::PaymentMethod> = payment_methods
            .iter()
            .filter(|pm| pm.customer_id == customer_id && pm.merchant_id == merchant_id)
            .cloned()
            .collect();

        if payment_methods_found.is_empty() {
            Err(
                errors::StorageError::ValueNotFound("cannot find payment method".to_string())
                    .into(),
            )
        } else {
            Ok(payment_methods_found)
        }
    }

    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::PaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let payment_methods_found: Vec<storage_types::PaymentMethod> = payment_methods
            .iter()
            .filter(|pm| {
                pm.customer_id == customer_id
                    && pm.merchant_id == merchant_id
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
            Ok(payment_methods_found)
        }
    }

    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;
        match payment_methods.iter().position(|pm| {
            pm.merchant_id == merchant_id && pm.payment_method_id == payment_method_id
        }) {
            Some(index) => {
                let deleted_payment_method = payment_methods.remove(index);
                Ok(deleted_payment_method)
            }
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to delete".to_string(),
            )
            .into()),
        }
    }

    async fn update_payment_method(
        &self,
        payment_method: storage_types::PaymentMethod,
        payment_method_update: storage_types::PaymentMethodUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::PaymentMethod, errors::StorageError> {
        let pm_update_res = self
            .payment_methods
            .lock()
            .await
            .iter_mut()
            .find(|pm| pm.id == payment_method.id)
            .map(|pm| {
                let payment_method_updated =
                    PaymentMethodUpdateInternal::from(payment_method_update)
                        .create_payment_method(pm.clone());
                *pm = payment_method_updated.clone();
                payment_method_updated
            });

        match pm_update_res {
            Some(result) => Ok(result),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find payment method to update".to_string(),
            )
            .into()),
        }
    }
}
