//! Applies a reported card change by forking the payment method record.
//!
//! The id does not change: a new row carrying the updated card is written under the same id and the
//! row holding the superseded card is deleted, so references the merchant already holds keep
//! working. The superseded row is deleted only once its replacement exists, so a failure leaves the
//! merchant with the card they already had.

use std::str::FromStr;

use common_utils::{
    errors::CustomResult,
    ext_traits::{OptionExt, ValueExt},
    id_type, type_name,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payment_method_data::CardDetailsPaymentMethod;
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

#[instrument(skip_all)]
pub async fn create_payment_method_for_updated_card(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    updated_card: payments_grpc::CardDetailsWithNoCvc,
) -> CustomResult<Option<domain::PaymentMethod>, AccountUpdaterError> {
    let stored_card = stored_card_details(payment_method)?;

    let customer_id = payment_method
        .customer_id
        .clone()
        .get_required_value("customer_id")
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Payment method has no customer to vault the updated card against")?;

    let new_card = build_new_card(updated_card, &stored_card)?;

    let vaulting_data = domain::PaymentMethodVaultingData::Card(new_card);

    let (locker_fingerprint_id, auxiliary_fingerprint_id) = tokio::join!(
        vault::get_fingerprint_id_for_payment_method(
            state,
            &vaulting_data,
            customer_id.get_string_repr().to_owned(),
        ),
        vault::get_auxiliary_fingerprint_id_for_payment_method(
            state,
            &vaulting_data,
            customer_id.get_string_repr().to_owned(),
        ),
    );

    let locker_fingerprint_id = locker_fingerprint_id
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Failed to fingerprint the updated card")?;

    let auxiliary_fingerprint_id = auxiliary_fingerprint_id
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Failed to compute the auxiliary fingerprint for the updated card")?;

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
            auxiliary_fingerprint_id,
        )
        .await
        .map(Some),
    }
}

#[allow(clippy::too_many_arguments)]
async fn write_forked_record(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    vaulting_data: domain::PaymentMethodVaultingData,
    customer_id: &id_type::GlobalCustomerId,
    locker_fingerprint_id: String,
    auxiliary_fingerprint_id: String,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    let bin_enriched = vaulting_data
        .populate_bin_details_for_payment_method(state)
        .await;
    let vaulting_data = bin_enriched.data;

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
    .attach_printable("Failed to vault the updated card")?;

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

    let new_record = insert_new_row(state, platform, new_record).await?;

    pm_core::delete_payment_method_by_record(
        state.store.as_ref(),
        state,
        platform,
        profile,
        payment_method.clone(),
    )
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to delete the superseded payment method")?;

    activate_new_row(state, platform, new_record).await
}

fn stored_card_details(
    payment_method: &domain::PaymentMethod,
) -> CustomResult<CardDetailsPaymentMethod, AccountUpdaterError> {
    payment_method
        .payment_method_data
        .as_ref()
        .and_then(|data| data.get_inner().get_card_details())
        .ok_or_else(|| report!(AccountUpdaterError::StoreFailed))
        .attach_printable("Stored payment method data holds no card to apply a change to")
}

fn build_new_card(
    updated: payments_grpc::CardDetailsWithNoCvc,
    stored: &CardDetailsPaymentMethod,
) -> CustomResult<api::payment_methods::CardDetail, AccountUpdaterError> {
    let card_number = updated
        .card_number
        .ok_or_else(|| report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("Refreshed card carries no card number")?;

    let card_number = cards::CardNumber::from_str(&card_number.get_card_no())
        .change_context(AccountUpdaterError::RefreshReturnedError)
        .attach_printable("Refreshed card number failed validation")?;

    let card_exp_month = updated
        .card_exp_month
        .ok_or_else(|| report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("Refreshed card carries no expiry month")?;

    let card_exp_year = updated
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

async fn activate_new_row(
    state: &SessionState,
    platform: &domain::Platform,
    new_record: domain::PaymentMethod,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    let update = storage::PaymentMethodUpdate::StatusUpdate {
        status: Some(common_enums::PaymentMethodStatus::Active),
        last_modified_by: account_updater_created_by_string(platform),
    };

    state
        .store
        .update_payment_method(
            platform.get_provider().get_key_store(),
            new_record.clone(),
            update,
            platform.get_provider().get_account().storage_scheme,
            None,
        )
        .await
        .inspect_err(|error| {
            logger::warn!(
                ?error,
                "Account Updater stored the updated card but could not activate the row"
            )
        })
        .or(Ok(new_record))
}

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
    .attach_printable("Failed to encrypt the updated card for storage")?
    .deserialize_inner_value(|value| value.parse_value("PaymentMethodsData"))
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to parse the encrypted updated card")?;

    Ok(domain::PaymentMethod {
        payment_method_data: Some(encrypted_payment_method_data),
        locker_id,
        locker_fingerprint_id: Some(locker_fingerprint_id),
        auxiliary_fingerprint_id: Some(auxiliary_fingerprint_id),
        external_vault_source,
        status: common_enums::PaymentMethodStatus::New,
        created_at: now,
        last_modified: now,
        last_modified_by: account_updater_created_by(platform),
        updated_by: account_updater_created_by_string(platform),
        payment_method_subtype: payment_method_subtype.or(payment_method.payment_method_subtype),

        connector_mandate_details: None,
        client_secret: None,

        ..payment_method.clone()
    })
}

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
        .attach_printable("Failed to insert the updated payment method row")
}
