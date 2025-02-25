use diesel_models::customers;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for customers::Customer {}

// #[cfg(not(feature = "kv_store"))]
// mod storage {
use common_utils::{
    errors::CustomResult,
    ext_traits::AsyncExt,
    id_type,
    types::keymanager::KeyManagerState,
    pii::REDACTED,
};
use diesel_models::{customers as storage, enums};
use error_stack::{report, ResultExt};
use futures::future::try_join_all;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    customer as domain, merchant_key_store,
};
use masking::PeekInterface;
use router_env::{instrument, tracing};
use sample::customers::{CustomerInterface, CustomerListConstraints};

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> CustomerInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn find_customer_optional_by_customer_id_merchant_id(
        &self,
        state: &KeyManagerState,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let maybe_customer: Option<domain::Customer> =
            storage::Customer::find_optional_by_customer_id_merchant_id(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let maybe_customer: Option<domain::Customer> =
            storage::Customer::find_optional_by_customer_id_merchant_id(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Option<domain::Customer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let maybe_customer: Option<domain::Customer> =
            storage::Customer::find_optional_by_customer_id_merchant_id(
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
        _customer: domain::Customer,
        customer_update: domain::CustomerUpdate,
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Customer::update_by_customer_id_merchant_id(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let customer: domain::Customer =
            storage::Customer::find_by_customer_id_merchant_id(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let customer: domain::Customer =
            storage::Customer::find_by_merchant_reference_id_merchant_id(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        constraints: CustomerListConstraints,
    ) -> CustomResult<Vec<domain::Customer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let customer_list_constraints =
            diesel_models::query::customers::CustomerListConstraints::from(constraints);

        let encrypted_customers =
            storage::Customer::list_by_merchant_id(&conn, merchant_id, customer_list_constraints)
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
        customer_data: domain::Customer,
        state: &KeyManagerState,
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
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
        storage::Customer::delete_by_customer_id_merchant_id(
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
        customer: domain::Customer,
        merchant_id: &id_type::MerchantId,
        customer_update: storage::CustomerUpdate,
        key_store: &merchant_key_store::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Customer::update_by_global_id(&conn, id, customer_update.into())
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
        key_store: &merchant_key_store::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Customer, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let customer: domain::Customer =
            storage::Customer::find_by_global_id(&conn, customer_id, merchant_id)
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
// }
