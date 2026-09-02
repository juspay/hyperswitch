use std::str::FromStr;

use common_enums::AttemptStatus;
use common_types::primitive_wrappers::{ExtendedAuthorizationAppliedBool, OvercaptureEnabledBool};
use common_utils::{errors::ErrorSwitch, request::Method, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::{ApiErrorResponse, NotImplementedMessage},
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorResponseData, ErrorResponse,
        ExtendedAuthorizationResponseData,
    },
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use hyperswitch_masking::ExposeInterface;
use prost::Message;

use crate::{
    errors::ConnectorError,
    helpers::{ForeignFrom, ForeignTryFrom},
    unified_connector_service::payments_grpc,
};

/// UCS error code indicating the connector returned a 4xx/5xx HTTP response (with `http_status_code` set).
const CONNECTOR_ERROR_RESPONSE_CODE: &str = "CONNECTOR_ERROR_RESPONSE";

// Synthetic timeout status used when UCS reports a connector-side timeout over gRPC.
// No connector HTTP response is available in this path; this keeps UCS aligned with
// direct connector timeout handling.
const CONNECTOR_TIMEOUT_HTTP_STATUS_CODE: u16 = 504;

/// Unified Connector Service error variants
#[derive(Debug, Clone, thiserror::Error)]
pub enum UnifiedConnectorServiceError {
    /// Error occurred while communicating with the gRPC server.
    #[error("Error from gRPC Server : {0}")]
    ConnectionError(String),

    /// Failed to encode the request to the unified connector service.
    #[error("Failed to encode unified connector service request")]
    RequestEncodingFailed,

    /// Failed to process webhook from unified connector service.
    #[error("Failed to process webhook from unified connector service")]
    WebhookProcessingFailure,

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

    /// Tonic gRPC status error from UCS.
    /// Use http_status() to get the corresponding HTTP status code.
    #[error("UCS error: {code:?} - {message}")]
    TonicStatus {
        /// Tonic status code
        code: tonic::Code,
        /// Error message from UCS
        message: String,
    },

    /// Connector error received through UCS.
    /// For connector HTTP errors, this contains the original connector HTTP status code.
    /// For connector timeout errors without a connector HTTP response, this contains the
    /// synthetic timeout status used by Hyperswitch.
    #[error("Connector error via UCS: {0:?}")]
    ConnectorError(Box<ConnectorErrorInner>),

    /// Failed to perform Payment Create Order from gRPC Server
    #[error("Failed to perform Payment Create Order from gRPC Server")]
    PaymentCreateOrderFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform. Granular Payment Authorize from gRPC Server")]
    PaymentAuthorizeGranularFailure,

    /// Failed to perform Create Session Token from gRPC Server
    #[error("Failed to perform Create Session Token from gRPC Server")]
    CreateSessionTokenFailure,

    /// Failed to perform Create Access Token from gRPC Server
    #[error("Failed to perform Create Access Token from gRPC Server")]
    CreateAccessTokenFailure,

    /// Failed to perform Payment Method Tokenize from gRPC Server
    #[error("Failed to perform Payment Method Tokenize from gRPC Server")]
    PaymentMethodTokenizeFailure,

    /// Failed to perform Create Connector Customer from gRPC Server
    #[error("Failed to perform Create Connector Customer from gRPC Server")]
    CreateConnectorCustomerFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Payment Authorize from gRPC Server")]
    PaymentAuthorizeFailure,

    /// Failed to perform Payment Authenticate from gRPC Server
    #[error("Failed to perform Payment Pre Authenticate from gRPC Server")]
    PaymentPreAuthenticateFailure,

    /// Failed to perform Payment Authenticate from gRPC Server
    #[error("Failed to perform Payment Authenticate from gRPC Server")]
    PaymentAuthenticateFailure,

    /// Failed to perform Payment Authenticate from gRPC Server
    #[error("Failed to perform Payment Post Authenticate from gRPC Server")]
    PaymentPostAuthenticateFailure,

    /// Failed to perform Payment Get from gRPC Server
    #[error("Failed to perform Payment Get from gRPC Server")]
    PaymentGetFailure,

    /// Failed to perform Payment Capture from gRPC Server
    #[error("Failed to perform Payment Capture from gRPC Server")]
    PaymentCaptureFailure,

    /// Failed to perform Payment Setup Recurring from gRPC Server
    #[error("Failed to perform Setup Recurring from gRPC Server")]
    PaymentSetupRecurringFailure,

    /// Failed to perform Recurring Payment Charge from gRPC Server
    #[error("Failed to perform Recurring Payment Charge from gRPC Server")]
    RecurringPaymentChargeFailure,

    /// Failed to perform Payment Refund from gRPC Server
    #[error("Failed to perform Payment Refund from gRPC Server")]
    PaymentRefundFailure,

    /// Failed to perform Refund Sync from gRPC Server
    #[error("Failed to perform Refund Sync from gRPC Server")]
    RefundSyncFailure,

    /// Failed to handle incoming webhook event from gRPC Server
    #[error("Failed to handle incoming webhook event from gRPC Server")]
    IncomingWebhookHandleEventFailure,

    /// Failed to parse incoming webhook event from gRPC Server
    #[error("Failed to parse incoming webhook event from gRPC Server")]
    IncomingWebhookParseEventFailure,

    /// Failed to perform Payment Void from gRPC Server
    #[error("Failed to perform Void from gRPC Server")]
    PaymentVoidFailure,

    /// Failed to perform Create Sdk Session Token from gRPC Server
    #[error("Failed to perform Create Sdk Session Token from gRPC Server")]
    CreateSdkSessionTokenFailure,

    /// Failed to perform Payment Incremental Authorization from gRPC Server
    #[error("Failed to perform Payment Incremental Authorization from gRPC Server")]
    PaymentIncrementalAuthorizationFailure,

    /// Failed to perform Payout Create from gRPC Server
    #[error("Failed to perform Payout Create from gRPC Server")]
    PayoutCreateFailure,

    /// Failed to perform Payout Transfer from gRPC Server
    #[error("Failed to perform Payout Transfer from gRPC Server")]
    PayoutTransferFailure,

    /// Failed to perform Payout Get from gRPC Server
    #[error("Failed to perform Payout Get from gRPC Server")]
    PayoutGetFailure,

    /// Failed to perform Payout Void from gRPC Server
    #[error("Failed to perform Payout Void from gRPC Server")]
    PayoutVoidFailure,

    /// Failed to perform Payout Stage from gRPC Server
    #[error("Failed to perform Payout Stage from gRPC Server")]
    PayoutStageFailure,

    /// Failed to perform Payout Create Recipient from gRPC Server
    #[error("Failed to perform Payout Create Recipient from gRPC Server")]
    PayoutCreateRecipientFailure,

    /// Failed to perform Payout Enroll Disburse Account from gRPC Server
    #[error("Failed to perform Payout Enroll Disburse Account from gRPC Server")]
    PayoutEnrollDisburseAccountFailure,

    /// Failed to perform Surcharge Calculate from gRPC Server
    #[error("Failed to perform Surcharge Calculate from gRPC Server")]
    SurchargeCalculateFailure,

    /// Failed to perform Notify Connector via gRPC Server
    #[error("Failed to perform Notify Connector from gRPC Server")]
    NotifyConnectorFailure,
}

/// Inner data for [`UnifiedConnectorServiceError::ConnectorError`].
/// Boxed to keep the enum's memory footprint small.
#[derive(Debug, Clone)]
pub struct ConnectorErrorInner {
    /// Connector error code
    pub code: String,
    /// Connector error message
    pub message: String,
    /// Connector HTTP status code, or the synthetic timeout status when no connector HTTP
    /// response was received.
    pub status_code: u16,
    /// Optional reason for the error
    pub reason: Option<String>,
    /// Name of the connector that returned the error
    pub connector: String,
    /// Connector's unique transaction identifier (e.g. Adyen `pspReference`), when the
    /// connector returns one alongside the error response
    pub connector_transaction_id: Option<String>,
    /// Network decline code from card scheme (e.g. Visa/Mastercard decline code)
    pub network_decline_code: Option<String>,
    /// Network advice code for retry logic
    pub network_advice_code: Option<String>,
    /// Network-specific error message
    pub network_error_message: Option<String>,
}

impl From<&ConnectorErrorInner> for ErrorResponse {
    fn from(error: &ConnectorErrorInner) -> Self {
        Self {
            code: error.code.clone(),
            message: error.message.clone(),
            reason: error.reason.clone(),
            status_code: error.status_code,
            attempt_status: None,
            connector_transaction_id: error.connector_transaction_id.clone(),
            connector_response_reference_id: None,
            network_decline_code: error.network_decline_code.clone(),
            network_advice_code: error.network_advice_code.clone(),
            network_error_message: error.network_error_message.clone(),
            connector_metadata: None,
        }
    }
}

impl ForeignTryFrom<payments_grpc::PaymentChargeType> for common_enums::PaymentChargeType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        charge_type: payments_grpc::PaymentChargeType,
    ) -> Result<Self, Self::Error> {
        match charge_type {
            payments_grpc::PaymentChargeType::StripeDirect => {
                Ok(Self::Stripe(common_enums::StripeChargeType::Direct))
            }
            payments_grpc::PaymentChargeType::StripeDestination => {
                Ok(Self::Stripe(common_enums::StripeChargeType::Destination))
            }
            payments_grpc::PaymentChargeType::Unspecified => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ParsingFailed,
            )
            .attach_printable("Received unspecified PaymentChargeType from gRPC")),
        }
    }
}

impl ForeignTryFrom<payments_grpc::AdyenSplitType> for common_enums::AdyenSplitType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(split_type: payments_grpc::AdyenSplitType) -> Result<Self, Self::Error> {
        match split_type {
            payments_grpc::AdyenSplitType::Unspecified => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ParsingFailed,
            )
            .attach_printable("Received unspecified AdyenSplitType from gRPC")),
            payments_grpc::AdyenSplitType::BalanceAccount => Ok(Self::BalanceAccount),
            payments_grpc::AdyenSplitType::AcquiringFees => Ok(Self::AcquiringFees),
            payments_grpc::AdyenSplitType::PaymentFee => Ok(Self::PaymentFee),
            payments_grpc::AdyenSplitType::AdyenFees => Ok(Self::AdyenFees),
            payments_grpc::AdyenSplitType::AdyenCommission => Ok(Self::AdyenCommission),
            payments_grpc::AdyenSplitType::AdyenMarkup => Ok(Self::AdyenMarkup),
            payments_grpc::AdyenSplitType::Interchange => Ok(Self::Interchange),
            payments_grpc::AdyenSplitType::SchemeFee => Ok(Self::SchemeFee),
            payments_grpc::AdyenSplitType::Commission => Ok(Self::Commission),
            payments_grpc::AdyenSplitType::TopUp => Ok(Self::TopUp),
            payments_grpc::AdyenSplitType::Vat => Ok(Self::Vat),
        }
    }
}

impl ForeignTryFrom<payments_grpc::AdyenSplitItem> for common_types::domain::AdyenSplitItem {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(item: payments_grpc::AdyenSplitItem) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.map(MinorUnit::new),
            split_type: common_enums::AdyenSplitType::foreign_try_from(
                payments_grpc::AdyenSplitType::try_from(item.split_type).map_err(|_| {
                    error_stack::Report::new(UnifiedConnectorServiceError::ParsingFailed)
                        .attach_printable(format!(
                            "Invalid AdyenSplitType value: {}",
                            item.split_type
                        ))
                })?,
            )?,
            account: item.account,
            reference: item.reference,
            description: item.description,
        })
    }
}

impl ForeignTryFrom<payments_grpc::AdyenSplitData> for common_types::domain::AdyenSplitData {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(data: payments_grpc::AdyenSplitData) -> Result<Self, Self::Error> {
        Ok(Self {
            store: data.store,
            split_items: data
                .split_items
                .into_iter()
                .map(common_types::domain::AdyenSplitItem::foreign_try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl ForeignTryFrom<payments_grpc::StripeSplitResponseData>
    for common_types::payments::StripeChargeResponseData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        stripe: payments_grpc::StripeSplitResponseData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            charge_id: stripe.charge_id,
            charge_type: common_enums::PaymentChargeType::foreign_try_from(
                payments_grpc::PaymentChargeType::try_from(stripe.charge_type).map_err(|_| {
                    error_stack::Report::new(UnifiedConnectorServiceError::ParsingFailed)
                        .attach_printable(format!(
                            "Invalid PaymentChargeType value: {:?}",
                            stripe.charge_type
                        ))
                })?,
            )?,
            application_fees: stripe.application_fees.map(MinorUnit::new),
            transfer_account_id: stripe.transfer_account_id,
            on_behalf_of: stripe.on_behalf_of,
        })
    }
}

impl ForeignTryFrom<payments_grpc::ConnectorSplitResponseData>
    for common_types::payments::ConnectorChargeResponseData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        split_data: payments_grpc::ConnectorSplitResponseData,
    ) -> Result<Self, Self::Error> {
        match split_data.split_response_type {
            Some(
                payments_grpc::connector_split_response_data::SplitResponseType::StripeSplitResponse(
                    stripe,
                ),
            ) => Ok(Self::StripeSplitPayment(
                common_types::payments::StripeChargeResponseData::foreign_try_from(stripe)?,
            )),
            Some(
                payments_grpc::connector_split_response_data::SplitResponseType::AdyenSplitResponse(
                    adyen,
                ),
            ) => Ok(Self::AdyenSplitPayment(
                common_types::domain::AdyenSplitData::foreign_try_from(adyen)?,
            )),
            None => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ParsingFailed,
            )
            .attach_printable("ConnectorSplitResponseData has no split_response_type")),
        }
    }
}

