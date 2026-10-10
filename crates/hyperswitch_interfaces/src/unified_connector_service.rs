use std::str::FromStr;

use common_enums::AttemptStatus;
use common_utils::{
    errors::CustomResult,
    types::{AmountConvertor, MinorUnit, StringMinorUnitForConnector},
};
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
        ) => Err(UnifiedConnectorServiceError::NotImplemented(
            "UCS payouts webhook response handling".to_string(),
        ))?,
        None => Err(UnifiedConnectorServiceError::WebhookProcessingFailure)
            .attach_printable("Missing payments response in UCS webhook content")?,
    }
}

/// Builds the dispute payload from UCS webhook content.
///
/// Returns `Ok(None)` when the UCS dispute response carries no `dispute_amount` (UCS builds
/// that predate it), so the caller can fall back to the connector's own `get_dispute_details`.
pub fn get_dispute_payload_from_ucs_webhook_content(
    event_content: payments_grpc::EventContent,
) -> CustomResult<Option<crate::disputes::DisputePayload>, UnifiedConnectorServiceError> {
    let dispute_response = match event_content.content {
        Some(payments_grpc::event_content::Content::DisputesResponse(dispute_response)) => {
            dispute_response
        }
        Some(_) | None => Err(UnifiedConnectorServiceError::WebhookProcessingFailure)
            .attach_printable("UCS webhook content does not carry a disputes response")?,
    };

    let Some(dispute_amount) = dispute_response.dispute_amount else {
        return Ok(None);
    };

    let connector_dispute_id = dispute_response.connector_dispute_id.ok_or(
        UnifiedConnectorServiceError::MissingRequiredField {
            field_name: "connector_dispute_id".into(),
        },
    )?;

    let currency = match payments_grpc::Currency::try_from(dispute_amount.currency) {
        Ok(payments_grpc::Currency::Unspecified) | Err(_) => {
            Err(UnifiedConnectorServiceError::MissingRequiredField {
                field_name: "currency".into(),
            })
        }
        Ok(currency) => common_enums::Currency::from_str(currency.as_str_name())
            .map_err(|_| UnifiedConnectorServiceError::ParsingFailed),
    }
    .attach_printable("Failed to parse currency from UCS dispute response")?;

    let amount = StringMinorUnitForConnector
        .convert(MinorUnit::new(dispute_amount.minor_amount), currency)
        .change_context(UnifiedConnectorServiceError::ParsingFailed)
        .attach_printable("Failed to convert UCS dispute amount")?;

    let dispute_stage = match payments_grpc::DisputeStage::try_from(dispute_response.dispute_stage)
    {
        Ok(payments_grpc::DisputeStage::PreDispute) => common_enums::DisputeStage::PreDispute,
        Ok(payments_grpc::DisputeStage::ActiveDispute) => common_enums::DisputeStage::Dispute,
        Ok(payments_grpc::DisputeStage::PreArbitration) => {
            common_enums::DisputeStage::PreArbitration
        }
        Ok(payments_grpc::DisputeStage::Unspecified) | Err(_) => {
            Err(UnifiedConnectorServiceError::MissingRequiredField {
                field_name: "dispute_stage".into(),
            })?
        }
    };

    let connector_status = match dispute_response.connector_status_code {
        Some(connector_status_code) => connector_status_code,
        None => match payments_grpc::DisputeStatus::try_from(dispute_response.dispute_status) {
            Ok(payments_grpc::DisputeStatus::Unspecified) | Err(_) => {
                Err(UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "dispute_status".into(),
                })?
            }
            Ok(dispute_status) => dispute_status.as_str_name().to_string(),
        },
    };

    Ok(Some(crate::disputes::DisputePayload {
        amount,
        currency,
        dispute_stage,
        connector_status,
        connector_dispute_id,
        connector_reason: dispute_response
            .dispute_reason
            .or(dispute_response.dispute_message),
        connector_reason_code: None,
        challenge_required_by: dispute_response
            .due_date
            .map(unix_timestamp_to_primitive_date_time)
            .transpose()?,
        created_at: dispute_response
            .dispute_date
            .map(unix_timestamp_to_primitive_date_time)
            .transpose()?,
        updated_at: None,
        additional_details: None,
    }))
}

fn unix_timestamp_to_primitive_date_time(
    timestamp: i64,
) -> CustomResult<time::PrimitiveDateTime, UnifiedConnectorServiceError> {
    let date_time = time::OffsetDateTime::from_unix_timestamp(timestamp)
        .change_context(UnifiedConnectorServiceError::ParsingFailed)
        .attach_printable("Invalid unix timestamp in UCS dispute response")?;
    Ok(time::PrimitiveDateTime::new(
        date_time.date(),
        date_time.time(),
    ))
}
