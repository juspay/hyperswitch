use common_utils::{id_type, pii};
use diesel_models::{customers as storage, kv};
use error_stack::ResultExt;
use futures::future::try_join_all;
use hyperswitch_domain_models::{customer as domain, merchant_key_store::MerchantKeyStore};
use masking::PeekInterface;
use router_env::{instrument, tracing};

#[cfg(feature = "v1")]
use crate::diesel_error_to_data_error;
use crate::{
    behaviour::{Conversion, ReverseConversion},
    errors::StorageError,
    kv_router_store,
    redis::kv_store::{decide_storage_scheme, KvStorePartition, Op, PartitionKey},
    store::enums::MerchantStorageScheme,
    utils::{pg_connection_read, pg_connection_write, ForeignFrom},
    CustomResult, DatabaseStore, KeyManagerState, MockDb, RouterStore,
};
use crate::utils::ForeignInto;
use common_utils::encryption::Encryption;
use common_utils::errors::ValidationError;
use common_utils::types::keymanager;
use hyperswitch_domain_models::type_encryption::crypto_operation;
use hyperswitch_domain_models::type_encryption::CryptoOperation;
use masking::Secret;
use common_utils::types::keymanager::ToEncryptable;
use masking::SwitchStrategy;



use common_utils::crypto::Encryptable;
use common_utils::date_time;

impl KvStorePartition for storage::Customer {}

#[cfg(feature = "v2")]
mod label {
    use common_utils::id_type;

    pub(super) const MODEL_NAME: &str = "customer_v2";
    pub(super) const CLUSTER_LABEL: &str = "cust";

    pub(super) fn get_global_id_label(global_customer_id: &id_type::GlobalCustomerId) -> String {
        format!(
            "customer_global_id_{}",
            global_customer_id.get_string_repr()
        )
    }

