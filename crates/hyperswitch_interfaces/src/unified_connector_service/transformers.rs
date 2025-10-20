use common_enums::AttemptStatus;

use crate::helpers::ForeignTryFrom;
use diesel_models::enums as storage_enums;
use hyperswitch_domain_models::{
    router_data::{ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::RefundsData,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};
use masking::ExposeInterface;
use std::collections::HashMap;
use unified_connector_service_client::payments::{self as payments_grpc, Identifier};

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
    /// Failed to perform Payment Refund from gRPC Server
    #[error("Failed to perform Payment Refund from gRPC Server")]
    PaymentRefundFailure,

    /// Failed to perform Refund Sync from gRPC Server
    #[error("Failed to perform Refund Sync from gRPC Server")]
    RefundSyncFailure,

    /// Error of unhandled grpc flow
    #[error("Unhandled ucs flow")]
    InternalError,
}

impl ForeignTryFrom<hyperswitch_domain_models::router_request_types::BrowserInformation>
    for payments_grpc::BrowserInformation
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        browser_info: hyperswitch_domain_models::router_request_types::BrowserInformation,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            color_depth: browser_info.color_depth.map(|v| v.into()),
            java_enabled: browser_info.java_enabled,
            java_script_enabled: browser_info.java_script_enabled,
            language: browser_info.language,
            screen_height: browser_info.screen_height,
            screen_width: browser_info.screen_width,
            ip_address: browser_info.ip_address.map(|ip| ip.to_string()),
            accept_header: browser_info.accept_header,
            user_agent: browser_info.user_agent,
            os_type: browser_info.os_type,
            os_version: browser_info.os_version,
            device_model: browser_info.device_model,
            accept_language: browser_info.accept_language,
            time_zone_offset_minutes: browser_info.time_zone,
            referer: browser_info.referer,
        })
    }
}

impl ForeignTryFrom<storage_enums::CaptureMethod> for payments_grpc::CaptureMethod {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(capture_method: storage_enums::CaptureMethod) -> Result<Self, Self::Error> {
        match capture_method {
            common_enums::CaptureMethod::Automatic => Ok(Self::Automatic),
            common_enums::CaptureMethod::Manual => Ok(Self::Manual),
            common_enums::CaptureMethod::ManualMultiple => Ok(Self::ManualMultiple),
            common_enums::CaptureMethod::Scheduled => Ok(Self::Scheduled),
            common_enums::CaptureMethod::SequentialAutomatic => Ok(Self::SequentialAutomatic),
        }
    }
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

/// Transform UCS RefundResponse into Result<RefundsResponseData, ErrorResponse>
impl ForeignTryFrom<payments_grpc::RefundResponse> for Result<RefundsResponseData, ErrorResponse> {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(response: payments_grpc::RefundResponse) -> Result<Self, Self::Error> {
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

        let response = if response.error_code.is_some() {
            Err(ErrorResponse {
                code: response.error_code().to_owned(),
                message: response.error_message().to_owned(),
                reason: Some(response.error_message().to_owned()),
                status_code,
                attempt_status: None,
                connector_transaction_id: connector_response_reference_id,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let refund_status = match response.status {
                0 => common_enums::RefundStatus::Pending, // REFUND_STATUS_UNSPECIFIED
                1 => common_enums::RefundStatus::Failure, // REFUND_FAILURE
                2 => common_enums::RefundStatus::ManualReview, // REFUND_MANUAL_REVIEW
                3 => common_enums::RefundStatus::Pending, // REFUND_PENDING
                4 => common_enums::RefundStatus::Success, // REFUND_SUCCESS
                5 => common_enums::RefundStatus::TransactionFailure, // REFUND_TRANSACTION_FAILURE
                _ => common_enums::RefundStatus::Pending, // Default fallback
            };

            Ok(RefundsResponseData {
                connector_refund_id: response.refund_id,
                refund_status,
            })
        };

        Ok(response)
    }
}

impl ForeignTryFrom<common_enums::Currency> for payments_grpc::Currency {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(currency: common_enums::Currency) -> Result<Self, Self::Error> {
        Self::from_str_name(&currency.to_string()).ok_or_else(|| {
            UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                "Failed to parse currency".to_string(),
            )
            .into()
        })
    }
}

// REFUND TRANSFORMERS
// ============================================================================

/// Transform RouterData for Execute refund into UCS PaymentServiceRefundRequest
impl ForeignTryFrom<&RouterData<Execute, RefundsData, RefundsResponseData>>
    for payments_grpc::PaymentServiceRefundRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<Execute, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let currency = payments_grpc::Currency::foreign_try_from(router_data.request.currency)?;

