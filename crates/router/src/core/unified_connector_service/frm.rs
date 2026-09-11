//! FRM-specific orchestration for the Unified Connector Service.
//!
//! Native FRM providers (Signifyd, Riskified, CyberSource Decision Manager) run
//! in-process. UCS-backed providers instead have their risk evaluation executed
//! by the connector-service, which owns the provider-specific transformation.
//!
//! This module holds what has no payments analogue: the access-token fetch for
//! bearer-authenticated providers and the lifecycle notification sender. The
//! `RouterData ──▶ FrmServicePreRiskCheckRequest` builder lives with every other
//! request builder in `transformers.rs`, the verdict handler with its siblings
//! in the parent module, and the gRPC call itself in `core::fraud_check::gateway`.

use common_utils::{id_type, types::MinorUnit};
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{
    platform::Processor,
    router_data::{AccessToken, RouterData},
    router_flow_types::{fraud_check as frm_api, AccessTokenAuth},
    router_request_types::{fraud_check::FraudCheckCheckoutData, AccessTokenRequestData},
    router_response_types::fraud_check::FraudCheckResponseData,
};
use unified_connector_service_client::payments as payments_grpc;

use super::{build_unified_connector_service_auth_metadata, get_ucs_client};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, helpers::MerchantConnectorAccountType},
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

