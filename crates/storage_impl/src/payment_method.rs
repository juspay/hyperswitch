pub use diesel_models::payment_method::PaymentMethod;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for PaymentMethod {}

use common_enums::enums::MerchantStorageScheme;
use common_utils::{errors::CustomResult, id_type};
#[cfg(feature = "v1")]
use diesel_models::kv;
use diesel_models::payment_method::{PaymentMethodUpdate, PaymentMethodUpdateInternal};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::behaviour::ReverseConversion;
use hyperswitch_domain_models::{
    behaviour::Conversion,
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

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentMethodInterface for KVRouterStore<T> {
    type Error = errors::StorageError;
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resource_by_id(
            key_store,
            storage_scheme,
            PaymentMethod::find_by_payment_method_id(&conn, payment_method_id),
            FindResourceBy::LookupId(format!("payment_method_{payment_method_id}")),
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resource_by_id(
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
        key_store: &MerchantKeyStore,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resource_by_id(
            key_store,
            storage_scheme,
            PaymentMethod::find_by_locker_id(&conn, locker_id),
            FindResourceBy::LookupId(format!("payment_method_locker_{locker_id}")),
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .insert_payment_method(key_store, payment_method, storage_scheme)
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn insert_payment_method(
        &self,
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
            reverse_lookups.push(format!("payment_method_locker_{locker_id}"))
        }
        let payment_method = (&payment_method_new.clone()).into();
        self.insert_resource(
            key_store,
            storage_scheme,
            payment_method_new.clone().insert(&conn),
            payment_method,
            InsertResourceParams {
                insertable: kv::Insertable::PaymentMethod(Box::new(payment_method_new.clone())),
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .update_payment_method(
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
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_by_customer_id_merchant_id_list(
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
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_list_by_global_customer_id(key_store, customer_id, limit)
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.filter_resources(
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
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_by_global_customer_id_merchant_id_status(
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
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .delete_payment_method_by_merchant_id_payment_method_id(
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .delete_payment_method(key_store, payment_method)
            .await
    }

    // Check if KV stuff is needed here
    #[cfg(feature = "v2")]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .find_payment_method_by_fingerprint_id(key_store, fingerprint_id)
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
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            key_store,
            PaymentMethod::find_by_payment_method_id(&conn, payment_method_id),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            key_store,
            PaymentMethod::find_by_id(&conn, payment_method_id),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_locker_id(
        &self,
        key_store: &MerchantKeyStore,
        locker_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method_new = payment_method
            .construct_new()
            .await
            .change_context(errors::StorageError::DecryptionError)?;

        let conn = pg_connection_write(self).await?;
        self.call_database(key_store, payment_method_new.insert(&conn))
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_method(
        &self,
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
            key_store,
            payment_method.update_with_payment_method_id(&conn, payment_method_update.into()),
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_method(
        &self,
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
            key_store,
            payment_method.update_with_id(&conn, payment_method_update.into()),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
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
        key_store: &MerchantKeyStore,
        id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            key_store,
            PaymentMethod::find_by_global_customer_id(&conn, id, limit),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
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
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
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
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        self.call_database(
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        let conn = pg_connection_write(self).await?;
        let payment_method_update = PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
            last_modified_by: None,
        };
        self.call_database(
            key_store,
            payment_method.update_with_id(&conn, payment_method_update.into()),
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
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
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
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
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
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
        key_store: &MerchantKeyStore,
        locker_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
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
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resources(
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
        _key_store: &MerchantKeyStore,
        _id: &id_type::GlobalCustomerId,
        _limit: Option<i64>,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resources(
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
        self.get_resources(key_store, payment_methods, find_pm_by, error_message)
            .await
    }

    #[cfg(feature = "v1")]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
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
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
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
        key_store: &MerchantKeyStore,
        payment_method: DomainPaymentMethod,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method_update = PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
            last_modified_by: None,
        };
        let payment_method_updated = PaymentMethodUpdateInternal::from(payment_method_update)
            .apply_changeset(
                Conversion::convert(payment_method.clone())
                    .await
                    .change_context(errors::StorageError::EncryptionError)?,
            );
        self.update_resource::<PaymentMethod, _>(
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
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource::<PaymentMethod, _>(
            key_store,
            payment_methods,
            |pm| pm.locker_fingerprint_id == Some(fingerprint_id.to_string()),
            "cannot find payment method".to_string(),
        )
        .await
    }
}
