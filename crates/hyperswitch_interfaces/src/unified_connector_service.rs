use common_enums::AttemptStatus;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_response_types::PaymentsResponseData,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::helpers::ForeignTryFrom;

/// Unified Connector Service (UCS) related transformers
pub mod transformers;

pub use transformers::UnifiedConnectorServiceError;

/// Type alias for return type used by unified connector service response handlers
type UnifiedConnectorServiceResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
>;

#[allow(missing_docs)]
pub fn handle_unified_connector_service_response_for_payment_get(
    response: payments_grpc::PaymentServiceGetResponse,
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

    Ok((router_data_response, status_code))
}

/// Extracts the payments response from UCS webhook content
pub fn get_payments_response_from_ucs_webhook_content(
    event_content: payments_grpc::EventContent,
) -> CustomResult<payments_grpc::PaymentServiceGetResponse, UnifiedConnectorServiceError> {
    match event_content.content {
        Some(
            unified_connector_service_client::payments::event_content::Content::PaymentsResponse(
                payments_response,
            ),
        ) => Ok(payments_response),
        Some(
            unified_connector_service_client::payments::event_content::Content::RefundsResponse(_),
        ) => Err(UnifiedConnectorServiceError::WebhookProcessingFailure).attach_printable(
            "UCS webhook contains refunds response but payments response was expected",
        )?,
        Some(
            unified_connector_service_client::payments::event_content::Content::DisputesResponse(_),
        ) => Err(UnifiedConnectorServiceError::WebhookProcessingFailure).attach_printable(
            "UCS webhook contains disputes response but payments response was expected",
        )?,
        Some(
            unified_connector_service_client::payments::event_content::Content::PayoutsResponse(_),
        ) => Err(UnifiedConnectorServiceError::WebhookProcessingFailure).attach_printable(
            "UCS webhook contains payouts response but payments response was expected",
        )?,
        None => Err(UnifiedConnectorServiceError::WebhookProcessingFailure)
            .attach_printable("Missing payments response in UCS webhook content")?,
    }
}

/// Extracts the payouts response from UCS webhook content.
///
/// Mirrors [`get_payments_response_from_ucs_webhook_content`]. UCS returns a
/// `PayoutServiceGetResponse` for payout events -- the same shape a
/// `PayoutService.Get` produces -- so the caller consumes the webhook directly
/// instead of re-deriving the status from the event type.
#[cfg(feature = "payouts")]
pub fn get_payouts_response_from_ucs_webhook_content(
    event_content: payments_grpc::EventContent,
) -> CustomResult<payments_grpc::PayoutServiceGetResponse, UnifiedConnectorServiceError> {
    match event_content.content {
        Some(
            unified_connector_service_client::payments::event_content::Content::PayoutsResponse(
                payouts_response,
            ),
        ) => Ok(payouts_response),
        Some(
            unified_connector_service_client::payments::event_content::Content::PaymentsResponse(_),
        ) => Err(UnifiedConnectorServiceError::WebhookProcessingFailure).attach_printable(
            "UCS webhook contains payments response but payouts response was expected",
        )?,
        Some(
            unified_connector_service_client::payments::event_content::Content::RefundsResponse(_),
        ) => Err(UnifiedConnectorServiceError::WebhookProcessingFailure).attach_printable(
            "UCS webhook contains refunds response but payouts response was expected",
        )?,
        Some(
            unified_connector_service_client::payments::event_content::Content::DisputesResponse(_),
        ) => Err(UnifiedConnectorServiceError::WebhookProcessingFailure).attach_printable(
            "UCS webhook contains disputes response but payouts response was expected",
        )?,
        None => Err(UnifiedConnectorServiceError::WebhookProcessingFailure)
            .attach_printable("Missing payouts response in UCS webhook content")?,
    }
}