/// Fetch (and cache) the OAuth token a bearer-authenticated FRM provider needs.
///
/// Kount's risk API is bearer-authenticated: prism reads the token from
/// `state.access_token` and cannot mint one itself on the plain FRM service.
/// Reads Redis first, falling back to UCS `CreateServerAuthenticationToken`.
/// Providers using a static key (nSure) never reach this — `should_do_access_token`
/// is connector-side, so we gate on whether UCS returns a token at all.
#[cfg(feature = "v1")]
pub async fn get_frm_access_token(
    state: &SessionState,
    processor: &Processor,
    router_data: &RouterData<frm_api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>,
    merchant_connector_account: &MerchantConnectorAccountType,
    lineage_ids: LineageIds,
    execution_mode: common_enums::ExecutionMode,
) -> RouterResult<Option<AccessToken>> {
    let merchant_id = processor.get_account().get_id();
    let connector_name = router_data.connector.as_str();

    // Scope the cached token to the merchant connector account, the way the
    // payments path does (`access_token::get_cached_access_token_for_ucs`).
    // Keying on the connector name alone would make two accounts for the same
    // FRM provider under one merchant share a token.
    let merchant_connector_id = match merchant_connector_account {
        MerchantConnectorAccountType::DbVal(mca) => Some(mca.get_id()),
        MerchantConnectorAccountType::CacheVal(_) => None,
    };

    let access_token_key = common_utils::access_token::get_default_access_token_key(
        merchant_id,
        merchant_connector_id
            .as_ref()
            .map(|id| id.get_string_repr().to_string())
            .unwrap_or_else(|| connector_name.to_string()),
    );

    if let Ok(Some(token)) = state.store.get_access_token(access_token_key.clone()).await {
        router_env::logger::debug!(connector = connector_name, "FRM access token cache hit");
        return Ok(Some(token));
    }

    let ucs_client = get_ucs_client(state)?;
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account.clone(),
        merchant_id,
        connector_name.to_string(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for the FRM access token")?;

    let header_payload = state
        .get_grpc_headers_ucs(execution_mode)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None);

    // Same audit wrapper the payments access-token gateway uses, so an FRM
    // token fetch produces the same request/response/timing event. The router
    // data is re-typed to the access-token flow first (as `add_access_token`
    // does) so the event is labelled `AccessTokenAuth`, not `Checkout`.
    let access_token_request =
        AccessTokenRequestData::try_from(router_data.connector_auth_type.clone())
            .attach_printable(
                "Could not create FRM access token request from connector credentials",
            )?;
    let access_token_router_data = payments::helpers::router_data_type_conversion::<
        _,
        AccessTokenAuth,
        _,
        _,
        _,
        AccessToken,
    >(
        router_data.clone(),
        access_token_request,
        Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
    );

    let (_, token_result) = Box::pin(super::ucs_logging_wrapper_granular(
        access_token_router_data,
        state,
        payments_grpc::MerchantAuthenticationServiceCreateServerAuthenticationTokenRequest::default(),
        header_payload,
        execution_mode,
        |router_data, request, grpc_headers| async move {
            let response = ucs_client
                .create_access_token(
                    request,
                    connector_auth_metadata,
                    grpc_headers,
                    common_enums::ConnectorType::PaymentVas,
                )
                .await
                .attach_printable("UCS create_access_token gRPC call failed for FRM")?;

            let create_access_token_response = response.into_inner();

            let (token_result, _status) =
                super::handle_unified_connector_service_response_for_create_access_token(
                    create_access_token_response.clone(),
                )
                .attach_printable("Failed to parse the UCS FRM access token response")?;

            Ok((router_data, Some(token_result), create_access_token_response))
        },
    ))
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let Some(token_result) = token_result else {
        return Ok(None);
    };

    match token_result {
        Ok(token) => {
            // Best-effort cache; a write failure only costs an extra token call.
            let _ = super::set_access_token_for_ucs(
                state,
                processor,
                connector_name,
                token.clone(),
                merchant_connector_id.as_ref(),
                None,
            )
            .await;
            Ok(Some(token))
        }
        Err(err) => {
            router_env::logger::error!(
                connector = connector_name,
                error = ?err,
                "UCS returned an error for the FRM access token"
            );
            Ok(None)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle notifications
//
// prism exposes four FRM notification events on a single `NotifyConnector` RPC:
//
//   FRM_PAYMENT_SUCCEEDED · FRM_PAYMENT_FAILURE
//   FRM_REFUND_PROCESSED  · FRM_CHARGEBACK_RECEIVED
//
// They differ only in the event type and which `notification_type` variant is
// populated; auth, headers, token and response handling are identical. The
// generic sender below owns that shared work so each event is a thin caller.
// Chargeback is wired here; the payment/refund events follow the same shape.
// ─────────────────────────────────────────────────────────────────────────────

/// Which lifecycle event to report, plus the detail payload it carries.
pub enum FrmNotification {
    /// A chargeback/dispute was opened against a previously-scored payment.
    ChargebackReceived {
        connector_dispute_id: Option<String>,
        merchant_dispute_id: Option<String>,
        chargeback_reason: Option<String>,
    },
}

impl FrmNotification {
    fn event_type(&self) -> payments_grpc::NotifyEventType {
        match self {
            Self::ChargebackReceived { .. } => {
                payments_grpc::NotifyEventType::FrmChargebackReceived
            }
        }
    }

    fn notification_type(self) -> payments_grpc::frm_notification_content::NotificationType {
        match self {
            Self::ChargebackReceived {
                connector_dispute_id,
                merchant_dispute_id,
                chargeback_reason,
            } => payments_grpc::frm_notification_content::NotificationType::Chargeback(
                payments_grpc::FrmChargebackDetails {
                    connector_dispute_id,
                    merchant_dispute_id,
                    chargeback_reason,
                },
            ),
        }
    }
}

/// Facts shared by every FRM lifecycle notification.
pub struct FrmNotificationContext<'a> {
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub connector_transaction_id: Option<String>,
    /// Correlation id returned by the original risk check. Without it the
    /// provider cannot tie the notification to the transaction it scored.
    pub frm_transaction_id: Option<String>,
    pub profile_id: &'a id_type::ProfileId,
}

/// Send an FRM lifecycle notification to the connector-service.
///
/// Generic over the event: the caller supplies the [`FrmNotification`] variant
/// and this handles auth metadata, `x-frm-connector` routing, the access token
/// for bearer-authenticated providers, and the gRPC call.
#[cfg(feature = "v1")]
pub async fn call_unified_connector_service_for_frm_notification(
    state: &SessionState,
    processor: &Processor,
    merchant_connector_account: MerchantConnectorAccountType,
    connector_name: String,
    context: FrmNotificationContext<'_>,
    notification: FrmNotification,
) -> RouterResult<()> {
    let ucs_client = get_ucs_client(state)?;

    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        processor.get_account().get_id(),
        connector_name.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for the FRM notification")?;

    let grpc_currency = payments_grpc::Currency::foreign_try_from(context.currency)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert currency for the FRM notification")?;

    let event_type = notification.event_type();

    let content = payments_grpc::NotifyConnectorContent {
        content: Some(
            payments_grpc::notify_connector_content::Content::FrmNotification(
                payments_grpc::FrmNotificationContent {
                    connector_transaction_id: context.connector_transaction_id.clone(),
                    amount: Some(payments_grpc::Money {
                        minor_amount: context.amount.get_amount_as_i64(),
                        currency: grpc_currency.into(),
                    }),
                    frm_transaction_id: context.frm_transaction_id.clone(),
                    notification_type: Some(notification.notification_type()),
                    ..Default::default()
                },
            ),
        ),
    };

    let request = payments_grpc::NotifyConnectorRequest {
        event_id: format!("frm-{}", common_utils::generate_id_with_default_len("evt")),
        event_type: event_type.into(),
        content: Some(content),
        timestamp: common_utils::date_time::now_unix_timestamp(),
        ..Default::default()
    };

    let grpc_headers = state
        .get_grpc_headers_ucs(common_enums::ExecutionMode::Primary)
        .lineage_ids(LineageIds::new(
            processor.get_account().get_id().clone(),
            context.profile_id.clone(),
        ))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .build();

    ucs_client
        .notify_connector(request, connector_auth_metadata, grpc_headers, event_type)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("UCS notify_connector gRPC call failed for the FRM notification")?;

    Ok(())
}
