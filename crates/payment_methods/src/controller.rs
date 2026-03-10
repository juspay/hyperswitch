use std::fmt::Debug;

#[cfg(feature = "payouts")]
use api_models::payouts;
use api_models::{enums as api_enums, payment_methods as api};
#[cfg(feature = "v1")]
use common_enums::enums as common_enums;
#[cfg(feature = "v2")]
use common_utils::encryption;
use common_utils::{crypto, ext_traits, id_type, type_name, types::keymanager};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payment_methods::PaymentMethodVaultSourceDetails;
use hyperswitch_domain_models::{merchant_key_store, payment_methods, type_encryption};
use masking::{PeekInterface, Secret};
#[cfg(feature = "v1")]
use scheduler::errors as sch_errors;
use serde::{Deserialize, Serialize};
use storage_impl::{errors as storage_errors, payment_method};

use crate::core::errors;

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
    #[cfg(feature = "v1")]
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
        connector_mandate_details: Option<serde_json::Value>,
        status: Option<common_enums::PaymentMethodStatus>,
        network_transaction_id: Option<String>,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        card_scheme: Option<String>,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
        vault_source_details: Option<PaymentMethodVaultSourceDetails>,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v1")]
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
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
        vault_source_details: Option<PaymentMethodVaultSourceDetails>,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v2")]
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
        payment_method_billing_address: Option<encryption::Encryption>,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v1")]
    async fn add_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
    ) -> errors::PmResponse<api::PaymentMethodResponse>;

    #[cfg(feature = "v1")]
    async fn retrieve_payment_method(
        &self,
        pm: api::PaymentMethodId,
    ) -> errors::PmResponse<api::PaymentMethodResponse>;

    #[cfg(feature = "v1")]
    async fn delete_payment_method(
        &self,
        pm_id: api::PaymentMethodId,
    ) -> errors::PmResponse<api::PaymentMethodDeleteResponse>;

    async fn add_card_hs(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        locker_choice: api_enums::LockerChoice,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    /// The response will be the tuple of PaymentMethodResponse and the duplication check of payment_method
    async fn add_card_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    #[cfg(feature = "payouts")]
    async fn add_bank_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        bank: &payouts::Bank,
        customer_id: &id_type::CustomerId,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    #[cfg(feature = "v1")]
    async fn get_or_insert_payment_method(
        &self,
        req: api::PaymentMethodCreate,
        resp: &mut api::PaymentMethodResponse,
        customer_id: &id_type::CustomerId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v2")]
    async fn get_or_insert_payment_method(
        &self,
        _req: api::PaymentMethodCreate,
        _resp: &mut api::PaymentMethodResponse,
        _customer_id: &id_type::CustomerId,
        _key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn get_card_details_with_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<Option<api::CardDetailFromLocker>>;

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
    fn store_default_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>);

    #[cfg(feature = "v2")]
    fn store_default_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>);

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn save_network_token_and_update_payment_method(
        &self,
        req: &api::PaymentMethodMigrate,
        key_store: &merchant_key_store::MerchantKeyStore,
        network_token_data: &api_models::payment_methods::MigrateNetworkTokenData,
        network_token_requestor_ref_id: String,
        pm_id: String,
    ) -> errors::PmResult<bool>;

    #[cfg(feature = "v1")]
    async fn set_default_payment_method(
        &self,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        payment_method_id: String,
    ) -> errors::PmResponse<api_models::payment_methods::CustomerDefaultPaymentMethodResponse>;

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
    async fn get_card_details_from_locker(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<api::CardDetailFromLocker>;
}

pub async fn create_encrypted_data<T>(
    key_manager_state: &keymanager::KeyManagerState,
    key_store: &merchant_key_store::MerchantKeyStore,
    data: T,
) -> Result<
    crypto::Encryptable<Secret<serde_json::Value>>,
    error_stack::Report<storage_errors::StorageError>,
>
where
    T: Debug + Serialize,
{
    let key = key_store.key.get_inner().peek();
    let identifier = keymanager::Identifier::Merchant(key_store.merchant_id.clone());

    let encoded_data = ext_traits::Encode::encode_to_value(&data)
        .change_context(storage_errors::StorageError::SerializationFailed)
        .attach_printable("Unable to encode data")?;

    let secret_data = Secret::<_, masking::WithType>::new(encoded_data);

    let encrypted_data = type_encryption::crypto_operation(
        key_manager_state,
        type_name!(payment_method::PaymentMethod),
        type_encryption::CryptoOperation::Encrypt(secret_data),
        identifier.clone(),
        key,
    )
    .await
    .and_then(|val| val.try_into_operation())
    .change_context(storage_errors::StorageError::EncryptionError)
    .attach_printable("Unable to encrypt data")?;

    Ok(encrypted_data)
}
