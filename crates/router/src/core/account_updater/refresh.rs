use std::{str::FromStr, time::Duration};

use common_enums::ExecutionMode;
use external_services::grpc_client::LineageIds;
use hyperswitch_interfaces::{
    consts as interfaces_consts, unified_connector_service::UnifiedConnectorServiceError,
};
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

const ACCOUNT_UPDATER_CONNECTOR_NAME: &str = "juspay";

/// Sent as the `grpc-timeout` deadline, so UCS abandons the inquiry rather than us alone.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(5);

/// Backstop for a UCS that does not honour `grpc-timeout`. Kept above `REFRESH_TIMEOUT` so the
/// gRPC deadline always wins the race and timeouts classify consistently.
const REFRESH_TIMEOUT_BACKSTOP: Duration = Duration::from_secs(7);

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

    tokio::time::timeout(
        REFRESH_TIMEOUT_BACKSTOP,
        Box::pin(client.payment_method_refresh(
            request,
            connector_auth_metadata,
            grpc_headers,
            REFRESH_TIMEOUT,
        )),
    )
    .await
    .map_err(|_elapsed| {
        logger::warn!("Account Updater refresh call to UCS outlived its gRPC deadline");
        AccountUpdaterFailure::RefreshTimedOut
    })?
    .map(|response| response.into_inner())
    .map_err(|error| {
        let failure = classify_call_error(error.current_context());
        logger::warn!(
            ?error,
            ?failure,
            "Account Updater refresh call to UCS failed"
        );
        failure
    })
    .and_then(classify_response)
}

fn classify_call_error(error: &UnifiedConnectorServiceError) -> AccountUpdaterFailure {
    match error {
        UnifiedConnectorServiceError::ConnectorError(inner)
            if inner.code == interfaces_consts::REQUEST_TIMEOUT_ERROR_CODE =>
        {
            AccountUpdaterFailure::RefreshTimedOut
        }
        _ => AccountUpdaterFailure::RefreshCallFailed,
    }
}

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
        logger::warn!("Account Updater unvaulted a card number that UCS rejected as invalid");
        AccountUpdaterFailure::CardNumberInvalid
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

impl ForeignFrom<payments_grpc::CardRefreshOutcome> for RefreshOutcome {
    fn foreign_from(outcome: payments_grpc::CardRefreshOutcome) -> Self {
        match outcome {
            payments_grpc::CardRefreshOutcome::CardRefreshAccountUpdated => Self::AccountUpdated,
            payments_grpc::CardRefreshOutcome::CardRefreshExpiryUpdated => Self::ExpiryUpdated,
            payments_grpc::CardRefreshOutcome::CardRefreshNoChange => Self::NoChange,
            payments_grpc::CardRefreshOutcome::CardRefreshClosed => Self::Closed,
            payments_grpc::CardRefreshOutcome::CardRefreshNotFound => Self::NotFound,
            payments_grpc::CardRefreshOutcome::CardRefreshContactIssuer => Self::ContactIssuer,
            payments_grpc::CardRefreshOutcome::Unspecified => Self::Unspecified,
        }
    }
}
