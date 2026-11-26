//! Uses the decrypted wallet's exact eight-digit BIN to determine issuer country.

use std::{future::Future, str::FromStr};

use api_models::{
    enums::RoutableConnectors,
    payments::{ApplepayCombinedSessionTokenData, GooglePayWalletDetails},
};
use common_enums::{Country, CountryAlpha2, PaymentMethodType, TransactionType};
use common_utils::{ext_traits::OptionExt, id_type::MerchantConnectorAccountId};
use euclid::backend::inputs::{BackendInput, IssuerDataInput};
use hyperswitch_domain_models::{
    payment_method_data::{EligibilityPaymentMethodData, PaymentMethodData, WalletData},
    routing::{PaymentRoutingInfo, PreRoutingConnectorChoice},
};

use super::utils;
use crate::{
    core::{
        configs::dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        payments::{
            decrypt_apple_pay_wallet_for_eligibility, decrypt_google_pay_wallet_for_eligibility,
            helpers::{get_merchant_connector_account, is_googlepay_predecrypted_flow_supported},
            OperationSessionGetters,
        },
        routing::{helpers::get_merchant_default_config, PaymentsDslInput},
    },
    logger,
    types::{api, domain},
    SessionState,
};

pub(in crate::core::payments) async fn route_wallet_confirmation<F, D>(
    state: &SessionState,
    processor: &domain::Processor,
    business_profile: &domain::Profile,
    payment_data: &D,
    wallet_type: PaymentMethodType,
    dimensions: &DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    eligible_connectors: Option<&Vec<RoutableConnectors>>,
) -> Option<api::ConnectorData>
where
    F: Clone,
    D: OperationSessionGetters<F> + Sync,
{
    if !state.conf.open_router.static_routing_enabled
        || !dimensions
            .with_payment_method_type(wallet_type)
            .get_session_override(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                None,
            )
            .await
        || !utils::is_decision_engine_routing_effective(state, dimensions).await
    {
        return None;
    }

    let payment = PaymentsDslInput::new(
        payment_data.get_setup_mandate(),
        payment_data.get_payment_attempt(),
        payment_data.get_payment_intent(),
        payment_data.get_payment_method_data(),
        payment_data.get_address(),
        payment_data.get_recurring_details(),
        payment_data.get_currency(),
    );
    let mut input = super::make_dsl_input(&payment).ok()?;
    enrich_wallet_issuer_country(state, processor, business_profile, &payment, &mut input).await;
    input.issuer_data.as_ref()?.country?;

    let profile_id = business_profile.get_id();
    let key_store = processor.get_key_store();
    let fallback_config = get_merchant_default_config(
        state.store.as_ref(),
        profile_id.get_string_repr(),
        &TransactionType::Payment,
    )
    .await
    .ok()?;
    let accounts = super::get_active_merchant_connector_accounts(state, key_store, profile_id)
        .await
        .ok()?;
    let mut candidates = Vec::new();
    for choice in fallback_config {
        let connector_name = choice.connector.to_string();
        let Some(account) = accounts.iter().find(|account| {
            choice.merchant_connector_id.as_ref() == Some(&account.merchant_connector_id)
                && account.connector_name == connector_name
        }) else {
            continue;
        };
        let supports_decrypted_token = match wallet_type {
            PaymentMethodType::ApplePay => account
                .get_metadata()
                .parse_value::<ApplepayCombinedSessionTokenData>("ApplepayCombinedSessionTokenData")
                .is_ok_and(|metadata| {
                    metadata
                        .apple_pay_combined
                        .connector_configured_for_decrypted_token()
                }),
            PaymentMethodType::GooglePay => {
                is_googlepay_predecrypted_flow_supported(account.get_metadata())
                    || get_merchant_connector_account(
                        state,
                        processor,
                        None,
                        profile_id,
                        &connector_name,
                        Some(&account.merchant_connector_id),
                    )
                    .await
                    .ok()
                    .and_then(|account| {
                        account
                            .get_connector_wallets_details()
                            .parse_value::<GooglePayWalletDetails>("GooglePayWalletDetails")
                            .ok()
                    })
                    .is_some_and(|details| details.connector_configured_for_decrypted_token())
            }
            _ => false,
        };
        if supports_decrypted_token {
            candidates.push(choice);
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let candidates = super::perform_cgraph_filtering(
        state,
        key_store,
        candidates,
        input.clone(),
        eligible_connectors,
        profile_id,
        &TransactionType::Payment,
        &accounts.get_ids(),
        business_profile.get_auto_fallback_capture_method(),
    )
    .await
    .ok()?;
    if candidates.is_empty() {
        return None;
    }
    let payment_id = payment
        .payment_attempt
        .payment_id
        .get_string_repr()
        .to_owned();
    let static_request = utils::build_static_routing_request_for_hybrid(
        profile_id.get_string_repr().to_owned(),
        payment_id.clone(),
        input,
        candidates.clone(),
    )
    .ok()?;
    let outcome = utils::decision_engine_hybrid_routing(
        state,
        business_profile,
        payment_id,
        utils::HybridRoutingRequest {
            static_routing_request: Some(static_request),
            dynamic_routing_request: None,
        },
        Vec::new(),
    )
    .await
    .inspect_err(|_| logger::warn!("Wallet confirm routing failed, keeping the session connector"))
    .ok()?;
    let selected = outcome.connectors.into_iter().next()?;
    if !outcome.first_connector_matches_static_rule || !candidates.contains(&selected) {
        return None;
    }

    api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &selected.connector.to_string(),
        api::GetToken::Connector,
        selected.merchant_connector_id,
    )
    .ok()
}

/// Best effort: unavailable wallet keys, decryption failures, and missing BIN records must
/// not make a payment fail. The normal connector-specific decryption flow still owns errors.
/// The encrypted payment data is not replaced or persisted by this routing-only lookup.
pub(crate) async fn enrich_wallet_issuer_country(
    state: &SessionState,
    processor: &domain::Processor,
    business_profile: &domain::Profile,
    payment: &PaymentsDslInput<'_>,
    input: &mut BackendInput,
) {
    let issuer_country = input.issuer_data.as_ref().and_then(|issuer| issuer.country);
    if let (None, Some(PaymentMethodData::Wallet(wallet))) =
        (issuer_country, payment.payment_method_data)
    {
        let method_type = match wallet {
            WalletData::ApplePay(_) => Some(PaymentMethodType::ApplePay),
            WalletData::GooglePay(_) => Some(PaymentMethodType::GooglePay),
            _ => None,
        };

        // A session token's issuing MCA identifies the wallet keys. Never select an arbitrary
        // connector's keys before routing, and never use billing country as issuer country.
        let mca_id = method_type
            .filter(|_| wallet_bin(wallet).is_none())
            .and_then(|method_type| {
                payment
                    .payment_attempt
                    .straight_through_algorithm
                    .as_ref()
                    .and_then(|value| {
                        serde_json::from_value::<PaymentRoutingInfo>(value.clone()).ok()
                    })
                    .and_then(|info| wallet_session_mca(info, method_type))
            });
        let decrypted = if let Some(mca_id) = mca_id {
            let result = match wallet {
                WalletData::ApplePay(data) => {
                    decrypt_apple_pay_wallet_for_eligibility(
                        state,
                        processor,
                        business_profile,
                        &mca_id,
                        data,
                    )
                    .await
                }
                WalletData::GooglePay(data) => {
                    decrypt_google_pay_wallet_for_eligibility(state, processor, &mca_id, data).await
                }
                _ => Ok(None),
            };
            match result {
                Ok(Some(EligibilityPaymentMethodData::Wallet(wallet))) => Some(wallet),
                Ok(_) => None,
                Err(_) => {
                    logger::warn!("Wallet issuer-country enrichment could not decrypt the session token; continuing without issuer country");
                    None
                }
            }
        } else {
            None
        };
        let wallet = decrypted.as_ref().unwrap_or(wallet);
        enrich_input_from_wallet(input, wallet, |bin| async move {
            state
                .store
                .get_card_info(&bin)
                .await
                .map(|record| record.and_then(|record| record.country_code))
        })
        .await;
    }
}

fn wallet_session_mca(
    info: PaymentRoutingInfo,
    method_type: PaymentMethodType,
) -> Option<MerchantConnectorAccountId> {
    let choice = info.pre_routing_results?.remove(&method_type)?;
    match choice {
        PreRoutingConnectorChoice::Single(choice) => choice.merchant_connector_id,
        // Matches the session connector selection used by wallet eligibility/decryption.
        PreRoutingConnectorChoice::Multiple(choices) => {
            choices.into_iter().next()?.merchant_connector_id
        }
    }
}

fn wallet_bin(wallet: &WalletData) -> Option<String> {
    match wallet {
        WalletData::ApplePay(data) => data
            .payment_data
            .get_decrypted_apple_pay_payment_data_optional()
            .map(|data| {
                data.application_primary_account_number
                    .get_extended_card_bin()
            }),
        WalletData::GooglePay(data) => data
            .tokenization_data
            .get_decrypted_google_pay_payment_data_optional()
            .map(|data| {
                data.application_primary_account_number
                    .get_extended_card_bin()
            }),
        _ => None,
    }
}

async fn enrich_input_from_wallet<F, Fut, E>(
    input: &mut BackendInput,
    wallet: &WalletData,
    lookup: F,
) where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<Option<String>, E>>,
{
    if input
        .issuer_data
        .as_ref()
        .and_then(|issuer| issuer.country)
        .is_none()
    {
        if let Some(bin) = wallet_bin(wallet) {
            let country = match lookup(bin).await {
                Ok(Some(code)) => CountryAlpha2::from_str(&code.trim().to_ascii_uppercase())
                    .ok()
                    .map(Country::from_alpha2),
                Ok(None) => None,
                Err(_) => {
                    logger::warn!(
                        "Wallet issuer-country BIN lookup failed; continuing without issuer country"
                    );
                    None
                }
            };
            if let Some(country) = country {
                input
                    .issuer_data
                    .get_or_insert(IssuerDataInput {
                        name: None,
                        country: None,
                    })
                    .country = Some(country);
            }
        }
    }
}
