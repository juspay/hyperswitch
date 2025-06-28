pub use diesel_models::payment_method::PaymentMethod;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for PaymentMethod {}

use common_enums::enums::MerchantStorageScheme;
use common_utils::{errors::CustomResult, id_type, types::keymanager::KeyManagerState};
#[cfg(feature = "v1")]
use diesel_models::kv;
use diesel_models::payment_method::{PaymentMethodUpdate, PaymentMethodUpdateInternal};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::behaviour::ReverseConversion;
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    payment_methods::{PaymentMethod as DomainPaymentMethod, PaymentMethodInterface},
};
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    diesel_error_to_data_error, errors,
    kv_router_store::{FindResourceBy, KVRouterStore},
    utils::{pg_connection_read, pg_connection_write},
    DatabaseStore, RouterStore,
};
#[cfg(feature = "v1")]
use crate::{
    kv_router_store::{FilterResourceParams, InsertResourceParams, UpdateResourceParams},
    redis::kv_store::{Op, PartitionKey},
};

use crate::behaviour::Conversion;
use common_utils::errors::ValidationError;
use common_utils::types::keymanager;
use masking::Secret;
use hyperswitch_domain_models::type_encryption::crypto_operation;
use common_utils::type_name;
use hyperswitch_domain_models::type_encryption::CryptoOperation;
use hyperswitch_domain_models::payment_methods::EncryptedPaymentMethod;
use hyperswitch_domain_models::payment_methods::VaultId;

use hyperswitch_domain_models::payment_methods::PaymentMethodSession;
use hyperswitch_domain_models::payment_methods::EncryptedPaymentMethodSession;