impl ForeignTryFrom<(payments_grpc::PaymentServiceGetResponse, AttemptStatus)>
    for Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (response, prev_status): (payments_grpc::PaymentServiceGetResponse, AttemptStatus),
    ) -> Result<Self, Self::Error> {
        let status_code = convert_connector_service_status_code(response.status_code)?;

        let connector_transaction_id = if response.connector_transaction_id.is_empty() {
            hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId
        } else {
            hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                response.connector_transaction_id.clone(),
            )
        };

        let connector_details = response
            .error
            .as_ref()
            .and_then(|e| e.connector_details.as_ref());

        let response = if let Some(error_code) =
            connector_details.and_then(|details| details.code.clone())
        {
            let attempt_status = match response.status() {
                payments_grpc::PaymentStatus::Unspecified => None,
                _ => Some(AttemptStatus::foreign_try_from((
                    response.status(),
                    prev_status,
                ))?),
            };

            Err(ErrorResponse {
                code: error_code,
                message: connector_details
                    .as_ref()
                    .and_then(|cd| cd.message.clone())
                    .ok_or(
                        error_stack::Report::new(
                            UnifiedConnectorServiceError::ResponseDeserializationFailed,
                        )
                        .attach_printable("Missing error message in UCS response ErrorInfo"),
                    )?,
                reason: connector_details.as_ref().and_then(|cd| cd.reason.clone()),
                status_code,
                attempt_status,
                connector_transaction_id: connector_transaction_id.get_optional_response_id(),
                connector_response_reference_id: response.connector_reference_id,
                network_decline_code: response.error.as_ref().and_then(|error| {
                    error.issuer_details.as_ref().and_then(|id| {
                        id.network_details
                            .as_ref()
                            .and_then(|nd| nd.decline_code.clone())
                    })
                }),
                network_advice_code: response.error.as_ref().and_then(|error| {
                    error.issuer_details.as_ref().and_then(|id| {
                        id.network_details
                            .as_ref()
                            .and_then(|nd| nd.advice_code.clone())
                    })
                }),
                network_error_message: response.error.as_ref().and_then(|error| {
                    error.issuer_details.as_ref().and_then(|id| {
                        id.network_details
                            .as_ref()
                            .and_then(|nd| nd.error_message.clone())
                    })
                }),
                connector_metadata: None,
            })
        } else {
            let status = AttemptStatus::foreign_try_from((response.status(), prev_status))?;

            let connector_metadata = response.connector_feature_data.as_ref().and_then(|m| {
                let raw = m.clone().expose();
                match serde_json::from_str::<serde_json::Value>(&raw) {
                    Ok(v) => Some(v),
                    Err(err) => {
                        router_env::logger::warn!(
                            error = %err,
                            "failed to deserialize PSync response.connector_feature_data into \
                             connector_metadata"
                        );
                        None
                    }
                }
            });

            Ok((
                PaymentsResponseData::TransactionResponse {
                    resource_id: connector_transaction_id,
                    redirection_data: Box::new(
                        response
                            .redirection_data
                            .clone()
                            .map(ForeignTryFrom::foreign_try_from)
                            .transpose()?,
                    ),
                    mandate_reference: Box::new(
                        response
                            .mandate_reference_details
                            .map(
                                hyperswitch_domain_models::router_response_types::MandateReference::foreign_try_from,
                            )
                            .transpose()?,
                    ),
                    connector_metadata,
                    network_txn_id: response.network_transaction_id.clone(),
                    network_txn_link_id: response.network_txn_link_id.clone(),
                    connector_response_reference_id: response.connector_reference_id,
                    payment_account_reference: response.payment_account_reference,
                    incremental_authorization_allowed: response.incremental_authorization_allowed,
                    authentication_data: None,
                    charges: response.splits.map(common_types::payments::ConnectorChargeResponseData::foreign_try_from).transpose()?,
                },
                status,
            ))
        };

        Ok(response)
    }
}

impl ForeignTryFrom<payments_grpc::MandateReferenceDetails>
    for hyperswitch_domain_models::router_response_types::MandateReference
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: payments_grpc::MandateReferenceDetails,
    ) -> Result<Self, Self::Error> {
        let mandate_metadata = value
            .mandate_metadata
            .map(|metadata| {
                let raw = metadata.expose();
                serde_json::from_str::<serde_json::Value>(&raw)
                    .map(hyperswitch_masking::Secret::new)
                    .change_context(UnifiedConnectorServiceError::ResponseDeserializationFailed)
                    .attach_printable("Failed to deserialize UCS mandate_metadata")
            })
            .transpose()?;

        Ok(Self {
            connector_mandate_id: value.connector_mandate_id,
            payment_method_id: value.payment_method_id,
            mandate_metadata,
            connector_mandate_request_reference_id: value.connector_mandate_request_reference_id,
        })
    }
}

impl ForeignTryFrom<(payments_grpc::PaymentStatus, Self)> for AttemptStatus {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        (grpc_status, prev_status): (payments_grpc::PaymentStatus, Self),
    ) -> Result<Self, Self::Error> {
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
            payments_grpc::PaymentStatus::VoidedPostCapture => Ok(Self::Voided),
            payments_grpc::PaymentStatus::Unspecified => Ok(prev_status),
            payments_grpc::PaymentStatus::PartiallyAuthorized => Ok(Self::PartiallyAuthorized),
            payments_grpc::PaymentStatus::Expired => Ok(Self::Expired),
        }
    }
}

// Transformer for ConnectorResponseData from UCS proto to Hyperswitch domain type
impl ForeignTryFrom<payments_grpc::ConnectorResponseData> for ConnectorResponseData {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::ConnectorResponseData) -> Result<Self, Self::Error> {
        // Extract additional_payment_method_data
        let additional_payment_method_data = value
            .additional_payment_method_data
            .and_then(|apmd| {
                AdditionalPaymentMethodConnectorResponse::foreign_try_from(apmd)
                    .inspect_err(|e| {
                        router_env::logger::warn!(
                            error=?e,
                            "Failed to deserialize additional_payment_method_data from UCS - setting to None"
                        );
                    })
                    .ok()
            });

        let extended_authorization_response_data =
            value.extended_authorization_response_data.map(|data| {
                ExtendedAuthorizationResponseData {
                    capture_before: data
                        .capture_before
                        .and_then(|ts| time::OffsetDateTime::from_unix_timestamp(ts).ok())
                        .map(|offset_dt| {
                            time::PrimitiveDateTime::new(offset_dt.date(), offset_dt.time())
                        }),
                    extended_authentication_applied: data
                        .extended_authentication_applied
                        .map(ExtendedAuthorizationAppliedBool::from),
                    extended_authorization_last_applied_at: None, // This field has to be added to UCS
                }
            });

        let is_overcapture_enabled = value
            .is_overcapture_enabled
            .map(OvercaptureEnabledBool::new);

        Ok(Self::new(
            additional_payment_method_data,
            is_overcapture_enabled,
            extended_authorization_response_data,
            None,
        ))
    }
}

// Transformer for AdditionalPaymentMethodConnectorResponse
impl ForeignTryFrom<payments_grpc::AdditionalPaymentMethodConnectorResponse>
    for AdditionalPaymentMethodConnectorResponse
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        value: payments_grpc::AdditionalPaymentMethodConnectorResponse,
    ) -> Result<Self, Self::Error> {
        match value.payment_method_data {
            Some(
                payments_grpc::additional_payment_method_connector_response::PaymentMethodData::Card(
                    card_data,
                ),
            ) => Ok(Self::Card {
                authentication_data: card_data.authentication_data.and_then(|data| {
                    serde_json::from_slice(data.as_slice())
                        .inspect_err(|e| {
                            router_env::logger::warn!(
                                deserialization_error=?e,
                                "Failed to deserialize authentication_data from UCS connector response"
                            );
                        })
                        .ok()
                }),
                payment_checks: card_data.payment_checks.and_then(|data| {
                    serde_json::from_slice(data.as_slice())
                        .inspect_err(|e| {
                            router_env::logger::warn!(
                                deserialization_error=?e,
                                "Failed to deserialize payment_checks from UCS connector response"
                            );
                        })
                        .ok()
                }),
                card_network: card_data.card_network,
                domestic_network: card_data.domestic_network,
                auth_code: card_data.auth_code,
            }),
            Some(payments_grpc::additional_payment_method_connector_response::PaymentMethodData::Upi(upi_data)) => {
                let upi_mode = upi_data
                    .upi_mode
                    .map(|mode| {
                        payments_grpc::UpiSource::try_from(mode)
                            .change_context(UnifiedConnectorServiceError::ParsingFailed)
                            .attach_printable("Failed to parse upi_mode from UCS connector response")
                    })
                    .transpose()?
                    .map(hyperswitch_domain_models::payment_method_data::UpiSource::foreign_from);
                Ok(Self::Upi { upi_mode })
            }
            Some(
                payments_grpc::additional_payment_method_connector_response::PaymentMethodData::GooglePay(
                    google_pay_data,
                ),
            ) => Ok(Self::GooglePay {
                auth_code: google_pay_data.auth_code,
            }),
            Some(
                payments_grpc::additional_payment_method_connector_response::PaymentMethodData::ApplePay(
                    apple_pay_data,
                ),
            ) => Ok(Self::ApplePay {
                auth_code: apple_pay_data.auth_code,
            }),
            Some(payments_grpc::additional_payment_method_connector_response::PaymentMethodData::BankRedirect(bank_redirect_data)) => {
                let interac = bank_redirect_data.interac.map(|proto_interac| {
                    hyperswitch_domain_models::router_data::InteracCustomerInfo {
                        customer_info: proto_interac.customer_info.map(|info| {
                            common_types::payments::InteracCustomerInfoDetails {
                                customer_name: info.customer_name.map(|secret| hyperswitch_masking::Secret::new(secret.expose())),
                                customer_email: info.customer_email
                                    .and_then(|secret| {
                                        common_utils::pii::Email::from_str(&secret.expose())
                                            .map_err(|e| {
                                                router_env::logger::warn!(
                                                    email_parse_error=?e,
                                                    "Failed to parse customer_email from UCS InteracCustomerInfo"
                                                );
                                                e
                                            })
                                            .ok()
                                    }),
                                customer_phone_number: info.customer_phone_number.map(|secret| hyperswitch_masking::Secret::new(secret.expose())),
                                customer_bank_id: info.customer_bank_id.map(|secret| hyperswitch_masking::Secret::new(secret.expose())),
                                customer_bank_name: info.customer_bank_name.map(|secret| hyperswitch_masking::Secret::new(secret.expose())),
                            }
                        }),
                    }
                });
                Ok(Self::BankRedirect { interac })

            }
            None => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
            )
            .attach_printable("Unexpected error: payment_method_data is None in UCS connector response")),
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

// Bank Debit Reverse Transformations: Proto -> Hyperswitch

impl ForeignTryFrom<payments_grpc::Ach>
    for hyperswitch_domain_models::payment_method_data::BankDebitData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(ach: payments_grpc::Ach) -> Result<Self, Self::Error> {
        let bank_name = payments_grpc::BankNames::try_from(ach.bank_name)
            .ok()
            .and_then(|bn| common_enums::BankNames::foreign_try_from(bn).ok());

        let bank_type = payments_grpc::BankType::try_from(ach.bank_type)
            .ok()
            .and_then(|bt| common_enums::BankType::foreign_try_from(bt).ok());

        let bank_holder_type = payments_grpc::BankHolderType::try_from(ach.bank_holder_type)
            .ok()
            .and_then(|bht| common_enums::BankHolderType::foreign_try_from(bht).ok());

        Ok(Self::AchBankDebit {
            account_number: hyperswitch_masking::Secret::new(
                ach.account_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "account_number",
                    })?
                    .expose(),
            ),
            routing_number: hyperswitch_masking::Secret::new(
                ach.routing_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "routing_number",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: ach
                .bank_account_holder_name
                .map(|s| hyperswitch_masking::Secret::new(s.expose())),
            bank_name,
            bank_type,
            bank_holder_type,
        })
    }
}

impl ForeignTryFrom<payments_grpc::Sepa>
    for hyperswitch_domain_models::payment_method_data::BankDebitData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(sepa: payments_grpc::Sepa) -> Result<Self, Self::Error> {
        Ok(Self::SepaBankDebit {
            iban: hyperswitch_masking::Secret::new(
                sepa.iban
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "iban",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: sepa
                .bank_account_holder_name
                .map(|name| hyperswitch_masking::Secret::new(name.expose())),
        })
    }
}

impl ForeignTryFrom<payments_grpc::Bacs>
    for hyperswitch_domain_models::payment_method_data::BankDebitData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bacs: payments_grpc::Bacs) -> Result<Self, Self::Error> {
        Ok(Self::BacsBankDebit {
            account_number: hyperswitch_masking::Secret::new(
                bacs.account_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "account_number",
                    })?
                    .expose(),
            ),
            sort_code: hyperswitch_masking::Secret::new(
                bacs.sort_code
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "sort_code",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: bacs
                .bank_account_holder_name
                .map(|name| hyperswitch_masking::Secret::new(name.expose())),
        })
    }
}

impl ForeignTryFrom<payments_grpc::Becs>
    for hyperswitch_domain_models::payment_method_data::BankDebitData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(becs: payments_grpc::Becs) -> Result<Self, Self::Error> {
        Ok(Self::BecsBankDebit {
            account_number: hyperswitch_masking::Secret::new(
                becs.account_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "account_number",
                    })?
                    .expose(),
            ),
            bsb_number: hyperswitch_masking::Secret::new(
                becs.bsb_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "bsb_number",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: becs
                .bank_account_holder_name
                .map(|name| hyperswitch_masking::Secret::new(name.expose())),
        })
    }
}

impl ForeignTryFrom<payments_grpc::BankType> for common_enums::BankType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bank_type: payments_grpc::BankType) -> Result<Self, Self::Error> {
        match bank_type {
            payments_grpc::BankType::Checking => Ok(Self::Checking),
            payments_grpc::BankType::Savings => Ok(Self::Savings),
            payments_grpc::BankType::Salary => Ok(Self::Salary),
            payments_grpc::BankType::Payment => Ok(Self::Payment),
            payments_grpc::BankType::Bond => Ok(Self::Bond),
            payments_grpc::BankType::Transmission => Ok(Self::Transmission),
            payments_grpc::BankType::Current => Ok(Self::Current),
            payments_grpc::BankType::SubscriptionShare => Ok(Self::SubscriptionShare),
            payments_grpc::BankType::Unspecified => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
            )
            .attach_printable("BankType unsupported")),
        }
    }
}

