use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for diesel_models::PaymentMethod {}

// #[cfg(not(feature = "kv_store"))]
// mod storage {

use common_utils::{id_type, errors::CustomResult, types::keymanager::KeyManagerState};
use diesel_models::{enums, payment_method as storage};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store, payment_methods as domain,
};
use router_env::{instrument, tracing};
use sample::payment_method::PaymentMethodInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentMethodInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &merchant_key_store::MerchantKeyStore,
        payment_method_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id)
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
        key_store: &merchant_key_store::MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_id(&conn, payment_method_id)
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
        key_store: &merchant_key_store::MerchantKeyStore,
        locker_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_locker_id(&conn, locker_id)
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
        storage::PaymentMethod::get_count_by_customer_id_merchant_id_status(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        _storage_scheme: enums::MerchantStorageScheme,
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
        key_store: &merchant_key_store::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
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
        key_store: &merchant_key_store::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
        payment_method_update: storage::PaymentMethodUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
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
        key_store: &merchant_key_store::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let payment_methods = storage::PaymentMethod::find_by_customer_id_merchant_id(
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
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        state: &KeyManagerState,
        key_store: &merchant_key_store::MerchantKeyStore,
        id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let payment_methods =
            storage::PaymentMethod::find_by_global_customer_id(&conn, customer_id, limit)
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
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &merchant_key_store::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let payment_methods =
            storage::PaymentMethod::find_by_customer_id_merchant_id_status(
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

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[instrument(skip_all)]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &merchant_key_store::MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<domain::PaymentMethod>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let payment_methods =
            storage::PaymentMethod::find_by_global_customer_id_merchant_id_status(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::PaymentMethod::delete_by_merchant_id_payment_method_id(
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
        key_store: &merchant_key_store::MerchantKeyStore,
        payment_method: domain::PaymentMethod,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        let conn = connection::pg_connection_write(self).await?;
        let payment_method_update = storage::PaymentMethodUpdate::StatusUpdate {
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
        key_store: &merchant_key_store::MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<domain::PaymentMethod, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentMethod::find_by_fingerprint_id(&conn, fingerprint_id)
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
// }
