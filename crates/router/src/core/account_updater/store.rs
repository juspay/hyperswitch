use std::str::FromStr;

use common_enums::connector_enums::Connector;
use common_utils::{
    errors::CustomResult,
    ext_traits::{OptionExt, ValueExt},
    id_type, type_name,
    types::CreatedBy,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payment_method_data::CardDetailsPaymentMethod;
use router_env::{instrument, logger, tracing};
use unified_connector_service_client::payments as payments_grpc;

use super::types::{AccountUpdaterError, CardRefreshedData, RefreshedCard};
use crate::{
    core::{
        payment_methods::{self as pm_core, vault, PaymentMethodExt},
        utils as core_utils,
    },
    routes::SessionState,
    types::{api, domain, payment_methods as pm_types, storage},
};

#[instrument(skip_all)]
pub async fn apply_card_refresh_result(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    card_result: CardRefreshedData,
) -> CustomResult<Option<domain::PaymentMethod>, AccountUpdaterError> {
    let CardRefreshedData { outcome, service } = card_result;

    match outcome.get_new_card_details() {
        Some(RefreshedCard::CardOpen(refreshed_card)) => {
            Box::pin(write_refreshed_card(
                state,
                platform,
                profile,
                payment_method,
                service,
                *refreshed_card,
            ))
            .await
        }
        Some(RefreshedCard::CardClosed) => update_payment_method_status(
            state,
            platform,
            service,
            payment_method.clone(),
            common_enums::PaymentMethodStatus::Inactive,
        )
        .await
        .attach_printable("Failed to deactivate the closed payment method")
        .map(Some),
        None => Ok(None),
    }
}

async fn write_refreshed_card(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    service: Connector,
    refreshed_card: payments_grpc::CardDetailsWithNoCvc,
) -> CustomResult<Option<domain::PaymentMethod>, AccountUpdaterError> {
    let stored_card = payment_method
        .payment_method_data
        .as_ref()
        .and_then(|data| data.get_inner().get_card_details())
        .ok_or_else(|| report!(AccountUpdaterError::StoreFailed))
        .attach_printable("Stored payment method data holds no card to apply a change to")?;

    let customer_id = payment_method
        .customer_id
        .clone()
        .get_required_value("customer_id")
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Payment method has no customer to vault the refreshed card against")?;

    let new_card = build_new_card(refreshed_card, &stored_card)?;

    let vaulting_data = domain::PaymentMethodVaultingData::Card(new_card);

    let (fingerprint_id_result, auxiliary_fingerprint_id_result) = tokio::join!(
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

    let locker_fingerprint_id = fingerprint_id_result
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Failed to fingerprint the refreshed card")?;

    let auxiliary_fingerprint_id = auxiliary_fingerprint_id_result
        .change_context(AccountUpdaterError::StoreFailed)
        .attach_printable("Failed to compute the auxiliary fingerprint for the refreshed card")?;

    if payment_method.locker_fingerprint_id.as_deref() == Some(locker_fingerprint_id.as_str()) {
        logger::info!(
            "Account Updater reported a change whose fingerprint matches the stored card; nothing written"
        );
        Ok(None)
    } else {
        Box::pin(store_refreshed_card(
            state,
            platform,
            profile,
            payment_method,
            service,
            vaulting_data,
            customer_id,
            locker_fingerprint_id,
            auxiliary_fingerprint_id,
        ))
        .await
        .map(Some)
    }
}

#[allow(clippy::too_many_arguments)]
async fn store_refreshed_card(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    service: Connector,
    vaulting_data: domain::PaymentMethodVaultingData,
    customer_id: id_type::GlobalCustomerId,
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
        &customer_id,
        Some(pm_types::WriteMode::Insert),
    )
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to vault the refreshed card")?;

    let refreshed_payment_method = build_refreshed_payment_method(
        state,
        platform,
        payment_method,
        service,
        &vaulting_data,
        Some(vault_response.vault_id.clone()),
        locker_fingerprint_id,
        auxiliary_fingerprint_id,
        external_vault_source,
        bin_enriched.payment_method_subtype,
    )
    .await?;

    let inserted_payment_method =
        insert_refreshed_payment_method(state, platform, refreshed_payment_method).await?;

    Box::pin(pm_core::delete_payment_method_by_record(
        state.store.as_ref(),
        state,
        platform,
        profile,
        payment_method.clone(),
    ))
    .await
    .change_context(AccountUpdaterError::StoreFailed)
    .attach_printable("Failed to delete the superseded payment method")?;

    update_payment_method_status(
        state,
        platform,
        service,
        inserted_payment_method,
        common_enums::PaymentMethodStatus::Active,
    )
    .await
    .inspect_err(|error| {
        logger::warn!(
            ?error,
            "Account Updater stored the refreshed card but could not activate the row"
        )
    })
}

fn build_new_card(
    refreshed: payments_grpc::CardDetailsWithNoCvc,
    stored: &CardDetailsPaymentMethod,
) -> CustomResult<api::payment_methods::CardDetail, AccountUpdaterError> {
    let card_number = refreshed
        .card_number
        .ok_or_else(|| report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("Refreshed card carries no card number")?
        .get_card_no()
        .parse::<cards::CardNumber>()
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
        card_issuing_country: stored
            .issuer_country
            .as_ref()
            .map(|country| common_enums::CountryAlpha2::from_str(country))
            .transpose()
            .ok()
            .flatten(),
        card_issuer: stored.card_issuer.clone(),
        card_type: stored
            .card_type
            .as_ref()
            .map(|card_type| api::payment_methods::CardType::from_str(card_type))
            .transpose()
            .ok()
            .flatten(),
        card_cvc: None,
    })
}

