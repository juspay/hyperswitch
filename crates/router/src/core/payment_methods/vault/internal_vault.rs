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

use common_utils::id_type;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

#[cfg(feature = "v2")]
use crate::types::payment_methods as pm_types;
#[cfg(feature = "v2")]
use crate::{
    core::errors::{self, RouterResult, StorageErrorExt},
    routes::SessionState,
    types::domain,
    utils::{ext_traits::OptionExt, when},
};

/// Vault a payment method using the internal vault
///
/// This function performs the following steps:
/// 1. Gets the fingerprint_id from the vault for duplicate detection
/// 2. Checks if a payment method with the same fingerprint already exists in the database
/// 3. Adds the payment method to the internal vault via API call
/// 4. Returns the vault response enriched with the fingerprint_id
///
/// # Arguments
///
/// * `state` - The application session state
/// * `pmd` - The payment method data to vault
/// * `platform` - The platform context (provider/processor)
/// * `existing_vault_id` - Optional existing vault ID for updates
/// * `customer_id` - The customer ID associated with the payment method
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method(
    state: &SessionState,
    pmd: &domain::PaymentMethodVaultingData,
    platform: &domain::Platform,
    existing_vault_id: Option<domain::VaultId>,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<pm_types::AddVaultResponse> {
    let db = &*state.store;

    // Step 1: Get fingerprint_id from vault for duplicate detection
    let fingerprint_id_from_vault =
        super::get_fingerprint_id_from_vault(state, pmd, customer_id.get_string_repr().to_owned())
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
        super::add_payment_method_to_vault(state, platform, pmd, existing_vault_id, customer_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in vault")?;

    // Step 4: Enrich response with fingerprint_id
    resp_from_vault.fingerprint_id = Some(fingerprint_id_from_vault);

    Ok(resp_from_vault)
}

/// Retrieve a payment method from the internal vault
///
/// Uses the internal vault API to fetch payment method data by vault ID.
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    vault_id: &domain::VaultId,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    super::retrieve_payment_method_from_vault_internal(state, platform, vault_id, customer_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve payment method from internal vault")
}

/// Delete a payment method from the internal vault
///
/// Uses the internal vault API to delete payment method data.
#[cfg(feature = "v2")]
pub async fn delete_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    vault_id: domain::VaultId,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<pm_types::VaultDeleteResponse> {
    super::delete_payment_method_data_from_vault_internal(state, platform, vault_id, customer_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete payment method from internal vault")
}