    pub(super) fn get_merchant_scoped_id_label(
        merchant_id: &id_type::MerchantId,
        merchant_reference_id: &id_type::CustomerId,
    ) -> String {
        format!(
            "customer_mid_{}_mrefid_{}",
            merchant_id.get_string_repr(),
            merchant_reference_id.get_string_repr()
        )
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> domain::CustomerInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    // check customer not found in kv and fallback to db
    #[cfg(feature = "v1")]
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
                storage::Customer::find_optional_by_customer_id_merchant_id(
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
    #[cfg(feature = "v1")]
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
            storage::Customer::find_optional_by_customer_id_merchant_id(
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

    #[cfg(feature = "v2")]
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
                storage::Customer::find_optional_by_merchant_id_merchant_reference_id(
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

    #[cfg(feature = "v1")]
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
            storage::Customer::update_by_customer_id_merchant_id(
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

    #[cfg(feature = "v2")]
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
                storage::Customer::find_by_merchant_reference_id_merchant_id(
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

    #[cfg(feature = "v1")]
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
                storage::Customer::find_by_customer_id_merchant_id(
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

    #[cfg(feature = "v2")]
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

        let decided_storage_scheme = Box::pin(decide_storage_scheme::<_, storage::Customer>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        new_customer.update_storage_scheme(decided_storage_scheme);

        let mut reverse_lookups = Vec::new();

        if let Some(ref merchant_ref_id) = new_customer.merchant_reference_id {
            let reverse_lookup_merchant_scoped_id =
                label::get_merchant_scoped_id_label(&new_customer.merchant_id, merchant_ref_id);
            reverse_lookups.push(reverse_lookup_merchant_scoped_id);
        }

        self.insert_resource(
            state,
            key_store,
            decided_storage_scheme,
            new_customer.clone().insert(&conn),
            new_customer.clone().into(),
            kv_router_store::InsertResourceParams {
                insertable: kv::Insertable::Customer(new_customer.clone()),
                reverse_lookups,
                identifier,
                key,
                resource_type: "customer",
            },
        )
        .await
    }

    #[cfg(feature = "v1")]
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
        let storage_scheme = Box::pin(decide_storage_scheme::<_, storage::Customer>(
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let result: domain::Customer = self
            .find_resource_by_id(
                state,
                key_store,
                storage_scheme,
                storage::Customer::find_by_global_id(&conn, id),
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

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        customer: domain::Customer,
        customer_update: domain::CustomerUpdate,
        key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {

        let conn = pg_connection_write(self).await?;
        let customer = Conversion::convert(customer)
            .await
            .change_context(StorageError::EncryptionError)?;
        let database_call =
            storage::Customer::update_by_id(&conn, id.clone(), customer_update.clone().foreign_into());
        let key = PartitionKey::GlobalId {
            id: id.get_string_repr(),
        };
        let field = format!("cust_{}", id.get_string_repr());
        self.update_resource(
            state,
            key_store,
            storage_scheme,
            database_call,
            diesel_models::CustomerUpdateInternal::foreign_from(customer_update.clone())
                .apply_changeset(customer.clone()),
            kv_router_store::UpdateResourceParams {
                updateable: kv::Updateable::CustomerUpdate(kv::CustomerUpdateMems {
                    orig: customer.clone(),
                    update_data: customer_update.foreign_into(),
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
    #[cfg(feature = "v1")]
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
                storage::Customer::find_optional_by_customer_id_merchant_id(
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
    #[cfg(feature = "v1")]
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
            storage::Customer::find_optional_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
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
                storage::Customer::find_optional_by_merchant_id_merchant_reference_id(
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

    #[cfg(feature = "v1")]
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
            storage::Customer::update_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id.clone(),
                customer_update.into(),
            ),
        )
        .await
    }

    #[cfg(feature = "v1")]
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
                storage::Customer::find_by_customer_id_merchant_id(
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

    #[cfg(feature = "v2")]
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
                storage::Customer::find_by_merchant_reference_id_merchant_id(
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
            diesel_models::query::customers::CustomerListConstraints::foreign_from(constraints);
        self.find_resources(
            state,
            key_store,
            storage::Customer::list_by_merchant_id(&conn, merchant_id, customer_list_constraints),
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        _customer: domain::Customer,
        customer_update: domain::CustomerUpdate,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            storage::Customer::update_by_id(&conn, id.clone(), customer_update.foreign_into()),
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_customer_by_global_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::GlobalCustomerId,
        key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        let conn = pg_connection_read(self).await?;
        let customer: domain::Customer = self
            .call_database(
                state,
                key_store,
                storage::Customer::find_by_global_id(&conn, id),
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
    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v2")]
    async fn find_optional_by_merchant_id_merchant_reference_id(
        &self,
        _state: &KeyManagerState,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
        _key_store: &MerchantKeyStore,
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v2")]
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

    #[cfg(feature = "v1")]
    async fn delete_customer_by_customer_id_merchant_id(
        &self,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn update_customer_by_global_id(
        &self,
        _state: &KeyManagerState,
        _id: &id_type::GlobalCustomerId,
        _customer: domain::Customer,
        _customer_update: domain::CustomerUpdate,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v2")]
    async fn find_customer_by_global_id(
        &self,
        _state: &KeyManagerState,
        _id: &id_type::GlobalCustomerId,
        _key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(StorageError::MockDbError)?
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for domain::Customer {
    type DstType = storage::Customer;
    type NewDstType = storage::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(storage::Customer {
            id: self.id,
            merchant_reference_id: self.merchant_reference_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            phone_country_code: self.phone_country_code,
            description: self.description,
            created_at: self.created_at,
            metadata: self.metadata,
            modified_at: self.modified_at,
            connector_customer: self.connector_customer,
            default_payment_method_id: self.default_payment_method_id,
            updated_by: self.updated_by,
            default_billing_address: self.default_billing_address,
            default_shipping_address: self.default_shipping_address,
            version: self.version,
            status: self.status,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_store_ref_id: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let decrypted = crypto_operation(
            state,
            common_utils::type_name!(Self::DstType),
            CryptoOperation::BatchDecrypt(domain::EncryptedCustomer::to_encryptable(
                domain::EncryptedCustomer {
                    name: item.name.clone(),
                    phone: item.phone.clone(),
                    email: item.email.clone(),
                },
            )),
            keymanager::Identifier::Merchant(item.merchant_id.clone()),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
        })?;
        let encryptable_customer = domain::EncryptedCustomer::from_encryptable(decrypted)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting customer data".to_string(),
            })?;

        Ok(Self {
            id: item.id,
            merchant_reference_id: item.merchant_reference_id,
            merchant_id: item.merchant_id,
            name: encryptable_customer.name,
            email: encryptable_customer.email.map(|email| {

                let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
            phone: encryptable_customer.phone,
            phone_country_code: item.phone_country_code,
            description: item.description,
            created_at: item.created_at,
            metadata: item.metadata,
            modified_at: item.modified_at,
            connector_customer: item.connector_customer,
            default_payment_method_id: item.default_payment_method_id,
            updated_by: item.updated_by,
            default_billing_address: item.default_billing_address,
            default_shipping_address: item.default_shipping_address,
            version: item.version,
            status: item.status,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(storage::CustomerNew {
            id: self.id,
            merchant_reference_id: self.merchant_reference_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            description: self.description,
            phone_country_code: self.phone_country_code,
            metadata: self.metadata,
            default_payment_method_id: None,
            created_at: now,
            modified_at: now,
            connector_customer: self.connector_customer,
            updated_by: self.updated_by,
            default_billing_address: self.default_billing_address,
            default_shipping_address: self.default_shipping_address,
            version: common_utils::consts::API_VERSION,
            status: self.status,
        })
    }
}

#[cfg(feature = "v2")]
impl ForeignFrom<domain::CustomerUpdate> for storage::CustomerUpdateInternal {
    fn foreign_from(customer_update: domain::CustomerUpdate) -> Self {
        match customer_update {
            domain::CustomerUpdate::Update(update) => {
                let domain::CustomerGeneralUpdate {
                    name,
                    email,
                    phone,
                    description,
                    phone_country_code,
                    metadata,
                    connector_customer,
                    default_billing_address,
                    default_shipping_address,
                    default_payment_method_id,
                    status,
                } = *update;
                Self {
                    name: name.map(Encryption::from),
                    email: email.map(Encryption::from),
                    phone: phone.map(Encryption::from),
                    description,
                    phone_country_code,
                    metadata,
                    connector_customer: *connector_customer,
                    modified_at: date_time::now(),
                    default_billing_address,
                    default_shipping_address,
                    default_payment_method_id,
                    updated_by: None,
                    status,
                }
            }
            domain::CustomerUpdate::ConnectorCustomer { connector_customer } => Self {
                connector_customer,
                name: None,
                email: None,
                phone: None,
                description: None,
                phone_country_code: None,
                metadata: None,
                modified_at: date_time::now(),
                default_payment_method_id: None,
                updated_by: None,
                default_billing_address: None,
                default_shipping_address: None,
                status: None,
            },
            domain::CustomerUpdate::UpdateDefaultPaymentMethod {
                default_payment_method_id,
            } => Self {
                default_payment_method_id,
                modified_at: date_time::now(),
                name: None,
                email: None,
                phone: None,
                description: None,
                phone_country_code: None,
                metadata: None,
                connector_customer: None,
                updated_by: None,
                default_billing_address: None,
                default_shipping_address: None,
                status: None,
            },
        }
    }
}

impl ForeignFrom<domain::CustomerListConstraints>
    for diesel_models::query::customers::CustomerListConstraints
{
    fn foreign_from(value: domain::CustomerListConstraints) -> Self {
        Self {
            limit: i64::from(value.limit),
            offset: value.offset.map(i64::from),
        }
    }
}
