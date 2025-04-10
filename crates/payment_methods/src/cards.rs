use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use api_models::{enums as api_enums, payment_methods as api, payouts};
use common_enums::enums as common_enums;
use common_utils::{crypto, ext_traits::OptionExt, id_type, types::keymanager};
use hyperswitch_domain_models::{
    errors::api_error_response, merchant_account, merchant_key_store, payment_methods,
};
use scheduler::errors as sch_errors;
use serde::{Deserialize, Serialize};
use storage_impl::errors::StorageError;

use crate::{
    core::{errors, migration::PaymentMethodsMigrateForm},
    state::PaymentMethodsStorageInterface,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataDuplicationCheck {
    Duplicated,
    MetaDataChanged,
}

#[async_trait::async_trait]
pub trait PaymentMethodsController {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2"),
        not(feature = "customer_v2")
    ))]
    #[allow(clippy::too_many_arguments)]
    async fn create_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        payment_method_id: &str,
        locker_id: Option<String>,
        merchant_id: &id_type::MerchantId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        payment_method_data: crypto::OptionalEncryptableValue,
        key_store: &merchant_key_store::MerchantKeyStore,
        connector_mandate_details: Option<serde_json::Value>,
        status: Option<common_enums::PaymentMethodStatus>,
        network_transaction_id: Option<String>,
        storage_scheme: common_enums::MerchantStorageScheme,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        card_scheme: Option<String>,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    #[allow(clippy::too_many_arguments)]
    async fn insert_payment_method(
        &self,
        resp: &api::PaymentMethodResponse,
        req: &api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        locker_id: Option<String>,
        connector_mandate_details: Option<serde_json::Value>,
        network_transaction_id: Option<String>,
        storage_scheme: common_enums::MerchantStorageScheme,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(all(
        any(feature = "v2", feature = "v1"),
        not(feature = "payment_methods_v2")
    ))]
    async fn retrieve_payment_method(
        &self,
        pm: api::PaymentMethodId,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_account: merchant_account::MerchantAccount,
    ) -> errors::PmResponse<api::PaymentMethodResponse>;

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn delete_payment_method(
        &self,
        merchant_account: merchant_account::MerchantAccount,
        pm_id: api::PaymentMethodId,
        key_store: merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResponse<api::PaymentMethodDeleteResponse>;

    async fn add_card_hs(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        merchant_account: &merchant_account::MerchantAccount,
        locker_choice: api_enums::LockerChoice,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    /// The response will be the tuple of PaymentMethodResponse and the duplication check of payment_method
    async fn add_card_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        merchant_account: &merchant_account::MerchantAccount,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    #[cfg(feature = "payouts")]
    async fn add_bank_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        merchant_account: &merchant_account::MerchantAccount,
        key_store: &merchant_key_store::MerchantKeyStore,
        bank: &payouts::Bank,
        customer_id: &id_type::CustomerId,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_or_insert_payment_method(
        &self,
        req: api::PaymentMethodCreate,
        resp: &mut api::PaymentMethodResponse,
        merchant_account: &merchant_account::MerchantAccount,
        customer_id: &id_type::CustomerId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(all(
        any(feature = "v2", feature = "v1"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_card_details_with_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<Option<api::CardDetailFromLocker>>;

    #[cfg(all(
        any(feature = "v2", feature = "v1"),
        not(feature = "payment_methods_v2")
    ))]
    async fn get_card_details_without_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<api::CardDetailFromLocker>;

    async fn delete_card_from_locker(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        card_reference: &str,
    ) -> errors::PmResult<DeleteCardResp>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    fn store_default_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>);

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2"),
        not(feature = "customer_v2")
    ))]
    #[allow(clippy::too_many_arguments)]
    async fn save_network_token_and_update_payment_method(
        &self,
        req: &api::PaymentMethodMigrate,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_account: &merchant_account::MerchantAccount,
        network_token_data: &api_models::payment_methods::MigrateNetworkTokenData,
        network_token_requestor_ref_id: String,
        pm_id: String,
    ) -> errors::PmResult<bool>;
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    async fn set_default_payment_method(
        &self,
        merchant_id: &id_type::MerchantId,
        key_store: merchant_key_store::MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        payment_method_id: String,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> errors::PmResponse<api_models::payment_methods::CustomerDefaultPaymentMethodResponse>;

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    async fn add_payment_method_status_update_task(
        &self,
        payment_method: &payment_methods::PaymentMethod,
        prev_status: common_enums::PaymentMethodStatus,
        curr_status: common_enums::PaymentMethodStatus,
        merchant_id: &id_type::MerchantId,
    ) -> Result<(), sch_errors::ProcessTrackerError>;

    #[cfg(feature = "v1")]
    async fn validate_merchant_connector_ids_in_connector_mandate_details(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        connector_mandate_details: &api_models::payment_methods::CommonMandateReference,
        merchant_id: &id_type::MerchantId,
        card_network: Option<common_enums::CardNetwork>,
    ) -> errors::PmResult<()>;
}

pub async fn create_encrypted_data<T>(
    data: T,
) -> Result<
    crypto::Encryptable<masking::Secret<serde_json::Value>>,
    error_stack::Report<StorageError>,
>
where
    T: Debug + serde::Serialize,
{
    // Implementation here
    unimplemented!()
}
