//! FRM-specific orchestration for the Unified Connector Service.
//!
//! Native FRM providers (Signifyd, Riskified, CyberSource Decision Manager) run
//! in-process. UCS-backed providers instead have their risk evaluation executed
//! by the connector-service, which owns the provider-specific transformation.
//!
//! This module holds what has no payments analogue: the lifecycle notification
//! sender. Access tokens are handled in `core::fraud_check::access_token`. The
//! `RouterData ──▶ FrmServicePreRiskCheckRequest` builder lives with every other
//! request builder in `transformers.rs`, the verdict handler with its siblings
//! in the parent module, and the gRPC call itself in `core::fraud_check::gateway`.

use common_utils::{id_type, types::MinorUnit};
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::platform::Processor;
use unified_connector_service_client::payments as payments_grpc;

use super::{build_unified_connector_service_auth_metadata, get_ucs_client};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers::MerchantConnectorAccountType,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

// Lifecycle notifications: the connector-service exposes four FRM events
// (payment succeeded/failure, refund processed, chargeback received) on one
// `NotifyConnector` RPC, differing only in event type and payload variant.

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
/// and this handles auth metadata, `x-frm-connector` routing, and the gRPC
/// call.
///
/// No access token is sent. Before a bearer-authenticated provider is
/// onboarded, thread one into `NotifyConnectorRequest.state` (see
/// `core::fraud_check::access_token`) — without it the notification reaches
/// the provider unauthenticated.
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
        .payment_method(None)
        .payment_method_type(None)
        .lineage_ids(LineageIds::new(
            processor.get_account().get_id().clone(),
            context.profile_id.clone(),
        ))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .build();

    let response = ucs_client
        .notify_connector(request, connector_auth_metadata, grpc_headers, event_type)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("UCS notify_connector gRPC call failed for the FRM notification")?
        .into_inner();

    let response =
        hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData::foreign_try_from(
            response,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse UCS FRM notification response")?;

    validate_notify_connector_response(response, connector_name)
}

#[cfg(feature = "v1")]
fn validate_notify_connector_response(
    response: hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData,
    connector_name: String,
) -> RouterResult<()> {
    if response.error_code.is_some()
        || response.error_message.is_some()
        || !(200..300).contains(&response.status_code)
    {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::ExternalConnectorError {
                code: response
                    .error_code
                    .unwrap_or_else(|| hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
                message: response.error_message.unwrap_or_else(|| {
                    hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()
                }),
                connector: connector_name,
                status_code: response.status_code,
                reason: None,
            }
        ));
    }

    Ok(())
}

#[cfg(all(test, feature = "v1"))]
mod tests {
    use hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData;

    use super::validate_notify_connector_response;

    #[test]
    fn frm_notification_connector_error_is_not_treated_as_success() {
        let response = NotifyConnectorResponseData {
            status_code: 502,
            error_code: Some("bad_gateway".to_string()),
            error_message: Some("connector unavailable".to_string()),
        };

        assert!(validate_notify_connector_response(response, "stripe".to_string()).is_err());
    }

    #[test]
    fn frm_notification_success_is_accepted() {
        let response = NotifyConnectorResponseData {
            status_code: 200,
            error_code: None,
            error_message: None,
        };

        assert!(validate_notify_connector_response(response, "stripe".to_string()).is_ok());
    }
}
