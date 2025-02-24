use common_enums::enums::MerchantStorageScheme;
#[cfg(feature = "v2")]
use common_utils::{
    crypto::Encryptable, encryption::Encryption, ext_traits::ValueExt,
    types::keymanager::ToEncryptable,
};
use common_utils::{errors::CustomResult, id_type, types::keymanager::KeyManagerState};
use diesel_models::PaymentMethodUpdate;
#[cfg(feature = "v2")]
use rustc_hash::FxHashMap;
#[cfg(feature = "v2")]
use serde_json::Value;

#[cfg(feature = "v2")]
use crate::{
    address::Address,
    consts, router_response_types,
    type_encryption::{crypto_operation, CryptoOperation},
};
use crate::{errors, merchant_key_store::MerchantKeyStore, payment_methods::PaymentMethod};

#[async_trait::async_trait]
pub trait PaymentMethodInterface {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_locker_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<PaymentMethod>, errors::StorageError>;

    // Need to fix this once we start moving to v2 for payment method
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<PaymentMethod>, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    #[allow(clippy::too_many_arguments)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentMethod>, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    #[allow(clippy::too_many_arguments)]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentMethod>, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, errors::StorageError>;

    async fn insert_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: PaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    async fn update_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: PaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn delete_payment_method(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        payment_method: PaymentMethod,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<PaymentMethod, errors::StorageError>;
}
