use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use futures::future::try_join_all;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage::{self as storage_types, enums::MerchantStorageScheme},
    },
};

#[async_trait::async_trait]
pub trait CustomerInterface
where
    domain::Customer:
        Conversion<DstType = storage_types::Customer, NewDstType = storage_types::CustomerNew>,
{
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError>;

    async fn update_customer_by_customer_id_merchant_id(
        &self,
        customer_id: String,
        merchant_id: String,
        customer: domain::Customer,
        customer_update: storage_types::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError>;

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError>;

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError>;

    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::ext_traits::AsyncExt;
    use diesel_models::kv;
    use error_stack::{report, ResultExt};
    use futures::future::try_join_all;
    use masking::PeekInterface;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation, PartitionKey};

    use super::CustomerInterface;
    use crate::{
        connection,
        core::{
            customers::REDACTED,
            errors::{self, CustomResult},
        },
        services::Store,
        types::{
            domain::{
                self,
                behaviour::{Conversion, ReverseConversion},
            },
            storage::{self as storage_types, enums::MerchantStorageScheme},
        },
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl CustomerInterface for Store {
        #[instrument(skip_all)]
        // check customer not found in kv and fallback to db
        async fn find_customer_optional_by_customer_id_merchant_id(
            &self,
            customer_id: &str,
            merchant_id: &str,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|err| report!(errors::StorageError::from(err)))
            };

            let maybe_customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };
                    let field = format!("cust_{}", customer_id);
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        // check for ValueNotFound
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Customer>::HGet(&field),
                                key,
                            )
                            .await?
                            .try_into_hget()
                            .map(Some)
                        },
                        database_call,
                    ))
                    .await
                }
            }?;

            let maybe_result = maybe_customer
                .async_map(|c| async {
                    c.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
                .transpose()?;

            maybe_result.map_or(Ok(None), |customer: domain::Customer| match customer.name {
                Some(ref name) if name.peek() == REDACTED => {
                    Err(errors::StorageError::CustomerRedacted)?
                }
                _ => Ok(Some(customer)),
            })
        }

        #[instrument(skip_all)]
        async fn update_customer_by_customer_id_merchant_id(
            &self,
            customer_id: String,
            merchant_id: String,
            customer: domain::Customer,
            customer_update: storage_types::CustomerUpdate,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            let customer = Conversion::convert(customer)
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            let database_call = || async {
                storage_types::Customer::update_by_customer_id_merchant_id(
                    &conn,
                    customer_id.clone(),
                    merchant_id.clone(),
                    customer_update.clone().into(),
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };

            let updated_object = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id: merchant_id.as_str(),
                        customer_id: customer_id.as_str(),
                    };
                    let field = format!("cust_{}", customer_id);
                    let updated_customer =
                        diesel_models::CustomerUpdateInternal::from(customer_update.clone())
                            .apply_changeset(customer.clone());

                    let redis_value = serde_json::to_string(&updated_customer)
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::CustomerUpdate(kv::CustomerUpdateMems {
                                orig: customer,
                                update_data: customer_update.into(),
                            }),
                        },
                    };

                    kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<diesel_models::Customer>(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_customer)
                }
            };

            updated_object?
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[instrument(skip_all)]
        async fn find_customer_by_customer_id_merchant_id(
            &self,
            customer_id: &str,
            merchant_id: &str,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Customer::find_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };

            let customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };
                    let field = format!("cust_{}", customer_id);
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Customer>::HGet(&field),
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

            let result: domain::Customer = customer
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            //.await

            match result.name {
                Some(ref name) if name.peek() == REDACTED => {
                    Err(errors::StorageError::CustomerRedacted)?
                }
                _ => Ok(result),
            }
        }

        #[instrument(skip_all)]
        async fn list_customers_by_merchant_id(
            &self,
            merchant_id: &str,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;

            let encrypted_customers =
                storage_types::Customer::list_by_merchant_id(&conn, merchant_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))?;

            let customers = try_join_all(encrypted_customers.into_iter().map(
                |encrypted_customer| async {
                    encrypted_customer
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                },
            ))
            .await?;

            Ok(customers)
        }

        #[instrument(skip_all)]
        async fn insert_customer(
            &self,
            customer_data: domain::Customer,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Customer, errors::StorageError> {
            let customer_id = customer_data.customer_id.clone();
            let merchant_id = customer_data.merchant_id.clone();
            let new_customer = customer_data
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?;

            let create_customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    new_customer
                        .insert(&conn)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id: merchant_id.as_str(),
                        customer_id: customer_id.as_str(),
                    };
                    let field = format!("cust_{}", customer_id.clone());

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: kv::Insertable::Customer(new_customer.clone()),
                        },
                    };
                    let storage_customer = new_customer.into();

                    match kv_wrapper::<diesel_models::Customer, _, _>(
                        self,
                        KvOperation::HSetNx::<diesel_models::Customer>(
                            &field,
                            &storage_customer,
                            redis_entry,
                        ),
                        key,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hsetnx()
                    {
                        Ok(redis_interface::HsetnxReply::KeyNotSet) => {
                            Err(report!(errors::StorageError::DuplicateValue {
                                entity: "customer",
                                key: Some(customer_id),
                            }))
                        }
                        Ok(redis_interface::HsetnxReply::KeySet) => Ok(storage_customer),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }?;

            create_customer
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[instrument(skip_all)]
        async fn delete_customer_by_customer_id_merchant_id(
            &self,
            customer_id: &str,
            merchant_id: &str,
        ) -> CustomResult<bool, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Customer::delete_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use common_utils::ext_traits::AsyncExt;
    use error_stack::{report, ResultExt};
    use futures::future::try_join_all;
    use masking::PeekInterface;
    use router_env::{instrument, tracing};

    use super::CustomerInterface;
    use crate::{
        connection,
        core::{
            customers::REDACTED,
            errors::{self, CustomResult},
        },
        services::Store,
        types::{
            domain::{
                self,
                behaviour::{Conversion, ReverseConversion},
            },
            storage::{self as storage_types, enums::MerchantStorageScheme},
        },
    };

    #[async_trait::async_trait]
    impl CustomerInterface for Store {
        #[instrument(skip_all)]
        async fn find_customer_optional_by_customer_id_merchant_id(
            &self,
            customer_id: &str,
            merchant_id: &str,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let maybe_customer: Option<domain::Customer> =
                storage_types::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .async_map(|c| async {
                    c.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
                .transpose()?;
            maybe_customer.map_or(Ok(None), |customer| {
                // in the future, once #![feature(is_some_and)] is stable, we can make this more concise:
                // `if customer.name.is_some_and(|ref name| name == REDACTED) ...`
                match customer.name {
                    Some(ref name) if name.peek() == REDACTED => {
                        Err(errors::StorageError::CustomerRedacted)?
                    }
                    _ => Ok(Some(customer)),
                }
            })
        }

        #[instrument(skip_all)]
        async fn update_customer_by_customer_id_merchant_id(
            &self,
            customer_id: String,
            merchant_id: String,
            _customer: domain::Customer,
            customer_update: storage_types::CustomerUpdate,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Customer::update_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
                customer_update.into(),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|c| async {
                c.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        }

        #[instrument(skip_all)]
        async fn find_customer_by_customer_id_merchant_id(
            &self,
            customer_id: &str,
            merchant_id: &str,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let customer: domain::Customer =
                storage_types::Customer::find_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|c| async {
                    c.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await?;
            match customer.name {
                Some(ref name) if name.peek() == REDACTED => {
                    Err(errors::StorageError::CustomerRedacted)?
                }
                _ => Ok(customer),
            }
        }

        #[instrument(skip_all)]
        async fn list_customers_by_merchant_id(
            &self,
            merchant_id: &str,
            key_store: &domain::MerchantKeyStore,
        ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;

            let encrypted_customers =
                storage_types::Customer::list_by_merchant_id(&conn, merchant_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))?;

            let customers = try_join_all(encrypted_customers.into_iter().map(
                |encrypted_customer| async {
                    encrypted_customer
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                },
            ))
            .await?;

            Ok(customers)
        }

        #[instrument(skip_all)]
        async fn insert_customer(
            &self,
            customer_data: domain::Customer,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<domain::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            customer_data
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|c| async {
                    c.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
        }

        #[instrument(skip_all)]
        async fn delete_customer_by_customer_id_merchant_id(
            &self,
            customer_id: &str,
            merchant_id: &str,
        ) -> CustomResult<bool, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Customer::delete_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[async_trait::async_trait]
impl CustomerInterface for MockDb {
    #[allow(clippy::panic)]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        customer_id: &str,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;
        let customer = customers
            .iter()
            .find(|customer| {
                customer.customer_id == customer_id && customer.merchant_id == merchant_id
            })
            .cloned();
        customer
            .async_map(|c| async {
                c.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()
    }

    async fn list_customers_by_merchant_id(
        &self,
        merchant_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;

        let customers = try_join_all(
            customers
                .iter()
                .filter(|customer| customer.merchant_id == merchant_id)
                .map(|customer| async {
                    customer
                        .to_owned()
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                }),
        )
        .await?;

        Ok(customers)
    }

    #[instrument(skip_all)]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: String,
        _merchant_id: String,
        _customer: domain::Customer,
        _customer_update: storage_types::CustomerUpdate,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let mut customers = self.customers.lock().await;

        let customer = Conversion::convert(customer_data)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        customers.push(customer.clone());

        customer
            .convert(key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
