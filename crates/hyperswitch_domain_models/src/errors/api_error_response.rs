use api_models::errors::types::Extra;
use common_utils::errors::ErrorSwitch;
use http::StatusCode;

use crate::router_data;

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

// CE	Connector Error	Errors originating from connector's end
// HE	Hyperswitch Error	Errors originating from Hyperswitch's end
// IR	Invalid Request Error	Error caused due to invalid fields and values in API request
// WE	Webhook Error	Errors related to Webhooks
#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = ErrorType)]
pub enum ApiErrorResponse {
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
    #[error(error_type = ErrorType::ProcessingError, code = "CE_05", message = "The card has expired")]
    CardExpired { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_06", message = "Refund failed while processing with connector. Retry refund")]
    RefundFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_07", message = "Verification failed while processing with connector. Retry operation")]
    VerificationFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_08", message = "Dispute operation failed while processing with connector. Retry operation")]
    DisputeFailed { data: Option<serde_json::Value> },

    #[error(error_type = ErrorType::LockTimeout, code = "HE_00", message = "Resource is busy. Please try again later.")]
    ResourceBusy,
    #[error(error_type = ErrorType::ServerNotAvailable, code = "HE_00", message = "Something went wrong")]
    InternalServerError,
    #[error(error_type = ErrorType::ServerNotAvailable, code= "HE_00", message = "{component} health check is failing with error: {message}")]
    HealthCheckError {
        component: &'static str,
        message: String,
    },
    #[error(error_type = ErrorType::ValidationError, code = "HE_00", message = "Failed to convert currency to minor unit")]
    CurrencyConversionFailed,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "Duplicate refund request. Refund already attempted with the refund ID")]
    DuplicateRefundRequest,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "Duplicate mandate request. Mandate already attempted with the Mandate ID")]
    DuplicateMandate,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The merchant account with the specified details already exists in our records")]
    DuplicateMerchantAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The merchant connector account with the specified profile_id '{profile_id}' and connector_label '{connector_label}' already exists in our records")]
    DuplicateMerchantConnectorAccount {
        profile_id: String,
        connector_label: String,
    },
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payment method with the specified details already exists in our records")]
    DuplicatePaymentMethod,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payment with the specified payment_id already exists in our records")]
    DuplicatePayment {
        payment_id: common_utils::id_type::PaymentId,
    },
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payout with the specified payout_id '{payout_id}' already exists in our records")]
    DuplicatePayout { payout_id: String },
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The config with the specified key already exists in our records")]
    DuplicateConfig,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Refund does not exist in our records")]
    RefundNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Payment Link does not exist in our records")]
    PaymentLinkNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Customer does not exist in our records")]
    CustomerNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Config key does not exist in our records.")]
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
    ProfileNotFound { id: String },
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Poll with the given id  '{id}' does not exist in our records")]
    PollNotFound { id: String },
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Resource ID does not exist in our records")]
    ResourceIdNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Mandate does not exist in our records")]
    MandateNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Authentication does not exist in our records")]
    AuthenticationNotFound { id: String },
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Failed to update mandate")]
    MandateUpdateFailed,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "API Key does not exist in our records")]
    ApiKeyNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Payout does not exist in our records")]
    PayoutNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Event does not exist in our records")]
    EventNotFound,
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
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "{code}: {message}")]
    PaymentBlockedError {
        code: u16,
        message: String,
        status: String,
        reason: String,
    },
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "File validation failed")]
    FileValidationFailed { reason: String },
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Dispute status validation failed")]
    DisputeStatusValidationFailed { reason: String },
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
    #[error(error_type = ErrorType::ProcessingError, code = "HE_05", message = "Missing tenant id")]
    MissingTenantId,
    #[error(error_type = ErrorType::ProcessingError, code = "HE_05", message = "Invalid tenant id: {tenant_id}")]
    InvalidTenant { tenant_id: String },
    #[error(error_type = ErrorType::ValidationError, code = "HE_06", message = "Failed to convert amount to {amount_type} type")]
    AmountConversionFailed { amount_type: &'static str },
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
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_13", message = "The refund amount exceeds the amount captured")]
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
    #[error(
        error_type = ErrorType::ProcessingError, code = "IR_24",
        message = "Invalid {wallet_name} wallet token"
    )]
    InvalidWalletToken { wallet_name: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_25", message = "Cannot delete the default payment method")]
    PaymentMethodDeleteFailed,
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_26",
        message = "Invalid Cookie"
    )]
    InvalidCookie,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_27", message = "Extended card info does not exist")]
    ExtendedCardInfoNotFound,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_28", message = "{message}")]
    CurrencyNotSupported { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_29", message = "{message}")]
    UnprocessableEntity { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_30", message = "Merchant connector account is configured with invalid {config}")]
    InvalidConnectorConfiguration { config: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_31", message = "Card with the provided iin does not exist")]
    InvalidCardIin,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_32", message = "The provided card IIN length is invalid, please provide an iin with 6 or 8 digits")]
    InvalidCardIinLength,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_33", message = "File not found / valid in the request")]
    MissingFile,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_34", message = "Dispute id not found in the request")]
    MissingDisputeId,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_35", message = "File purpose not found in the request or is invalid")]
    MissingFilePurpose,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_36", message = "File content type not found / valid")]
    MissingFileContentType,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_37", message = "{message}")]
    GenericNotFoundError { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_38", message = "{message}")]
    GenericDuplicateError { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_39", message = "required payment method is not configured or configured incorrectly for all configured connectors")]
    IncorrectPaymentMethodConfiguration,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_40", message = "{message}")]
    LinkConfigurationError { message: String },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_41", message = "Payout validation failed")]
    PayoutFailed { data: Option<serde_json::Value> },
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_42",
        message = "Cookies are not found in the request"
    )]
    CookieNotFound,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_43", message = "API does not support platform account operation")]
    PlatformAccountAuthNotSupported,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_44", message = "Invalid platform account operation")]
    InvalidPlatformOperation,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_01", message = "Failed to authenticate the webhook")]
    WebhookAuthenticationFailed,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_02", message = "Bad request received in webhook")]
    WebhookBadRequest,
    #[error(error_type = ErrorType::RouterError, code = "WE_03", message = "There was some issue processing the webhook")]
    WebhookProcessingFailure,
    #[error(error_type = ErrorType::ObjectNotFound, code = "WE_04", message = "Webhook resource not found")]
    WebhookResourceNotFound,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_05", message = "Unable to process the webhook body")]
    WebhookUnprocessableEntity,
    #[error(error_type = ErrorType::InvalidRequestError, code = "WE_06", message = "Merchant Secret set my merchant for webhook source verification is invalid")]
    WebhookInvalidMerchantSecret,
    #[error(error_type = ErrorType::ServerNotAvailable, code = "IE", message = "{reason} as data mismatched for {field_names}")]
    IntegrityCheckFailed {
        reason: String,
        field_names: String,
        connector_transaction_id: Option<String>,
    },
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

impl ErrorSwitch<api_models::errors::types::ApiErrorResponse> for ApiErrorResponse {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};

        match self {
            Self::ExternalConnectorError {
                code,
                message,
                connector,
                reason,
                status_code,
            } => AER::ConnectorError(ApiError::new("CE", 0, format!("{code}: {message}"), Some(Extra {connector: Some(connector.clone()), reason: reason.to_owned(), ..Default::default()})), StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)),
            Self::PaymentAuthorizationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 1, "Payment failed during authorization with connector. Retry payment", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::PaymentAuthenticationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 2, "Payment failed during authentication with connector. Retry payment", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::PaymentCaptureFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 3, "Capture attempt failed while processing with connector", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::InvalidCardData { data } => AER::BadRequest(ApiError::new("CE", 4, "The card data is invalid", Some(Extra { data: data.clone(), ..Default::default()}))),
            Self::CardExpired { data } => AER::BadRequest(ApiError::new("CE", 5, "The card has expired", Some(Extra { data: data.clone(), ..Default::default()}))),
            Self::RefundFailed { data } => AER::BadRequest(ApiError::new("CE", 6, "Refund failed while processing with connector. Retry refund", Some(Extra { data: data.clone(), ..Default::default()}))),
            Self::VerificationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 7, "Verification failed while processing with connector. Retry operation", Some(Extra { data: data.clone(), ..Default::default()})))
            },
            Self::DisputeFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 8, "Dispute operation failed while processing with connector. Retry operation", Some(Extra { data: data.clone(), ..Default::default()})))
            }

            Self::ResourceBusy => {
                AER::Unprocessable(ApiError::new("HE", 0, "There was an issue processing the webhook body", None))
            }
            Self::CurrencyConversionFailed => {
                AER::Unprocessable(ApiError::new("HE", 0, "Failed to convert currency to minor unit", None))
            }
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, "Something went wrong", None))
            },
            Self::HealthCheckError { message,component } => {
                AER::InternalServerError(ApiError::new("HE",0,format!("{} health check failed with error: {}",component,message),None))
            },
            Self::DuplicateRefundRequest => AER::BadRequest(ApiError::new("HE", 1, "Duplicate refund request. Refund already attempted with the refund ID", None)),
            Self::DuplicateMandate => AER::BadRequest(ApiError::new("HE", 1, "Duplicate mandate request. Mandate already attempted with the Mandate ID", None)),
            Self::DuplicateMerchantAccount => AER::BadRequest(ApiError::new("HE", 1, "The merchant account with the specified details already exists in our records", None)),
            Self::DuplicateMerchantConnectorAccount { profile_id, connector_label: connector_name } => {
                AER::BadRequest(ApiError::new("HE", 1, format!("The merchant connector account with the specified profile_id '{profile_id}' and connector_label '{connector_name}' already exists in our records"), None))
            }
            Self::DuplicatePaymentMethod => AER::BadRequest(ApiError::new("HE", 1, "The payment method with the specified details already exists in our records", None)),
            Self::DuplicatePayment { payment_id } => {
                AER::BadRequest(ApiError::new("HE", 1, "The payment with the specified payment_id already exists in our records", Some(Extra {reason: Some(format!("{payment_id:?} already exists")), ..Default::default()})))
            }
            Self::DuplicatePayout { payout_id } => {
                AER::BadRequest(ApiError::new("HE", 1, format!("The payout with the specified payout_id '{payout_id}' already exists in our records"), None))
            }
            Self::DuplicateConfig => {
                AER::BadRequest(ApiError::new("HE", 1, "The config with the specified key already exists in our records", None))
            }
            Self::RefundNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Refund does not exist in our records.", None))
            }
            Self::PaymentLinkNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Payment Link does not exist in our records", None))
            }
            Self::CustomerNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Customer does not exist in our records", None))
            }
            Self::ConfigNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Config key does not exist in our records.", None))
            },
            Self::PaymentNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Payment does not exist in our records", None))
            }
            Self::PaymentMethodNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Payment method does not exist in our records", None))
            }
            Self::MerchantAccountNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Merchant account does not exist in our records", None))
            }
            Self::MerchantConnectorAccountNotFound {id } => {
                AER::NotFound(ApiError::new("HE", 2, "Merchant connector account does not exist in our records", Some(Extra {reason: Some(format!("{id} does not exist")), ..Default::default()})))
            }
            Self::ProfileNotFound { id } => {
                AER::NotFound(ApiError::new("HE", 2, format!("Business profile with the given id {id} does not exist"), None))
            }
            Self::PollNotFound { .. } => {
                AER::NotFound(ApiError::new("HE", 2, "Poll does not exist in our records", None))
            },
            Self::ResourceIdNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Resource ID does not exist in our records", None))
            }
            Self::MandateNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Mandate does not exist in our records", None))
            }
            Self::AuthenticationNotFound { .. } => {
                AER::NotFound(ApiError::new("HE", 2, "Authentication does not exist in our records", None))
            },
            Self::MandateUpdateFailed => {
                AER::InternalServerError(ApiError::new("HE", 2, "Mandate update failed", None))
            },
            Self::ApiKeyNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "API Key does not exist in our records", None))
            }
            Self::PayoutNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Payout does not exist in our records", None))
            }
            Self::EventNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Event does not exist in our records", None))
            }
            Self::MandateSerializationFailed | Self::MandateDeserializationFailed => {
                AER::InternalServerError(ApiError::new("HE", 3, "Something went wrong", None))
            },
            Self::ReturnUrlUnavailable => AER::NotFound(ApiError::new("HE", 3, "Return URL is not configured and not passed in payments request", None)),
            Self::RefundNotPossible { connector } => {
                AER::BadRequest(ApiError::new("HE", 3, format!("This refund is not possible through Hyperswitch. Please raise the refund through {connector} dashboard"), None))
            }
            Self::MandateValidationFailed { reason } => {
                AER::BadRequest(ApiError::new("HE", 3, "Mandate Validation Failed", Some(Extra { reason: Some(reason.to_owned()), ..Default::default() })))
            }
            Self::PaymentNotSucceeded => AER::BadRequest(ApiError::new("HE", 3, "The payment has not succeeded yet. Please pass a successful payment to initiate refund", None)),
            Self::MerchantConnectorAccountDisabled => {
                AER::BadRequest(ApiError::new("HE", 3, "The selected merchant connector account is disabled", None))
            }
            Self::PaymentBlockedError {
                message,
                reason,
                ..
            } => AER::DomainError(ApiError::new("HE", 3, message, Some(Extra { reason: Some(reason.clone()), ..Default::default() }))),
            Self::FileValidationFailed { reason } => {
                AER::BadRequest(ApiError::new("HE", 3, format!("File validation failed {reason}"), None))
            }
            Self::DisputeStatusValidationFailed { .. } => {
                AER::BadRequest(ApiError::new("HE", 3, "Dispute status validation failed", None))
            }
            Self::SuccessfulPaymentNotFound => {
                AER::NotFound(ApiError::new("HE", 4, "Successful payment not found for the given payment id", None))
            }
            Self::IncorrectConnectorNameGiven => {
                AER::NotFound(ApiError::new("HE", 4, "The connector provided in the request is incorrect or not available", None))
            }
            Self::AddressNotFound => {
                AER::NotFound(ApiError::new("HE", 4, "Address does not exist in our records", None))
            },
            Self::DisputeNotFound { .. } => {
                AER::NotFound(ApiError::new("HE", 4, "Dispute does not exist in our records", None))
            },
            Self::FileNotFound => {
                AER::NotFound(ApiError::new("HE", 4, "File does not exist in our records", None))
            }
            Self::FileNotAvailable => {
                AER::NotFound(ApiError::new("HE", 4, "File not available", None))
            }
            Self::MissingTenantId => {
                AER::InternalServerError(ApiError::new("HE", 5, "Missing Tenant ID in the request".to_string(), None))
            }
            Self::InvalidTenant { tenant_id }  => {
                AER::InternalServerError(ApiError::new("HE", 5, format!("Invalid Tenant {tenant_id}"), None))
            }
            Self::AmountConversionFailed { amount_type }  => {
                AER::InternalServerError(ApiError::new("HE", 6, format!("Failed to convert amount to {amount_type} type"), None))
            }

            Self::NotImplemented { message } => {
                AER::NotImplemented(ApiError::new("IR", 0, format!("{message:?}"), None))
            }
            Self::Unauthorized => AER::Unauthorized(ApiError::new(
                "IR",
                1,
                "API key not provided or invalid API key used", None
            )),
            Self::InvalidRequestUrl => {
                AER::NotFound(ApiError::new("IR", 2, "Unrecognized request URL", None))
            }
            Self::InvalidHttpMethod => AER::MethodNotAllowed(ApiError::new(
                "IR",
                3,
                "The HTTP method is not applicable for this API", None
            )),
            Self::MissingRequiredField { field_name } => AER::BadRequest(
                ApiError::new("IR", 4, format!("Missing required param: {field_name}"), None),
            ),
            Self::InvalidDataFormat {
                field_name,
                expected_format,
            } => AER::Unprocessable(ApiError::new(
                "IR",
                5,
                format!(
                    "{field_name} contains invalid data. Expected format is {expected_format}"
                ), None
            )),
            Self::InvalidRequestData { message } => {
                AER::Unprocessable(ApiError::new("IR", 6, message.to_string(), None))
            }
            Self::InvalidDataValue { field_name } => AER::BadRequest(ApiError::new(
                "IR",
                7,
                format!("Invalid value provided: {field_name}"), None
            )),
            Self::ClientSecretNotGiven => AER::BadRequest(ApiError::new(
                "IR",
                8,
                "client_secret was not provided", None
            )),
            Self::ClientSecretExpired => AER::BadRequest(ApiError::new(
                "IR",
                8,
                "The provided client_secret has expired", None
            )),
            Self::ClientSecretInvalid => {
                AER::BadRequest(ApiError::new("IR", 9, "The client_secret provided does not match the client_secret associated with the Payment", None))
            }
            Self::MandateActive => {
                AER::BadRequest(ApiError::new("IR", 10, "Customer has active mandate/subsciption", None))
            }
            Self::CustomerRedacted => {
                AER::BadRequest(ApiError::new("IR", 11, "Customer has already been redacted", None))
            }
            Self::MaximumRefundCount => AER::BadRequest(ApiError::new("IR", 12, "Reached maximum refund attempts", None)),
            Self::RefundAmountExceedsPaymentAmount => {
                AER::BadRequest(ApiError::new("IR", 13, "The refund amount exceeds the amount captured", None))
            }
            Self::PaymentUnexpectedState {
                current_flow,
                field_name,
                current_value,
                states,
            } => AER::BadRequest(ApiError::new("IR", 14, format!("This Payment could not be {current_flow} because it has a {field_name} of {current_value}. The expected state is {states}"), None)),
            Self::InvalidEphemeralKey => AER::Unauthorized(ApiError::new("IR", 15, "Invalid Ephemeral Key for the customer", None)),
            Self::PreconditionFailed { message } => {
                AER::BadRequest(ApiError::new("IR", 16, message.to_string(), None))
            }
            Self::InvalidJwtToken => AER::Unauthorized(ApiError::new("IR", 17, "Access forbidden, invalid JWT token was used", None)),
            Self::GenericUnauthorized { message } => {
                AER::Unauthorized(ApiError::new("IR", 18, message.to_string(), None))
            },
            Self::NotSupported { message } => {
                AER::BadRequest(ApiError::new("IR", 19, "Payment method type not supported", Some(Extra {reason: Some(message.to_owned()), ..Default::default()})))
            },
            Self::FlowNotSupported { flow, connector } => {
                AER::BadRequest(ApiError::new("IR", 20, format!("{flow} flow not supported"), Some(Extra {connector: Some(connector.to_owned()), ..Default::default()}))) //FIXME: error message
            }
            Self::MissingRequiredFields { field_names } => AER::BadRequest(
                ApiError::new("IR", 21, "Missing required params".to_string(), Some(Extra {data: Some(serde_json::json!(field_names)), ..Default::default() })),
            ),
            Self::AccessForbidden {resource} => {
                AER::ForbiddenCommonResource(ApiError::new("IR", 22, format!("Access forbidden. Not authorized to access this resource {resource}"), None))
            },
            Self::FileProviderNotSupported { message } => {
                AER::BadRequest(ApiError::new("IR", 23, message.to_string(), None))
            },
            Self::InvalidWalletToken { wallet_name} => AER::Unprocessable(ApiError::new(
                "IR",
                24,
                format!("Invalid {wallet_name} wallet token"), None
            )),
            Self::PaymentMethodDeleteFailed => {
                AER::BadRequest(ApiError::new("IR", 25, "Cannot delete the default payment method", None))
            }
            Self::InvalidCookie => {
                AER::BadRequest(ApiError::new("IR", 26, "Invalid Cookie", None))
            }
            Self::ExtendedCardInfoNotFound => {
                AER::NotFound(ApiError::new("IR", 27, "Extended card info does not exist", None))
            }
            Self::CurrencyNotSupported { message } => {
                AER::BadRequest(ApiError::new("IR", 28, message, None))
            }
            Self::UnprocessableEntity {message} => AER::Unprocessable(ApiError::new("IR", 29, message.to_string(), None)),
            Self::InvalidConnectorConfiguration {config} => {
                AER::BadRequest(ApiError::new("IR", 30, format!("Merchant connector account is configured with invalid {config}"), None))
            }
            Self::InvalidCardIin => AER::BadRequest(ApiError::new("IR", 31, "The provided card IIN does not exist", None)),
            Self::InvalidCardIinLength  => AER::BadRequest(ApiError::new("IR", 32, "The provided card IIN length is invalid, please provide an IIN with 6 digits", None)),
            Self::MissingFile => {
                AER::BadRequest(ApiError::new("IR", 33, "File not found in the request", None))
            }
            Self::MissingDisputeId => {
                AER::BadRequest(ApiError::new("IR", 34, "Dispute id not found in the request", None))
            }
            Self::MissingFilePurpose => {
                AER::BadRequest(ApiError::new("IR", 35, "File purpose not found in the request or is invalid", None))
            }
            Self::MissingFileContentType => {
                AER::BadRequest(ApiError::new("IR", 36, "File content type not found", None))
            }
            Self::GenericNotFoundError { message } => {
                AER::NotFound(ApiError::new("IR", 37, message, None))
            },
            Self::GenericDuplicateError { message } => {
                AER::BadRequest(ApiError::new("IR", 38, message, None))
            }
            Self::IncorrectPaymentMethodConfiguration => {
                AER::BadRequest(ApiError::new("IR", 39, "No eligible connector was found for the current payment method configuration", None))
            }
            Self::LinkConfigurationError { message } => {
                AER::BadRequest(ApiError::new("IR", 40, message, None))
            },
            Self::PayoutFailed { data } => {
                AER::BadRequest(ApiError::new("IR", 41, "Payout failed while processing with connector.", Some(Extra { data: data.clone(), ..Default::default()})))
            },
            Self::CookieNotFound => {
                AER::Unauthorized(ApiError::new("IR", 42, "Cookies are not found in the request", None))
            },

            Self::WebhookAuthenticationFailed => {
                AER::Unauthorized(ApiError::new("WE", 1, "Webhook authentication failed", None))
            }
            Self::WebhookBadRequest => {
                AER::BadRequest(ApiError::new("WE", 2, "Bad request body received", None))
            }
            Self::WebhookProcessingFailure => {
                AER::InternalServerError(ApiError::new("WE", 3, "There was an issue processing the webhook", None))
            },
            Self::WebhookResourceNotFound => {
                AER::NotFound(ApiError::new("WE", 4, "Webhook resource was not found", None))
            }
            Self::WebhookUnprocessableEntity => {
                AER::Unprocessable(ApiError::new("WE", 5, "There was an issue processing the webhook body", None))
            },
            Self::WebhookInvalidMerchantSecret => {
                AER::BadRequest(ApiError::new("WE", 6, "Merchant Secret set for webhook source verification is invalid", None))
            }
            Self::IntegrityCheckFailed {
                reason,
                field_names,
                connector_transaction_id
            } => AER::InternalServerError(ApiError::new(
                "IE",
                0,
                format!("{} as data mismatched for {}", reason, field_names),
                Some(Extra {
                    connector_transaction_id: connector_transaction_id.to_owned(),
                    ..Default::default()
                })
            )),
            Self::PlatformAccountAuthNotSupported => {
                AER::BadRequest(ApiError::new("IR", 43, "API does not support platform operation", None))
            }
            Self::InvalidPlatformOperation => {
                AER::Unauthorized(ApiError::new("IR", 44, "Invalid platform account operation", None))
            }
        }
    }
}

impl actix_web::ResponseError for ApiErrorResponse {
    fn status_code(&self) -> StatusCode {
        ErrorSwitch::<api_models::errors::types::ApiErrorResponse>::switch(self).status_code()
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        ErrorSwitch::<api_models::errors::types::ApiErrorResponse>::switch(self).error_response()
    }
}

impl From<ApiErrorResponse> for router_data::ErrorResponse {
    fn from(error: ApiErrorResponse) -> Self {
        Self {
            code: error.error_code(),
            message: error.error_message(),
            reason: None,
            status_code: match error {
                ApiErrorResponse::ExternalConnectorError { status_code, .. } => status_code,
                _ => 500,
            },
            attempt_status: None,
            connector_transaction_id: None,
        }
    }
}