#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentMethodInterface for KVRouterStore<T> {
    type Error = errors::StorageError;
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resource_by_id(
            state,
            key_store,
            storage_scheme,
            PaymentMethod::find_by_payment_method_id(&conn, payment_method_id),
            FindResourceBy::LookupId(format!("payment_method_{}", payment_method_id)),
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resource_by_id(
            state,
            key_store,
            storage_scheme,
            PaymentMethod::find_by_id(&conn, payment_method_id),
            FindResourceBy::LookupId(format!(
                "payment_method_{}",
                payment_method_id.get_string_repr()
            )),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resource_by_id(
            state,
            key_store,
            storage_scheme,
            PaymentMethod::find_by_locker_id(&conn, locker_id),
            FindResourceBy::LookupId(format!("payment_method_locker_{}", locker_id)),
        )
        .await
    }

    // not supported in kv
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        self.router_store
            .get_payment_method_count_by_customer_id_merchant_id_status(
                customer_id,
                merchant_id,
                status,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn get_payment_method_count_by_merchant_id_status(
        &self,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        self.router_store
            .get_payment_method_count_by_merchant_id_status(merchant_id, status)
            .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn insert_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .insert_payment_method(state, key_store, payment_method, storage_scheme)
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn insert_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        let mut payment_method_new = payment_method
            .construct_new()
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        payment_method_new.update_storage_scheme(storage_scheme);

        let key = PartitionKey::MerchantIdCustomerId {
            merchant_id: &payment_method_new.merchant_id.clone(),
            customer_id: &payment_method_new.customer_id.clone(),
        };
        let identifier = format!("payment_method_id_{}", payment_method_new.get_id());
        let lookup_id1 = format!("payment_method_{}", payment_method_new.get_id());
        let mut reverse_lookups = vec![lookup_id1];
        if let Some(locker_id) = &payment_method_new.locker_id {
            reverse_lookups.push(format!("payment_method_locker_{}", locker_id))
        }
        let payment_method = (&payment_method_new.clone()).into();
        self.insert_resource(
            state,
            key_store,
            storage_scheme,
            payment_method_new.clone().insert(&conn),
            payment_method,
            InsertResourceParams {
                insertable: kv::Insertable::PaymentMethod(payment_method_new.clone()),
                reverse_lookups,
                key,
                identifier,
                resource_type: "payment_method",
            },
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {

        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        let merchant_id = payment_method.merchant_id.clone();
        let customer_id = payment_method.customer_id.clone();
        let key = PartitionKey::MerchantIdCustomerId {
            merchant_id: &merchant_id,
            customer_id: &customer_id,
        };
        let conn = pg_connection_write(self).await?;
        let field = format!("payment_method_id_{}", payment_method.get_id().clone());
        let p_update: PaymentMethodUpdateInternal =
            payment_method_update.convert_to_payment_method_update(storage_scheme);
        let updated_payment_method = p_update.clone().apply_changeset(payment_method.clone());
        self.update_resource(
            state,
            key_store,
            storage_scheme,
            payment_method
                .clone()
                .update_with_payment_method_id(&conn, p_update.clone()),
            updated_payment_method,
            UpdateResourceParams {
                updateable: kv::Updateable::PaymentMethodUpdate(Box::new(
                    kv::PaymentMethodUpdateMems {
                        orig: payment_method.clone(),
                        update_data: p_update.clone(),
                    },
                )),
                operation: Op::Update(
                    key.clone(),
                    &field,
                    payment_method.clone().updated_by.as_deref(),
                ),
            },
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .update_payment_method(
                state,
                key_store,
                payment_method,
                payment_method_update,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_by_customer_id_merchant_id_list(
                state,
                key_store,
                customer_id,
                merchant_id,
                limit,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_list_by_global_customer_id(state, key_store, customer_id, limit)
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.filter_resources(
            state,
            key_store,
            storage_scheme,
            PaymentMethod::find_by_customer_id_merchant_id_status(
                &conn,
                customer_id,
                merchant_id,
                status,
                limit,
            ),
            |pm| pm.status == status,
            FilterResourceParams {
                key: PartitionKey::MerchantIdCustomerId {
                    merchant_id,
                    customer_id,
                },
                pattern: "payment_method_id_*",
                limit,
            },
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_by_global_customer_id_merchant_id_status(
                state,
                key_store,
                customer_id,
                merchant_id,
                status,
                limit,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .delete_payment_method_by_merchant_id_payment_method_id(
                state,
                key_store,
                merchant_id,
                payment_method_id,
            )
            .await
    }

    // Soft delete, Check if KV stuff is needed here
    #[cfg(feature = "v2")]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .delete_payment_method(state, key_store, payment_method)
            .await
    }

    // Check if KV stuff is needed here
    #[cfg(feature = "v2")]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .find_payment_method_by_fingerprint_id(state, key_store, fingerprint_id)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentMethodInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            PaymentMethod::find_by_payment_method_id(&conn, payment_method_id),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            PaymentMethod::find_by_id(&conn, payment_method_id),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        locker_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            PaymentMethod::find_by_locker_id(&conn, locker_id),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        PaymentMethod::get_count_by_customer_id_merchant_id_status(
            &conn,
            customer_id,
            merchant_id,
            status,
        )
        .await
        .map_err(|error| {
            let new_err = diesel_error_to_data_error(*error.current_context());
            error.change_context(new_err)
        })
    }

    #[instrument(skip_all)]
    async fn get_payment_method_count_by_merchant_id_status(
        &self,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        PaymentMethod::get_count_by_merchant_id_status(&conn, merchant_id, status)
            .await
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
    }

    #[instrument(skip_all)]
    async fn insert_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method_new = payment_method
            .construct_new()
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        let conn = pg_connection_write(self).await?;
        self.call_database(state, key_store, payment_method_new.insert(&conn))
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            payment_method.update_with_payment_method_id(&conn, payment_method_update.into()),
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {

        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            payment_method.update_with_id(&conn, payment_method_update.into()),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            PaymentMethod::find_by_customer_id_merchant_id(&conn, customer_id, merchant_id, limit),
        )
        .await
    }

    // Need to fix this once we move to payment method for customer
    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            PaymentMethod::find_by_global_customer_id(&conn, id, limit),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            PaymentMethod::find_by_customer_id_merchant_id_status(
                &conn,
                customer_id,
                merchant_id,
                status,
                limit,
            ),
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            PaymentMethod::find_by_global_customer_id_merchant_id_status(
                &conn,
                customer_id,
                merchant_id,
                status,
                limit,
            ),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            PaymentMethod::delete_by_merchant_id_payment_method_id(
                &conn,
                merchant_id,
                payment_method_id,
            ),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        let conn = pg_connection_write(self).await?;
        let payment_method_update = PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
        };
        self.call_database(
            state,
            key_store,
            payment_method.update_with_id(&conn, payment_method_update.into()),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            PaymentMethod::find_by_fingerprint_id(&conn, fingerprint_id),
        )
        .await
    }
}

#[async_trait::async_trait]
impl PaymentMethodInterface for MockDb {
    type Error = errors::StorageError;
    #[cfg(feature = "v1")]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
            state,
            key_store,
            payment_methods,
            |pm| pm.get_id() == payment_method_id,
            "cannot find payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
            state,
            key_store,
            payment_methods,
            |pm| pm.get_id() == payment_method_id,
            "cannot find payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        locker_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
            state,
            key_store,
            payment_methods,
            |pm| pm.locker_id == Some(locker_id.to_string()),
            "cannot find payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v1")]
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

    async fn get_payment_method_count_by_merchant_id_status(
        &self,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let count = payment_methods
            .iter()
            .filter(|pm| pm.merchant_id == *merchant_id && pm.status == status)
            .count();
        i64::try_from(count).change_context(errors::StorageError::MockDbError)
    }

    async fn insert_payment_method(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let mut payment_methods = self.payment_methods.lock().await;

        let pm = Conversion::convert(payment_method.clone())
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        payment_methods.push(pm);
        Ok(payment_method)
    }

    #[cfg(feature = "v1")]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resources(
            state,
            key_store,
            payment_methods,
            |pm| pm.customer_id == *customer_id && pm.merchant_id == *merchant_id,
            "cannot find payment method".to_string(),
        )
        .await
    }

    // Need to fix this once we complete v2 payment method
    #[cfg(feature = "v2")]
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        _state: &KeyManagerState,
        _key_store: &MerchantKeyStore,
        _id: &id_type::GlobalCustomerId,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resources(
            state,
            key_store,
            payment_methods,
            |pm| {
                pm.customer_id == *customer_id
                    && pm.merchant_id == *merchant_id
                    && pm.status == status
            },
            "cannot find payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let find_pm_by = |pm: &&PaymentMethod| {
            pm.customer_id == *customer_id && pm.merchant_id == *merchant_id && pm.status == status
        };
        let error_message = "cannot find payment method".to_string();
        self.get_resources(state, key_store, payment_methods, find_pm_by, error_message)
            .await
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method_updated = PaymentMethodUpdateInternal::from(payment_method_update)
            .apply_changeset(
                Conversion::convert(payment_method.clone())
                    .await
                    .change_context(errors::StorageError::EncryptionError)?,
            );
        self.update_resource::<PaymentMethod, _>(
            state,
            key_store,
            self.payment_methods.lock().await,
            payment_method_updated,
            |pm| pm.get_id() == payment_method.get_id(),
            "cannot update payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method_update = PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
        };
        let payment_method_updated = PaymentMethodUpdateInternal::from(payment_method_update)
            .apply_changeset(
                Conversion::convert(payment_method.clone())
                    .await
                    .change_context(errors::StorageError::EncryptionError)?,
            );
        self.update_resource::<PaymentMethod, _>(
            state,
            key_store,
            self.payment_methods.lock().await,
            payment_method_updated,
            |pm| pm.get_id() == payment_method.get_id(),
            "cannot find payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
            state,
            key_store,
            payment_methods,
            |pm| pm.locker_fingerprint_id == Some(fingerprint_id.to_string()),
            "cannot find payment method".to_string(),
        )
        .await
    }
}