impl ForeignTryFrom<payments_grpc::BankHolderType> for common_enums::BankHolderType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        bank_holder_type: payments_grpc::BankHolderType,
    ) -> Result<Self, Self::Error> {
        match bank_holder_type {
            payments_grpc::BankHolderType::Personal => Ok(Self::Personal),
            payments_grpc::BankHolderType::Business => Ok(Self::Business),
            payments_grpc::BankHolderType::Unspecified => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
            )
            .attach_printable("BankHolderType unspecified")),
        }
    }
}

impl ForeignTryFrom<payments_grpc::BankNames> for common_enums::BankNames {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bank_name: payments_grpc::BankNames) -> Result<Self, Self::Error> {
        match bank_name {
            payments_grpc::BankNames::AmericanExpress => Ok(Self::AmericanExpress),
            payments_grpc::BankNames::AffinBank => Ok(Self::AffinBank),
            payments_grpc::BankNames::AgroBank => Ok(Self::AgroBank),
            payments_grpc::BankNames::AllianceBank => Ok(Self::AllianceBank),
            payments_grpc::BankNames::AmBank => Ok(Self::AmBank),
            payments_grpc::BankNames::BankOfAmerica => Ok(Self::BankOfAmerica),
            payments_grpc::BankNames::BankOfChina => Ok(Self::BankOfChina),
            payments_grpc::BankNames::BankIslam => Ok(Self::BankIslam),
            payments_grpc::BankNames::BankMuamalat => Ok(Self::BankMuamalat),
            payments_grpc::BankNames::BankRakyat => Ok(Self::BankRakyat),
            payments_grpc::BankNames::BankSimpananNasional => Ok(Self::BankSimpananNasional),
            payments_grpc::BankNames::Barclays => Ok(Self::Barclays),
            payments_grpc::BankNames::BlikPsp => Ok(Self::BlikPSP),
            payments_grpc::BankNames::CapitalOne => Ok(Self::CapitalOne),
            payments_grpc::BankNames::Chase => Ok(Self::Chase),
            payments_grpc::BankNames::Citi => Ok(Self::Citi),
            payments_grpc::BankNames::CimbBank => Ok(Self::CimbBank),
            payments_grpc::BankNames::Discover => Ok(Self::Discover),
            payments_grpc::BankNames::NavyFederalCreditUnion => Ok(Self::NavyFederalCreditUnion),
            payments_grpc::BankNames::PentagonFederalCreditUnion => {
                Ok(Self::PentagonFederalCreditUnion)
            }
            payments_grpc::BankNames::SynchronyBank => Ok(Self::SynchronyBank),
            payments_grpc::BankNames::WellsFargo => Ok(Self::WellsFargo),
            payments_grpc::BankNames::AbnAmro => Ok(Self::AbnAmro),
            payments_grpc::BankNames::AsnBank => Ok(Self::AsnBank),
            payments_grpc::BankNames::Bunq => Ok(Self::Bunq),
            payments_grpc::BankNames::Handelsbanken => Ok(Self::Handelsbanken),
            payments_grpc::BankNames::HongLeongBank => Ok(Self::HongLeongBank),
            payments_grpc::BankNames::HsbcBank => Ok(Self::HsbcBank),
            payments_grpc::BankNames::Ing => Ok(Self::Ing),
            payments_grpc::BankNames::Knab => Ok(Self::Knab),
            payments_grpc::BankNames::KuwaitFinanceHouse => Ok(Self::KuwaitFinanceHouse),
            payments_grpc::BankNames::Moneyou => Ok(Self::Moneyou),
            payments_grpc::BankNames::Rabobank => Ok(Self::Rabobank),
            payments_grpc::BankNames::Regiobank => Ok(Self::Regiobank),
            payments_grpc::BankNames::Revolut => Ok(Self::Revolut),
            payments_grpc::BankNames::SnsBank => Ok(Self::SnsBank),
            payments_grpc::BankNames::TriodosBank => Ok(Self::TriodosBank),
            payments_grpc::BankNames::VanLanschot => Ok(Self::VanLanschot),
            payments_grpc::BankNames::ArzteUndApothekerBank => Ok(Self::ArzteUndApothekerBank),
            payments_grpc::BankNames::AustrianAnadiBankAg => Ok(Self::AustrianAnadiBankAg),
            payments_grpc::BankNames::BankAustria => Ok(Self::BankAustria),
            payments_grpc::BankNames::Bank99Ag => Ok(Self::Bank99Ag),
            payments_grpc::BankNames::BankhausCarlSpangler => Ok(Self::BankhausCarlSpangler),
            payments_grpc::BankNames::BankhausSchelhammerUndSchatteraAg => {
                Ok(Self::BankhausSchelhammerUndSchatteraAg)
            }
            payments_grpc::BankNames::BankMillennium => Ok(Self::BankMillennium),
            payments_grpc::BankNames::BankPekaoSa => Ok(Self::BankPEKAOSA),
            payments_grpc::BankNames::BawagPskAg => Ok(Self::BawagPskAg),
            payments_grpc::BankNames::BksBankAg => Ok(Self::BksBankAg),
            payments_grpc::BankNames::BrullKallmusBankAg => Ok(Self::BrullKallmusBankAg),
            payments_grpc::BankNames::BtvVierLanderBank => Ok(Self::BtvVierLanderBank),
            payments_grpc::BankNames::CapitalBankGraweGruppeAg => {
                Ok(Self::CapitalBankGraweGruppeAg)
            }
            payments_grpc::BankNames::CeskaSporitelna => Ok(Self::CeskaSporitelna),
            payments_grpc::BankNames::Dolomitenbank => Ok(Self::Dolomitenbank),
            payments_grpc::BankNames::EasybankAg => Ok(Self::EasybankAg),
            payments_grpc::BankNames::EPlatbyVub => Ok(Self::EPlatbyVUB),
            payments_grpc::BankNames::ErsteBankUndSparkassen => Ok(Self::ErsteBankUndSparkassen),
            payments_grpc::BankNames::FrieslandBank => Ok(Self::FrieslandBank),
            payments_grpc::BankNames::HypoAlpeadriabankInternationalAg => {
                Ok(Self::HypoAlpeadriabankInternationalAg)
            }
            payments_grpc::BankNames::HypoNoeLbFurNiederosterreichUWien => {
                Ok(Self::HypoNoeLbFurNiederosterreichUWien)
            }
            payments_grpc::BankNames::HypoOberosterreichSalzburgSteiermark => {
                Ok(Self::HypoOberosterreichSalzburgSteiermark)
            }
            payments_grpc::BankNames::HypoTirolBankAg => Ok(Self::HypoTirolBankAg),
            payments_grpc::BankNames::HypoVorarlbergBankAg => Ok(Self::HypoVorarlbergBankAg),
            payments_grpc::BankNames::HypoBankBurgenlandAktiengesellschaft => {
                Ok(Self::HypoBankBurgenlandAktiengesellschaft)
            }
            payments_grpc::BankNames::KomercniBanka => Ok(Self::KomercniBanka),
            payments_grpc::BankNames::MBank => Ok(Self::MBank),
            payments_grpc::BankNames::MarchfelderBank => Ok(Self::MarchfelderBank),
            payments_grpc::BankNames::Maybank => Ok(Self::Maybank),
            payments_grpc::BankNames::OberbankAg => Ok(Self::OberbankAg),
            payments_grpc::BankNames::OsterreichischeArzteUndApothekerbank => {
                Ok(Self::OsterreichischeArzteUndApothekerbank)
            }
            payments_grpc::BankNames::OcbcBank => Ok(Self::OcbcBank),
            payments_grpc::BankNames::PayWithIng => Ok(Self::PayWithING),
            payments_grpc::BankNames::PlaceZipko => Ok(Self::PlaceZIPKO),
            payments_grpc::BankNames::PlatnoscOnlineKartaPlatnicza => {
                Ok(Self::PlatnoscOnlineKartaPlatnicza)
            }
            payments_grpc::BankNames::PosojilnicaBankEGen => Ok(Self::PosojilnicaBankEGen),
            payments_grpc::BankNames::PostovaBanka => Ok(Self::PostovaBanka),
            payments_grpc::BankNames::PublicBank => Ok(Self::PublicBank),
            payments_grpc::BankNames::RaiffeisenBankengruppeOsterreich => {
                Ok(Self::RaiffeisenBankengruppeOsterreich)
            }
            payments_grpc::BankNames::RhbBank => Ok(Self::RhbBank),
            payments_grpc::BankNames::SchelhammerCapitalBankAg => {
                Ok(Self::SchelhammerCapitalBankAg)
            }
            payments_grpc::BankNames::StandardCharteredBank => Ok(Self::StandardCharteredBank),
            payments_grpc::BankNames::SchoellerbankAg => Ok(Self::SchoellerbankAg),
            payments_grpc::BankNames::SpardaBankWien => Ok(Self::SpardaBankWien),
            payments_grpc::BankNames::SporoPay => Ok(Self::SporoPay),
            payments_grpc::BankNames::SantanderPrzelew24 => Ok(Self::SantanderPrzelew24),
            payments_grpc::BankNames::TatraPay => Ok(Self::TatraPay),
            payments_grpc::BankNames::Viamo => Ok(Self::Viamo),
            payments_grpc::BankNames::VolksbankGruppe => Ok(Self::VolksbankGruppe),
            payments_grpc::BankNames::VolkskreditbankAg => Ok(Self::VolkskreditbankAg),
            payments_grpc::BankNames::VrBankBraunau => Ok(Self::VrBankBraunau),
            payments_grpc::BankNames::UobBank => Ok(Self::UobBank),
            payments_grpc::BankNames::PayWithAliorBank => Ok(Self::PayWithAliorBank),
            payments_grpc::BankNames::BankiSpoldzielcze => Ok(Self::BankiSpoldzielcze),
            payments_grpc::BankNames::PayWithInteligo => Ok(Self::PayWithInteligo),
            payments_grpc::BankNames::BnpParibasPoland => Ok(Self::BNPParibasPoland),
            payments_grpc::BankNames::BankNowySa => Ok(Self::BankNowySA),
            payments_grpc::BankNames::CreditAgricole => Ok(Self::CreditAgricole),
            payments_grpc::BankNames::PayWithBos => Ok(Self::PayWithBOS),
            payments_grpc::BankNames::PayWithCitiHandlowy => Ok(Self::PayWithCitiHandlowy),
            payments_grpc::BankNames::PayWithPlusBank => Ok(Self::PayWithPlusBank),
            payments_grpc::BankNames::ToyotaBank => Ok(Self::ToyotaBank),
            payments_grpc::BankNames::VeloBank => Ok(Self::VeloBank),
            payments_grpc::BankNames::ETransferPocztowy24 => Ok(Self::ETransferPocztowy24),
            payments_grpc::BankNames::PlusBank => Ok(Self::PlusBank),
            payments_grpc::BankNames::BankiSpbdzielcze => Ok(Self::BankiSpbdzielcze),
            payments_grpc::BankNames::BankNowyBfgSa => Ok(Self::BankNowyBfgSa),
            payments_grpc::BankNames::GetinBank => Ok(Self::GetinBank),
            payments_grpc::BankNames::BlikPoland => Ok(Self::Blik),
            payments_grpc::BankNames::NoblePay => Ok(Self::NoblePay),
            payments_grpc::BankNames::IdeaBank => Ok(Self::IdeaBank),
            payments_grpc::BankNames::EnveloBank => Ok(Self::EnveloBank),
            payments_grpc::BankNames::NestPrzelew => Ok(Self::NestPrzelew),
            payments_grpc::BankNames::MbankMtransfer => Ok(Self::MbankMtransfer),
            payments_grpc::BankNames::Inteligo => Ok(Self::Inteligo),
            payments_grpc::BankNames::PbacZIpko => Ok(Self::PbacZIpko),
            payments_grpc::BankNames::BnpParibas => Ok(Self::BnpParibas),
            payments_grpc::BankNames::VolkswagenBank => Ok(Self::VolkswagenBank),
            payments_grpc::BankNames::AliorBank => Ok(Self::AliorBank),
            payments_grpc::BankNames::Boz => Ok(Self::Boz),
            payments_grpc::BankNames::BangkokBank => Ok(Self::BangkokBank),
            payments_grpc::BankNames::KrungsriBank => Ok(Self::KrungsriBank),
            payments_grpc::BankNames::KrungThaiBank => Ok(Self::KrungThaiBank),
            payments_grpc::BankNames::TheSiamCommercialBank => Ok(Self::TheSiamCommercialBank),
            payments_grpc::BankNames::KasikornBank => Ok(Self::KasikornBank),
            payments_grpc::BankNames::OpenBankSuccess => Ok(Self::OpenBankSuccess),
            payments_grpc::BankNames::OpenBankFailure => Ok(Self::OpenBankFailure),
            payments_grpc::BankNames::OpenBankCancelled => Ok(Self::OpenBankCancelled),
            payments_grpc::BankNames::Aib => Ok(Self::Aib),
            payments_grpc::BankNames::BankOfScotland => Ok(Self::BankOfScotland),
            payments_grpc::BankNames::DanskeBank => Ok(Self::DanskeBank),
            payments_grpc::BankNames::FirstDirect => Ok(Self::FirstDirect),
            payments_grpc::BankNames::FirstTrust => Ok(Self::FirstTrust),
            payments_grpc::BankNames::Halifax => Ok(Self::Halifax),
            payments_grpc::BankNames::Lloyds => Ok(Self::Lloyds),
            payments_grpc::BankNames::Monzo => Ok(Self::Monzo),
            payments_grpc::BankNames::NatWest => Ok(Self::NatWest),
            payments_grpc::BankNames::NationwideBank => Ok(Self::NationwideBank),
            payments_grpc::BankNames::RoyalBankOfScotland => Ok(Self::RoyalBankOfScotland),
            payments_grpc::BankNames::Starling => Ok(Self::Starling),
            payments_grpc::BankNames::TsbBank => Ok(Self::TsbBank),
            payments_grpc::BankNames::TescoBank => Ok(Self::TescoBank),
            payments_grpc::BankNames::UlsterBank => Ok(Self::UlsterBank),
            payments_grpc::BankNames::Yoursafe => Ok(Self::Yoursafe),
            payments_grpc::BankNames::N26 => Ok(Self::N26),
            payments_grpc::BankNames::NationaleNederlanden => Ok(Self::NationaleNederlanden),
            payments_grpc::BankNames::Absa => Ok(Self::Absa),
            payments_grpc::BankNames::PostBank => Ok(Self::PostBank),
            payments_grpc::BankNames::AibBusiness => Ok(Self::AibBusiness),
            payments_grpc::BankNames::Aktia => Ok(Self::Aktia),
            payments_grpc::BankNames::Alandsbanken => Ok(Self::Alandsbanken),
            payments_grpc::BankNames::AllianzBankFinancialAdvisorsSpa => {
                Ok(Self::AllianzBankFinancialAdvisorsSpa)
            }
            payments_grpc::BankNames::AllianzBanque => Ok(Self::AllianzBanque),
            payments_grpc::BankNames::AlliedIrishBank => Ok(Self::AlliedIrishBank),
            payments_grpc::BankNames::AlliedIrishBankCorporate => {
                Ok(Self::AlliedIrishBankCorporate)
            }
            payments_grpc::BankNames::AltoAdige => Ok(Self::AltoAdige),
            payments_grpc::BankNames::AltoAdigeBancaSuedtirolBank => {
                Ok(Self::AltoAdigeBancaSuedtirolBank)
            }
            payments_grpc::BankNames::Argenta => Ok(Self::Argenta),
            payments_grpc::BankNames::ArkeaBanqueEntreprisesEtInstitutionnels => {
                Ok(Self::ArkeaBanqueEntreprisesEtInstitutionnels)
            }
            payments_grpc::BankNames::ArkeaBanquePrivee => Ok(Self::ArkeaBanquePrivee),
            payments_grpc::BankNames::AxaBanque => Ok(Self::AxaBanque),
            payments_grpc::BankNames::Banca360CreditoCooperativoFvg => {
                Ok(Self::Banca360CreditoCooperativoFvg)
            }
            payments_grpc::BankNames::BancaAdriaColliEuganei => Ok(Self::BancaAdriaColliEuganei),
            payments_grpc::BankNames::BancaAgricolaPopolareDiRagusa => {
                Ok(Self::BancaAgricolaPopolareDiRagusa)
            }
            payments_grpc::BankNames::BancaAlpiMarittimeCcCarru => {
                Ok(Self::BancaAlpiMarittimeCcCarru)
            }
            payments_grpc::BankNames::BancaAltaToscana => Ok(Self::BancaAltaToscana),
            payments_grpc::BankNames::BancaAnnia => Ok(Self::BancaAnnia),
            payments_grpc::BankNames::BancaCentroEmilia => Ok(Self::BancaCentroEmilia),
            payments_grpc::BankNames::BancaCentroLazio => Ok(Self::BancaCentroLazio),
            payments_grpc::BankNames::BancaCentroToscanaUmbria => {
                Ok(Self::BancaCentroToscanaUmbria)
            }
            payments_grpc::BankNames::BancaCentropadana => Ok(Self::BancaCentropadana),
            payments_grpc::BankNames::BancaCesarePonti => Ok(Self::BancaCesarePonti),
            payments_grpc::BankNames::BancaDelCatanzarese => Ok(Self::BancaDelCatanzarese),
            payments_grpc::BankNames::BancaDelCilentoDiSassanoEv => {
                Ok(Self::BancaDelCilentoDiSassanoEV)
            }
            payments_grpc::BankNames::BancaDelPiceno => Ok(Self::BancaDelPiceno),
            payments_grpc::BankNames::BancaDelPiemonte => Ok(Self::BancaDelPiemonte),
            payments_grpc::BankNames::BancaDelTerritorioLombardo => {
                Ok(Self::BancaDelTerritorioLombardo)
            }
            payments_grpc::BankNames::BancaDelVenetoCentrale => Ok(Self::BancaDelVenetoCentrale),
            payments_grpc::BankNames::BancaDellaMarcaCredcooperativo => {
                Ok(Self::BancaDellaMarcaCredcooperativo)
            }
            payments_grpc::BankNames::BancaDelleTerreVenete => Ok(Self::BancaDelleTerreVenete),
            payments_grpc::BankNames::BancaDiAlbaCreditoCooperativo => {
                Ok(Self::BancaDiAlbaCreditoCooperativo)
            }
            payments_grpc::BankNames::BancaDiAnghiariEStiaCc => Ok(Self::BancaDiAnghiariEStiaCc),
            payments_grpc::BankNames::BancaDiBologna => Ok(Self::BancaDiBologna),
            payments_grpc::BankNames::BancaDiCaraglio => Ok(Self::BancaDiCaraglio),
            payments_grpc::BankNames::BancaDiCreditoPopolareScpa => {
                Ok(Self::BancaDiCreditoPopolareScpa)
            }
            payments_grpc::BankNames::BancaDiImolaSpa => Ok(Self::BancaDiImolaSpa),
            payments_grpc::BankNames::BancaDiPesaro => Ok(Self::BancaDiPesaro),
            payments_grpc::BankNames::BancaDiPesciaECascina => Ok(Self::BancaDiPesciaECascina),
            payments_grpc::BankNames::BancaDiPiacenzaScpa => Ok(Self::BancaDiPiacenzaScpa),
            payments_grpc::BankNames::BancaDiTarantoBcc => Ok(Self::BancaDiTarantoBcc),
            payments_grpc::BankNames::BancaDiUdineCreditoCoop => Ok(Self::BancaDiUdineCreditoCoop),
            payments_grpc::BankNames::BancaDonRizzo => Ok(Self::BancaDonRizzo),
            payments_grpc::BankNames::BancaFideuram => Ok(Self::BancaFideuram),
            payments_grpc::BankNames::BancaFinnatEuramericaSpa => {
                Ok(Self::BancaFinnatEuramericaSpa)
            }
            payments_grpc::BankNames::BancaGeneraliSpa => Ok(Self::BancaGeneraliSpa),
            payments_grpc::BankNames::BancaLazioNord => Ok(Self::BancaLazioNord),
            payments_grpc::BankNames::BancaMalatestiana => Ok(Self::BancaMalatestiana),
            payments_grpc::BankNames::BancaMonteDeiPaschiDiSiena => {
                Ok(Self::BancaMonteDeiPaschiDiSiena)
            }
            payments_grpc::BankNames::BancaPassadore => Ok(Self::BancaPassadore),
            payments_grpc::BankNames::BancaPatavina => Ok(Self::BancaPatavina),
            payments_grpc::BankNames::BancaPatrimoniSella => Ok(Self::BancaPatrimoniSella),
            payments_grpc::BankNames::BancaPerIlTrentinoaltoadige => {
                Ok(Self::BancaPerIlTrentinoaltoadige)
            }
            payments_grpc::BankNames::BancaPopolareDelLazioScpa => {
                Ok(Self::BancaPopolareDelLazioScpa)
            }
            payments_grpc::BankNames::BancaPopolareDellAltoAdige => {
                Ok(Self::BancaPopolareDellAltoAdige)
            }
            payments_grpc::BankNames::BancaPopolareDiSondrio => Ok(Self::BancaPopolareDiSondrio),
            payments_grpc::BankNames::BancaPopolarePugliese => Ok(Self::BancaPopolarePugliese),
            payments_grpc::BankNames::BancaPopolareValconcaScpa => {
                Ok(Self::BancaPopolareValconcaScpa)
            }
            payments_grpc::BankNames::BancaSanFrancescoCreditoCoop => {
                Ok(Self::BancaSanFrancescoCreditoCoop)
            }
            payments_grpc::BankNames::BancaSella => Ok(Self::BancaSella),
            payments_grpc::BankNames::BancaSistemaSpa => Ok(Self::BancaSistemaSpa),
            payments_grpc::BankNames::BancaSviluppoCooperazCredito => {
                Ok(Self::BancaSviluppoCooperazCredito)
            }
            payments_grpc::BankNames::BancaTema => Ok(Self::BancaTema),
            payments_grpc::BankNames::BancaTerreEtruscheEDiMaremma => {
                Ok(Self::BancaTerreEtruscheEDiMaremma)
            }
            payments_grpc::BankNames::BancaTerritoriDelMonviso => {
                Ok(Self::BancaTerritoriDelMonviso)
            }
            payments_grpc::BankNames::BancaValsabbina => Ok(Self::BancaValsabbina),
            payments_grpc::BankNames::BancaVeroneseCcDiConcamarise => {
                Ok(Self::BancaVeroneseCcDiConcamarise)
            }
            payments_grpc::BankNames::BancoAzzoaglio => Ok(Self::BancoAzzoaglio),
            payments_grpc::BankNames::BancoBpmSpaServizioWebank => {
                Ok(Self::BancoBpmSpaServizioWebank)
            }
            payments_grpc::BankNames::BancoBpmSpaServizioYouweb => {
                Ok(Self::BancoBpmSpaServizioYouweb)
            }
            payments_grpc::BankNames::BancoBpmSpaYoubusinessWeb => {
                Ok(Self::BancoBpmSpaYoubusinessWeb)
            }
            payments_grpc::BankNames::BancoBpmWeBank => Ok(Self::BancoBpmWeBank),
            payments_grpc::BankNames::BancoBpmYouWeb => Ok(Self::BancoBpmYouWeb),
            payments_grpc::BankNames::BancoDeSabadell => Ok(Self::BancoDeSabadell),
            payments_grpc::BankNames::BancoDesioBrianza => Ok(Self::BancoDesioBrianza),
            payments_grpc::BankNames::BancoDiSardegna => Ok(Self::BancoDiSardegna),
            payments_grpc::BankNames::BancoMarchigiano => Ok(Self::BancoMarchigiano),
            payments_grpc::BankNames::BancoPosta => Ok(Self::BancoPosta),
            payments_grpc::BankNames::BancoSantander => Ok(Self::BancoSantander),
            payments_grpc::BankNames::BankOfIreland => Ok(Self::BankOfIreland),
            payments_grpc::BankNames::BankOfIrelandBusiness => Ok(Self::BankOfIrelandBusiness),
            payments_grpc::BankNames::BankOfIrelandUk => Ok(Self::BankOfIrelandUk),
            payments_grpc::BankNames::BankOfScotlandBusiness => Ok(Self::BankOfScotlandBusiness),
            payments_grpc::BankNames::Bankinter => Ok(Self::Bankinter),
            payments_grpc::BankNames::BanqueDeSavoie => Ok(Self::BanqueDeSavoie),
            payments_grpc::BankNames::BanquePopulaire => Ok(Self::BanquePopulaire),
            payments_grpc::BankNames::Barclaycard => Ok(Self::Barclaycard),
            payments_grpc::BankNames::BarclaysBusiness => Ok(Self::BarclaysBusiness),
            payments_grpc::BankNames::BawagPsk => Ok(Self::BawagPsk),
            payments_grpc::BankNames::Bbva => Ok(Self::Bbva),
            payments_grpc::BankNames::BccAbruzzeseCappelleSulTavo => {
                Ok(Self::BccAbruzzeseCappelleSulTavo)
            }
            payments_grpc::BankNames::BccAbruzziEMolise => Ok(Self::BccAbruzziEMolise),
            payments_grpc::BankNames::BccAdriaticoTeramano => Ok(Self::BccAdriaticoTeramano),
            payments_grpc::BankNames::BccAgroBresciano => Ok(Self::BccAgroBresciano),
            payments_grpc::BankNames::BccAgroPontino => Ok(Self::BccAgroPontino),
            payments_grpc::BankNames::BccAlberobelloSammicheleMonopoli => {
                Ok(Self::BccAlberobelloSammicheleMonopoli)
            }
            payments_grpc::BankNames::BccAltoTirrenoDellaCalabria => {
                Ok(Self::BccAltoTirrenoDellaCalabria)
            }
            payments_grpc::BankNames::BccAnagni => Ok(Self::BccAnagni),
            payments_grpc::BankNames::BccBasilicata => Ok(Self::BccBasilicata),
            payments_grpc::BankNames::BccBellegra => Ok(Self::BccBellegra),
            payments_grpc::BankNames::BccBrescia => Ok(Self::BccBrescia),
            payments_grpc::BankNames::BccBrianzaELaghi => Ok(Self::BccBrianzaELaghi),
            payments_grpc::BankNames::BccCampaniaCentro => Ok(Self::BccCampaniaCentro),
            payments_grpc::BankNames::BccCapaccioPaestum => Ok(Self::BccCapaccioPaestum),
            payments_grpc::BankNames::BccCastelliRomaniETuscolo => {
                Ok(Self::BccCastelliRomaniETuscolo)
            }
            payments_grpc::BankNames::BccCentroCalabria => Ok(Self::BccCentroCalabria),
            payments_grpc::BankNames::BccConversano => Ok(Self::BccConversano),
            payments_grpc::BankNames::BccDegliUliviTerraDiBari => {
                Ok(Self::BccDegliUliviTerraDiBari)
            }
            payments_grpc::BankNames::BccDeiCastelliEDegliIblei => {
                Ok(Self::BccDeiCastelliEDegliIblei)
            }
            payments_grpc::BankNames::BccDeiColliAlbani => Ok(Self::BccDeiColliAlbani),
            payments_grpc::BankNames::BccDelCirceoEPrivernate => Ok(Self::BccDelCirceoEPrivernate),
            payments_grpc::BankNames::BccDelGarda => Ok(Self::BccDelGarda),
            payments_grpc::BankNames::BccDelMetauro => Ok(Self::BccDelMetauro),
            payments_grpc::BankNames::BccDelVelino => Ok(Self::BccDelVelino),
            payments_grpc::BankNames::BccDellAltaMurgia => Ok(Self::BccDellAltaMurgia),
            payments_grpc::BankNames::BccDellaProvinciaRomana => Ok(Self::BccDellaProvinciaRomana),
            payments_grpc::BankNames::BccDellaRomagnaOccidentale => {
                Ok(Self::BccDellaRomagnaOccidentale)
            }
            payments_grpc::BankNames::BccDelleMadonie => Ok(Self::BccDelleMadonie),
            payments_grpc::BankNames::BccDiAltofonteECaccamo => Ok(Self::BccDiAltofonteECaccamo),
            payments_grpc::BankNames::BccDiAquara => Ok(Self::BccDiAquara),
            payments_grpc::BankNames::BccDiArborea => Ok(Self::BccDiArborea),
            payments_grpc::BankNames::BccDiBari => Ok(Self::BccDiBari),
            payments_grpc::BankNames::BccDiBarlassina => Ok(Self::BccDiBarlassina),
            payments_grpc::BankNames::BccDiBeneVagienna => Ok(Self::BccDiBeneVagienna),
            payments_grpc::BankNames::BccDiBinasco => Ok(Self::BccDiBinasco),
            payments_grpc::BankNames::BccDiBuccinoEComuniCilentani => {
                Ok(Self::BccDiBuccinoEComuniCilentani)
            }
            payments_grpc::BankNames::BccDiBustoGarolfoEBuguggiate => {
                Ok(Self::BccDiBustoGarolfoEBuguggiate)
            }
            payments_grpc::BankNames::BccDiCagliari => Ok(Self::BccDiCagliari),
            payments_grpc::BankNames::BccDiCanosaLoconia => Ok(Self::BccDiCanosaLoconia),
            payments_grpc::BankNames::BccDiCaravaggio => Ok(Self::BccDiCaravaggio),
            payments_grpc::BankNames::BccDiCassanoDelleMurgeETolve => {
                Ok(Self::BccDiCassanoDelleMurgeETolve)
            }
            payments_grpc::BankNames::BccDiCherasco => Ok(Self::BccDiCherasco),
            payments_grpc::BankNames::BccDiFilottrano => Ok(Self::BccDiFilottrano),
            payments_grpc::BankNames::BccDiFlumeri => Ok(Self::BccDiFlumeri),
            payments_grpc::BankNames::BccDiGambatesa => Ok(Self::BccDiGambatesa),
            payments_grpc::BankNames::BccDiGaudianoDiLavello => Ok(Self::BccDiGaudianoDiLavello),
            payments_grpc::BankNames::BccDiLeverano => Ok(Self::BccDiLeverano),
            payments_grpc::BankNames::BccDiLocorotondo => Ok(Self::BccDiLocorotondo),
            payments_grpc::BankNames::BccDiMontepaone => Ok(Self::BccDiMontepaone),
            payments_grpc::BankNames::BccDiNapoli => Ok(Self::BccDiNapoli),
            payments_grpc::BankNames::BccDiOstraEMorroDAlba => Ok(Self::BccDiOstraEMorroDAlba),
            payments_grpc::BankNames::BccDiOstuni => Ok(Self::BccDiOstuni),
            payments_grpc::BankNames::BccDiPachino => Ok(Self::BccDiPachino),
            payments_grpc::BankNames::BccDiPergolaECorinaldo => Ok(Self::BccDiPergolaECorinaldo),
            payments_grpc::BankNames::BccDiPianfeiERoccaDeBaldi => {
                Ok(Self::BccDiPianfeiERoccaDeBaldi)
            }
            payments_grpc::BankNames::BccDiPontassieve => Ok(Self::BccDiPontassieve),
            payments_grpc::BankNames::BccDiRecanatiEColmurano => Ok(Self::BccDiRecanatiEColmurano),
            payments_grpc::BankNames::BccDiRoma => Ok(Self::BccDiRoma),
            payments_grpc::BankNames::BccDiSanGiovanniRotondo => Ok(Self::BccDiSanGiovanniRotondo),
            payments_grpc::BankNames::BccDiSanMarzanoDiSanGiuseppe => {
                Ok(Self::BccDiSanMarzanoDiSanGiuseppe)
            }
            payments_grpc::BankNames::BccDiSanteramoInColle => Ok(Self::BccDiSanteramoInColle),
            payments_grpc::BankNames::BccDiSarsina => Ok(Self::BccDiSarsina),
            payments_grpc::BankNames::BccDiScafatiECetara => Ok(Self::BccDiScafatiECetara),
            payments_grpc::BankNames::BccDiSmarcoDeiCavoti => Ok(Self::BccDiSmarcoDeiCavoti),
            payments_grpc::BankNames::BccDiSpelloEDelVelino => Ok(Self::BccDiSpelloEDelVelino),
            payments_grpc::BankNames::BccDiTerraDOtranto => Ok(Self::BccDiTerraDOtranto),
            payments_grpc::BankNames::BccFelsinea => Ok(Self::BccFelsinea),
            payments_grpc::BankNames::BccGTonioloDiSanCataldo => Ok(Self::BccGTonioloDiSanCataldo),
            payments_grpc::BankNames::BccGranSassoDItalia => Ok(Self::BccGranSassoDItalia),
            payments_grpc::BankNames::BccLaRiscossaDiRegalbuto => {
                Ok(Self::BccLaRiscossaDiRegalbuto)
            }
            payments_grpc::BankNames::BccLodi => Ok(Self::BccLodi),
            payments_grpc::BankNames::BccMilano => Ok(Self::BccMilano),
            payments_grpc::BankNames::BccMontePruno => Ok(Self::BccMontePruno),
            payments_grpc::BankNames::BccNettuno => Ok(Self::BccNettuno),
            payments_grpc::BankNames::BccOglioESerio => Ok(Self::BccOglioESerio),
            payments_grpc::BankNames::BccPordenoneseEMonsile => Ok(Self::BccPordenoneseEMonsile),
            payments_grpc::BankNames::BccPratolaPeligna => Ok(Self::BccPratolaPeligna),
            payments_grpc::BankNames::BccPrealpiSanBiagio => Ok(Self::BccPrealpiSanBiagio),
            payments_grpc::BankNames::BccRavennaForliImola => Ok(Self::BccRavennaForliImola),
            payments_grpc::BankNames::BccSanGiuseppeDiMussomeli => {
                Ok(Self::BccSanGiuseppeDiMussomeli)
            }
            payments_grpc::BankNames::BccTerraDiLavoro => Ok(Self::BccTerraDiLavoro),
            payments_grpc::BankNames::BccTriuggioValleDelLambro => {
                Ok(Self::BccTriuggioValleDelLambro)
            }
            payments_grpc::BankNames::BccValdarnoFiorentino => Ok(Self::BccValdarnoFiorentino),
            payments_grpc::BankNames::BccValdostana => Ok(Self::BccValdostana),
            payments_grpc::BankNames::BccValleDelTorto => Ok(Self::BccValleDelTorto),
            payments_grpc::BankNames::BccVeneta => Ok(Self::BccVeneta),
            payments_grpc::BankNames::BccVeneziaGiulia => Ok(Self::BccVeneziaGiulia),
            payments_grpc::BankNames::BccVersiliaLunigianaEGarfagnana => {
                Ok(Self::BccVersiliaLunigianaEGarfagnana)
            }
            payments_grpc::BankNames::BccVicentinoPojanaMaggiore => {
                Ok(Self::BccVicentinoPojanaMaggiore)
            }
            payments_grpc::BankNames::Belfius => Ok(Self::Belfius),
            payments_grpc::BankNames::Beobank => Ok(Self::Beobank),
            payments_grpc::BankNames::BiBanca => Ok(Self::BiBanca),
            payments_grpc::BankNames::BluBancaSpa => Ok(Self::BluBancaSpa),
            payments_grpc::BankNames::Bnl => Ok(Self::Bnl),
            payments_grpc::BankNames::BnpParibasFortis => Ok(Self::BnpParibasFortis),
            payments_grpc::BankNames::BoursoBank => Ok(Self::BoursoBank),
            payments_grpc::BankNames::Bozen => Ok(Self::Bozen),
            payments_grpc::BankNames::Bpe => Ok(Self::Bpe),
            payments_grpc::BankNames::BperBanca => Ok(Self::BperBanca),
            payments_grpc::BankNames::BvrBancaBancheVeneteRiunite => {
                Ok(Self::BvrBancaBancheVeneteRiunite)
            }
            payments_grpc::BankNames::CaisseDEpargne => Ok(Self::CaisseDEpargne),
            payments_grpc::BankNames::Caixa => Ok(Self::Caixa),
            payments_grpc::BankNames::CajaRural => Ok(Self::CajaRural),
            payments_grpc::BankNames::Cajamar => Ok(Self::Cajamar),
            payments_grpc::BankNames::CassaCentraleBanca => Ok(Self::CassaCentraleBanca),
            payments_grpc::BankNames::CassaDiRisparmioDiBolzano => {
                Ok(Self::CassaDiRisparmioDiBolzano)
            }
            payments_grpc::BankNames::CassaDiRisparmioDiFermoSpa => {
                Ok(Self::CassaDiRisparmioDiFermoSpa)
            }
            payments_grpc::BankNames::CassaDiRisparmioDiSavigliano => {
                Ok(Self::CassaDiRisparmioDiSavigliano)
            }
            payments_grpc::BankNames::CassaPadana => Ok(Self::CassaPadana),
            payments_grpc::BankNames::CassaRuraleAltaValsugana => {
                Ok(Self::CassaRuraleAltaValsugana)
            }
            payments_grpc::BankNames::CassaRuraleAltoGardaRovereto => {
                Ok(Self::CassaRuraleAltoGardaRovereto)
            }
            payments_grpc::BankNames::CassaRuraleDiLedro => Ok(Self::CassaRuraleDiLedro),
            payments_grpc::BankNames::CassaRuraleDiTreviglio => Ok(Self::CassaRuraleDiTreviglio),
            payments_grpc::BankNames::CassaRuraleFvg => Ok(Self::CassaRuraleFvg),
            payments_grpc::BankNames::CassaRuraleRenon => Ok(Self::CassaRuraleRenon),
            payments_grpc::BankNames::CassaRuraleValDiFiemme => Ok(Self::CassaRuraleValDiFiemme),
            payments_grpc::BankNames::CassaRuraleValDiSole => Ok(Self::CassaRuraleValDiSole),
            payments_grpc::BankNames::CassaRuraleVallagarina => Ok(Self::CassaRuraleVallagarina),
            payments_grpc::BankNames::CassaRuraleValsuganaETesino => {
                Ok(Self::CassaRuraleValsuganaETesino)
            }
            payments_grpc::BankNames::CastagnetoBanca1910 => Ok(Self::CastagnetoBanca1910),
            payments_grpc::BankNames::CbcBanque => Ok(Self::CbcBanque),
            payments_grpc::BankNames::CentromarcaBanca => Ok(Self::CentromarcaBanca),
            payments_grpc::BankNames::ChiantibancaCreditoCooperativo => {
                Ok(Self::ChiantibancaCreditoCooperativo)
            }
            payments_grpc::BankNames::Cic => Ok(Self::Cic),
            payments_grpc::BankNames::ClydesdaleBank => Ok(Self::ClydesdaleBank),
            payments_grpc::BankNames::Comdirect => Ok(Self::Comdirect),
            payments_grpc::BankNames::Commerzbank => Ok(Self::Commerzbank),
            payments_grpc::BankNames::Cortinabanca => Ok(Self::Cortinabanca),
            payments_grpc::BankNames::Coutts => Ok(Self::Coutts),
            payments_grpc::BankNames::CrValDiNonRotalianaEGiovo => {
                Ok(Self::CrValDiNonRotalianaEGiovo)
            }
            payments_grpc::BankNames::CraBccDiCantu => Ok(Self::CraBccDiCantu),
            payments_grpc::BankNames::CraDiBorgoSanGiacomo => Ok(Self::CraDiBorgoSanGiacomo),
            payments_grpc::BankNames::CraDiBoves => Ok(Self::CraDiBoves),
            payments_grpc::BankNames::CraDiPaliano => Ok(Self::CraDiPaliano),
            payments_grpc::BankNames::Credem => Ok(Self::Credem),
            payments_grpc::BankNames::Credifriuli => Ok(Self::Credifriuli),
            payments_grpc::BankNames::CreditMutuel => Ok(Self::CreditMutuel),
            payments_grpc::BankNames::CreditMutuelDeBretagne => Ok(Self::CreditMutuelDeBretagne),
            payments_grpc::BankNames::CreditMutuelDuSudOuest => Ok(Self::CreditMutuelDuSudOuest),
            payments_grpc::BankNames::CreditoCooperativoAgrigentino => {
                Ok(Self::CreditoCooperativoAgrigentino)
            }
            payments_grpc::BankNames::CreditoCooperativoMediocrati => {
                Ok(Self::CreditoCooperativoMediocrati)
            }
            payments_grpc::BankNames::CreditoCooperativoRomagnolo => {
                Ok(Self::CreditoCooperativoRomagnolo)
            }
            payments_grpc::BankNames::CreditoDiRomagna => Ok(Self::CreditoDiRomagna),
            payments_grpc::BankNames::CreditoLombardoVeneto => Ok(Self::CreditoLombardoVeneto),
            payments_grpc::BankNames::DanskeBankBusiness => Ok(Self::DanskeBankBusiness),
            payments_grpc::BankNames::Desio => Ok(Self::Desio),
            payments_grpc::BankNames::DeutscheBank => Ok(Self::DeutscheBank),
            payments_grpc::BankNames::Dkb => Ok(Self::Dkb),
            payments_grpc::BankNames::EasyBank => Ok(Self::EasyBank),
            payments_grpc::BankNames::Ebs => Ok(Self::Ebs),
            payments_grpc::BankNames::EmilbancaCc => Ok(Self::EmilbancaCc),
            payments_grpc::BankNames::ErsteBank => Ok(Self::ErsteBank),
            payments_grpc::BankNames::EvoBanco => Ok(Self::EvoBanco),
            payments_grpc::BankNames::Fineco => Ok(Self::Fineco),
            payments_grpc::BankNames::Fintro => Ok(Self::Fintro),
            payments_grpc::BankNames::Fortuneo => Ok(Self::Fortuneo),
            payments_grpc::BankNames::FpbCassaDiFassaPrimieroBelluno => {
                Ok(Self::FpbCassaDiFassaPrimieroBelluno)
            }
            payments_grpc::BankNames::HelloBank => Ok(Self::HelloBank),
            payments_grpc::BankNames::Hsbc => Ok(Self::Hsbc),
            payments_grpc::BankNames::HsbcBusiness => Ok(Self::HsbcBusiness),
            payments_grpc::BankNames::Hype => Ok(Self::Hype),
            payments_grpc::BankNames::HypoVereinsbank => Ok(Self::HypoVereinsbank),
            payments_grpc::BankNames::Ibercaja => Ok(Self::Ibercaja),
            payments_grpc::BankNames::IccreaBancaSpa => Ok(Self::IccreaBancaSpa),
            payments_grpc::BankNames::Illimity => Ok(Self::Illimity),
            payments_grpc::BankNames::Imagin => Ok(Self::Imagin),
            payments_grpc::BankNames::ImprebancaSpa => Ok(Self::ImprebancaSpa),
            payments_grpc::BankNames::IntesaSanpaolo => Ok(Self::IntesaSanpaolo),
            payments_grpc::BankNames::IntesaSanpaoloInbiz => Ok(Self::IntesaSanpaoloInbiz),
            payments_grpc::BankNames::IntesaSanpaoloPrivateBankingSpa => {
                Ok(Self::IntesaSanpaoloPrivateBankingSpa)
            }
            payments_grpc::BankNames::Isybank => Ok(Self::Isybank),
            payments_grpc::BankNames::Kbc => Ok(Self::Kbc),
            payments_grpc::BankNames::KbcBrussels => Ok(Self::KbcBrussels),
            payments_grpc::BankNames::Kutxabank => Ok(Self::Kutxabank),
            payments_grpc::BankNames::LaBanquePostale => Ok(Self::LaBanquePostale),
            payments_grpc::BankNames::LaBanquePostaleBusiness => Ok(Self::LaBanquePostaleBusiness),
            payments_grpc::BankNames::LaCassaDiRavennaSpa => Ok(Self::LaCassaDiRavennaSpa),
            payments_grpc::BankNames::LaCassaRurale => Ok(Self::LaCassaRurale),
            payments_grpc::BankNames::LaboralKutxa => Ok(Self::LaboralKutxa),
            payments_grpc::BankNames::Lcl => Ok(Self::Lcl),
            payments_grpc::BankNames::LisPaySpa => Ok(Self::LisPaySpa),
            payments_grpc::BankNames::LloydsBusiness => Ok(Self::LloydsBusiness),
            payments_grpc::BankNames::LloydsCommercial => Ok(Self::LloydsCommercial),
            payments_grpc::BankNames::MsBank => Ok(Self::MSBank),
            payments_grpc::BankNames::Mbna => Ok(Self::Mbna),
            payments_grpc::BankNames::MettleBank => Ok(Self::MettleBank),
            payments_grpc::BankNames::Monabanq => Ok(Self::Monabanq),
            payments_grpc::BankNames::Mooney => Ok(Self::Mooney),
            payments_grpc::BankNames::Mps => Ok(Self::Mps),
            payments_grpc::BankNames::NatWestBankline => Ok(Self::NatWestBankline),
            payments_grpc::BankNames::Nationwide => Ok(Self::Nationwide),
            payments_grpc::BankNames::Nordea => Ok(Self::Nordea),
            payments_grpc::BankNames::OmaSp => Ok(Self::OmaSp),
            payments_grpc::BankNames::Op => Ok(Self::Op),
            payments_grpc::BankNames::Openbank => Ok(Self::Openbank),
            payments_grpc::BankNames::PopPankki => Ok(Self::PopPankki),
            payments_grpc::BankNames::PostePayEvolution => Ok(Self::PostePayEvolution),
            payments_grpc::BankNames::PrimacassaFvg => Ok(Self::PrimacassaFvg),
            payments_grpc::BankNames::Ptsb => Ok(Self::Ptsb),
            payments_grpc::BankNames::RaiffeisenAlgund => Ok(Self::RaiffeisenAlgund),
            payments_grpc::BankNames::RaiffeisenAltaPusteria => Ok(Self::RaiffeisenAltaPusteria),
            payments_grpc::BankNames::RaiffeisenAltaVenosta => Ok(Self::RaiffeisenAltaVenosta),
            payments_grpc::BankNames::RaiffeisenAltoAdige => Ok(Self::RaiffeisenAltoAdige),
            payments_grpc::BankNames::RaiffeisenBassaAtesina => Ok(Self::RaiffeisenBassaAtesina),
            payments_grpc::BankNames::RaiffeisenBassaValleIsarco => {
                Ok(Self::RaiffeisenBassaValleIsarco)
            }
            payments_grpc::BankNames::RaiffeisenBassaVenosta => Ok(Self::RaiffeisenBassaVenosta),
            payments_grpc::BankNames::RaiffeisenBolzano => Ok(Self::RaiffeisenBolzano),
            payments_grpc::BankNames::RaiffeisenBozen => Ok(Self::RaiffeisenBozen),
            payments_grpc::BankNames::RaiffeisenBruneck => Ok(Self::RaiffeisenBruneck),
            payments_grpc::BankNames::RaiffeisenBrunico => Ok(Self::RaiffeisenBrunico),
            payments_grpc::BankNames::RaiffeisenCampoDiTrens => Ok(Self::RaiffeisenCampoDiTrens),
            payments_grpc::BankNames::RaiffeisenCassaCentrAltoAdige => {
                Ok(Self::RaiffeisenCassaCentrAltoAdige)
            }
            payments_grpc::BankNames::RaiffeisenCastelrottoortisei => {
                Ok(Self::RaiffeisenCastelrottoortisei)
            }
            payments_grpc::BankNames::RaiffeisenDeutschnofenaldein => {
                Ok(Self::RaiffeisenDeutschnofenaldein)
            }
            payments_grpc::BankNames::RaiffeisenDobbiaco => Ok(Self::RaiffeisenDobbiaco),
            payments_grpc::BankNames::RaiffeisenEisacktal => Ok(Self::RaiffeisenEisacktal),
            payments_grpc::BankNames::RaiffeisenEtschtal => Ok(Self::RaiffeisenEtschtal),
            payments_grpc::BankNames::RaiffeisenFreienfeld => Ok(Self::RaiffeisenFreienfeld),
            payments_grpc::BankNames::RaiffeisenFunes => Ok(Self::RaiffeisenFunes),
            payments_grpc::BankNames::RaiffeisenGadertal => Ok(Self::RaiffeisenGadertal),
            payments_grpc::BankNames::RaiffeisenGroeden => Ok(Self::RaiffeisenGroeden),
            payments_grpc::BankNames::RaiffeisenHochpustertal => Ok(Self::RaiffeisenHochpustertal),
            payments_grpc::BankNames::RaiffeisenKastelruthstulrich => {
                Ok(Self::RaiffeisenKastelruthstulrich)
            }
            payments_grpc::BankNames::RaiffeisenLaas => Ok(Self::RaiffeisenLaas),
            payments_grpc::BankNames::RaiffeisenLaces => Ok(Self::RaiffeisenLaces),
            payments_grpc::BankNames::RaiffeisenLagundo => Ok(Self::RaiffeisenLagundo),
            payments_grpc::BankNames::RaiffeisenLana => Ok(Self::RaiffeisenLana),
            payments_grpc::BankNames::RaiffeisenLandesbankSuedtirol => {
                Ok(Self::RaiffeisenLandesbankSuedtirol)
            }
            payments_grpc::BankNames::RaiffeisenLasa => Ok(Self::RaiffeisenLasa),
            payments_grpc::BankNames::RaiffeisenLatsch => Ok(Self::RaiffeisenLatsch),
            payments_grpc::BankNames::RaiffeisenMarlengo => Ok(Self::RaiffeisenMarlengo),
            payments_grpc::BankNames::RaiffeisenMarling => Ok(Self::RaiffeisenMarling),
            payments_grpc::BankNames::RaiffeisenMeran => Ok(Self::RaiffeisenMeran),
            payments_grpc::BankNames::RaiffeisenMerano => Ok(Self::RaiffeisenMerano),
            payments_grpc::BankNames::RaiffeisenMonguelfocasiestesido => {
                Ok(Self::RaiffeisenMonguelfocasiestesido)
            }
            payments_grpc::BankNames::RaiffeisenNiederdorf => Ok(Self::RaiffeisenNiederdorf),
            payments_grpc::BankNames::Raiffeisenbank => Ok(Self::Raiffeisenbank),
            payments_grpc::BankNames::RoyalBankOfScotlandBankline => {
                Ok(Self::RoyalBankOfScotlandBankline)
            }
            payments_grpc::BankNames::SPankki => Ok(Self::SPankki),
            payments_grpc::BankNames::Saastopankki => Ok(Self::Saastopankki),
            payments_grpc::BankNames::Santander => Ok(Self::Santander),
            payments_grpc::BankNames::SantanderBusiness => Ok(Self::SantanderBusiness),
            payments_grpc::BankNames::SantanderPersonal => Ok(Self::SantanderPersonal),
            payments_grpc::BankNames::Sparkasse => Ok(Self::Sparkasse),
            payments_grpc::BankNames::TargoBank => Ok(Self::TargoBank),
            payments_grpc::BankNames::Tide => Ok(Self::Tide),
            payments_grpc::BankNames::Triodos => Ok(Self::Triodos),
            payments_grpc::BankNames::Tsb => Ok(Self::Tsb),
            payments_grpc::BankNames::UlsterBankline => Ok(Self::UlsterBankline),
            payments_grpc::BankNames::Unicaja => Ok(Self::Unicaja),
            payments_grpc::BankNames::VirginMoney => Ok(Self::VirginMoney),
            payments_grpc::BankNames::VirginMoneyMerged => Ok(Self::VirginMoneyMerged),
            payments_grpc::BankNames::VolksbankenRaiffeisenbanken => {
                Ok(Self::VolksbankenRaiffeisenbanken)
            }
            payments_grpc::BankNames::Wise => Ok(Self::Wise),
            payments_grpc::BankNames::YorkshireBank => Ok(Self::YorkshireBank),
            payments_grpc::BankNames::Zempler => Ok(Self::Zempler),
            payments_grpc::BankNames::RaiffeisenNovaLevante => Ok(Self::RaiffeisenNovaLevante),
            payments_grpc::BankNames::RaiffeisenNovaPonentealdino => {
                Ok(Self::RaiffeisenNovaPonentealdino)
            }
            payments_grpc::BankNames::RaiffeisenObervinschgau => Ok(Self::RaiffeisenObervinschgau),
            payments_grpc::BankNames::RaiffeisenOltradige => Ok(Self::RaiffeisenOltradige),
            payments_grpc::BankNames::RaiffeisenParcines => Ok(Self::RaiffeisenParcines),
            payments_grpc::BankNames::RaiffeisenPartschins => Ok(Self::RaiffeisenPartschins),
            payments_grpc::BankNames::RaiffeisenPasseier => Ok(Self::RaiffeisenPasseier),
            payments_grpc::BankNames::RaiffeisenPradtaufers => Ok(Self::RaiffeisenPradtaufers),
            payments_grpc::BankNames::RaiffeisenPratotubre => Ok(Self::RaiffeisenPratotubre),
            payments_grpc::BankNames::RaiffeisenSalorno => Ok(Self::RaiffeisenSalorno),
            payments_grpc::BankNames::RaiffeisenSalurn => Ok(Self::RaiffeisenSalurn),
            payments_grpc::BankNames::RaiffeisenSanMartinoInPassiria => {
                Ok(Self::RaiffeisenSanMartinoInPassiria)
            }
            payments_grpc::BankNames::RaiffeisenSarntal => Ok(Self::RaiffeisenSarntal),
            payments_grpc::BankNames::RaiffeisenScena => Ok(Self::RaiffeisenScena),
            payments_grpc::BankNames::RaiffeisenSchenna => Ok(Self::RaiffeisenSchenna),
            payments_grpc::BankNames::RaiffeisenSchlanders => Ok(Self::RaiffeisenSchlanders),
            payments_grpc::BankNames::RaiffeisenSchlernrosengarten => {
                Ok(Self::RaiffeisenSchlernrosengarten)
            }
            payments_grpc::BankNames::RaiffeisenSilandro => Ok(Self::RaiffeisenSilandro),
            payments_grpc::BankNames::RaiffeisenSuedtirol => Ok(Self::RaiffeisenSuedtirol),
            payments_grpc::BankNames::RaiffeisenTaufererahrntal => {
                Ok(Self::RaiffeisenTaufererahrntal)
            }
            payments_grpc::BankNames::RaiffeisenTesimo => Ok(Self::RaiffeisenTesimo),
            payments_grpc::BankNames::RaiffeisenTirol => Ok(Self::RaiffeisenTirol),
            payments_grpc::BankNames::RaiffeisenTirolo => Ok(Self::RaiffeisenTirolo),
            payments_grpc::BankNames::RaiffeisenTisens => Ok(Self::RaiffeisenTisens),
            payments_grpc::BankNames::RaiffeisenToblach => Ok(Self::RaiffeisenToblach),
            payments_grpc::BankNames::RaiffeisenTuresaurina => Ok(Self::RaiffeisenTuresaurina),
            payments_grpc::BankNames::RaiffeisenUeberetsch => Ok(Self::RaiffeisenUeberetsch),
            payments_grpc::BankNames::RaiffeisenUltenstpankrazlaurein => {
                Ok(Self::RaiffeisenUltenstpankrazlaurein)
            }
            payments_grpc::BankNames::RaiffeisenUltimospancrlaur => {
                Ok(Self::RaiffeisenUltimospancrlaur)
            }
            payments_grpc::BankNames::RaiffeisenUntereisacktal => {
                Ok(Self::RaiffeisenUntereisacktal)
            }
            payments_grpc::BankNames::RaiffeisenUnterland => Ok(Self::RaiffeisenUnterland),
            payments_grpc::BankNames::RaiffeisenUntervinschgau => {
                Ok(Self::RaiffeisenUntervinschgau)
            }
            payments_grpc::BankNames::RaiffeisenValBadia => Ok(Self::RaiffeisenValBadia),
            payments_grpc::BankNames::RaiffeisenValGardena => Ok(Self::RaiffeisenValGardena),
            payments_grpc::BankNames::RaiffeisenValPassiria => Ok(Self::RaiffeisenValPassiria),
            payments_grpc::BankNames::RaiffeisenValSarentino => Ok(Self::RaiffeisenValSarentino),
            payments_grpc::BankNames::RaiffeisenValleIsarco => Ok(Self::RaiffeisenValleIsarco),
            payments_grpc::BankNames::RaiffeisenVandoies => Ok(Self::RaiffeisenVandoies),
            payments_grpc::BankNames::RaiffeisenVillabassa => Ok(Self::RaiffeisenVillabassa),
            payments_grpc::BankNames::RaiffeisenVillnoess => Ok(Self::RaiffeisenVillnoess),
            payments_grpc::BankNames::RaiffeisenVintl => Ok(Self::RaiffeisenVintl),
            payments_grpc::BankNames::RaiffeisenWelsberggsiestaisten => {
                Ok(Self::RaiffeisenWelsberggsiestaisten)
            }
            payments_grpc::BankNames::RaiffeisenWelschnofen => Ok(Self::RaiffeisenWelschnofen),
            payments_grpc::BankNames::RaiffeisenWipptal => Ok(Self::RaiffeisenWipptal),
            payments_grpc::BankNames::RaiffeisenkasseRitten => Ok(Self::RaiffeisenkasseRitten),
            payments_grpc::BankNames::RivieraBanca => Ok(Self::RivieraBanca),
            payments_grpc::BankNames::RomagnaBanca => Ok(Self::RomagnaBanca),
            payments_grpc::BankNames::Sella => Ok(Self::Sella),
            payments_grpc::BankNames::Sicilbanca => Ok(Self::Sicilbanca),
            payments_grpc::BankNames::SolutionBank => Ok(Self::SolutionBank),
            payments_grpc::BankNames::Suedtiroler => Ok(Self::Suedtiroler),
            payments_grpc::BankNames::SuedtirolerSparkasse => Ok(Self::SuedtirolerSparkasse),
            payments_grpc::BankNames::SuedtirolerVolksbank => Ok(Self::SuedtirolerVolksbank),
            payments_grpc::BankNames::Unicredit => Ok(Self::Unicredit),
            payments_grpc::BankNames::UnicreditOnlineBanking => Ok(Self::UnicreditOnlineBanking),
            payments_grpc::BankNames::UnicreditUniwebCorporate => {
                Ok(Self::UnicreditUniwebCorporate)
            }
            payments_grpc::BankNames::ValpolicellaBenacoBanca => Ok(Self::ValpolicellaBenacoBanca),
            payments_grpc::BankNames::Volksbank => Ok(Self::Volksbank),
            payments_grpc::BankNames::VolksbankBancaPopolare => Ok(Self::VolksbankBancaPopolare),
            payments_grpc::BankNames::Widiba => Ok(Self::Widiba),
            payments_grpc::BankNames::ZkbCredcoopdiTriesteEGorizia => {
                Ok(Self::ZkbCredcoopdiTriesteEGorizia)
            }
            payments_grpc::BankNames::Asn => Ok(Self::Asn),
            payments_grpc::BankNames::Sns => Ok(Self::Sns),
            payments_grpc::BankNames::Seb => Ok(Self::Seb),
            payments_grpc::BankNames::Swedbank => Ok(Self::Swedbank),
            payments_grpc::BankNames::MockUkPayments => Ok(Self::MockUkPayments),
            payments_grpc::BankNames::Unspecified => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
            )
            .attach_printable("BankNames unspecified")),
            // Add remaining bank names as needed
            _ => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
            )
            .attach_printable("Unknown BankNames variant")),
        }
    }
}

