//! Stores a reported card change by forking the payment method record.
//!
//! The id does not change: the row holding the superseded card is retired and a new active row is
//! written under the same id, so references the merchant already holds keep working. The old locker
//! entry and `payment_method_data` are left in place, so the previous card stays retrievable.

use std::str::FromStr;

use common_utils::{
    errors::CustomResult,
    ext_traits::{OptionExt, ValueExt},
    id_type, type_name,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payment_method_data::{
    CardDetailsPaymentMethod, PaymentMethodsData,
};
use router_env::{instrument, logger, tracing};
use unified_connector_service_client::payments as payments_grpc;

use super::types::AccountUpdaterError;
use crate::{
    core::{
        payment_methods::{self as pm_core, vault, PaymentMethodExt},
        utils as core_utils,
    },
    routes::SessionState,
    types::{api, domain, payment_methods as pm_types, storage},
};

/// Writes the reported change as a new row under the same payment method id.
///
/// `Ok(None)` means the change was a no-op and nothing was written.
#[instrument(skip_all)]
pub async fn store_card_change(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    refreshed_card: payments_grpc::CardDetailsWithNoCvc,
) -> CustomResult<Option<domain::PaymentMethod>, AccountUpdaterError> {
    let stored_card = stored_card_details(payment_method)?;

    let customer_id = payment_method
        .customer_id
        .clone()
        .get_required_value("customer_id")
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Payment method has no customer to vault the refreshed card against")?;

    let new_card = build_new_card(refreshed_card, &stored_card)?;

    let vaulting_data = domain::PaymentMethodVaultingData::Card(new_card);

    // The locker fingerprint covers number + expiry, so it moves on both applied outcomes; the
    // auxiliary one covers the number alone, so an expiry-only change leaves it untouched.
    let locker_fingerprint_id = vault::get_fingerprint_id_for_payment_method(
        state,
        &vaulting_data,
        customer_id.get_string_repr().to_owned(),
    )
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to fingerprint the refreshed card")?;

    // Without this the duplicate surfaces as a unique violation on the composite index.
    match payment_method.locker_fingerprint_id.as_deref() == Some(locker_fingerprint_id.as_str()) {
        true => {
            logger::info!(
                "Account Updater reported a change whose fingerprint matches the stored card; nothing written"
            );
            Ok(None)
        }
        false => write_forked_record(
            state,
            platform,
            profile,
            payment_method,
            vaulting_data,
            &customer_id,
            locker_fingerprint_id,
        )
        .await
        .map(Some),
    }
}

/// Vaults the refreshed card, retires the superseded row and inserts the new one under the same id.
async fn write_forked_record(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    vaulting_data: domain::PaymentMethodVaultingData,
    customer_id: &id_type::GlobalCustomerId,
    locker_fingerprint_id: String,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    let auxiliary_fingerprint_id = vault::get_auxiliary_fingerprint_id_for_payment_method(
        state,
        &vaulting_data,
        customer_id.get_string_repr().to_owned(),
    )
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to compute the auxiliary fingerprint for the refreshed card")?;

    let bin_enriched = vaulting_data
        .populate_bin_details_for_payment_method(state)
        .await;
    let vaulting_data = bin_enriched.data;

    // `WriteMode::Insert` with no `existing_vault_id` leaves the superseded entry holding the
    // previous card.
    let (vault_response, external_vault_source) = pm_core::vault_payment_method(
        state,
        &vaulting_data,
        platform,
        profile,
        None,
        Some(locker_fingerprint_id.clone()),
        customer_id,
        Some(pm_types::WriteMode::Insert),
    )
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to vault the refreshed card")?;

    retire_superseded_row(state, platform, payment_method).await?;

    let new_record = build_new_record(
        state,
        platform,
        payment_method,
        &vaulting_data,
        Some(vault_response.vault_id.clone()),
        locker_fingerprint_id,
        auxiliary_fingerprint_id,
        external_vault_source,
        bin_enriched.payment_method_subtype,
    )
    .await?;

    match insert_new_row(state, platform, new_record).await {
        Ok(inserted) => Ok(inserted),
        Err(insert_error) => {
            restore_superseded_row(state, platform, payment_method).await;
            Err(insert_error)
        }
    }
}

/// The card the row currently holds.
fn stored_card_details(
    payment_method: &domain::PaymentMethod,
) -> CustomResult<CardDetailsPaymentMethod, AccountUpdaterError> {
    match payment_method
        .payment_method_data
        .as_ref()
        .map(|data| data.get_inner())
    {
        Some(PaymentMethodsData::Card(card)) => Ok(card.clone()),
        _ => Err(report!(AccountUpdaterError::StoreFailed)
            .attach_printable("Stored payment method data holds no card to apply a change to")),
    }
}

/// Number and expiry from the issuer, the rest from what we already stored. Issuer/country/type are
/// left unset because they describe the old number; the BIN lookup re-derives them from the new one.
fn build_new_card(
    refreshed: payments_grpc::CardDetailsWithNoCvc,
    stored: &CardDetailsPaymentMethod,
) -> CustomResult<api::payment_methods::CardDetail, AccountUpdaterError> {
    let card_number = refreshed
        .card_number
        .ok_or_else(|| report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("Refreshed card carries no card number")?;

    // UCS and the router each have their own card-number type; this is the same crossing the
    // eligibility check makes in the outbound direction.
    let card_number = cards::CardNumber::from_str(&card_number.get_card_no())
        .change_context(AccountUpdaterError::RefreshReturnedError)
        .attach_printable("Refreshed card number failed validation")?;

    let card_exp_month = refreshed
        .card_exp_month
        .ok_or_else(|| report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("Refreshed card carries no expiry month")?;

    let card_exp_year = refreshed
        .card_exp_year
        .ok_or_else(|| report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("Refreshed card carries no expiry year")?;

    Ok(api::payment_methods::CardDetail {
        card_number,
        card_exp_month,
        card_exp_year,
        card_holder_name: stored.card_holder_name.clone(),
        nick_name: stored.nick_name.clone(),
        card_network: stored.card_network.clone(),
        card_issuing_country: None,
        card_issuer: None,
        card_type: None,
        card_cvc: None,
    })
}

/// Clears the fingerprint to SQL NULL and moves the row to `Redacted`, so any number of retired rows
/// can coexist under one id. `compat_action: None` because the insert below writes the v1 mirror.
async fn retire_superseded_row(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method: &domain::PaymentMethod,
) -> CustomResult<(), AccountUpdaterError> {
    let update = storage::PaymentMethodUpdate::StatusAndFingerprintUpdate {
        status: Some(common_enums::PaymentMethodStatus::Redacted),
        locker_fingerprint_id: Some(None),
        last_modified_by: account_updater_created_by_string(platform),
    };

    state
        .store
        .update_payment_method(
            platform.get_provider().get_key_store(),
            payment_method.clone(),
            update,
            platform.get_provider().get_account().storage_scheme,
            None,
        )
        .await
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Failed to retire the superseded payment method row")?;

    Ok(())
}

async fn restore_superseded_row(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method: &domain::PaymentMethod,
) {
    let update = storage::PaymentMethodUpdate::StatusAndFingerprintUpdate {
        status: Some(common_enums::PaymentMethodStatus::Active),
        locker_fingerprint_id: Some(payment_method.locker_fingerprint_id.clone()),
        last_modified_by: account_updater_created_by_string(platform),
    };

    // The retired row no longer carries its fingerprint, so `update_with_id` must be pointed at the
    // row as it now stands.
    let retired = domain::PaymentMethod {
        locker_fingerprint_id: None,
        status: common_enums::PaymentMethodStatus::Redacted,
        ..payment_method.clone()
    };

    match state
        .store
        .update_payment_method(
            platform.get_provider().get_key_store(),
            retired,
            update,
            platform.get_provider().get_account().storage_scheme,
            None,
        )
        .await
    {
        Ok(_) => logger::warn!(
            "Account Updater restored the superseded payment method row after a failed insert"
        ),
        Err(error) => logger::error!(
            ?error,
            "Account Updater could not restore the superseded payment method row; the payment \
             method id may have no active row"
        ),
    }
}

/// The write is made by the system applying an issuer-reported change, not on behalf of the caller
/// who triggered the force-sync, so it is not attributed to the request's initiator.
fn account_updater_created_by(
    platform: &domain::Platform,
) -> Option<common_utils::types::CreatedBy> {
    Some(common_utils::types::CreatedBy::AccountUpdater {
        merchant_id: platform
            .get_provider()
            .get_account()
            .get_id()
            .get_string_repr()
            .to_owned(),
    })
}

fn account_updater_created_by_string(platform: &domain::Platform) -> Option<String> {
    account_updater_created_by(platform).map(|created_by| created_by.to_string())
}

/// Everything the change does not touch is carried across from the superseded row, so the merchant
/// sees the same payment method with a new card rather than a new payment method.
#[allow(clippy::too_many_arguments)]
async fn build_new_record(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method: &domain::PaymentMethod,
    vaulting_data: &domain::PaymentMethodVaultingData,
    locker_id: Option<domain::VaultId>,
    locker_fingerprint_id: String,
    auxiliary_fingerprint_id: String,
    external_vault_source: Option<id_type::MerchantConnectorAccountId>,
    payment_method_subtype: Option<common_enums::PaymentMethodType>,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    let now = common_utils::date_time::now();

    let encrypted_payment_method_data = core_utils::create_encrypted_data(
        &state.into(),
        platform.get_provider().get_key_store(),
        vaulting_data.get_payment_methods_data(),
        type_name!(diesel_models::payment_method::PaymentMethod),
    )
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to encrypt the refreshed card for storage")?
    .deserialize_inner_value(|value| value.parse_value("PaymentMethodsData"))
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to parse the encrypted refreshed card")?;

    Ok(domain::PaymentMethod {
        id: payment_method.id.clone(),

        // Recomputed for the new card.
        payment_method_data: Some(encrypted_payment_method_data),
        locker_id,
        locker_fingerprint_id: Some(locker_fingerprint_id),
        auxiliary_fingerprint_id: Some(auxiliary_fingerprint_id),
        external_vault_source,
        status: common_enums::PaymentMethodStatus::Active,
        created_at: now,
        last_modified: now,
        last_modified_by: account_updater_created_by(platform),
        updated_by: account_updater_created_by_string(platform),

        // Deliberately not carried over.
        connector_mandate_details: None,
        client_secret: None,

        // Carried across unchanged — the card changed, the payment method did not.
        last_used_at: payment_method.last_used_at,
        created_by: payment_method.created_by.clone(),
        compatibility_updated_at: payment_method.compatibility_updated_at,
        customer_id: payment_method.customer_id.clone(),
        merchant_id: payment_method.merchant_id.clone(),
        payment_method_type: payment_method.payment_method_type,
        payment_method_subtype: payment_method_subtype.or(payment_method.payment_method_subtype),
        version: payment_method.version,
        customer_acceptance: payment_method.customer_acceptance.clone(),
        payment_method_billing_address: payment_method.payment_method_billing_address.clone(),
        customer_details: payment_method.customer_details.clone(),
        network_transaction_id: payment_method.network_transaction_id.clone(),
        network_transaction_link_id: payment_method.network_transaction_link_id.clone(),
        network_token_requestor_reference_id: payment_method
            .network_token_requestor_reference_id
            .clone(),
        network_token_locker_id: payment_method.network_token_locker_id.clone(),
        network_token_payment_method_data: payment_method.network_token_payment_method_data.clone(),
        network_tokenization_data: payment_method.network_tokenization_data.clone(),
        external_vault_token_data: payment_method.external_vault_token_data.clone(),
        vault_type: payment_method.vault_type,
    })
}

/// The compat action is attached so the v1 mirror — keyed by the same id, therefore single-row —
/// ends up holding the new card.
async fn insert_new_row(
    state: &SessionState,
    platform: &domain::Platform,
    new_record: domain::PaymentMethod,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    state
        .store
        .insert_payment_method(
            platform.get_provider().get_key_store(),
            new_record,
            platform.get_provider().get_account().storage_scheme,
            Some(pm_core::payment_method_modular_backward_compat_action(
                state,
                &platform.get_provider().get_account().organization_id,
            )),
        )
        .await
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Failed to insert the refreshed payment method row")
}
