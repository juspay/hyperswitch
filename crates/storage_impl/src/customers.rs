use common_utils::{id_type, pii};
use diesel_models::{customers, kv};
use error_stack::ResultExt;
use futures::future::try_join_all;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    customer as domain,
    merchant_key_store::MerchantKeyStore,
};
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    diesel_error_to_data_error,
    errors::StorageError,
    kv_router_store,
    redis::kv_store::{decide_storage_scheme, KvStorePartition, Op, PartitionKey},
    store::enums::MerchantStorageScheme,
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, KeyManagerState, MockDb, RouterStore,
};

impl KvStorePartition for customers::Customer {}

#[async_trait::async_trait]
impl<T: DatabaseStore> domain::CustomerInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    // check customer not found in kv and fallback to db
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let maybe_result = self
            .find_optional_resource_by_id(
                state,
                key_store,
                storage_scheme,
                customers::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                ),
                kv_router_store::FindResourceBy::Id(
                    format!("cust_{}", customer_id.get_string_repr()),
                    PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    },
                ),
            )
            .await?;

        maybe_result.map_or(Ok(None), |customer: domain::Customer| match customer.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
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
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_optional_resource_by_id(
            state,
            key_store,
            storage_scheme,
            customers::Customer::find_optional_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
            ),
            kv_router_store::FindResourceBy::Id(
                format!("cust_{}", customer_id.get_string_repr()),
                PartitionKey::MerchantIdCustomerId {
                    merchant_id,
                    customer_id,
                },
            ),
        )
        .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        state: &KeyManagerState,
        merchant_reference_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let maybe_result = self
            .find_optional_resource_by_id(
                state,
                key_store,
                storage_scheme,
                customers::Customer::find_optional_by_merchant_id_merchant_reference_id(
                    &conn,
                    merchant_reference_id,
                    merchant_id,
                ),
                kv_router_store::FindResourceBy::Id(
                    format!("cust_{}", merchant_reference_id.get_string_repr()),
                    PartitionKey::MerchantIdMerchantReferenceId {
                        merchant_id,
                        merchant_reference_id: merchant_reference_id.get_string_repr(),
                    },
                ),
            )
            .await?;

        maybe_result.map_or(Ok(None), |customer: domain::Customer| match customer.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
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
        customer: domain::Customer,
        customer_update: domain::CustomerUpdate,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let customer = Conversion::convert(customer)
            .await
            .change_context(StorageError::EncryptionError)?;
        let updated_customer = diesel_models::CustomerUpdateInternal::from(customer_update.clone())
            .apply_changeset(customer.clone());
        let key = PartitionKey::MerchantIdCustomerId {
            merchant_id: &merchant_id,
            customer_id: &customer_id,
        };
        let field = format!("cust_{}", customer_id.get_string_repr());
        self.update_resource(
            state,
            key_store,
            storage_scheme,
            customers::Customer::update_by_customer_id_merchant_id(
                &conn,
                customer_id.clone(),
                merchant_id.clone(),
                customer_update.clone().into(),
            ),
            updated_customer,
            kv_router_store::UpdateResourceParams {
                updateable: kv::Updateable::CustomerUpdate(kv::CustomerUpdateMems {
                    orig: customer.clone(),
                    update_data: customer_update.clone().into(),
                }),
                operation: Op::Update(key.clone(), &field, customer.updated_by.as_deref()),
            },
        )
        .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn find_customer_by_merchant_reference_id_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_reference_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let result: domain::Customer = self
            .find_resource_by_id(
                state,
                key_store,
                storage_scheme,
                customers::Customer::find_by_merchant_reference_id_merchant_id(
                    &conn,
                    merchant_reference_id,
                    merchant_id,
                ),
                kv_router_store::FindResourceBy::Id(
                    format!("cust_{}", merchant_reference_id.get_string_repr()),
                    PartitionKey::MerchantIdMerchantReferenceId {
                        merchant_id,
                        merchant_reference_id: merchant_reference_id.get_string_repr(),
                    },
                ),
            )
            .await?;

        match result.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
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
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let result: domain::Customer = self
            .find_resource_by_id(
                state,
                key_store,
                storage_scheme,
                customers::Customer::find_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                ),
                kv_router_store::FindResourceBy::Id(
                    format!("cust_{}", customer_id.get_string_repr()),
                    PartitionKey::MerchantIdCustomerId {
                        merchant_id,
                        customer_id,
                    },
                ),
            )
            .await?;

        match result.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
            _ => Ok(result),
        }
    }

    #[instrument(skip_all)]
    async fn list_customers_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        constraints: domain::CustomerListConstraints,
    ) -> CustomResult<Vec<domain::Customer>, StorageError> {
        self.router_store
            .list_customers_by_merchant_id(state, merchant_id, key_store, constraints)
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let id = customer_data.id.clone();
        let key = PartitionKey::GlobalId {
            id: id.get_string_repr(),
        };
        let identifier = format!("cust_{}", id.get_string_repr());
        let mut new_customer = customer_data
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?;
        let storage_scheme = Box::pin(decide_storage_scheme::<_, customers::Customer>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        new_customer.update_storage_scheme(storage_scheme);
        self.insert_resource(
            state,
            key_store,
            storage_scheme,
            new_customer.clone().insert(&conn),
            new_customer.clone().into(),
            kv_router_store::InsertResourceParams {
                insertable: kv::Insertable::Customer(new_customer.clone()),
                reverse_lookups: vec![],
                identifier,
                key,
                resource_type: "customer",
            },
        )
        .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[instrument(skip_all)]
    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let key = PartitionKey::MerchantIdCustomerId {
            merchant_id: &customer_data.merchant_id.clone(),
            customer_id: &customer_data.customer_id.clone(),
        };
        let identifier = format!("cust_{}", customer_data.customer_id.get_string_repr());
        let mut new_customer = customer_data
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?;
        let storage_scheme = Box::pin(decide_storage_scheme::<_, customers::Customer>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        new_customer.update_storage_scheme(storage_scheme);
        let customer = new_customer.clone().into();
        self.insert_resource(
            state,
            key_store,
            storage_scheme,
            new_customer.clone().insert(&conn),
            customer,
            kv_router_store::InsertResourceParams {
                insertable: kv::Insertable::Customer(new_customer.clone()),
                reverse_lookups: vec![],
                identifier,
                key,
                resource_type: "customer",
            },
        )
        .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[instrument(skip_all)]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        self.router_store
            .delete_customer_by_customer_id_merchant_id(customer_id, merchant_id)
            .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn find_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        _merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let result: domain::Customer = self
            .find_resource_by_id(
                state,
                key_store,
                storage_scheme,
                customers::Customer::find_by_global_id(&conn, id),
                kv_router_store::FindResourceBy::Id(
                    format!("cust_{}", id.get_string_repr()),
                    PartitionKey::GlobalId {
                        id: id.get_string_repr(),
                    },
                ),
            )
            .await?;

        if result.status == common_enums::DeleteStatus::Redacted {
            Err(StorageError::CustomerRedacted)?
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
        customer: domain::Customer,
        _merchant_id: &id_type::MerchantId,
        customer_update: domain::CustomerUpdate,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let customer = Conversion::convert(customer)
            .await
            .change_context(StorageError::EncryptionError)?;
        let database_call =
            customers::Customer::update_by_id(&conn, id.clone(), customer_update.clone().into());
        let key = PartitionKey::GlobalId {
            id: id.get_string_repr(),
        };
        let field = format!("cust_{}", id.get_string_repr());
        self.update_resource(
            state,
            key_store,
            storage_scheme,
            database_call,
            diesel_models::CustomerUpdateInternal::from(customer_update.clone())
                .apply_changeset(customer.clone()),
            kv_router_store::UpdateResourceParams {
                updateable: kv::Updateable::CustomerUpdate(kv::CustomerUpdateMems {
                    orig: customer.clone(),
                    update_data: customer_update.into(),
                }),
                operation: Op::Update(key.clone(), &field, customer.updated_by.as_deref()),
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> domain::CustomerInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let maybe_customer: Option<domain::Customer> = self
            .find_optional_resource(
                state,
                key_store,
                customers::Customer::find_optional_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                ),
            )
            .await?;
        maybe_customer.map_or(Ok(None), |customer| {
            // in the future, once #![feature(is_some_and)] is stable, we can make this more concise:
            // `if customer.name.is_some_and(|ref name| name == pii::REDACTED) ...`
            match customer.name {
                Some(ref name) if name.peek() == pii::REDACTED => {
                    Err(StorageError::CustomerRedacted)?
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
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_optional_resource(
            state,
            key_store,
            customers::Customer::find_optional_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let maybe_customer: Option<domain::Customer> = self
            .find_optional_resource(
                state,
                key_store,
                customers::Customer::find_optional_by_merchant_id_merchant_reference_id(
                    &conn,
                    customer_id,
                    merchant_id,
                ),
            )
            .await?;
        maybe_customer.map_or(Ok(None), |customer| {
            // in the future, once #![feature(is_some_and)] is stable, we can make this more concise:
            // `if customer.name.is_some_and(|ref name| name == pii::REDACTED) ...`
            match customer.name {
                Some(ref name) if name.peek() == pii::REDACTED => {
                    Err(StorageError::CustomerRedacted)?
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
        _customer: domain::Customer,
        customer_update: domain::CustomerUpdate,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            customers::Customer::update_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id.clone(),
                customer_update.into(),
            ),
        )
        .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[instrument(skip_all)]
    async fn find_customer_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let customer: domain::Customer = self
            .call_database(
                state,
                key_store,
                customers::Customer::find_by_customer_id_merchant_id(
                    &conn,
                    customer_id,
                    merchant_id,
                ),
            )
            .await?;
        match customer.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
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
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let customer: domain::Customer = self
            .call_database(
                state,
                key_store,
                customers::Customer::find_by_merchant_reference_id_merchant_id(
                    &conn,
                    merchant_reference_id,
                    merchant_id,
                ),
            )
            .await?;
        match customer.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
            _ => Ok(customer),
        }
    }

    #[instrument(skip_all)]
    async fn list_customers_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        constraints: domain::CustomerListConstraints,
    ) -> CustomResult<Vec<domain::Customer>, StorageError> {
        let conn = pg_connection_read(self).await?;
        let customer_list_constraints =
            diesel_models::query::customers::CustomerListConstraints::from(constraints);
        self.find_resources(
            state,
            key_store,
            customers::Customer::list_by_merchant_id(&conn, merchant_id, customer_list_constraints),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        let customer_new = customer_data
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?;
        self.call_database(state, key_store, customer_new.insert(&conn))
            .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    #[instrument(skip_all)]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        let conn = pg_connection_write(self).await?;
        customers::Customer::delete_by_customer_id_merchant_id(&conn, customer_id, merchant_id)
            .await
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        customer: domain::Customer,
        merchant_id: &id_type::MerchantId,
        customer_update: domain::CustomerUpdate,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            customers::Customer::update_by_id(&conn, id.clone(), customer_update.into()),
        )
        .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn find_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let customer: domain::Customer = self
            .call_database(
                state,
                key_store,
                customers::Customer::find_by_global_id(&conn, id),
            )
            .await?;
        match customer.name {
            Some(ref name) if name.peek() == pii::REDACTED => Err(StorageError::CustomerRedacted)?,
            _ => Ok(customer),
        }
    }
}

#[async_trait::async_trait]
impl domain::CustomerInterface for MockDb {
    type Error = StorageError;
    #[allow(clippy::panic)]
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let customers = self.customers.lock().await;
        self.find_resource(state, key_store, customers, |customer| {
            customer.customer_id == *customer_id && &customer.merchant_id == merchant_id
        })
        .await
    }

    #[allow(clippy::panic)]
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_with_redacted_customer_details_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        let customers = self.customers.lock().await;
        self.find_resource(state, key_store, customers, |customer| {
            customer.customer_id == *customer_id && &customer.merchant_id == merchant_id
        })
        .await
    }

    #[allow(clippy::panic)]
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, StorageError> {
        todo!()
    }

    async fn list_customers_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        key_store: &MerchantKeyStore,
        constraints: domain::CustomerListConstraints,
    ) -> CustomResult<Vec<domain::Customer>, StorageError> {
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
                        .change_context(StorageError::DecryptionError)
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
        _customer: domain::Customer,
        _customer_update: domain::CustomerUpdate,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_by_customer_id_merchant_id(
        &self,
        _state: &KeyManagerState,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_merchant_reference_id_merchant_id(
        &self,
        _state: &KeyManagerState,
        _merchant_reference_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_customer(
        &self,
        customer_data: domain::Customer,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let mut customers = self.customers.lock().await;

        let customer = Conversion::convert(customer_data)
            .await
            .change_context(StorageError::EncryptionError)?;

        customers.push(customer.clone());

        customer
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        _state: &KeyManagerState,
        _id: &id_type::GlobalCustomerId,
        _customer: domain::Customer,
        _merchant_id: &id_type::MerchantId,
        _customer_update: domain::CustomerUpdate,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_customer_by_global_id(
        &self,
        _state: &KeyManagerState,
        _id: &id_type::GlobalCustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
}