impl ForeignTryFrom<payments_grpc::RedirectForm> for RedirectForm {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::RedirectForm) -> Result<Self, Self::Error> {
        match value.form_type {
            Some(payments_grpc::redirect_form::FormType::Form(form)) => Ok(Self::Form {
                endpoint: form.clone().endpoint,
                method: Method::foreign_try_from(form.clone().method())?,
                form_fields: form.clone().form_fields,
            }),
            Some(payments_grpc::redirect_form::FormType::Html(html)) => Ok(Self::Html {
                html_data: html.html_data,
            }),
            Some(payments_grpc::redirect_form::FormType::Uri(_)) => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "URI form type is not implemented".to_string(),
                )
                .into(),
            ),
            Some(payments_grpc::redirect_form::FormType::HostedIframe(_)) => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Hosted iframe form type is not implemented".to_string(),
                )
                .into(),
            ),
            Some(payments_grpc::redirect_form::FormType::Braintree(braintree)) => {
                Ok(Self::Braintree {
                    client_token: braintree.client_token,
                    card_token: braintree.card_token,
                    bin: braintree.bin,
                    acs_url: braintree.acs_url,
                })
            }
            Some(payments_grpc::redirect_form::FormType::Mifinity(mifinity)) => {
                Ok(Self::Mifinity {
                    initialization_token: mifinity.initialization_token,
                })
            }
            Some(payments_grpc::redirect_form::FormType::Nmi(nmi)) => {
                let amount_money =
                    nmi.amount
                        .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                            field_name: "amount",
                        })?;
                let currency = match payments_grpc::Currency::try_from(amount_money.currency) {
                    Ok(payments_grpc::Currency::Unspecified) | Err(_) => {
                        Err(UnifiedConnectorServiceError::MissingRequiredField {
                            field_name: "currency",
                        })
                    }
                    Ok(c) => common_enums::Currency::from_str(c.as_str_name())
                        .map_err(|_| UnifiedConnectorServiceError::ParsingFailed),
                }
                .attach_printable("Failed to parse currency from UCS Nmi redirect form")?;
                Ok(Self::Nmi {
                    amount: MinorUnit::new(amount_money.minor_amount)
                        .to_major_unit_as_f64(currency)
                        .change_context(UnifiedConnectorServiceError::ParsingFailed)?
                        .get_amount_as_f64()
                        .to_string(),
                    currency,
                    public_key: hyperswitch_masking::Secret::new(
                        nmi.public_key
                            .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                                field_name: "public_key",
                            })?
                            .expose(),
                    ),
                    customer_vault_id: nmi.customer_vault_id,
                    order_id: nmi.order_id,
                })
            }
            Some(payments_grpc::redirect_form::FormType::Script(_)) => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Script form type is not implemented".to_string(),
                )
                .into(),
            ),
            None => Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                    "Missing form type".to_string(),
                )
                .into(),
            ),
        }
    }
}

