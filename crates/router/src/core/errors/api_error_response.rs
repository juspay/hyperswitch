#![allow(dead_code, unused_variables)]

use http::StatusCode;

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

    #[error(error_type = ErrorType::ConnectorError, code = "CE_00", message = "{code}: {message}", ignore = "status_code")]
    ExternalConnectorError {
        code: String,
        message: String,
        connector: String,
        status_code: u16,
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

    #[error(error_type = ErrorType::ServerNotAvailable, code = "HE_00", message = "Something went wrong")]
    InternalServerError,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "Duplicate refund request. Refund already attempted with the refund ID")]
    DuplicateRefundRequest,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "Duplicate mandate request. Mandate already attempted with the Mandate ID")]
    DuplicateMandate,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The merchant account with the specified details already exists in our records")]
    DuplicateMerchantAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The merchant connector account with the specified details already exists in our records")]
    DuplicateMerchantConnectorAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payment method with the specified details already exists in our records")]
    DuplicatePaymentMethod,
    #[error(error_type = ErrorType::DuplicateRequest, code = "HE_01", message = "The payment with the specified payment_id '{payment_id}' already exists in our records")]
    DuplicatePayment { payment_id: String },
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
    MerchantConnectorAccountNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Resource ID does not exist in our records")]
    ResourceIdNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "Mandate does not exist in our records")]
    MandateNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_02", message = "API Key does not exist in our records")]
    ApiKeyNotFound,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Return URL is not configured and not passed in payments request")]
    ReturnUrlUnavailable,
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "This refund is not possible through Hyperswitch. Please raise the refund through {connector} dashboard")]
    RefundNotPossible { connector: String },
    #[error(error_type = ErrorType::ValidationError, code = "HE_03", message = "Mandate Validation Failed" )]
    MandateValidationFailed { reason: String },
    #[error(error_type= ErrorType::ValidationError, code = "HE_03", message = "The payment has not succeeded yet. Please pass a successful payment to initiate refund")]
    PaymentNotSucceeded,
    #[error(error_type= ErrorType::ObjectNotFound, code = "HE_04", message = "Successful payment not found for the given payment id")]
    SuccessfulPaymentNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "The connector provided in the request is incorrect or not available")]
    IncorrectConnectorNameGiven,
    #[error(error_type = ErrorType::ObjectNotFound, code = "HE_04", message = "Address does not exist in our records")]
    AddressNotFound,
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
        match self {
            Self::Unauthorized
            | Self::InvalidEphemeralKey
            | Self::InvalidJwtToken
            | Self::GenericUnauthorized { .. } => StatusCode::UNAUTHORIZED, // 401
            Self::ExternalConnectorError { status_code, .. } => {
                StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::InvalidRequestUrl => StatusCode::NOT_FOUND, // 404
            Self::InvalidHttpMethod => StatusCode::METHOD_NOT_ALLOWED, // 405
            Self::MissingRequiredField { .. } | Self::InvalidDataValue { .. } => {
                StatusCode::BAD_REQUEST
            } // 400
            Self::InvalidDataFormat { .. } | Self::InvalidRequestData { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            } // 422
            Self::RefundAmountExceedsPaymentAmount => StatusCode::BAD_REQUEST, // 400
            Self::MaximumRefundCount => StatusCode::BAD_REQUEST, // 400
            Self::PreconditionFailed { .. } => StatusCode::BAD_REQUEST, // 400

            Self::PaymentAuthorizationFailed { .. }
            | Self::PaymentAuthenticationFailed { .. }
            | Self::PaymentCaptureFailed { .. }
            | Self::InvalidCardData { .. }
            | Self::CardExpired { .. }
            | Self::RefundFailed { .. }
            | Self::RefundNotPossible { .. }
            | Self::VerificationFailed { .. }
            | Self::PaymentUnexpectedState { .. }
            | Self::MandateValidationFailed { .. } => StatusCode::BAD_REQUEST, // 400

            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR, // 500
            Self::DuplicateRefundRequest | Self::DuplicatePayment { .. } => StatusCode::BAD_REQUEST, // 400
            Self::RefundNotFound
            | Self::CustomerNotFound
            | Self::MandateActive
            | Self::CustomerRedacted
            | Self::PaymentNotFound
            | Self::PaymentMethodNotFound
            | Self::MerchantAccountNotFound
            | Self::MerchantConnectorAccountNotFound
            | Self::MandateNotFound
            | Self::ClientSecretNotGiven
            | Self::ClientSecretInvalid
            | Self::SuccessfulPaymentNotFound
            | Self::IncorrectConnectorNameGiven
            | Self::ResourceIdNotFound
            | Self::ConfigNotFound
            | Self::AddressNotFound
            | Self::ApiKeyNotFound => StatusCode::BAD_REQUEST, // 400
            Self::DuplicateMerchantAccount
            | Self::DuplicateMerchantConnectorAccount
            | Self::DuplicatePaymentMethod
            | Self::DuplicateMandate => StatusCode::BAD_REQUEST, // 400
            Self::ReturnUrlUnavailable => StatusCode::SERVICE_UNAVAILABLE, // 503
            Self::PaymentNotSucceeded => StatusCode::BAD_REQUEST,          // 400
            Self::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,    // 501
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::http::header;

        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .insert_header((header::VIA, "Juspay_Router"))
            .body(self.to_string())
    }
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse>
    for ApiErrorResponse
{
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};

        let error_message = self.error_message();
        let error_codes = self.error_code();
        let error_type = self.error_type();

        match self {
            Self::NotImplemented { message } => {
                AER::NotImplemented(ApiError::new("IR", 0, format!("{message:?}")))
            }
            Self::Unauthorized => AER::Unauthorized(ApiError::new(
                "IR",
                0,
                "API key not provided or invalid API key used",
            )),
            Self::InvalidRequestUrl => {
                AER::NotFound(ApiError::new("IR", 0, "Unrecognized request URL"))
            }
            Self::InvalidHttpMethod => AER::MethodNotAllowed(ApiError::new(
                "IR",
                0,
                "The HTTP method is not applicable for this API",
            )),
            Self::MissingRequiredField { field_name } => AER::BadRequest(
                ApiError::new("IR", 0, format!("Missing required param: {field_name}")),
            ),
            Self::InvalidDataFormat {
                field_name,
                expected_format,
            } => AER::Unprocessable(ApiError::new(
                "IR",
                0,
                format!(
                    "{field_name} contains invalid data. Expected format is {expected_format}"
                ),
            )),
            Self::InvalidRequestData { message } => {
                AER::Unprocessable(ApiError::new("IR", 0, message.to_string()))
            }
            Self::InvalidDataValue { field_name } => AER::BadRequest(ApiError::new(
                "IR",
                0,
                format!("Invalid value provided: {field_name}"),
            )),
            Self::ClientSecretNotGiven => AER::ForbiddenPrivateResource(ApiError::new(
                "IR",
                0,
                "Client secret was not provided",
            )),
            Self::ClientSecretInvalid => {
                AER::ForbiddenPrivateResource(ApiError::new("IR", 0, "The client_secret provided does not match the client_secret associated with the Payment"))
            }
            Self::MandateActive => {
                AER::ForbiddenPrivateResource(ApiError::new("IR", 0, "Customer has active mandate/subsciption"))
            }
            Self::CustomerRedacted => {
                AER::ForbiddenPrivateResource(ApiError::new("IR", 0, "Customer has already been redacted"))
            }
            Self::MaximumRefundCount => AER::BadRequest(ApiError::new("IR", 0, "Reached maximum refund attempts")),
            Self::RefundAmountExceedsPaymentAmount => {
                AER::BadRequest(ApiError::new("IR", 0, "Refund amount exceeds the payment amount"))
            }
            Self::PaymentUnexpectedState {
                current_flow,
                field_name,
                current_value,
                states,
            } => AER::BadRequest(ApiError::new("IR", 0, format!("This Payment could not be {current_flow} because it has a {field_name} of {current_value}. The expected state is {states}"))),
            Self::InvalidEphemeralKey => AER::Unauthorized(ApiError::new("IR", 0, "Invalid Ephemeral Key for the customer")),
            Self::PreconditionFailed { message } => {
                AER::BadRequest(ApiError::new("IR", 0, message.to_string()))
            }
            Self::InvalidJwtToken => AER::Unauthorized(ApiError::new("IR", 0, "Access forbidden, invalid JWT token was used")),
            Self::GenericUnauthorized { message } => {
                AER::Unauthorized(ApiError::new("IR", 0, message.to_string()))
            }
            Self::ExternalConnectorError {
                code,
                message,
                connector,
                status_code,
            } => AER::ConnectorError(ApiError::new("CE", 0, format!("{code}: {message}")), StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)),
            Self::PaymentAuthorizationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 0, "Payment failed during authorization with connector. Retry payment"))
            }
            Self::PaymentAuthenticationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 0, "Payment failed during authentication with connector. Retry payment"))
            }
            Self::PaymentCaptureFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 0, "Capture attempt failed while processing with connector"))
            }
            Self::InvalidCardData { data } => AER::BadRequest(ApiError::new("CE", 0, "The card data is invalid")),
            Self::CardExpired { data } => AER::BadRequest(ApiError::new("CE", 0, "The card has expired")),
            Self::RefundFailed { data } => AER::BadRequest(ApiError::new("CE", 0, "Refund failed while processing with connector. Retry refund")),
            Self::VerificationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 0, "Verification failed while processing with connector. Retry operation"))
            }
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, "Something went wrong"))
            }
            Self::DuplicateRefundRequest => AER::BadRequest(ApiError::new("HE", 0, "Duplicate refund request. Refund already attempted with the refund ID")),
            Self::DuplicateMandate => AER::BadRequest(ApiError::new("HE", 0, "Duplicate mandate request. Mandate already attempted with the Mandate ID")),
            Self::DuplicateMerchantAccount => AER::BadRequest(ApiError::new("HE", 0, "The merchant account with the specified details already exists in our records")),
            Self::DuplicateMerchantConnectorAccount => {
                AER::BadRequest(ApiError::new("HE", 0, "The merchant connector account with the specified details already exists in our records"))
            }
            Self::DuplicatePaymentMethod => AER::BadRequest(ApiError::new("HE", 0, "The payment method with the specified details already exists in our records")),
            Self::DuplicatePayment { payment_id } => {
                AER::BadRequest(ApiError::new("HE", 0, format!("The payment with the specified payment_id '{payment_id}' already exists in our records")))
            }
            Self::RefundNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Refund does not exist in our records."))
            }
            Self::CustomerNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Customer does not exist in our records"))
            }
            Self::ConfigNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Config key does not exist in our records."))
            }
            Self::PaymentNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Payment does not exist in our records"))
            }
            Self::PaymentMethodNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Payment method does not exist in our records"))
            }
            Self::MerchantAccountNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Merchant account does not exist in our records"))
            }
            Self::MerchantConnectorAccountNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Merchant connector account does not exist in our records"))
            }
            Self::ResourceIdNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Resource ID does not exist in our records"))
            }
            Self::MandateNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Mandate does not exist in our records"))
            }
            Self::ReturnUrlUnavailable => AER::NotFound(ApiError::new("HE", 0, "Return URL is not configured and not passed in payments request")),
            Self::RefundNotPossible { connector } => {
                AER::BadRequest(ApiError::new("HE", 0, "This refund is not possible through Hyperswitch. Please raise the refund through {connector} dashboard"))
            }
            Self::MandateValidationFailed { reason } => {
                AER::BadRequest(ApiError::new("HE", 0, "Mandate Validation Failed"))
            }
            Self::PaymentNotSucceeded => AER::BadRequest(ApiError::new("HE", 0, "The payment has not succeeded yet. Please pass a successful payment to initiate refund")),
            Self::SuccessfulPaymentNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Successful payment not found for the given payment id"))
            }
            Self::IncorrectConnectorNameGiven => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "The connector provided in the request is incorrect or not available"))
            }
            Self::AddressNotFound => {
                AER::ForbiddenPrivateResource(ApiError::new("HE", 0, "Address does not exist in our records"))
            }
        }

        // match self {
        //     Self::Unauthorized
        //     | Self::InvalidEphemeralKey
        //     | Self::InvalidJwtToken
        //     | Self::GenericUnauthorized { .. } => AER::Unauthorized(ApiError {
        //         sub_code: error_codes,
        //         error_identifier: 0,
        //         error_message: error_message,
        //     }), // 401
        //     Self::ExternalConnectorError { status_code, .. } => AER::ConnectorError(
        //         ApiError {
        //             error_identifier: 0,
        //             sub_code: error_codes,
        //             error_message: error_message,
        //         },
        //         StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        //     ),
        //     Self::InvalidRequestUrl => AER::NotFound(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 404
        //     Self::InvalidHttpMethod => AER::MethodNotAllowed(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 405
        //     Self::MissingRequiredField { .. }
        //     | Self::InvalidDataValue { .. }
        //     | Self::RefundAmountExceedsPaymentAmount
        //     | Self::PreconditionFailed { .. }
        //     | Self::MaximumRefundCount => AER::BadRequest(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }),
        //     Self::InvalidDataFormat { .. } | Self::InvalidRequestData { .. } => {
        //         AER::Unprocessable(ApiError {
        //             error_identifier: 0,
        //             error_message: error_message,
        //             sub_code: error_codes,
        //         })
        //     } // 422

        //     Self::PaymentAuthorizationFailed { .. }
        //     | Self::PaymentAuthenticationFailed { .. }
        //     | Self::PaymentCaptureFailed { .. }
        //     | Self::InvalidCardData { .. }
        //     | Self::CardExpired { .. }
        //     | Self::RefundFailed { .. }
        //     | Self::RefundNotPossible { .. }
        //     | Self::VerificationFailed { .. }
        //     | Self::PaymentUnexpectedState { .. }
        //     | Self::MandateValidationFailed { .. }
        //     | Self::DuplicateRefundRequest
        //     | Self::DuplicatePayment { .. } => AER::BadRequest(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 400

        //     Self::InternalServerError => AER::InternalServerError(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }),
        //     Self::RefundNotFound
        //     | Self::CustomerNotFound
        //     | Self::MandateActive
        //     | Self::CustomerRedacted
        //     | Self::PaymentNotFound
        //     | Self::PaymentMethodNotFound
        //     | Self::MerchantAccountNotFound
        //     | Self::MerchantConnectorAccountNotFound
        //     | Self::MandateNotFound
        //     | Self::ClientSecretNotGiven
        //     | Self::ClientSecretInvalid
        //     | Self::SuccessfulPaymentNotFound
        //     | Self::IncorrectConnectorNameGiven
        //     | Self::ResourceIdNotFound
        //     | Self::ConfigNotFound
        //     | Self::AddressNotFound => AER::ForbiddenPrivateResource(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 400
        //     Self::DuplicateMerchantAccount
        //     | Self::DuplicateMerchantConnectorAccount
        //     | Self::DuplicatePaymentMethod
        //     | Self::DuplicateMandate => AER::BadRequest(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 400
        //     Self::ReturnUrlUnavailable => AER::NotFound(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 503
        //     Self::PaymentNotSucceeded => AER::BadRequest(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 400
        //     Self::NotImplemented { .. } => AER::NotImplemented(ApiError {
        //         error_identifier: 0,
        //         error_message: error_message,
        //         sub_code: error_codes,
        //     }), // 501
        // }
    }
}
