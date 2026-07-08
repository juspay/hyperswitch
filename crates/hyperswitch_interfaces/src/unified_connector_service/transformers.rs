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

    /// Connector error received through UCS (contains original connector HTTP status code).
    /// Distinguishes connector errors from UCS errors by presence of status_code.
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
    /// Original HTTP status code from connector
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

        let connector_transaction_id =
            hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(
                response.connector_transaction_id.clone(),
            );

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
                connector_response_reference_id: response.merchant_transaction_id,
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
                    connector_response_reference_id: response.merchant_transaction_id,
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
            payments_grpc::BankType::Bond
            | payments_grpc::BankType::Transmission
            | payments_grpc::BankType::Current
            | payments_grpc::BankType::SubscriptionShare
            | payments_grpc::BankType::Unspecified => Err(error_stack::Report::new(
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
            tonic::Code::InvalidArgument | tonic::Code::FailedPrecondition => 400,
            tonic::Code::Unauthenticated => 401,
            tonic::Code::PermissionDenied => 403,
            tonic::Code::NotFound => 404,
            tonic::Code::AlreadyExists => 409,
            tonic::Code::Unimplemented => 501,
            tonic::Code::Unavailable => 503,
            tonic::Code::DeadlineExceeded => 504,
            _ => 500,
        }
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
                // `connector_details.code` is left unset by UCS when the connector
                // returned no specific error code (the transformer emits the
                // `NO_ERROR_CODE` sentinel, which is deliberately not surfaced as a
                // connector-specific code). In that case the connector's actual code
                // still lives verbatim in `unified_details.code`, so prefer it over the
                // generic `CONNECTOR_ERROR_RESPONSE` discriminator to stay in parity with
                // the native connector path (e.g. cybersource psync 404 -> "No error code").
                .or_else(|| {
                    connector_error
                        .error_info
                        .as_ref()
                        .and_then(|error_info| error_info.unified_details.as_ref())
                        .and_then(|unified_details| unified_details.code.clone())
                })
                .unwrap_or_else(|| connector_error.error_code.clone()),
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
            // UCS validation errors (4xx from tonic) → ProcessingStepFailed with encoded error
            // body so the upstream handler can return the right HTTP status code.
            Self::TonicStatus { code, message } => {
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
