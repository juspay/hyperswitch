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
        message = "API key not provided. Provide API key in the Authorization header, using api-key (e.g api-key: API_KEY)."
    )]
    Unauthorized,
    #[error(
        error_type = ErrorType::InvalidRequestError, code = "IR_02",
        message = "Access forbidden, invalid API key was used. Please create your new API key from \
                    the Dashboard Settings section."
    )]
    BadCredentials,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_10", message = "Invalid Ephemeral Key for the customer")]
    InvalidEphermeralKey,
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
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_08", message = "Refund amount exceeds the payment amount.")]
    RefundAmountExceedsPaymentAmount,
    /// Typically used when a field has invalid value, or deserialization of the value contained in
    /// a field fails.
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "Invalid value provided: {field_name}.")]
    InvalidDataValue { field_name: &'static str },
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_08", message = "Reached maximum refund attempts")]
    MaximumRefundCount,
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_09", message = "This PaymentIntent could not be {current_flow} because it has a {field_name} of {current_value}. The expected state is {states}.")]
    PaymentUnexpectedState {
        current_flow: String,
        field_name: String,
        current_value: String,
        states: String,
    },
    /// Typically used when information involving multiple fields or previously provided
    /// information doesn't satisfy a condition.
    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_10", message = "{message}")]
    PreconditionFailed { message: String },

    #[error(error_type = ErrorType::InvalidRequestError, code = "IR_07", message = "The client_secret provided does not match the client_secret associated with the Payment.")]
    ClientSecretInvalid,

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
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The merchant account with the specified details already exists in our records.")]
    DuplicateMerchantAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The merchant connector account with the specified details already exists in our records.")]
    DuplicateMerchantConnectorAccount,
    #[error(error_type = ErrorType::DuplicateRequest, code = "RE_04", message = "The payment method with the specified details already exists in our records.")]
    DuplicatePaymentMethod,
    #[error(error_type= ErrorType::InvalidRequestError, code = "RE_05", message = "The payment has not succeeded yet")]
    PaymentNotSucceeded,
    #[error(error_type= ErrorType::ObjectNotFound, code = "RE_05", message = "Successful payment not found for the given payment id")]
    SuccessfulPaymentNotFound,
    #[error(error_type = ErrorType::ObjectNotFound, code = "RE_05", message = "Address does not exist in our records.")]
    AddressNotFound,
    #[error(error_type = ErrorType::ServerNotAvailable, code = "IR_00", message = "This API is under development and will be made available soon.")]
    NotImplemented,
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
            ApiErrorResponse::Unauthorized
            | ApiErrorResponse::BadCredentials
            | ApiErrorResponse::InvalidEphermeralKey => StatusCode::UNAUTHORIZED, // 401
            ApiErrorResponse::InvalidRequestUrl => StatusCode::NOT_FOUND, // 404
            ApiErrorResponse::InvalidHttpMethod => StatusCode::METHOD_NOT_ALLOWED, // 405
            ApiErrorResponse::MissingRequiredField { .. }
            | ApiErrorResponse::InvalidDataValue { .. } => StatusCode::BAD_REQUEST, // 400
            ApiErrorResponse::InvalidDataFormat { .. }
            | ApiErrorResponse::InvalidRequestData { .. } => StatusCode::UNPROCESSABLE_ENTITY, // 422
            ApiErrorResponse::RefundAmountExceedsPaymentAmount => StatusCode::BAD_REQUEST, // 400
            ApiErrorResponse::MaximumRefundCount => StatusCode::BAD_REQUEST,               // 400
            ApiErrorResponse::PreconditionFailed { .. } => StatusCode::BAD_REQUEST,        // 400

            ApiErrorResponse::PaymentAuthorizationFailed { .. }
            | ApiErrorResponse::PaymentAuthenticationFailed { .. }
            | ApiErrorResponse::PaymentCaptureFailed { .. }
            | ApiErrorResponse::InvalidCardData { .. }
            | ApiErrorResponse::CardExpired { .. }
            | ApiErrorResponse::RefundFailed { .. }
            | ApiErrorResponse::VerificationFailed { .. }
            | ApiErrorResponse::PaymentUnexpectedState { .. } => StatusCode::BAD_REQUEST, // 400

            ApiErrorResponse::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR, // 500
            ApiErrorResponse::DuplicateRefundRequest => StatusCode::BAD_REQUEST,        // 400
            ApiErrorResponse::RefundNotFound
            | ApiErrorResponse::CustomerNotFound
            | ApiErrorResponse::PaymentNotFound
            | ApiErrorResponse::PaymentMethodNotFound
            | ApiErrorResponse::MerchantAccountNotFound
            | ApiErrorResponse::MerchantConnectorAccountNotFound
            | ApiErrorResponse::MandateNotFound
            | ApiErrorResponse::ClientSecretInvalid
            | ApiErrorResponse::SuccessfulPaymentNotFound
            | ApiErrorResponse::ResourceIdNotFound
            | ApiErrorResponse::AddressNotFound => StatusCode::BAD_REQUEST, // 400
            ApiErrorResponse::DuplicateMerchantAccount
            | ApiErrorResponse::DuplicateMerchantConnectorAccount
            | ApiErrorResponse::DuplicatePaymentMethod
            | ApiErrorResponse::DuplicateMandate => StatusCode::BAD_REQUEST, // 400
            ApiErrorResponse::ReturnUrlUnavailable => StatusCode::SERVICE_UNAVAILABLE,  // 503
            ApiErrorResponse::PaymentNotSucceeded => StatusCode::BAD_REQUEST,           // 400
            ApiErrorResponse::NotImplemented => StatusCode::NOT_IMPLEMENTED,            // 501
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