#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for hyperswitch_domain_models::payment_methods::PaymentMethod {
    type DstType = diesel_models::payment_method::PaymentMethod;
    type NewDstType = diesel_models::payment_method::PaymentMethodNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            id: self.id,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method_type_v2: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id.map(|id| id.get_string_repr().clone()),
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details.map(|cmd| cmd.into()),
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            locker_fingerprint_id: self.locker_fingerprint_id,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source: self.external_vault_source,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        use common_utils::ext_traits::ValueExt;

        async {
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentMethod::to_encryptable(
                    EncryptedPaymentMethod {
                        payment_method_data: storage_model.payment_method_data,
                        payment_method_billing_address: storage_model
                            .payment_method_billing_address,
                        network_token_payment_method_data: storage_model
                            .network_token_payment_method_data,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentMethod::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let payment_method_billing_address = data
                .payment_method_billing_address
                .map(|billing| {
                    billing.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            let payment_method_data = data
                .payment_method_data
                .map(|payment_method_data| {
                    payment_method_data
                        .deserialize_inner_value(|value| value.parse_value("Payment Method Data"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Payment Method Data")?;

            let network_token_payment_method_data = data
                .network_token_payment_method_data
                .map(|network_token_payment_method_data| {
                    network_token_payment_method_data.deserialize_inner_value(|value| {
                        value.parse_value("Network token Payment Method Data")
                    })
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Network token Payment Method Data")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                customer_id: storage_model.customer_id,
                merchant_id: storage_model.merchant_id,
                id: storage_model.id,
                created_at: storage_model.created_at,
                last_modified: storage_model.last_modified,
                payment_method_type: storage_model.payment_method_type_v2,
                payment_method_subtype: storage_model.payment_method_subtype,
                payment_method_data,
                locker_id: storage_model.locker_id.map(VaultId::generate),
                last_used_at: storage_model.last_used_at,
                connector_mandate_details: storage_model.connector_mandate_details.map(From::from),
                customer_acceptance: storage_model.customer_acceptance,
                status: storage_model.status,
                network_transaction_id: storage_model.network_transaction_id,
                client_secret: storage_model.client_secret,
                payment_method_billing_address,
                updated_by: storage_model.updated_by,
                locker_fingerprint_id: storage_model.locker_fingerprint_id,
                version: storage_model.version,
                network_token_requestor_reference_id: storage_model
                    .network_token_requestor_reference_id,
                network_token_locker_id: storage_model.network_token_locker_id,
                network_token_payment_method_data,
                external_vault_source: storage_model.external_vault_source,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            id: self.id,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method_type_v2: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id.map(|id| id.get_string_repr().clone()),
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details.map(|cmd| cmd.into()),
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            locker_fingerprint_id: self.locker_fingerprint_id,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
        })
    }
}




#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for PaymentMethodSession {
    type DstType = diesel_models::payment_methods_session::PaymentMethodSession;
    type NewDstType = diesel_models::payment_methods_session::PaymentMethodSession;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            id: self.id,
            customer_id: self.customer_id,
            billing: self.billing.map(|val| val.into()),
            psp_tokenization: self.psp_tokenization,
            network_tokenization: self.network_tokenization,
            tokenization_data: self.tokenization_data,
            expires_at: self.expires_at,
            associated_payment_methods: self.associated_payment_methods,
            associated_payment: self.associated_payment,
            return_url: self.return_url,
            associated_token_id: self.associated_token_id,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        use common_utils::ext_traits::ValueExt;

        async {

            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentMethodSession::to_encryptable(
                    EncryptedPaymentMethodSession {
                        billing: storage_model.billing,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentMethodSession::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let billing = data
                .billing
                .map(|billing| {
                    billing.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                id: storage_model.id,
                customer_id: storage_model.customer_id,
                billing,
                psp_tokenization: storage_model.psp_tokenization,
                network_tokenization: storage_model.network_tokenization,
                tokenization_data: storage_model.tokenization_data,
                expires_at: storage_model.expires_at,
                associated_payment_methods: storage_model.associated_payment_methods,
                associated_payment: storage_model.associated_payment,
                return_url: storage_model.return_url,
                associated_token_id: storage_model.associated_token_id,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            id: self.id,
            customer_id: self.customer_id,
            billing: self.billing.map(|val| val.into()),
            psp_tokenization: self.psp_tokenization,
            network_tokenization: self.network_tokenization,
            tokenization_data: self.tokenization_data,
            expires_at: self.expires_at,
            associated_payment_methods: self.associated_payment_methods,
            associated_payment: self.associated_payment,
            return_url: self.return_url,
            associated_token_id: self.associated_token_id,
        })
    }
}
