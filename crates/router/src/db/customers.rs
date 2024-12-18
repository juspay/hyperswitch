use common_utils::{ext_traits::AsyncExt, id_type, types::keymanager::KeyManagerState};
use diesel_models::query::customers::CustomerListConstraints as DieselCustomerListConstraints;
use error_stack::ResultExt;
use futures::future::try_join_all;
use hyperswitch_domain_models::customer;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
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

pub struct CustomerListConstraints {
    pub limit: u16,
    pub offset: Option<u32>,
}

impl From<CustomerListConstraints> for DieselCustomerListConstraints {
    fn from(value: CustomerListConstraints) -> Self {
        Self {
            limit: i64::from(value.limit),
            offset: value.offset.map(i64::from),
        }
    }
}

#[async_trait::async_trait]
pub trait CustomerInterface
where
    customer::Customer:
        Conversion<DstType = storage_types::Customer, NewDstType = storage_types::CustomerNew>,
{
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<customer::Customer>, errors::StorageError>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<customer::Customer>, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<customer::Customer>, errors::StorageError>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: id_type::CustomerId,
        merchant_id: id_type::MerchantId,
        customer: customer::Customer,
        customer_update: storage_types::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_merchant_reference_id_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_reference_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError>;

    async fn list_customers_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        constraints: CustomerListConstraints,
    ) -> CustomResult<Vec<customer::Customer>, errors::StorageError>;

    async fn insert_customer(
        &self,
        customer_data: customer::Customer,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        customer: customer::Customer,
        merchant_id: &id_type::MerchantId,
        customer_update: storage_types::CustomerUpdate,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::{ext_traits::AsyncExt, id_type, types::keymanager::KeyManagerState};
    use diesel_models::kv;
    use error_stack::{report, ResultExt};
    use futures::future::try_join_all;
    use hyperswitch_domain_models::customer;
    use masking::PeekInterface;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{
        decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey,
    };

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
        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        async fn find_customer_optional_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
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
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            let maybe_customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };
                    let field = format!("cust_{}", customer_id.get_string_repr());
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        // check for ValueNotFound
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Customer>::HGet(&field),
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
            }?;

            let maybe_result = maybe_customer
                .async_map(|c| async {
                    c.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
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
        // check customer not found in kv and fallback to db
        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
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
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            let maybe_customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };
                    let field = format!("cust_{}", customer_id.get_string_repr());
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        // check for ValueNotFound
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Customer>::HGet(&field),
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
            }?;

            let maybe_result = maybe_customer
                .async_map(|customer| async {
                    customer
                        .convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
                .transpose()?;

            Ok(maybe_result)
        }

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        async fn find_optional_by_merchant_id_merchant_reference_id(
            &self,
            state: &KeyManagerState,
            merchant_reference_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Customer::find_optional_by_merchant_id_merchant_reference_id(
                    &conn,
                    merchant_reference_id,
                    merchant_id,
                )
                .await
                .map_err(|err| report!(errors::StorageError::from(err)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            let maybe_customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdMerchantReferenceId {
                        merchant_id,
                        merchant_reference_id: merchant_reference_id.get_string_repr(),
                    };
                    let field = format!("cust_{}", merchant_reference_id.get_string_repr());
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
                    c.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
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

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn update_customer_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: id_type::CustomerId,
            merchant_id: id_type::MerchantId,
            customer: customer::Customer,
            customer_update: storage_types::CustomerUpdate,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
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
            let key = PartitionKey::MerchantIdCustomerId {
                merchant_id: &merchant_id,
                customer_id: &customer_id,
            };
            let field = format!("cust_{}", customer_id.get_string_repr());
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Update(key.clone(), &field, customer.updated_by.as_deref()),
            ))
            .await;
            let updated_object = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let updated_customer =
                        diesel_models::CustomerUpdateInternal::from(customer_update.clone())
                            .apply_changeset(customer.clone());

                    let redis_value = serde_json::to_string(&updated_customer)
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: Box::new(kv::Updateable::CustomerUpdate(
                                kv::CustomerUpdateMems {
                                    orig: customer,
                                    update_data: customer_update.into(),
                                },
                            )),
                        },
                    };

                    Box::pin(kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<diesel_models::Customer>(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_customer)
                }
            };

            updated_object?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn find_customer_by_merchant_reference_id_merchant_id(
            &self,
            state: &KeyManagerState,
            merchant_reference_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Customer::find_by_merchant_reference_id_merchant_id(
                    &conn,
                    merchant_reference_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            let customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdMerchantReferenceId {
                        merchant_id,
                        merchant_reference_id: merchant_reference_id.get_string_repr(),
                    };
                    let field = format!("cust_{}", merchant_reference_id.get_string_repr());
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

            let result: customer::Customer = customer
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            match result.name {
                Some(ref name) if name.peek() == REDACTED => {
                    Err(errors::StorageError::CustomerRedacted)?
                }
                _ => Ok(result),
            }
        }

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn find_customer_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
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
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            let customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    };
                    let field = format!("cust_{}", customer_id.get_string_repr());
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Customer>::HGet(&field),
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

            let result: customer::Customer = customer
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;

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
            state: &KeyManagerState,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            constraints: super::CustomerListConstraints,
        ) -> CustomResult<Vec<customer::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;

            let customer_list_constraints =
                diesel_models::query::customers::CustomerListConstraints::from(constraints);

            let encrypted_customers = storage_types::Customer::list_by_merchant_id(
                &conn,
                merchant_id,
                customer_list_constraints,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

            let customers = try_join_all(encrypted_customers.into_iter().map(
                |encrypted_customer| async {
                    encrypted_customer
                        .convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                },
            ))
            .await?;

            Ok(customers)
        }

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn insert_customer(
            &self,
            customer_data: customer::Customer,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let id = customer_data.id.clone();
            let mut new_customer = customer_data
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Insert,
            ))
            .await;
            new_customer.update_storage_scheme(storage_scheme);
            let create_customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    new_customer
                        .insert(&conn)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::GlobalId {
                        id: id.get_string_repr(),
                    };
                    let field = format!("cust_{}", id.get_string_repr());

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: Box::new(kv::Insertable::Customer(new_customer.clone())),
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
                                key: Some(id.get_string_repr().to_owned()),
                            }))
                        }
                        Ok(redis_interface::HsetnxReply::KeySet) => Ok(storage_customer),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }?;

            create_customer
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn insert_customer(
            &self,
            customer_data: customer::Customer,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let customer_id = customer_data.customer_id.clone();
            let merchant_id = customer_data.merchant_id.clone();
            let mut new_customer = customer_data
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Insert,
            ))
            .await;
            new_customer.update_storage_scheme(storage_scheme);
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
                        merchant_id: &merchant_id,
                        customer_id: &customer_id,
                    };
                    let field = format!("cust_{}", customer_id.get_string_repr());

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: Box::new(kv::Insertable::Customer(new_customer.clone())),
                        },
                    };
                    let storage_customer = new_customer.into();

                    match Box::pin(kv_wrapper::<diesel_models::Customer, _, _>(
                        self,
                        KvOperation::HSetNx::<diesel_models::Customer>(
                            &field,
                            &storage_customer,
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hsetnx()
                    {
                        Ok(redis_interface::HsetnxReply::KeyNotSet) => {
                            Err(report!(errors::StorageError::DuplicateValue {
                                entity: "customer",
                                key: Some(customer_id.get_string_repr().to_string()),
                            }))
                        }
                        Ok(redis_interface::HsetnxReply::KeySet) => Ok(storage_customer),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }?;

            create_customer
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn delete_customer_by_customer_id_merchant_id(
            &self,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
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

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn find_customer_by_global_id(
            &self,
            state: &KeyManagerState,
            id: &id_type::GlobalCustomerId,
            _merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Customer::find_by_global_id(&conn, id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            let customer = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::GlobalId {
                        id: id.get_string_repr(),
                    };
                    let field = format!("cust_{}", id.get_string_repr());
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

            let result: customer::Customer = customer
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;

            if result.status == common_enums::DeleteStatus::Redacted {
                Err(report!(errors::StorageError::CustomerRedacted))
            } else {
                Ok(result)
            }
        }

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn update_customer_by_global_id(
            &self,
            state: &KeyManagerState,
            id: &id_type::GlobalCustomerId,
            customer: customer::Customer,
            _merchant_id: &id_type::MerchantId,
            customer_update: storage_types::CustomerUpdate,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            let customer = Conversion::convert(customer)
                .await
                .change_context(errors::StorageError::EncryptionError)?;
            let database_call = || async {
                storage_types::Customer::update_by_id(
                    &conn,
                    id.clone(),
                    customer_update.clone().into(),
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let key = PartitionKey::GlobalId {
                id: id.get_string_repr(),
            };
            let field = format!("cust_{}", id.get_string_repr());
            let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Customer>(
                self,
                storage_scheme,
                Op::Update(key.clone(), &field, customer.updated_by.as_deref()),
            ))
            .await;
            let updated_object = match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let updated_customer =
                        diesel_models::CustomerUpdateInternal::from(customer_update.clone())
                            .apply_changeset(customer.clone());

                    let redis_value = serde_json::to_string(&updated_customer)
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: Box::new(kv::Updateable::CustomerUpdate(
                                kv::CustomerUpdateMems {
                                    orig: customer,
                                    update_data: customer_update.into(),
                                },
                            )),
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
    use common_utils::{ext_traits::AsyncExt, id_type, types::keymanager::KeyManagerState};
    use error_stack::{report, ResultExt};
    use futures::future::try_join_all;
    use hyperswitch_domain_models::customer;
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
        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        async fn find_customer_optional_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let maybe_customer: Option<customer::Customer> =
                storage_types::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .async_map(|c| async {
                    c.convert(state, key_store.key.get_inner(), merchant_id.clone().into())
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
        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let maybe_customer: Option<customer::Customer> =
                storage_types::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .async_map(|c| async {
                    c.convert(state, key_store.key.get_inner(), merchant_id.clone().into())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
                .transpose()?;
            Ok(maybe_customer)
        }

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        async fn find_optional_by_merchant_id_merchant_reference_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let maybe_customer: Option<customer::Customer> =
                storage_types::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?
                .async_map(|c| async {
                    c.convert(state, key_store.key.get_inner(), merchant_id.clone().into())
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

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn update_customer_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: id_type::CustomerId,
            merchant_id: id_type::MerchantId,
            _customer: customer::Customer,
            customer_update: storage_types::CustomerUpdate,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Customer::update_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id.clone(),
                customer_update.into(),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|c| async {
                c.convert(state, key_store.key.get_inner(), merchant_id.into())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        }

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn find_customer_by_customer_id_merchant_id(
            &self,
            state: &KeyManagerState,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let customer: customer::Customer =
                storage_types::Customer::find_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|c| async {
                    c.convert(state, key_store.key.get_inner(), merchant_id.clone().into())
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

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn find_customer_by_merchant_reference_id_merchant_id(
            &self,
            state: &KeyManagerState,
            merchant_reference_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let customer: customer::Customer =
                storage_types::Customer::find_by_merchant_reference_id_merchant_id(
                    &conn,
                    merchant_reference_id,
                    merchant_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|c| async {
                    c.convert(state, key_store.key.get_inner(), merchant_id.clone().into())
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
            state: &KeyManagerState,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            constraints: super::CustomerListConstraints,
        ) -> CustomResult<Vec<customer::Customer>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;

            let customer_list_constraints =
                diesel_models::query::customers::CustomerListConstraints::from(constraints);

            let encrypted_customers = storage_types::Customer::list_by_merchant_id(
                &conn,
                merchant_id,
                customer_list_constraints,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

            let customers = try_join_all(encrypted_customers.into_iter().map(
                |encrypted_customer| async {
                    encrypted_customer
                        .convert(state, key_store.key.get_inner(), merchant_id.clone().into())
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
            customer_data: customer::Customer,
            state: &KeyManagerState,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            customer_data
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|c| async {
                    c.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
                })
                .await
        }

        #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
        #[instrument(skip_all)]
        async fn delete_customer_by_customer_id_merchant_id(
            &self,
            customer_id: &id_type::CustomerId,
            merchant_id: &id_type::MerchantId,
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

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[allow(clippy::too_many_arguments)]
        async fn update_customer_by_global_id(
            &self,
            state: &KeyManagerState,
            id: &id_type::GlobalCustomerId,
            customer: customer::Customer,
            merchant_id: &id_type::MerchantId,
            customer_update: storage_types::CustomerUpdate,
            key_store: &domain::MerchantKeyStore,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Customer::update_by_global_id(&conn, id, customer_update.into())
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|c| async {
                    c.convert(state, key_store.key.get_inner(), merchant_id)
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
        }

        #[cfg(all(feature = "v2", feature = "customer_v2"))]
        #[instrument(skip_all)]
        async fn find_customer_by_global_id(
            &self,
            state: &KeyManagerState,
            id: &id_type::GlobalCustomerId,
            merchant_id: &id_type::MerchantId,
            key_store: &domain::MerchantKeyStore,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<customer::Customer, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let customer: customer::Customer =
                storage_types::Customer::find_by_global_id(&conn, customer_id, merchant_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                    .async_and_then(|c| async {
                        c.convert(state, key_store.key.get_inner(), merchant_id.clone().into())
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
    }
}

#[async_trait::async_trait]
impl CustomerInterface for MockDb {
    #[allow(clippy::panic)]
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;
        let customer = customers
            .iter()
            .find(|customer| {
                customer.customer_id == *customer_id && &customer.merchant_id == merchant_id
            })
            .cloned();
        customer
            .async_map(|c| async {
                c.convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()
    }

    #[allow(clippy::panic)]
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;
        let customer = customers
            .iter()
            .find(|customer| {
                customer.customer_id == *customer_id && &customer.merchant_id == merchant_id
            })
            .cloned();
        customer
            .async_map(|c| async {
                c.convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()
    }

    #[allow(clippy::panic)]
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<customer::Customer>, errors::StorageError> {
        todo!()
    }

    async fn list_customers_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        constraints: CustomerListConstraints,
    ) -> CustomResult<Vec<customer::Customer>, errors::StorageError> {
        let customers = self.customers.lock().await;

        let customers = try_join_all(
            customers
                .iter()
                .filter(|customer| customer.merchant_id == *merchant_id)
                .take(usize::from(constraints.limit))
                .skip(usize::try_from(constraints.offset.unwrap_or(0)).unwrap_or(0))
                .map(|customer| async {
                    customer
                        .to_owned()
                        .convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                }),
        )
        .await?;

        Ok(customers)
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[instrument(skip_all)]
    async fn update_customer_by_customer_id_merchant_id(
        &self,
        _state: &KeyManagerState,
        _customer_id: id_type::CustomerId,
        _merchant_id: id_type::MerchantId,
        _customer: customer::Customer,
        _customer_update: storage_types::CustomerUpdate,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_by_customer_id_merchant_id(
        &self,
        _state: &KeyManagerState,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_merchant_reference_id_merchant_id(
        &self,
        _state: &KeyManagerState,
        _merchant_reference_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_customer(
        &self,
        customer_data: customer::Customer,
        state: &KeyManagerState,
        key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError> {
        let mut customers = self.customers.lock().await;

        let customer = Conversion::convert(customer_data)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        customers.push(customer.clone());

        customer
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        _state: &KeyManagerState,
        _id: &id_type::GlobalCustomerId,
        _customer: customer::Customer,
        _merchant_id: &id_type::MerchantId,
        _customer_update: storage_types::CustomerUpdate,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_global_id(
        &self,
        _state: &KeyManagerState,
        _id: &id_type::GlobalCustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &domain::MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<customer::Customer, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
