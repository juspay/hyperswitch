#![allow(dead_code, unused_variables)]

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
}

#[allow(dead_code)]
#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = ErrorType)]
pub enum ApiErrorResponse {
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_01",
        message = "API key not provided or invalid API key used. Provide API key in the Authorization header or create new API key, using api-key (e.g api-key: API_KEY)."
    )]
    Unauthorized,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_03", message = "Unrecognized request URL.")]
    InvalidRequestUrl,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_04", message = "The HTTP method is not applicable for this API.")]
    InvalidHttpMethod,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_05", message = "Missing required param: {field_name}.")]
    MissingRequiredField { field_name: String },
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_06",
        message = "{field_name} contains invalid data. Expected format is {expected_format}."
    )]
    InvalidDataFormat {
        field_name: String,
        expected_format: String,
    },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "{message}")]
    InvalidRequestData { message: String },
    /// Typically used when a field has invalid value, or deserialization of the value contained in
    /// a field fails.
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "Invalid value provided: {field_name}.")]
    InvalidDataValue { field_name: &'static str },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "Client secret was not provided")]
    ClientSecretNotGiven,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "The client_secret provided does not match the client_secret associated with the Payment.")]
    ClientSecretInvalid,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "Customer has existing mandate/subsciption.")]
    MandateActive,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "Customer has already redacted.")]
    CustomerRedacted,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_08", message = "Reached maximum refund attempts")]
    MaximumRefundCount,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_08", message = "Refund amount exceeds the payment amount.")]
    RefundAmountExceedsPaymentAmount,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_09", message = "This PaymentIntent could not be {current_flow} because it has a {field_name} of {current_value}. The expected state is {states}.")]
    PaymentUnexpectedState {
        current_flow: String,
        field_name: String,
        current_value: String,
        states: String,
    },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_10", message = "Invalid Ephemeral Key for the customer")]
    InvalidEphermeralKey,
    /// Typically used when information involving multiple fields or previously provided
    /// information doesn't satisfy a condition.
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_10", message = "{message}")]
    PreconditionFailed { message: String },
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_11",
        message = "Access forbidden, invalid JWT token was used."
    )]
    InvalidJwtToken,

    #[error(error_type = ErrorType::ProcessingError, code = "CE_01", message = "Payment failed while processing with connector. Retry payment.")]
    PaymentAuthorizationFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_02", message = "Payment failed while processing with connector. Retry payment.")]
    PaymentAuthenticationFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_03", message = "Capture attempt failed while processing with connector.")]
    PaymentCaptureFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_04", message = "Capture attempt failed while processing with connector.")]
    InvalidCardData { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_05", message = "Payment failed while processing with connector. Retry payment.")]
    CardExpired { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_06", message = "Refund failed while processing with connector. Retry refund.")]
    RefundFailed { data: Option<serde_json::Value> },
    #[error(error_type = ErrorType::ProcessingError, code = "CE_01", message = "Verification failed while processing with connector. Retry operation.")]
    VerificationFailed { data: Option<serde_json::Value> },

    #[error(error_type = ErrorType::ServerNotAvailable, code = "RE_00", message = "Something went wrong.")]
    InternalServerError,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_01", message = "Duplicate refund request. Refund already attempted with the refund ID.")]
    DuplicateRefundRequest,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Refund does not exist in our records.")]
    RefundNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Customer does not exist in our records.")]
    CustomerNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Payment does not exist in our records.")]
    PaymentNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Payment method does not exist in our records.")]
    PaymentMethodNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Merchant account does not exist in our records.")]
    MerchantAccountNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Merchant connector account does not exist in our records.")]
    MerchantConnectorAccountNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Resource ID does not exist in our records.")]
    ResourceIdNotFound,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_01", message = "Duplicate mandate request. Mandate already attempted with the Mandate ID.")]
    DuplicateMandate,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_02", message = "Mandate does not exist in our records.")]
    MandateNotFound,
    #[error(error_type = ErrorType::ValidationError, code = "RE_03", message = "Return URL is not configured and not passed in payments request.")]
    ReturnUrlUnavailable,
    #[error(error_type = ErrorType::ValidationError, code = "RE_03", message = "Refunds not possible through hyperswitch. Please raise Refunds through {connector} dashboard")]
    RefundNotPossible { connector: String },
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The merchant account with the specified details already exists in our records.")]
    DuplicateMerchantAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The merchant connector account with the specified details already exists in our records.")]
    DuplicateMerchantConnectorAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The payment method with the specified details already exists in our records.")]
    DuplicatePaymentMethod,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The payment with the specified payment_id '{payment_id}' already exists in our records.")]
    DuplicatePayment { payment_id: String },
    #[error(error_type= ErrorType::InvalidRequestError, code = "RE_05", message = "The payment has not succeeded yet")]
    PaymentNotSucceeded,
    #[error(error_type= ErrorType::ObjectNotFound, code = "RE_05", message = "Successful payment not found for the given payment id")]
    SuccessfulPaymentNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_05", message = "The connector provided in the request is incorrect or not available")]
    IncorrectConnectorNameGiven,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_05", message = "Address does not exist in our records.")]
    AddressNotFound,
    #[error(error_type = ErrorType::ValidationError, code = "RE_03", message = "Mandate Validation Failed" )]
    MandateValidationFailed { reason: String },
    #[error(error_type = ErrorType::ServerNotAvailable, code = "IR_00", message = "{message:?}")]
    NotImplemented { message: NotImplementedMessage },
}

#[derive(Clone)]
pub enum NotImplementedMessage {
    Reason(String),
    Default,
}

impl std::fmt::Debug for NotImplementedMessage {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Reason(m) => format!("{} is not implemented", m),
            Self::Default => {
                "This API is under development and will be made available soon.".to_string()
            }
        };
        write!(fmt, "{message}")
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
    fn status_code(&self) -> reqwest::StatusCode {
        use reqwest::StatusCode;

        match self {
            Self::Unauthorized | Self::InvalidEphermeralKey | Self::InvalidJwtToken => {
                StatusCode::UNAUTHORIZED
            } // 401
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
            | Self::AddressNotFound => StatusCode::BAD_REQUEST, // 400
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
