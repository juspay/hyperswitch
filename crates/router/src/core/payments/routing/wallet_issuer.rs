//! Issuer-country enrichment before the confirm flow evaluates routing rules.
//!
//! Wallet identity remains `wallet` / `apple_pay` or `google_pay`. Only the issuer country
//! is enriched from the decrypted payment number's eight-digit BIN. Unknown country is left absent.

use std::{future::Future, str::FromStr};

use common_enums::{Country, CountryAlpha2, PaymentMethodType};
use common_utils::id_type::MerchantConnectorAccountId;
use euclid::backend::inputs::{BackendInput, IssuerDataInput};
use hyperswitch_domain_models::{
    payment_method_data::{EligibilityPaymentMethodData, PaymentMethodData, WalletData},
    routing::{PaymentRoutingInfo, PreRoutingConnectorChoice},
};

use crate::{core::routing::PaymentsDslInput, logger, types::domain, SessionState};

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
    if input
        .issuer_data
        .as_ref()
        .and_then(|issuer| issuer.country)
        .is_some()
    {
        return;
    }
    let Some(PaymentMethodData::Wallet(wallet)) = payment.payment_method_data else {
        return;
    };
    let method_type = match wallet {
        WalletData::ApplePay(_) => PaymentMethodType::ApplePay,
        WalletData::GooglePay(_) => PaymentMethodType::GooglePay,
        _ => return,
    };

    // A session token's issuing MCA identifies the wallet keys. Never select an arbitrary
    // connector's keys before routing, and never use billing country as issuer country.
    let decrypted = if wallet_bin(wallet).is_none() {
        let Some(mca_id) = payment
            .payment_attempt
            .straight_through_algorithm
            .as_ref()
            .and_then(|value| serde_json::from_value::<PaymentRoutingInfo>(value.clone()).ok())
            .and_then(|info| wallet_session_mca(info, method_type))
        else {
            return;
        };
        let result = match wallet {
            WalletData::ApplePay(data) => {
                super::super::decrypt_apple_pay_wallet_for_eligibility(
                    state,
                    processor,
                    business_profile,
                    &mca_id,
                    data,
                )
                .await
            }
            WalletData::GooglePay(data) => {
                super::super::decrypt_google_pay_wallet_for_eligibility(
                    state, processor, &mca_id, data,
                )
                .await
            }
            _ => return,
        };
        match result {
            Ok(Some(EligibilityPaymentMethodData::Wallet(wallet))) => Some(wallet),
            Ok(_) => return,
            Err(_) => {
                logger::warn!("Wallet issuer-country enrichment could not decrypt the session token; continuing without issuer country");
                return;
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
        .is_some()
    {
        return;
    }
    let Some(bin) = wallet_bin(wallet) else {
        return;
    };
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

#[cfg(test)]
mod tests;
