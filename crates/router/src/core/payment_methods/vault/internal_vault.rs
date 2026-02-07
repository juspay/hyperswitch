//! Internal Vault implementation
//!
//! This module contains the implementation for the Hyperswitch internal vault,
//! which uses JWE/JWS encrypted API calls to store and retrieve payment method data.
//!
//! # Design Pattern
//!
//! This module implements the **Strategy Pattern** as part of the vault strategy
//! hierarchy. The internal vault uses direct API calls to the Hyperswitch vault
//! service with cryptographic protection (JWE/JWS).

#[cfg(feature = "v2")]
use crate::types::payment_methods as pm_types;
#[cfg(feature = "v2")]
use crate::{
    core::errors::{self, RouterResult, StorageErrorExt},
    routes::SessionState,
    types::domain,
    utils::ext_traits::OptionExt,
};
#[cfg(feature = "v2")]
use common_utils::fp_utils::when;

/// Internal vault strategy implementation
#[cfg(feature = "v2")]
#[derive(Clone)]
pub(super) struct InternalVault;

#[cfg(feature = "v2")]
impl InternalVault {
    /// Extract vault_id from payment method
    fn extract_vault_id(pm: &domain::PaymentMethod) -> RouterResult<domain::VaultId> {
        pm.locker_id
            .clone()
            .ok_or(errors::VaultError::MissingRequiredField {
                field_name: "locker_id",
            })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing locker_id for vault operation")
    }

