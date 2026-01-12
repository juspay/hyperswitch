use common_enums::AttemptStatus;
use common_types::primitive_wrappers::{ExtendedAuthorizationAppliedBool, OvercaptureEnabledBool};
use hyperswitch_domain_models::{
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorResponseData, ErrorResponse,
        ExtendedAuthorizationResponseData,
    },
    router_response_types::PaymentsResponseData,
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

    /// Failed to perform Create Order from gRPC Server
    #[error("Failed to perform Create Order from gRPC Server")]
    PaymentCreateOrderFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform. Granular Payment Authorize from gRPC Server")]
    PaymentAuthorizeGranularFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Payment Session Token Create from gRPC Server")]
    PaymentCreateSessionTokenFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Payment Access Token Create from gRPC Server")]
    PaymentCreateAccessTokenFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Payment Method Token Create from gRPC Server")]
    PaymentMethodTokenCreateFailure,

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Connector Customer Create from gRPC Server")]
    PaymentConnectorCustomerCreateFailure,

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

    /// Failed to perform Payment Setup Mandate from gRPC Server
    #[error("Failed to perform Setup Mandate from gRPC Server")]
    PaymentRegisterFailure,

    /// Failed to perform Payment Repeat Payment from gRPC Server
    #[error("Failed to perform Repeat Payment from gRPC Server")]
    PaymentRepeatEverythingFailure,

    /// Failed to perform Payment Refund from gRPC Server
    #[error("Failed to perform Payment Refund from gRPC Server")]
    PaymentRefundFailure,

    /// Failed to perform Refund Sync from gRPC Server
    #[error("Failed to perform Refund Sync from gRPC Server")]
    RefundSyncFailure,

    /// Failed to transform incoming webhook from gRPC Server
    #[error("Failed to transform incoming webhook from gRPC Server")]
    WebhookTransformFailure,

    /// Failed to perform Payment Cancel from gRPC Server
    #[error("Failed to perform Cancel from gRPC Server")]
    PaymentCancelFailure,

    /// Failed to perform Sdk Session Token from gRPC Server
    #[error("Failed to perform Sdk Session Token from gRPC Server")]
    SdkSessionTokenFailure,
}

/// UCS Webhook transformation status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WebhookTransformationStatus {
    /// Transformation completed successfully, no further action needed
    Complete,
    /// Transformation incomplete, requires second call for final status
    Incomplete,
}

#[allow(missing_docs)]
/// Webhook transform data structure containing UCS response information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookTransformData {
    pub event_type: api_models::webhooks::IncomingWebhookEvent,
    pub source_verified: bool,
    pub webhook_content: Option<payments_grpc::WebhookResponseContent>,
    pub response_ref_id: Option<String>,
    pub webhook_transformation_status: WebhookTransformationStatus,
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
                connector_transaction_id: resource_id.get_optional_response_id(),
                connector_response_reference_id,
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
                            payment_method_id: grpc_mandate.payment_method_id,
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
            payments_grpc::PaymentStatus::VoidedPostCapture => Ok(Self::Voided),
            payments_grpc::PaymentStatus::AttemptStatusUnspecified => Ok(Self::Unresolved),
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
            .map(AdditionalPaymentMethodConnectorResponse::foreign_try_from)
            .transpose()?;

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
        let card_data = value.card.unwrap_or_default();

        Ok(Self::Card {
            authentication_data: card_data.authentication_data.and_then(|data| {
                serde_json::from_slice(&data)
                    .inspect_err(|e| {
                        router_env::logger::warn!(
                            deserialization_error=?e,
                            "Failed to deserialize authentication_data from UCS connector response"
                        );
                    })
                    .ok()
            }),
            payment_checks: card_data.payment_checks.and_then(|data| {
                serde_json::from_slice(&data)
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
        })
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
        use unified_connector_service_masking::ExposeInterface;

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
            account_number: masking::Secret::new(
                ach.account_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "account_number",
                    })?
                    .expose(),
            ),
            routing_number: masking::Secret::new(
                ach.routing_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "routing_number",
                    })?
                    .expose(),
            ),
            card_holder_name: ach
                .card_holder_name
                .map(|s| masking::Secret::new(s.expose())),
            bank_account_holder_name: ach
                .bank_account_holder_name
                .map(|s| masking::Secret::new(s.expose())),
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
        use unified_connector_service_masking::ExposeInterface;

        Ok(Self::SepaBankDebit {
            iban: masking::Secret::new(
                sepa.iban
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "iban",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: sepa
                .bank_account_holder_name
                .map(|name| masking::Secret::new(name.expose())),
        })
    }
}

impl ForeignTryFrom<payments_grpc::Bacs>
    for hyperswitch_domain_models::payment_method_data::BankDebitData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bacs: payments_grpc::Bacs) -> Result<Self, Self::Error> {
        use unified_connector_service_masking::ExposeInterface;

        Ok(Self::BacsBankDebit {
            account_number: masking::Secret::new(
                bacs.account_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "account_number",
                    })?
                    .expose(),
            ),
            sort_code: masking::Secret::new(
                bacs.sort_code
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "sort_code",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: bacs
                .bank_account_holder_name
                .map(|name| masking::Secret::new(name.expose())),
        })
    }
}

impl ForeignTryFrom<payments_grpc::Becs>
    for hyperswitch_domain_models::payment_method_data::BankDebitData
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(becs: payments_grpc::Becs) -> Result<Self, Self::Error> {
        use unified_connector_service_masking::ExposeInterface;

        Ok(Self::BecsBankDebit {
            account_number: masking::Secret::new(
                becs.account_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "account_number",
                    })?
                    .expose(),
            ),
            bsb_number: masking::Secret::new(
                becs.bsb_number
                    .ok_or(UnifiedConnectorServiceError::MissingRequiredField {
                        field_name: "bsb_number",
                    })?
                    .expose(),
            ),
            bank_account_holder_name: becs
                .bank_account_holder_name
                .map(|name| masking::Secret::new(name.expose())),
        })
    }
}

impl ForeignTryFrom<payments_grpc::BankType> for common_enums::BankType {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(bank_type: payments_grpc::BankType) -> Result<Self, Self::Error> {
        match bank_type {
            payments_grpc::BankType::Checking => Ok(Self::Checking),
            payments_grpc::BankType::Savings => Ok(Self::Savings),
            payments_grpc::BankType::Unspecified => Err(error_stack::Report::new(
                UnifiedConnectorServiceError::ResponseDeserializationFailed,
            )
            .attach_printable("BankType unspecified")),
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
