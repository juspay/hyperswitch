// use diesel_models::payment_method::PaymentMethod;

use crate::redis::kv_store::KvStorePartition;

impl KvStorePartition for diesel_models::payment_method::PaymentMethod {}

#[cfg(feature = "v1")]
use std::collections::HashSet;

use common_enums::enums::MerchantStorageScheme;
use common_utils::{errors::CustomResult, id_type};
#[cfg(feature = "v1")]
use diesel_models::kv;
use diesel_models::payment_method::{PaymentMethodUpdate, PaymentMethodUpdateInternal};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payment_methods::PaymentMethodVaultSourceDetails;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::platform::Initiator;
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    payment_methods::{PaymentMethod as DomainPaymentMethod, PaymentMethodInterface},
};
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    behaviour::{Conversion, ReverseConversion},
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
            diesel_models::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id),
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
            diesel_models::PaymentMethod::find_by_id(&conn, payment_method_id),
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
            diesel_models::PaymentMethod::find_by_locker_id(&conn, locker_id),
            FindResourceBy::LookupId(format!("payment_method_locker_{locker_id}")),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_methods_by_merchant_id_payment_method_ids(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_ids: &[String],
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_methods_by_merchant_id_payment_method_ids(
                key_store,
                merchant_id,
                payment_method_ids,
                storage_scheme,
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
        Box::pin(
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
            ),
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
            diesel_models::PaymentMethod::find_by_customer_id_merchant_id_status(
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

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status_pm_type(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        payment_method_type: common_enums::PaymentMethodType,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.filter_resources(
            key_store,
            storage_scheme,
            diesel_models::PaymentMethod::find_by_customer_id_merchant_id_status_pm_type(
                &conn,
                customer_id,
                merchant_id,
                status,
                payment_method_type,
                limit,
            ),
            |pm| pm.status == status && pm.payment_method_type == Some(payment_method_type),
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

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_method_by_global_customer_id_merchant_id_statuses(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        statuses: Vec<common_enums::PaymentMethodStatus>,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        self.router_store
            .find_payment_method_by_global_customer_id_merchant_id_statuses(
                key_store,
                customer_id,
                merchant_id,
                statuses,
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
        initiator: Option<&Initiator>,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        self.router_store
            .delete_payment_method(key_store, payment_method, initiator)
            .await
    }

    // Check if KV stuff is needed here
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
        self.call_database_new(
            key_store,
            diesel_models::PaymentMethod::find_by_payment_method_id(&conn, payment_method_id),
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
        self.call_database_new(
            key_store,
            diesel_models::PaymentMethod::find_by_id(&conn, payment_method_id),
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
        self.call_database_new(
            key_store,
            diesel_models::PaymentMethod::find_by_locker_id(&conn, locker_id),
        )
        .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_methods_by_merchant_id_payment_method_ids(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_ids: &[String],
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_merchant_id_payment_method_ids(
                &conn,
                merchant_id,
                payment_method_ids,
                Some(200),
            ),
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
        diesel_models::PaymentMethod::get_count_by_customer_id_merchant_id_status(
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
        diesel_models::PaymentMethod::get_count_by_merchant_id_status(&conn, merchant_id, status)
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
        self.call_database_new(key_store, payment_method_new.insert(&conn))
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
        self.call_database_new(
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
        self.call_database_new(
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
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_customer_id_merchant_id(
                &conn,
                customer_id,
                merchant_id,
                limit,
            ),
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
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_global_customer_id(&conn, id, limit),
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
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_customer_id_merchant_id_status(
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
    #[instrument(skip_all)]
    async fn find_payment_method_by_customer_id_merchant_id_status_pm_type(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        payment_method_type: common_enums::PaymentMethodType,
        limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_customer_id_merchant_id_status_pm_type(
                &conn,
                customer_id,
                merchant_id,
                status,
                payment_method_type,
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
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_global_customer_id_merchant_id_status(
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
    async fn find_payment_method_by_global_customer_id_merchant_id_statuses(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        statuses: Vec<common_enums::PaymentMethodStatus>,
        limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.find_resources_new(
            key_store,
            diesel_models::PaymentMethod::find_by_global_customer_id_merchant_id_statuses(
                &conn,
                customer_id,
                merchant_id,
                statuses,
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
        self.call_database_new(
            key_store,
            diesel_models::PaymentMethod::delete_by_merchant_id_payment_method_id(
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
        initiator: Option<&Initiator>,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method = Conversion::convert(payment_method)
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        let conn = pg_connection_write(self).await?;
        let payment_method_update = PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
            last_modified_by: initiator
                .and_then(|initiator| initiator.to_created_by())
                .map(|last_modified_by| last_modified_by.to_string()),
        };
        self.call_database_new(
            key_store,
            payment_method.update_with_id(&conn, payment_method_update.into()),
        )
        .await
    }

    async fn find_payment_method_by_fingerprint_id(
        &self,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        self.call_database_new(
            key_store,
            diesel_models::PaymentMethod::find_by_fingerprint_id(&conn, fingerprint_id),
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
        self.get_resource_new::<diesel_models::PaymentMethod, _>(
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
        self.get_resource_new::<diesel_models::PaymentMethod, _>(
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
        self.get_resource_new::<diesel_models::PaymentMethod, _>(
            key_store,
            payment_methods,
            |pm| pm.locker_id == Some(locker_id.to_string()),
            "cannot find payment method".to_string(),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_payment_methods_by_merchant_id_payment_method_ids(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_ids: &[String],
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        if payment_method_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: HashSet<_> = payment_method_ids.iter().cloned().collect();
        let payment_methods = self.payment_methods.lock().await;
        self.get_resources_new(
            key_store,
            payment_methods,
            |pm| pm.merchant_id == *merchant_id && ids.contains(pm.get_id()),
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
        self.get_resources_new(
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
        self.get_resources_new(
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
    #[cfg(feature = "v1")]
    async fn find_payment_method_by_customer_id_merchant_id_status_pm_type(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        payment_method_type: common_enums::PaymentMethodType,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, Self::Error> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resources_new(
            key_store,
            payment_methods,
            |pm| {
                pm.customer_id == *customer_id
                    && pm.merchant_id == *merchant_id
                    && pm.status == status
                    && pm.payment_method_type == Some(payment_method_type)
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
        let find_pm_by = |pm: &&diesel_models::PaymentMethod| {
            let customer_id_matches = pm
                .customer_id
                .as_ref()
                .map(|id| id == customer_id)
                .unwrap_or(false);
            customer_id_matches && pm.merchant_id == *merchant_id && pm.status == status
        };
        let error_message = "cannot find payment method".to_string();
        self.get_resources_new(key_store, payment_methods, find_pm_by, error_message)
            .await
    }

    #[cfg(feature = "v2")]
    async fn find_payment_method_by_global_customer_id_merchant_id_statuses(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        statuses: Vec<common_enums::PaymentMethodStatus>,
        _limit: Option<i64>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<DomainPaymentMethod>, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        let find_pm_by = |pm: &&diesel_models::PaymentMethod| {
            let customer_id_matches = pm
                .customer_id
                .as_ref()
                .map(|id| id == customer_id)
                .unwrap_or(false);
            customer_id_matches && pm.merchant_id == *merchant_id && statuses.contains(&pm.status)
        };
        let error_message = "cannot find payment method".to_string();
        self.get_resources_new(key_store, payment_methods, find_pm_by, error_message)
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
        self.update_resource_new::<diesel_models::PaymentMethod, _>(
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
        initiator: Option<&Initiator>,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_method_update = PaymentMethodUpdate::StatusUpdate {
            status: Some(common_enums::PaymentMethodStatus::Inactive),
            last_modified_by: initiator
                .and_then(|initiator| initiator.to_created_by())
                .map(|last_modified_by| last_modified_by.to_string()),
        };
        let payment_method_updated = PaymentMethodUpdateInternal::from(payment_method_update)
            .apply_changeset(
                Conversion::convert(payment_method.clone())
                    .await
                    .change_context(errors::StorageError::EncryptionError)?,
            );
        self.update_resource_new::<diesel_models::PaymentMethod, _>(
            key_store,
            self.payment_methods.lock().await,
            payment_method_updated,
            |pm| pm.get_id() == payment_method.get_id(),
            "cannot find payment method".to_string(),
        )
        .await
    }

    async fn find_payment_method_by_fingerprint_id(
        &self,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<DomainPaymentMethod, errors::StorageError> {
        let payment_methods = self.payment_methods.lock().await;
        self.get_resource_new::<diesel_models::PaymentMethod, _>(
            key_store,
            payment_methods,
            |pm| pm.locker_fingerprint_id == Some(fingerprint_id.to_string()),
            "cannot find payment method".to_string(),
        )
        .await
    }
}

#[cfg(feature = "v2")]
use api_models::payment_methods::PaymentMethodsData;
// specific imports because of using the macro
#[cfg(feature = "v2")]
use common_utils::{crypto::Encryptable, encryption::Encryption, types::keymanager::ToEncryptable};
use common_utils::{
    errors::ValidationError,
    ext_traits::OptionExt,
    type_name,
    types::{keymanager, CreatedBy},
};
pub use diesel_models::{
    enums as storage_enums, PaymentMethodUpdate as StoragePaymentMethodUpdate,
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payment_methods::EncryptedPaymentMethod;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payment_methods::EncryptedPaymentMethodSession;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::type_encryption::AsyncLift;
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
#[cfg(feature = "v2")]
use hyperswitch_masking::ExposeInterface;
use hyperswitch_masking::{PeekInterface, Secret};
#[cfg(feature = "v2")]
use serde_json::Value;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl Conversion for hyperswitch_domain_models::payment_methods::PaymentMethod {
    type DstType = diesel_models::payment_method::PaymentMethod;
    type NewDstType = diesel_models::payment_method::PaymentMethodNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let (vault_type, external_vault_source) = self.vault_source_details.into();
        // Note: caller must ensure customer_id is not null before calling convert as storage model requires it.
        Ok(Self::DstType {
            customer_id: self.customer_id.get_required_value("customer_id")?,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            accepted_currency: self.accepted_currency,
            scheme: self.scheme,
            token: self.token,
            cardholder_name: self.cardholder_name,
            issuer_name: self.issuer_name,
            issuer_country: self.issuer_country,
            payer_country: self.payer_country,
            is_stored: self.is_stored,
            swift_code: self.swift_code,
            direct_debit_token: self.direct_debit_token,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
            payment_method_issuer: self.payment_method_issuer,
            payment_method_issuer_code: self.payment_method_issuer_code,
            metadata: self.metadata,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id,
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details,
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source,
            vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
            customer_details: self.customer_details.map(|val| val.into()),
            locker_fingerprint_id: self.locker_fingerprint_id,
            network_tokenization_data: self.network_tokenization_data.map(|val| val.into()),
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // Decrypt encrypted fields first
        let (
            payment_method_data,
            payment_method_billing_address,
            network_token_payment_method_data,
            network_tokenization_data,
            customer_details,
        ) = async {
            let payment_method_data = item
                .payment_method_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let payment_method_billing_address = item
                .payment_method_billing_address
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let network_token_payment_method_data = item
                .network_token_payment_method_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let network_tokenization_data = item
                .network_tokenization_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let customer_details = item
                .customer_details
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            Ok::<_, error_stack::Report<common_utils::errors::CryptoError>>((
                payment_method_data,
                payment_method_billing_address,
                network_token_payment_method_data,
                network_tokenization_data,
                customer_details,
            ))
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })?;

        let vault_source_details = PaymentMethodVaultSourceDetails::try_from((
            item.vault_type,
            item.external_vault_source,
        ))?;

        // Construct the domain type
        // Storage always has customer_id, wrap in Some for domain
        Ok(Self {
            customer_id: Some(item.customer_id),
            merchant_id: item.merchant_id,
            payment_method_id: item.payment_method_id,
            accepted_currency: item.accepted_currency,
            scheme: item.scheme,
            token: item.token,
            cardholder_name: item.cardholder_name,
            issuer_name: item.issuer_name,
            issuer_country: item.issuer_country,
            payer_country: item.payer_country,
            is_stored: item.is_stored,
            swift_code: item.swift_code,
            direct_debit_token: item.direct_debit_token,
            created_at: item.created_at,
            last_modified: item.last_modified,
            payment_method: item.payment_method,
            payment_method_type: item.payment_method_type,
            payment_method_issuer: item.payment_method_issuer,
            payment_method_issuer_code: item.payment_method_issuer_code,
            metadata: item.metadata,
            payment_method_data,
            locker_id: item.locker_id,
            last_used_at: item.last_used_at,
            connector_mandate_details: item.connector_mandate_details,
            customer_acceptance: item.customer_acceptance,
            status: item.status,
            network_transaction_id: item.network_transaction_id,
            client_secret: item.client_secret,
            payment_method_billing_address,
            updated_by: item.updated_by,
            version: item.version,
            network_token_requestor_reference_id: item.network_token_requestor_reference_id,
            network_token_locker_id: item.network_token_locker_id,
            network_token_payment_method_data,
            vault_source_details,
            created_by: item
                .created_by
                .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
            last_modified_by: item
                .last_modified_by
                .and_then(|last_modified_by| last_modified_by.parse::<CreatedBy>().ok()),
            customer_details,
            locker_fingerprint_id: item.locker_fingerprint_id,
            network_tokenization_data,
            storage_type: None,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let (vault_type, external_vault_source) = self.vault_source_details.into();
        // Note: caller must ensure customer_id is not null before calling convert as storage model requires it.
        Ok(Self::NewDstType {
            customer_id: self.customer_id.get_required_value("customer_id")?,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            accepted_currency: self.accepted_currency,
            scheme: self.scheme,
            token: self.token,
            cardholder_name: self.cardholder_name,
            issuer_name: self.issuer_name,
            issuer_country: self.issuer_country,
            payer_country: self.payer_country,
            is_stored: self.is_stored,
            swift_code: self.swift_code,
            direct_debit_token: self.direct_debit_token,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
            payment_method_issuer: self.payment_method_issuer,
            payment_method_issuer_code: self.payment_method_issuer_code,
            metadata: self.metadata,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id,
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details,
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source,
            vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
            customer_details: self.customer_details.map(|val| val.into()),
            locker_fingerprint_id: self.locker_fingerprint_id,
            network_tokenization_data: self.network_tokenization_data.map(|val| val.into()),
        })
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for hyperswitch_domain_models::payment_methods::PaymentMethod {
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
            network_transaction_id: self.network_transaction_id.map(Secret::new),
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
            external_vault_token_data: self.external_vault_token_data.map(|val| val.into()),
            vault_type: self.vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
            customer_details: self.customer_details.map(|val| val.into()),
            network_tokenization_data: self.network_tokenization_data.map(|val| val.into()),
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
                        external_vault_token_data: storage_model.external_vault_token_data,
                        customer_details: storage_model.customer_details,
                        network_tokenization_data: storage_model.network_tokenization_data,
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

            let customer_details = data
                .customer_details
                .map(|customer_details| {
                    customer_details.deserialize_inner_value(|value| {
                        value.parse_value("Payment Method Customer Details")
                    })
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Payment Method Customer Details")?;

            let external_vault_token_data = data
                .external_vault_token_data
                .map(|external_vault_token_data| {
                    external_vault_token_data.deserialize_inner_value(|value| {
                        value.parse_value("External Vault Token Data")
                    })
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing External Vault Token Data")?;

            let network_tokenization_data = data
                .network_tokenization_data
                .map(|tokenization_data| {
                    tokenization_data.deserialize_inner_value(|value| {
                        value.parse_value("Network Tokenization Data")
                    })
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Network Tokenization Data")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                customer_id: storage_model.customer_id,
                merchant_id: storage_model.merchant_id,
                id: storage_model.id,
                created_at: storage_model.created_at,
                last_modified: storage_model.last_modified,
                payment_method_type: storage_model.payment_method_type_v2,
                payment_method_subtype: storage_model.payment_method_subtype,
                payment_method_data,
                locker_id: storage_model
                    .locker_id
                    .map(hyperswitch_domain_models::payment_methods::VaultId::generate),
                last_used_at: storage_model.last_used_at,
                connector_mandate_details: storage_model.connector_mandate_details.map(From::from),
                customer_acceptance: storage_model.customer_acceptance,
                status: storage_model.status,
                network_transaction_id: storage_model
                    .network_transaction_id
                    .map(ExposeInterface::expose),
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
                external_vault_token_data,
                vault_type: storage_model.vault_type,
                created_by: storage_model
                    .created_by
                    .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
                last_modified_by: storage_model
                    .last_modified_by
                    .and_then(|last_modified_by| last_modified_by.parse::<CreatedBy>().ok()),
                customer_details,
                network_tokenization_data,
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
            external_vault_token_data: self.external_vault_token_data.map(|val| val.into()),
            vault_type: self.vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
            customer_details: self.customer_details.map(|val| val.into()),
        })
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl Conversion for hyperswitch_domain_models::payment_methods::PaymentMethodSession {
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
            storage_type: self.storage_type,
            keep_alive: self.keep_alive,
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
                storage_type: storage_model.storage_type,
                keep_alive: storage_model.keep_alive,
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
            storage_type: self.storage_type,
            keep_alive: self.keep_alive,
        })
    }
}