async fn update_payment_method_status(
    state: &SessionState,
    platform: &domain::Platform,
    service: Connector,
    payment_method: domain::PaymentMethod,
    status: common_enums::PaymentMethodStatus,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    let update = storage::PaymentMethodUpdate::StatusUpdate {
        status: Some(status),
        last_modified_by: Some(
            CreatedBy::AccountUpdater {
                service: service.to_string(),
            }
            .to_string(),
        ),
    };

    state
        .store
        .update_payment_method(
            platform.get_provider().get_key_store(),
            payment_method,
            update,
            platform.get_provider().get_account().storage_scheme,
            None,
        )
        .await
        .change_context(AccountUpdaterError::StoreFailed)
}

#[allow(clippy::too_many_arguments)]
async fn build_refreshed_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method: &domain::PaymentMethod,
    service: Connector,
    vaulting_data: &domain::PaymentMethodVaultingData,
    locker_id: Option<domain::VaultId>,
    locker_fingerprint_id: String,
    auxiliary_fingerprint_id: String,
    external_vault_source: Option<id_type::MerchantConnectorAccountId>,
    payment_method_subtype: Option<common_enums::PaymentMethodType>,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    let now = common_utils::date_time::now();
    let created_by = CreatedBy::AccountUpdater {
        service: service.to_string(),
    };
    let created_by_string = created_by.to_string();

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
        customer_id: payment_method.customer_id.clone(),
        merchant_id: payment_method.merchant_id.clone(),
        external_vault_source,
        created_at: now,
        last_modified: now,
        payment_method_type: payment_method.payment_method_type,
        payment_method_subtype: payment_method_subtype.or(payment_method.payment_method_subtype),
        payment_method_data: Some(encrypted_payment_method_data),
        locker_id,
        last_used_at: now,
        connector_mandate_details: None,
        customer_acceptance: payment_method.customer_acceptance.clone(),
        status: common_enums::PaymentMethodStatus::New,
        network_transaction_id: payment_method.network_transaction_id.clone(),
        network_transaction_link_id: payment_method.network_transaction_link_id.clone(),
        client_secret: None,
        payment_method_billing_address: payment_method.payment_method_billing_address.clone(),
        updated_by: Some(created_by_string),
        locker_fingerprint_id: Some(locker_fingerprint_id),
        auxiliary_fingerprint_id: Some(auxiliary_fingerprint_id),
        version: payment_method.version,
        network_token_requestor_reference_id: payment_method
            .network_token_requestor_reference_id
            .clone(),
        network_token_locker_id: payment_method.network_token_locker_id.clone(),
        network_token_payment_method_data: payment_method.network_token_payment_method_data.clone(),
        external_vault_token_data: payment_method.external_vault_token_data.clone(),
        vault_type: payment_method.vault_type,
        created_by: Some(created_by.clone()),
        last_modified_by: Some(created_by),
        customer_details: payment_method.customer_details.clone(),
        network_tokenization_data: payment_method.network_tokenization_data.clone(),
        compatibility_updated_at: None,
    })
}

async fn insert_refreshed_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method: domain::PaymentMethod,
) -> CustomResult<domain::PaymentMethod, AccountUpdaterError> {
    state
        .store
        .insert_payment_method(
            platform.get_provider().get_key_store(),
            payment_method,
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