    /// Extract customer_id from payment method
    fn extract_customer_id(pm: &domain::PaymentMethod) -> RouterResult<id_type::GlobalCustomerId> {
        pm.customer_id
            .clone()
            .get_required_value("GlobalCustomerId")
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::VaultStrategy for InternalVault {
    async fn vault_payment_method(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        pmd: &domain::PaymentMethodVaultingData,
        existing_vault_id: Option<domain::VaultId>,
        customer_id: &id_type::GlobalCustomerId,
    ) -> RouterResult<(
        pm_types::AddVaultResponse,
        Option<id_type::MerchantConnectorAccountId>,
    )> {
        let db = &*state.store;

        // Step 1: Get fingerprint_id from vault for duplicate detection
        let fingerprint_id_from_vault =
            get_fingerprint_id_from_vault(state, pmd, customer_id.get_string_repr().to_owned())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get fingerprint_id from vault")?;

        // Step 2: Check for duplicate payment methods
        // This prevents the same card from being vaulted multiple times for the same merchant
        when(
            db.find_payment_method_by_fingerprint_id(
                platform.get_provider().get_key_store(),
                &fingerprint_id_from_vault,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to find payment method by fingerprint_id")
            .inspect_err(|e| router_env::logger::error!("Vault Fingerprint_id error: {:?}", e))
            .is_ok(),
            || {
                Err(report!(errors::ApiErrorResponse::DuplicatePaymentMethod)
                    .attach_printable("Cannot vault duplicate payment method"))
            },
        )?;

        // Step 3: Add payment method to vault
        let mut resp_from_vault =
            add_payment_method_to_vault(state, pmd, existing_vault_id, customer_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to add payment method in vault")?;

        // Step 4: Enrich response with fingerprint_id
        resp_from_vault.fingerprint_id = Some(fingerprint_id_from_vault);

        Ok((resp_from_vault, None))
    }

    async fn retrieve_payment_method(
        &self,
        state: &SessionState,
        _platform: &domain::Platform,
        _profile: &domain::Profile,
        pm: &domain::PaymentMethod,
    ) -> RouterResult<pm_types::VaultRetrieveResponse> {
        let vault_id = Self::extract_vault_id(pm)?;
        let customer_id = Self::extract_customer_id(pm)?;

        let request = pm_types::VaultRetrieveRequest {
            entity_id: customer_id,
            vault_id,
        };

        retrieve_payment_method_from_vault(state, request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve payment method from internal vault")
    }

    async fn delete_payment_method(
        &self,
        state: &SessionState,
        _platform: &domain::Platform,
        _profile: &domain::Profile,
        pm: &domain::PaymentMethod,
    ) -> RouterResult<pm_types::VaultDeleteResponse> {
        let vault_id = Self::extract_vault_id(pm)?;
        let customer_id = Self::extract_customer_id(pm)?;
        delete_payment_method(state, vault_id, &customer_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to delete payment method from internal vault")
    }
}

/// Delete a payment method from the internal vault
///
/// Uses the internal vault API to delete payment method data.
/// Internal function to delete payment method from vault
#[cfg(feature = "v2")]
pub(super) async fn delete_payment_method(
    state: &SessionState,
    vault_id: domain::VaultId,
    customer_id: &id_type::GlobalCustomerId,
) -> CustomResult<pm_types::VaultDeleteResponse, errors::VaultError> {
    use crate::core::payment_methods::vault::call_to_vault;
    use common_utils::ext_traits::Encode;

    let payload = pm_types::VaultDeleteRequest {
        entity_id: customer_id.to_owned(),
        vault_id,
    }
    .encode_to_vec()
    .change_context(errors::VaultError::RequestEncodingFailed)
    .attach_printable("Failed to encode VaultDeleteRequest")?;

    let resp = call_to_vault::<pm_types::VaultDelete>(state, payload)
        .await
        .change_context(errors::VaultError::VaultAPIError)
        .attach_printable("Call to vault failed")?;

    let stored_pm_resp: pm_types::VaultDeleteResponse = resp
        .parse_struct("VaultDeleteResponse")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to parse data into VaultDeleteResponse")?;

    Ok(stored_pm_resp)
}

/// Internal function to get fingerprint_id from vault
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub(super) async fn get_fingerprint_id_from_vault<
    D: domain::VaultingDataInterface + serde::Serialize,
>(
    state: &SessionState,
    data: &D,
    key: String,
) -> CustomResult<String, errors::VaultError> {
    use crate::{
        core::payment_methods::vault::{call_to_vault, transformers},
        headers, settings,
    };
    use common_utils::ext_traits::Encode;

    let data = serde_json::to_string(data)
        .change_context(errors::VaultError::RequestEncodingFailed)
        .attach_printable("Failed to encode Vaulting data to string")?;

    let payload = pm_types::VaultFingerprintRequest { key, data }
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)
        .attach_printable("Failed to encode VaultFingerprintRequest")?;

    let resp = call_to_vault::<pm_types::GetVaultFingerprint>(state, payload)
        .await
        .change_context(errors::VaultError::VaultAPIError)
        .attach_printable("Call to vault failed")?;

    let fingerprint_resp: pm_types::VaultFingerprintResponse = resp
        .parse_struct("VaultFingerprintResponse")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to parse data into VaultFingerprintResponse")?;

    Ok(fingerprint_resp.fingerprint_id)
}

/// Internal function to add payment method to vault
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub(super) async fn add_payment_method_to_vault(
    state: &SessionState,
    pmd: &domain::PaymentMethodVaultingData,
    existing_vault_id: Option<domain::VaultId>,
    customer_id: &id_type::GlobalCustomerId,
) -> CustomResult<pm_types::AddVaultResponse, errors::VaultError> {
    use crate::core::payment_methods::vault::call_to_vault;
    use common_utils::ext_traits::Encode;

    let payload = pm_types::AddVaultRequest {
        entity_id: customer_id.to_owned(),
        vault_id: existing_vault_id
            .unwrap_or(domain::VaultId::generate(uuid::Uuid::now_v7().to_string())),
        data: pmd,
        ttl: state.conf.locker.ttl_for_storage_in_secs,
    }
    .encode_to_vec()
    .change_context(errors::VaultError::RequestEncodingFailed)
    .attach_printable("Failed to encode AddVaultRequest")?;

    let resp = call_to_vault::<pm_types::AddVault>(state, payload)
        .await
        .change_context(errors::VaultError::VaultAPIError)
        .attach_printable("Call to vault failed")?;

    let stored_pm_resp: pm_types::AddVaultResponse = resp
        .parse_struct("AddVaultResponse")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to parse data into AddVaultResponse")?;

    Ok(stored_pm_resp)
}

/// Internal function to retrieve payment method from vault
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub(super) async fn retrieve_payment_method_from_vault(
    state: &SessionState,
    request: pm_types::VaultRetrieveRequest,
) -> CustomResult<pm_types::VaultRetrieveResponse, errors::VaultError> {
    let resp = retrieve_value_from_vault(state, request).await?;

    let stored_pm_resp: pm_types::VaultRetrieveResponse = resp
        .parse_value("VaultRetrieveResponse")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to parse data into VaultRetrieveResponse")?;

    Ok(stored_pm_resp)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub(super) async fn retrieve_value_from_vault(
    state: &SessionState,
    request: pm_types::VaultRetrieveRequest,
) -> CustomResult<serde_json::Value, errors::VaultError> {
    use crate::core::payment_methods::vault::call_to_vault;
    use common_utils::ext_traits::Encode;

    let payload = request
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)
        .attach_printable("Failed to encode VaultRetrieveRequest")?;

    let resp = call_to_vault::<pm_types::VaultRetrieve>(state, payload)
        .await
        .change_context(errors::VaultError::VaultAPIError)
        .attach_printable("Call to vault failed")?;

    let stored_pm_resp: serde_json::Value = resp
        .parse_struct("VaultRetrieveResponse")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to parse data into VaultRetrieveResponse")?;

    Ok(stored_pm_resp)
}