impl ForeignTryFrom<payments_grpc::HttpMethod> for Method {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(value: payments_grpc::HttpMethod) -> Result<Self, Self::Error> {
        match value {
            payments_grpc::HttpMethod::Get => Ok(Self::Get),
            payments_grpc::HttpMethod::Post => Ok(Self::Post),
            payments_grpc::HttpMethod::Put => Ok(Self::Put),
            payments_grpc::HttpMethod::Delete => Ok(Self::Delete),
            payments_grpc::HttpMethod::Unspecified => {
                Err(UnifiedConnectorServiceError::ResponseDeserializationFailed)
                    .attach_printable("Invalid Http Method")
            }
        }
    }
}

impl ForeignFrom<payments_grpc::UpiSource>
    for hyperswitch_domain_models::payment_method_data::UpiSource
{
    fn foreign_from(upi_source: payments_grpc::UpiSource) -> Self {
        match upi_source {
            payments_grpc::UpiSource::UpiCc => Self::UpiCc,
            payments_grpc::UpiSource::UpiCl => Self::UpiCl,
            payments_grpc::UpiSource::UpiAccount => Self::UpiAccount,
            payments_grpc::UpiSource::UpiCcCl => Self::UpiCcCl,
            payments_grpc::UpiSource::UpiPpi => Self::UpiPpi,
            payments_grpc::UpiSource::UpiVoucher => Self::UpiVoucher,
        }
    }
}

