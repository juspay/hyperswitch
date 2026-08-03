use std::{str::FromStr, time::Duration};

use common_enums::ExecutionMode;
use external_services::grpc_client::LineageIds;
use router_env::{instrument, logger, tracing};
use unified_connector_service_cards::CardNumber;
use unified_connector_service_client::payments as payments_grpc;

use super::{
    connector_config::build_account_updater_connector_config,
    types::{AccountUpdaterFailure, RefreshOutcome, ResolvedAccountUpdaterConfig, SyncCard},
};
use crate::{
    consts,
    core::unified_connector_service::build_unified_connector_service_auth_metadata_without_mca,
    routes::SessionState,
    types::{domain, transformers::ForeignFrom},
};

pub(crate) const ACCOUNT_UPDATER_CONNECTOR_NAME: &str = "juspay";

#[instrument(skip_all)]
pub async fn refresh_card(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    config: &ResolvedAccountUpdaterConfig,
    sync_card: SyncCard,
) -> Result<RefreshOutcome, AccountUpdaterFailure> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .as_ref()
        .ok_or(AccountUpdaterFailure::UnifiedConnectorServiceUnavailable)?;

    let connector_config = build_account_updater_connector_config(config)?;

    let request = build_refresh_request(sync_card)?;

    let connector_auth_metadata = build_unified_connector_service_auth_metadata_without_mca(
        ACCOUNT_UPDATER_CONNECTOR_NAME.to_string(),
        consts::UCS_AUTH_HEADER_KEY.to_string(),
        platform.get_processor().get_account().get_id(),
        Some(connector_config),
    );

    let grpc_headers = state
        .get_grpc_headers_ucs(ExecutionMode::Primary)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .lineage_ids(LineageIds::new(
            platform.get_processor().get_account().get_id().clone(),
            profile.get_id().clone(),
        ))
        .build();

    Box::pin(client.payment_method_refresh(
        request,
        connector_auth_metadata,
        grpc_headers,
        Duration::from_millis(config.refresh_timeout_ms),
    ))
    .await
    .map(|response| response.into_inner())
    .map_err(|error| {
        logger::warn!(?error, "Account Updater refresh call to UCS failed");
        AccountUpdaterFailure::RefreshCallFailed
    })
    .and_then(classify_response)
}

/// Branches on `error` before reading the outcome: a failed response also carries an unspecified
/// outcome, so the outcome alone cannot separate "asked and got an odd answer" from "could not ask".
fn classify_response(
    response: payments_grpc::PaymentMethodServiceRefreshResponse,
) -> Result<RefreshOutcome, AccountUpdaterFailure> {
    if let Some(error) = response.error.as_ref() {
        logger::warn!(
            ?error,
            status_code = response.status_code,
            "Account Updater refresh returned an error"
        );
        return Err(AccountUpdaterFailure::RefreshReturnedError);
    }

    response
        .result
        .and_then(|result| result.result)
        .map(|result| match result {
            payments_grpc::refresh_result::Result::Card(card) => {
                // An unrecognised code is still a successful inquiry, so it normalizes rather
                // than failing.
                payments_grpc::CardRefreshOutcome::try_from(card.outcome)
                    .map(RefreshOutcome::foreign_from)
                    .unwrap_or(RefreshOutcome::Unspecified)
            }
        })
        .ok_or(AccountUpdaterFailure::RefreshResultMissing)
}

fn build_refresh_request(
    sync_card: SyncCard,
) -> Result<payments_grpc::PaymentMethodServiceRefreshRequest, AccountUpdaterFailure> {
    let card_number = CardNumber::from_str(&sync_card.card_number.get_card_no()).map_err(|_| {
        logger::warn!("Account Updater could not encode the card number for UCS");
        AccountUpdaterFailure::RefreshCallFailed
    })?;

    let card = payments_grpc::CardDetailsWithNoCvc {
        card_number: Some(card_number),
        card_exp_month: Some(sync_card.expiry_month),
        card_exp_year: Some(sync_card.expiry_year),
        card_network: Some(i32::from(payments_grpc::CardNetwork::foreign_from(
            sync_card.network,
        ))),
        card_holder_name: None,
        card_issuer: None,
        card_type: None,
        card_issuing_country_alpha2: None,
        bank_code: None,
        nick_name: None,
    };

    Ok(payments_grpc::PaymentMethodServiceRefreshRequest {
        payment_method: Some(payments_grpc::PaymentMethod {
            payment_method: Some(payments_grpc::payment_method::PaymentMethod::CardWithNoCvc(
                card,
            )),
        }),
    })
}
