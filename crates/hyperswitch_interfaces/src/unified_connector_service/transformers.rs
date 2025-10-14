use common_enums::AttemptStatus;
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_response_types::PaymentsResponseData,
};

use crate::{helpers::ForeignTryFrom, unified_connector_service::payments_grpc};

/// Unified Connector Service error variants
#[derive(Debug, Clone, thiserror::Error)]
pub enum UnifiedConnectorServiceError {
    /// Error occurred while communicating with the gRPC server.
    #[error("Error from gRPC Server : {0}")]
    ConnectionError(String),

    /// Failed to encode the request to the unified connector service.
    #[error("Failed to encode unified connector service request")]
    RequestEncodingFailed,

    /// Request encoding failed due to a specific reason.
    #[error("Request encoding failed : {0}")]
    RequestEncodingFailedWithReason(String),

    /// Failed to deserialize the response from the connector.
    #[error("Failed to deserialize connector response")]
    ResponseDeserializationFailed,

    /// The connector name provided is invalid or unrecognized.
    #[error("An invalid connector name was provided")]
    InvalidConnectorName,

    /// Connector name is missing
    #[error("Connector name is missing")]
    MissingConnectorName,

    /// A required field was missing in the request.
    #[error("Missing required field: {field_name}")]
    MissingRequiredField {
        /// Missing Field
        field_name: &'static str,
    },

    /// Multiple required fields were missing in the request.
    #[error("Missing required fields: {field_names:?}")]
    MissingRequiredFields {
        /// Missing Fields
        field_names: Vec<&'static str>,
    },

    /// The requested step or feature is not yet implemented.
    #[error("This step has not been implemented for: {0}")]
    NotImplemented(String),

    /// Parsing of some value or input failed.
    #[error("Parsing failed")]
    ParsingFailed,

    /// Data format provided is invalid
    #[error("Invalid Data format")]
    InvalidDataFormat {
        /// Field Name for which data is invalid
        field_name: &'static str,
    },

    /// Failed to obtain authentication type
    #[error("Failed to obtain authentication type")]
    FailedToObtainAuthType,

    /// Failed to inject metadata into request headers
    #[error("Failed to inject metadata into request headers: {0}")]
    HeaderInjectionFailed(String),

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Payment Authorize from gRPC Server")]
    PaymentAuthorizeFailure,

    /// Failed to perform Payment Get from gRPC Server
    #[error("Failed to perform Payment Get from gRPC Server")]
    PaymentGetFailure,

    /// Failed to perform Payment Setup Mandate from gRPC Server
    #[error("Failed to perform Setup Mandate from gRPC Server")]
    PaymentRegisterFailure,

    /// Failed to perform Payment Repeat Payment from gRPC Server
    #[error("Failed to perform Repeat Payment from gRPC Server")]
    PaymentRepeatEverythingFailure,

    /// Failed to transform incoming webhook from gRPC Server
    #[error("Failed to transform incoming webhook from gRPC Server")]
    WebhookTransformFailure,
}

#[allow(missing_docs)]
/// Webhook transform data structure containing UCS response information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookTransformData {
    pub event_type: api_models::webhooks::IncomingWebhookEvent,
    pub source_verified: bool,
    pub webhook_content: Option<payments_grpc::WebhookResponseContent>,
    pub response_ref_id: Option<String>,
}