impl UnifiedConnectorServiceError {
    /// Converts tonic::Code to HTTP status code.
    pub fn tonic_to_http_status(code: tonic::Code) -> u16 {
        match code {
            tonic::Code::Cancelled => 408,
            tonic::Code::InvalidArgument => 400,
            tonic::Code::Unauthenticated => 401,
            tonic::Code::PermissionDenied => 403,
            tonic::Code::NotFound => 404,
            tonic::Code::AlreadyExists => 409,
            tonic::Code::ResourceExhausted => 429,
            tonic::Code::FailedPrecondition => 412,
            tonic::Code::Aborted => 409,
            tonic::Code::OutOfRange => 416,
            tonic::Code::Unimplemented => 501,
            tonic::Code::Unavailable => 503,
            tonic::Code::DeadlineExceeded => 504,
            _ => 500,
        }
    }

    fn tonic_status_is_ucs_server_error(code: tonic::Code) -> bool {
        matches!(
            code,
            tonic::Code::Unknown
                | tonic::Code::Internal
                | tonic::Code::Unavailable
                | tonic::Code::DataLoss
        )
    }

    /// Returns HTTP status code for this error.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::TonicStatus { code, .. } => Self::tonic_to_http_status(*code),
            Self::ConnectorError(inner) => inner.status_code,
            Self::ConnectionError(_) => 503,
            Self::InvalidDataFormat { .. }
            | Self::MissingRequiredField { .. }
            | Self::MissingRequiredFields { .. }
            | Self::RequestEncodingFailed
            | Self::RequestEncodingFailedWithReason(_)
            | Self::InvalidConnectorName
            | Self::MissingConnectorName => 400,
            Self::NotImplemented(_) => 501,
            _ => 500,
        }
    }

    /// Maps tonic::Status to UnifiedConnectorServiceError.
    /// First tries to extract a connector HTTP error from proto-encoded status details.
    pub fn from_grpc_error(status: &tonic::Status, connector_name: &str) -> Self {
        // Try to extract ConnectorError from proto-encoded status details
        if let Some(error_from_details) =
            Self::decode_connector_error_response(status, connector_name)
        {
            return error_from_details;
        }

        if let Some(timeout_error) = Self::decode_connector_timeout(status, connector_name) {
            return timeout_error;
        }

        Self::TonicStatus {
            code: status.code(),
            message: status.message().to_string(),
        }
    }

    /// Decodes a connector HTTP error (4xx/5xx) from tonic status details, returning `None` for UCS-side errors.
    fn decode_connector_error_response(
        status: &tonic::Status,
        connector_name: &str,
    ) -> Option<Self> {
        let details = status.details();
        if details.is_empty() {
            return None;
        }

        let connector_error = payments_grpc::ConnectorError::decode(details)
            .inspect_err(|e| {
                router_env::logger::warn!(
                    error = ?e,
                    connector_name = connector_name,
                    "Failed to decode ConnectorError from tonic status details"
                );
            })
            .ok()?;

        // Only treat as a connector HTTP error when the error code explicitly signals it.
        // Other error_code values are UCS-side errors and should fall back to TonicStatus.
        if connector_error.error_code != CONNECTOR_ERROR_RESPONSE_CODE {
            return None;
        }

        let status_code = u16::try_from(connector_error.http_status_code?).ok()?;

        Some(Self::ConnectorError(Box::new(ConnectorErrorInner {
            code: connector_error
                .error_info
                .as_ref()
                .and_then(|error_info| error_info.connector_details.as_ref())
                .and_then(|connector_details| connector_details.code.clone())
                .unwrap_or_else(|| crate::consts::NO_ERROR_CODE.to_string()),
            message: connector_error.error_message,
            status_code,
            reason: connector_error
                .error_info
                .as_ref()
                .and_then(|ei| ei.connector_details.as_ref())
                .and_then(|cd| cd.reason.clone()),
            connector: connector_name.to_string(),
            connector_transaction_id: connector_error
                .error_info
                .as_ref()
                .and_then(|ei| ei.connector_details.as_ref())
                .and_then(|cd| cd.connector_transaction_id.clone()),
            network_decline_code: connector_error
                .error_info
                .as_ref()
                .and_then(|ei| ei.issuer_details.as_ref())
                .and_then(|id| id.network_details.as_ref())
                .and_then(|nd| nd.decline_code.clone()),
            network_advice_code: connector_error
                .error_info
                .as_ref()
                .and_then(|ei| ei.issuer_details.as_ref())
                .and_then(|id| id.network_details.as_ref())
                .and_then(|nd| nd.advice_code.clone()),
            network_error_message: connector_error
                .error_info
                .as_ref()
                .and_then(|ei| ei.issuer_details.as_ref())
                .and_then(|id| id.network_details.as_ref())
                .and_then(|nd| nd.error_message.clone()),
        })))
    }

    fn decode_connector_timeout(status: &tonic::Status, connector_name: &str) -> Option<Self> {
        // UCS maps connector/API client request timeouts to gRPC DeadlineExceeded. The
        // status message is not a stable contract, so rely only on the gRPC code.
        if status.code() != tonic::Code::DeadlineExceeded {
            return None;
        }

        Some(Self::ConnectorError(Box::new(ConnectorErrorInner {
            code: crate::consts::REQUEST_TIMEOUT_ERROR_CODE.to_string(),
            message: crate::consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string(),
            status_code: CONNECTOR_TIMEOUT_HTTP_STATUS_CODE,
            reason: Some(crate::consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string()),
            connector: connector_name.to_string(),
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })))
    }
}

