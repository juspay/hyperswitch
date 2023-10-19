#![allow(dead_code, unused_variables)]

use http::StatusCode;
use scheduler::errors::{PTError, ProcessTrackerError};

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    InvalidRequestError,
    ObjectNotFound,
    RouterError,
    ProcessingError,
    BadGateway,
    ServerNotAvailable,
    DuplicateRequest,
    ValidationError,
    ConnectorError,
    LockTimeout,
}

#[allow(dead_code)]
#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = ErrorType)]
pub enum ApiErrorResponse {
    #[error(error_type = ErrorType::ServerNotAvailable, code = "IR_00", message = "{message:?}")]
    NotImplemented { message: NotImplementedMessage },
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_01",
        message = "API key not provided or invalid API key used"
    )]
    Unauthorized,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_02", message = "Unrecognized request URL")]
    InvalidRequestUrl,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_03", message = "The HTTP method is not applicable for this API")]
    InvalidHttpMethod,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_04", message = "Missing required param: {field_name}")]
    MissingRequiredField { field_name: &'static str },
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_05",
        message = "{field_name} contains invalid data. Expected format is {expected_format}"
    )]
    InvalidDataFormat {
        field_name: String,
        expected_format: String,
    },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_06", message = "{message}")]
    InvalidRequestData { message: String },
    /// Typically used when a field has invalid value, or deserialization of the value contained in a field fails.
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "Invalid value provided: {field_name}")]
    InvalidDataValue { field_name: &'static str },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_08", message = "Client secret was not provided")]
    ClientSecretNotGiven,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_08", message = "Client secret has expired")]
    ClientSecretExpired,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_09", message = "The client_secret provided does not match the client_secret associated with the Payment")]
    ClientSecretInvalid,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_10", message = "Customer has active mandate/subsciption")]
    MandateActive,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_11", message = "Customer has already been redacted")]
    CustomerRedacted,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_12", message = "Reached maximum refund attempts")]
    MaximumRefundCount,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_13", message = "Refund amount exceeds the payment amount")]
    RefundAmountExceedsPaymentAmount,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_14", message = "This Payment could not be {current_flow} because it has a {field_name} of {current_value}. The expected state is {states}")]
    PaymentUnexpectedState {
        current_flow: String,
        field_name: String,
        current_value: String,
        states: String,
    },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_15", message = "Invalid Ephemeral Key for the customer")]
    InvalidEphemeralKey,
    /// Typically used when information involving multiple fields or previously provided information doesn't satisfy a condition.
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_16", message = "{message}")]
    PreconditionFailed { message: String },
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_17",
        message = "Access forbidden, invalid JWT token was used"
    )]
    InvalidJwtToken,
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_18",
        message = "{message}",
    )]
    GenericUnauthorized { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_19", message = "{message}")]
    NotSupported { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_20", message = "{flow} flow not supported by the {connector} connector")]
    FlowNotSupported { flow: String, connector: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_21", message = "Missing required params")]
    MissingRequiredFields { field_names: Vec<&'static str> },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_22", message = "Access forbidden. Not authorized to access this resource {resource}")]
    AccessForbidden { resource: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_23", message = "{message}")]
    FileProviderNotSupported { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_23", message = "{message}")]
    UnprocessableEntity { message: String },
    #[error(error_type = ErrorType::ConnectorError, code = "CE_00", message = "{code}: {message}", ignore = "status_code")]
    ExternalConnectorError {
        code: String,
        message: String,
        connector: String,
        status_code: u16,
        reason: Option<String>,
    },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_01", message = "Payment failed during authorization with connector. Retry payment")]
    PaymentAuthorizationFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_02", message = "Payment failed during authentication with connector. Retry payment")]
    PaymentAuthenticationFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_03", message = "Capture attempt failed while processing with connector")]
    PaymentCaptureFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_04", message = "The card data is invalid")]
    InvalidCardData { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::InvalidRequestError, code = "CE_04", message = "Payout validation failed")]
    PayoutFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_05", message = "The card has expired")]
    CardExpired { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_06", message = "Refund failed while processing with connector. Retry refund")]
    RefundFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_07", message = "Verification failed while processing with connector. Retry operation")]
    VerificationFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_08", message = "Dispute operation failed while processing with connector. Retry operation")]
    DisputeFailed { data: Option<serde_json::Value> },

    #[error(error_type = ErrorType::ServerNotAvailable, code = "HE_00", message = "Something went wrong")]
    InternalServerError,
    #[error(error_type = ErrorType::LockTimeout, code = "HE_00", message = "Resource is busy. Please try again later.")]
    ResourceBusy,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "Duplicate refund request. Refund already attempted with the refund ID")]
    DuplicateRefundRequest,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "Duplicate mandate request. Mandate already attempted with the Mandate ID")]
    DuplicateMandate,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The merchant account with the specified details already exists in our records")]
    DuplicateMerchantAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The merchant connector account with the specified profile_id '{profile_id}' and connector_name '{connector_name}' already exists in our records")]
    DuplicateMerchantConnectorAccount {
        profile_id: String,
        connector_name: String,
    },
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payment method with the specified details already exists in our records")]
    DuplicatePaymentMethod,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payment with the specified payment_id already exists in our records")]
    DuplicatePayment { payment_id: String },
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payout with the specified payout_id '{payout_id}' already exists in our records")]
    DuplicatePayout { payout_id: String },
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The config with the specified key already exists in our records")]
    DuplicateConfig,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Refund does not exist in our records")]
    RefundNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Customer does not exist in our records")]
    CustomerNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Config key does not exist in our records.")]
    ConfigNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Payment does not exist in our records")]
    PaymentNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Payment method does not exist in our records")]
    PaymentMethodNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Merchant account does not exist in our records")]
    MerchantAccountNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Merchant connector account does not exist in our records")]
    MerchantConnectorAccountNotFound { id: String },
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Business profile with the given id  '{id}' does not exist in our records")]
    BusinessProfileNotFound { id: String },
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Resource ID does not exist in our records")]
    ResourceIdNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Mandate does not exist in our records")]
    MandateNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Failed to update mandate")]
    MandateUpdateFailed,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "API Key does not exist in our records")]
    ApiKeyNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Payout does not exist in our records")]
    PayoutNotFound,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Invalid mandate id passed from connector")]
    MandateSerializationFailed,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Unable to parse the mandate identifier passed from connector")]
    MandateDeserializationFailed,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Return URL is not configured and not passed in payments request")]
    ReturnUrlUnavailable,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "This refund is not possible through Hyperswitch. Please raise the refund through {connector} dashboard")]
    RefundNotPossible { connector: String },
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Mandate Validation Failed" )]
    MandateValidationFailed { reason: String },
    #[error(error_type= ErrorType::ValidationError, code = "HE_03", message = "The payment has not succeeded yet. Please pass a successful payment to initiate refund")]
    PaymentNotSucceeded,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "The specified merchant connector account is disabled")]
    MerchantConnectorAccountDisabled,
    #[error(error_type= ErrorType::ObjectNotFound, code = "HE_04", message = "Successful payment not found for the given payment id")]
    SuccessfulPaymentNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "The connector provided in the request is incorrect or not available")]
    IncorrectConnectorNameGiven,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "Address does not exist in our records")]
    AddressNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "Dispute does not exist in our records")]
    DisputeNotFound { dispute_id: String },
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "File does not exist in our records")]
    FileNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "File not available")]
    FileNotAvailable,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "Dispute status validation failed")]
    DisputeStatusValidationFailed { reason: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "Card with the provided iin does not exist")]
    InvalidCardIin,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "The provided card IIN length is invalid, please provide an iin with 6 or 8 digits")]
    InvalidCardIinLength,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "File validation failed")]
    FileValidationFailed { reason: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "File not found / valid in the request")]
    MissingFile,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "Dispute id not found in the request")]
    MissingDisputeId,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "File purpose not found in the request or is invalid")]
    MissingFilePurpose,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "File content type not found / valid")]
    MissingFileContentType,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_05", message = "{message}")]
    GenericNotFoundError { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_01", message = "{message}")]
    GenericDuplicateError { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_01", message = "Failed to authenticate the webhook")]
    WebhookAuthenticationFailed,
    #[error(error_type = ErrorType::ObjectNotFound, code = "WE_04", message = "Webhook resource not found")]
    WebhookResourceNotFound,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_02", message = "Bad request received in webhook")]
    WebhookBadRequest,
    #[error(error_type = ErrorType::RouterError, code = "WE_03", message = "There was some issue processing the webhook")]
    WebhookProcessingFailure,
    #[error(error_type = ErrorType::InvalidRequestError, code = "HE_04", message = "required payment method is not configured or configured incorrectly for all configured connectors")]
    IncorrectPaymentMethodConfiguration,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_05", message = "Unable to process the webhook body")]
    WebhookUnprocessableEntity,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Payment Link does not exist in our records")]
    PaymentLinkNotFound,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_05", message = "Merchant Secret set my merchant for webhook source verification is invalid")]
    WebhookInvalidMerchantSecret,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_19", message = "{message}")]
    CurrencyNotSupported { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_24", message = "Merchant connector account is configured with invalid {config}")]
    InvalidConnectorConfiguration { config: String },
}

impl PTError for ApiErrorResponse {
    fn to_pt_error(&self) -> ProcessTrackerError {
        ProcessTrackerError::EApiErrorResponse
    }
}

#[derive(Clone)]
pub enum NotImplementedMessage {
    Reason(String),
    Default,
}

impl std::fmt::Debug for NotImplementedMessage {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reason(message) => write!(fmt, "{message} is not implemented"),
            Self::Default => {
                write!(
                    fmt,
                    "This API is under development and will be made available soon."
                )
            }
        }
    }
}

impl ::core::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(self).unwrap_or_else(|_| "API error response".to_string())
        )
    }
}

impl actix_web::ResponseError for ApiErrorResponse {
    fn status_code(&self) -> StatusCode {
        common_utils::errors::ErrorSwitch::<api_models::errors::types::ApiErrorResponse>::switch(
            self,
        )
        .status_code()
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        common_utils::errors::ErrorSwitch::<api_models::errors::types::ApiErrorResponse>::switch(
            self,
        )
        .error_response()
    }
}

impl crate::services::EmbedError for error_stack::Report<ApiErrorResponse> {}