impl ForeignTryFrom<payments_grpc::PaymentServiceGetResponse>
    for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        response: payments_grpc::PaymentServiceGetResponse,
    ) -> Result<Self, Self::Error> {
        let connector_response_reference_id =
            response.response_ref_id.as_ref().and_then(|identifier| {
                identifier
                    .id_type
                    .clone()
                    .and_then(|id_type| match id_type {
                        payments_grpc::identifier::IdType::Id(id) => Some(id),
                        payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                            Some(encoded_data)
                        }
                        payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                    })
            });

        let status_code = convert_connector_service_status_code(response.status_code)?;

        let resource_id: hyperswitch_domain_models::router_request_types::ResponseId = match response.transaction_id.as_ref().and_then(|id| id.id_type.clone()) {
            Some(payments_grpc::identifier::IdType::Id(id)) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(id),
            Some(payments_grpc::identifier::IdType::EncodedData(encoded_data)) => hyperswitch_domain_models::router_request_types::ResponseId::EncodedData(encoded_data),
            Some(payments_grpc::identifier::IdType::NoResponseIdMarker(_)) | None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
        };

        let response = if response.error_code.is_some() {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::AttemptStatusUnspecified => None,
                _ => Some(AttemptStatus::foreign_try_from(response.status())?),
            };

            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status,
                connector_transaction_id: connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from(response.status())?;

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(response.mandate_reference.map(|grpc_mandate| {
                        hyperswitch_domain_models::router_response_types::MandateReference {
                            connector_mandate_id: grpc_mandate.mandate_id,
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        }
                    })),
                    connector_metadata: None,
                    network_txn_id: response.network_txn_id.clone(),
                    connector_response_reference_id,
                    incremental_authorization_allowed: None,
                    charges: None,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl ForeignTryFrom<payments_grpc::PaymentStatus> for AttemptStatus {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(grpc_status: payments_grpc::PaymentStatus) -> Result<Self, Self::Error> {
        match grpc_status {
            payments_grpc::PaymentStatus::Started => Ok(Self::Started),
            payments_grpc::PaymentStatus::AuthenticationFailed => Ok(Self::AuthenticationFailed),
            payments_grpc::PaymentStatus::RouterDeclined => Ok(Self::RouterDeclined),
            payments_grpc::PaymentStatus::AuthenticationPending => Ok(Self::AuthenticationPending),
            payments_grpc::PaymentStatus::AuthenticationSuccessful => {
                Ok(Self::AuthenticationSuccessful)
            }
            payments_grpc::PaymentStatus::Authorized => Ok(Self::Authorized),
            payments_grpc::PaymentStatus::AuthorizationFailed => Ok(Self::AuthorizationFailed),
            payments_grpc::PaymentStatus::Charged => Ok(Self::Charged),
            payments_grpc::PaymentStatus::Authorizing => Ok(Self::Authorizing),
            payments_grpc::PaymentStatus::CodInitiated => Ok(Self::CodInitiated),
            payments_grpc::PaymentStatus::Voided => Ok(Self::Voided),
            payments_grpc::PaymentStatus::VoidInitiated => Ok(Self::VoidInitiated),
            payments_grpc::PaymentStatus::CaptureInitiated => Ok(Self::CaptureInitiated),
            payments_grpc::PaymentStatus::CaptureFailed => Ok(Self::CaptureFailed),
            payments_grpc::PaymentStatus::VoidFailed => Ok(Self::VoidFailed),
            payments_grpc::PaymentStatus::AutoRefunded => Ok(Self::AutoRefunded),
            payments_grpc::PaymentStatus::PartialCharged => Ok(Self::PartialCharged),
            payments_grpc::PaymentStatus::PartialChargedAndChargeable => {
                Ok(Self::PartialChargedAndChargeable)
            }
            payments_grpc::PaymentStatus::Unresolved => Ok(Self::Unresolved),
            payments_grpc::PaymentStatus::Pending => Ok(Self::Pending),
            payments_grpc::PaymentStatus::Failure => Ok(Self::Failure),
            payments_grpc::PaymentStatus::PaymentMethodAwaited => Ok(Self::PaymentMethodAwaited),
            payments_grpc::PaymentStatus::ConfirmationAwaited => Ok(Self::ConfirmationAwaited),
            payments_grpc::PaymentStatus::DeviceDataCollectionPending => {
                Ok(Self::DeviceDataCollectionPending)
            }
            payments_grpc::PaymentStatus::AttemptStatusUnspecified => Ok(Self::Unresolved),
        }
    }
}

#[allow(missing_docs)]
pub fn convert_connector_service_status_code(
    status_code: u32,
) -> Result<u16, error_stack::Report<UnifiedConnectorServiceError>> {
    u16::try_from(status_code).map_err(|err| {
        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(format!(
            "Failed to convert connector service status code to u16: {err}"
        ))
        .into()
    })
}