impl ErrorSwitch<ApiErrorResponse> for UnifiedConnectorServiceError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::TonicStatus { code, message } => match code {
                tonic::Code::InvalidArgument | tonic::Code::FailedPrecondition => {
                    ApiErrorResponse::InvalidRequestData {
                        message: message.clone(),
                    }
                }
                tonic::Code::NotFound => ApiErrorResponse::InvalidRequestData {
                    message: format!("Resource not found: {message}"),
                },
                tonic::Code::AlreadyExists => ApiErrorResponse::InvalidRequestData {
                    message: format!("Resource already exists: {message}"),
                },
                tonic::Code::PermissionDenied => ApiErrorResponse::AccessForbidden {
                    resource: message.clone(),
                },
                tonic::Code::Unauthenticated => ApiErrorResponse::Unauthorized,
                tonic::Code::Unimplemented => ApiErrorResponse::NotImplemented {
                    message: NotImplementedMessage::Reason(message.clone()),
                },
                tonic::Code::Unavailable
                | tonic::Code::DeadlineExceeded
                | tonic::Code::Internal => ApiErrorResponse::InternalServerError,
                _ => ApiErrorResponse::InternalServerError,
            },
            Self::ConnectorError(inner) => ApiErrorResponse::ExternalConnectorError {
                code: inner.code.clone(),
                message: inner.message.clone(),
                connector: inner.connector.clone(),
                status_code: inner.status_code,
                reason: inner.reason.clone(),
            },
            _ => ApiErrorResponse::InternalServerError,
        }
    }
}

