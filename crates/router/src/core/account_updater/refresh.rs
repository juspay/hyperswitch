use std::time::Duration;

use common_enums::ExecutionMode;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use external_services::grpc_client::LineageIds;
use router_env::{instrument, tracing};
use unified_connector_service_client::payments as payments_grpc;

use super::types::{AccountUpdaterError, ResolvedAccountUpdaterConfig};
use crate::{
    core::unified_connector_service::build_unified_connector_service_auth_metadata_without_mca,
    routes::SessionState, types::domain,
};

/// Sent as the `grpc-timeout` deadline, so UCS abandons the inquiry rather than us alone.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(5);

#[instrument(skip_all)]
pub async fn request_account_updater_refresh(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    config: &ResolvedAccountUpdaterConfig,
    refreshable_payment_method: payments_grpc::PaymentMethod,
) -> CustomResult<payments_grpc::CardRefreshOutcome, AccountUpdaterError> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .as_ref()
        .ok_or(report!(AccountUpdaterError::RefreshCallFailed))
        .attach_printable("Unified Connector Service client is not configured")?;

    let request = payments_grpc::PaymentMethodServiceRefreshRequest {
        payment_method: Some(refreshable_payment_method),
    };

    let (connector, auth_type, connector_config) = config.to_connector_auth();

    let connector_auth_metadata = build_unified_connector_service_auth_metadata_without_mca(
        connector,
        &auth_type,
        platform.get_processor().get_account().get_id(),
        Some(&connector_config),
    )
    .change_context(AccountUpdaterError::RefreshCallFailed)
    .attach_printable("Failed to build the Account Updater auth metadata")?;

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

    let response = Box::pin(client.payment_method_refresh(
        request,
        connector_auth_metadata,
        grpc_headers,
        REFRESH_TIMEOUT,
    ))
    .await
    .change_context(AccountUpdaterError::RefreshCallFailed)
    .attach_printable("Account Updater refresh call to UCS failed")?
    .into_inner();

    classify_response(response)
}

fn classify_response(
    response: payments_grpc::PaymentMethodServiceRefreshResponse,
) -> CustomResult<payments_grpc::CardRefreshOutcome, AccountUpdaterError> {
    if let Some(error) = response.error.as_ref() {
        return Err(
            report!(AccountUpdaterError::RefreshReturnedError).attach_printable(format!(
                "UCS returned error {error:?} with status code {}",
                response.status_code
            )),
        );
    }

    response
        .result
        .and_then(|result| result.result)
        .map(|result| match result {
            payments_grpc::refresh_result::Result::Card(card) => {
                payments_grpc::CardRefreshOutcome::try_from(card.outcome)
                    .unwrap_or(payments_grpc::CardRefreshOutcome::Unspecified)
            }
        })
        .ok_or(report!(AccountUpdaterError::RefreshReturnedError))
        .attach_printable("UCS returned neither a result nor an error")
}