        let transaction_id = Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.request.connector_transaction_id.clone(),
            )),
        };

        let request_ref_id = Some(Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(
                router_data.connector_request_reference_id.clone(),
            )),
        });

        // Convert metadata to gRPC format
        let metadata = router_data
            .request
            .connector_metadata
            .as_ref()
            .map(|metadata| {
                metadata
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.to_string()))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let refund_metadata = router_data
            .request
            .refund_connector_metadata
            .as_ref()
            .map(|metadata| {
                metadata
                    .clone()
                    .expose()
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.to_string()))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_else(|| {
                // Try to extract payment method details from original payment's connector_metadata
                extract_payment_details_for_refund(router_data.request.connector_metadata.as_ref())
                    .unwrap_or_else(|| {
                        // Provide default metadata when missing to avoid connector errors
                        HashMap::from([("refund_source".to_string(), "hyperswitch".to_string())])
                    })
            });

        Ok(Self {
            request_ref_id,
            refund_id: router_data.request.refund_id.clone(),
            transaction_id: Some(transaction_id),
            payment_amount: router_data.request.payment_amount,
            currency: currency as i32,
            minor_payment_amount: router_data.request.minor_payment_amount.get_amount_as_i64(),
            refund_amount: router_data.request.refund_amount,
            minor_refund_amount: router_data.request.minor_refund_amount.get_amount_as_i64(),
            reason: router_data.request.reason.clone(),
            webhook_url: router_data.request.webhook_url.clone(),
            merchant_account_id: router_data
                .request
                .merchant_account_id
                .as_ref()
                .map(|id| id.clone().expose().clone()),
            capture_method: router_data
                .request
                .capture_method
                .map(|cm| payments_grpc::CaptureMethod::foreign_try_from(cm))
                .transpose()
                .map_err(|_| {
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to convert capture method".to_string(),
                    )
                })?
                .map(|cm| cm as i32),
            metadata,
            refund_metadata,
            browser_info: router_data
                .request
                .browser_info
                .clone()
                .map(|bi| payments_grpc::BrowserInformation::foreign_try_from(bi))
                .transpose()
                .map_err(|_| {
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                        "Failed to convert browser info".to_string(),
                    )
                })?,
            access_token: router_data
                .access_token
                .as_ref()
                .map(|token| token.token.clone().expose()),
        })
    }
}

/// Extract payment method details from original payment's connector_metadata for refund purposes
/// This is specifically needed for connectors like Authorize.Net that require original payment details for refunds
fn extract_payment_details_for_refund(
    connector_metadata: Option<&serde_json::Value>,
) -> Option<HashMap<String, String>> {
    connector_metadata?.as_object().and_then(|metadata_obj| {
        // Look for payment details structures that connectors typically store
        // For Authorize.Net, this would be the PaymentDetails::CreditCard structure stored by construct_refund_payment_details()

        // Check if this is a PaymentDetails::CreditCard structure
        if let Some(credit_card_obj) = metadata_obj.get("CreditCard") {
            // This matches the PaymentDetails::CreditCard(CreditCardDetails) variant
            if let (Some(card_number), Some(expiration_date)) = (
                credit_card_obj.get("card_number").and_then(|v| v.as_str()),
                credit_card_obj
                    .get("expiration_date")
                    .and_then(|v| v.as_str()),
            ) {
                // Transform to the format expected by UCS backend
                let payment_structure = serde_json::json!({
                    "payment": {
                        "creditCard": {
                            "cardNumber": card_number,
                            "expirationDate": expiration_date
                        }
                    }
                });

                return Some(HashMap::from([(
                    "payment".to_string(),
                    payment_structure.to_string(),
                )]));
            }
        }

        // Check for direct credit card structure
        if let Some(credit_card_obj) = metadata_obj.get("creditCard") {
            // If we have creditCard details directly, wrap them in payment structure
            let payment_structure = serde_json::json!({
                "payment": {
                    "creditCard": credit_card_obj
                }
            });

            return Some(HashMap::from([(
                "payment".to_string(),
                payment_structure.to_string(),
            )]));
        }

        // Check if we already have a properly formatted payment object
        if let Some(payment_obj) = metadata_obj.get("payment") {
            return Some(HashMap::from([(
                "payment".to_string(),
                payment_obj.to_string(),
            )]));
        }

        // For other metadata structures, try to preserve all fields as they might be needed
        Some(
            metadata_obj
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
        )
    })
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