impl ErrorSwitch<ConnectorError> for UnifiedConnectorServiceError {
    fn switch(&self) -> ConnectorError {
        match self {
            Self::TonicStatus { code, message } => {
                // UCS/Prism server failures must surface as Hyperswitch 5xx errors, not as
                // payment authorization failures with a nested 5xx payload.
                if Self::tonic_status_is_ucs_server_error(*code) {
                    return ConnectorError::ResponseHandlingFailed;
                }

                if *code == tonic::Code::Unimplemented {
                    return ConnectorError::NotImplemented(message.clone());
                }

                // UCS validation/client-class errors keep the encoded payload for callers that
                // already rely on structured processing-step data.
                let status_code = Self::tonic_to_http_status(*code);
                let error_body = serde_json::json!({
                    "code": format!("UCS_{}", status_code),
                    "message": message,
                    "status_code": status_code,
                });
                ConnectorError::ProcessingStepFailed(Some(bytes::Bytes::from(
                    error_body.to_string(),
                )))
            }
            // Connector errors with status code → ResponseHandlingFailed
            Self::ConnectorError(_) => ConnectorError::ResponseHandlingFailed,
            // Connection/availability errors → ResponseHandlingFailed
            Self::ConnectionError(_) => ConnectorError::ResponseHandlingFailed,
            // Request encoding errors
            Self::RequestEncodingFailed
            | Self::RequestEncodingFailedWithReason(_)
            | Self::InvalidDataFormat { .. } => ConnectorError::RequestEncodingFailed,
            // Missing field errors
            Self::MissingRequiredField { field_name } => {
                ConnectorError::MissingRequiredField { field_name }
            }
            Self::MissingRequiredFields { field_names } => ConnectorError::MissingRequiredFields {
                field_names: field_names.clone(),
            },
            // Response deserialization errors
            Self::ResponseDeserializationFailed | Self::ParsingFailed => {
                ConnectorError::ResponseDeserializationFailed
            }
            // Auth errors
            Self::FailedToObtainAuthType => ConnectorError::FailedToObtainAuthType,
            // Not implemented
            Self::NotImplemented(msg) => ConnectorError::NotImplemented(msg.clone()),
            // Invalid connector name
            Self::InvalidConnectorName | Self::MissingConnectorName => {
                ConnectorError::InvalidConnectorName
            }
            // Header injection errors → request encoding failure
            Self::HeaderInjectionFailed(_) => ConnectorError::RequestEncodingFailed,
            // Webhook processing errors
            Self::WebhookProcessingFailure => ConnectorError::ResponseHandlingFailed,
            // All other gRPC operation failures
            Self::PaymentCreateOrderFailure
            | Self::PaymentAuthorizeGranularFailure
            | Self::CreateSessionTokenFailure
            | Self::CreateAccessTokenFailure
            | Self::PaymentMethodTokenizeFailure
            | Self::CreateConnectorCustomerFailure
            | Self::PaymentAuthorizeFailure
            | Self::PaymentPreAuthenticateFailure
            | Self::PaymentAuthenticateFailure
            | Self::PaymentPostAuthenticateFailure
            | Self::PaymentGetFailure
            | Self::PaymentCaptureFailure
            | Self::PaymentSetupRecurringFailure
            | Self::RecurringPaymentChargeFailure
            | Self::PaymentRefundFailure
            | Self::RefundSyncFailure
            | Self::IncomingWebhookHandleEventFailure
            | Self::IncomingWebhookParseEventFailure
            | Self::PaymentVoidFailure
            | Self::CreateSdkSessionTokenFailure
            | Self::PaymentIncrementalAuthorizationFailure
            | Self::PayoutCreateFailure
            | Self::PayoutTransferFailure
            | Self::PayoutGetFailure
            | Self::PayoutVoidFailure
            | Self::PayoutStageFailure
            | Self::PayoutCreateRecipientFailure
            | Self::SurchargeCalculateFailure
            | Self::PayoutEnrollDisburseAccountFailure
            | Self::NotifyConnectorFailure => ConnectorError::ResponseHandlingFailed,
        }
    }
}

/// Why a UCS failure should return a rollout scope to the direct connector integration.
///
/// Named by where the failure originated, because that decides who fixes it: a `Hyperswitch`
/// reason is a bug in this repo's request or response mapping, a `Ucs` reason is not.
///
/// Doubles as the metric label, so it is a small fixed set rather than the error itself — the
/// error carries free-form strings and would be unbounded label cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum UcsKillSwitchReason {
    /// Hyperswitch could not build a valid request. UCS was never reached.
    HyperswitchRequestInvalid,
    /// Hyperswitch could not decode what UCS returned. The two models disagree.
    HyperswitchResponseUndecodable,
    /// UCS rejected the request Hyperswitch sent it.
    UcsRejectedRequest,
    /// UCS does not implement this flow for this connector.
    UcsFlowUnsupported,
    /// UCS failed internally, or failed the flow without saying more.
    UcsInternalError,
    /// UCS could not be reached, or was too unwell to answer.
    UcsUnreachable,
    /// The connector rejected the request. May be a legitimate decline or a request UCS built
    /// wrongly — indistinguishable at this layer, so we trip conservatively because falling back
    /// to the battle-tested direct path is always safe.
    ConnectorOutcome,
}

impl UnifiedConnectorServiceError {
    /// Whether this failure should return the rollout scope to the direct connector integration.
    ///
    /// Every error qualifies, including [`Self::ConnectorError`]. A connector rejection may be a
    /// legitimate decline or a request UCS built wrongly — since the two are indistinguishable,
    /// we trip conservatively: falling back to the battle-tested direct path is always safe.
    ///
    /// Exhaustive: a new variant must be classified here rather than defaulting.
    pub fn ucs_kill_switch_reason(&self) -> Option<UcsKillSwitchReason> {
        match self {
            // Hyperswitch could not read UCS's answer.
            Self::ResponseDeserializationFailed | Self::ParsingFailed => {
                Some(UcsKillSwitchReason::HyperswitchResponseUndecodable)
            }

            // Hyperswitch could not build the request. UCS was never reached.
            Self::RequestEncodingFailed
            | Self::RequestEncodingFailedWithReason(_)
            | Self::MissingRequiredField { .. }
            | Self::MissingRequiredFields { .. }
            | Self::MissingConnectorName
            | Self::InvalidConnectorName
            | Self::InvalidDataFormat { .. }
            | Self::FailedToObtainAuthType
            | Self::HeaderInjectionFailed(_) => {
                Some(UcsKillSwitchReason::HyperswitchRequestInvalid)
            }

            // Raised by Hyperswitch, but it reports a flow UCS cannot serve.
            Self::NotImplemented(_) => Some(UcsKillSwitchReason::UcsFlowUnsupported),

            // UCS-side by construction: `from_grpc_error` extracts connector errors first.
            Self::TonicStatus { code, .. } => match code {
                // UCS-wide rather than scope-specific. Still worth falling back for: while UCS
                // is unwell the payment has nowhere else to go and fails outright.
                tonic::Code::Unavailable
                | tonic::Code::DeadlineExceeded
                | tonic::Code::ResourceExhausted
                | tonic::Code::Aborted
                | tonic::Code::Cancelled => Some(UcsKillSwitchReason::UcsUnreachable),

                // UCS rejected what Hyperswitch sent it.
                tonic::Code::InvalidArgument
                | tonic::Code::FailedPrecondition
                | tonic::Code::OutOfRange
                | tonic::Code::NotFound
                | tonic::Code::AlreadyExists
                | tonic::Code::Unauthenticated
                | tonic::Code::PermissionDenied => Some(UcsKillSwitchReason::UcsRejectedRequest),

                tonic::Code::Unimplemented => Some(UcsKillSwitchReason::UcsFlowUnsupported),

                // UCS itself failed. `Unknown` is included because a rolling deploy surfaces as
                // `Unavailable`, not `Unknown`, so `Unknown` is more often a server-side panic.
                tonic::Code::Internal
                | tonic::Code::DataLoss
                | tonic::Code::Unknown
                | tonic::Code::Ok => Some(UcsKillSwitchReason::UcsInternalError),
            },

            // Could not reach UCS at all. Same treatment as `Unavailable` above.
            Self::ConnectionError(_) => Some(UcsKillSwitchReason::UcsUnreachable),

            // No usable status never reaches here: `from_grpc_error` falls through to
            // `TonicStatus`. A zero means UCS supplied one, which is UCS-side.
            Self::ConnectorError(inner) if inner.status_code == 0 => {
                Some(UcsKillSwitchReason::UcsInternalError)
            }

            // The connector answered. This may be a legitimate decline or a request UCS built
            // wrongly — indistinguishable here. We trip conservatively: a false bypass to the
            // direct path is safe (it served merchants for years), while a missed trip leaves
            // merchants on a potentially broken UCS path.
            Self::ConnectorError(_) => Some(UcsKillSwitchReason::ConnectorOutcome),

            // Per-flow failure markers carrying no further detail.
            Self::WebhookProcessingFailure
            | Self::PaymentCreateOrderFailure
            | Self::PaymentAuthorizeGranularFailure
            | Self::CreateSessionTokenFailure
            | Self::CreateAccessTokenFailure
            | Self::PaymentMethodTokenizeFailure
            | Self::CreateConnectorCustomerFailure
            | Self::PaymentAuthorizeFailure
            | Self::PaymentPreAuthenticateFailure
            | Self::PaymentAuthenticateFailure
            | Self::PaymentPostAuthenticateFailure
            | Self::PaymentGetFailure
            | Self::PaymentCaptureFailure
            | Self::PaymentSetupRecurringFailure
            | Self::RecurringPaymentChargeFailure
            | Self::PaymentRefundFailure
            | Self::RefundSyncFailure
            | Self::IncomingWebhookHandleEventFailure
            | Self::IncomingWebhookParseEventFailure
            | Self::PaymentVoidFailure
            | Self::CreateSdkSessionTokenFailure
            | Self::PaymentIncrementalAuthorizationFailure
            | Self::PayoutCreateFailure
            | Self::PayoutTransferFailure
            | Self::PayoutGetFailure
            | Self::PayoutVoidFailure
            | Self::PayoutStageFailure
            | Self::PayoutCreateRecipientFailure
            | Self::PayoutEnrollDisburseAccountFailure
            | Self::SurchargeCalculateFailure
            | Self::NotifyConnectorFailure => Some(UcsKillSwitchReason::UcsInternalError),
        }
    }
}

#[cfg(test)]
mod ucs_kill_switch_reason_tests {
    use super::*;

    fn tonic_status(code: tonic::Code) -> UnifiedConnectorServiceError {
        UnifiedConnectorServiceError::TonicStatus {
            code,
            message: "from ucs".to_string(),
        }
    }

    #[test]
    fn ucs_wide_grpc_codes_still_qualify() {
        // While UCS is unwell the payment has no fallback and fails outright, so these qualify
        // like any other. They share a label so a run of them reads as one UCS-wide event.
        for code in [
            tonic::Code::Unavailable,
            tonic::Code::DeadlineExceeded,
            tonic::Code::ResourceExhausted,
            tonic::Code::Aborted,
            tonic::Code::Cancelled,
        ] {
            assert_eq!(
                tonic_status(code).ucs_kill_switch_reason(),
                Some(UcsKillSwitchReason::UcsUnreachable),
                "{code:?}"
            );
        }
    }

    #[test]
    fn ucs_side_grpc_codes_are_kill_switch_worthy() {
        // `from_grpc_error` extracts connector errors and timeouts before producing TonicStatus,
        // so what reaches here is UCS-side and repeats identically for this scope.
        let cases = [
            (
                tonic::Code::InvalidArgument,
                UcsKillSwitchReason::UcsRejectedRequest,
            ),
            (
                tonic::Code::FailedPrecondition,
                UcsKillSwitchReason::UcsRejectedRequest,
            ),
            (
                tonic::Code::Unauthenticated,
                UcsKillSwitchReason::UcsRejectedRequest,
            ),
            (
                tonic::Code::Unimplemented,
                UcsKillSwitchReason::UcsFlowUnsupported,
            ),
            (tonic::Code::Internal, UcsKillSwitchReason::UcsInternalError),
            (tonic::Code::Unknown, UcsKillSwitchReason::UcsInternalError),
        ];

        for (code, expected) in cases {
            assert_eq!(
                tonic_status(code).ucs_kill_switch_reason(),
                Some(expected),
                "{code:?}"
            );
        }
    }
}
