#![allow(unused_variables)]
use crate::core::errors::ApiErrorResponse;

#[derive(Debug, router_derive::ApiError)]
#[error(error_type_enum = StripeErrorType)]
pub(crate) enum ErrorCode {
    /*
    "error": {
        "message": "Invalid API Key provided: sk_jkjgs****nlgs",
        "type": "invalid_request_error"
    }
    */
    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "IR_01",
        message = "Invalid API Key provided"
    )]
    Unauthorized,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "IR_02", message = "Unrecognized request URL.")]
    InvalidRequestUrl,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "parameter_missing", message = "Missing required param: {field_name}.")]
    ParameterMissing { field_name: String, param: String },

    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "parameter_unknown",
        message = "{field_name} contains invalid data. Expected format is {expected_format}."
    )]
    ParameterUnknown {
        field_name: String,
        expected_format: String,
    },

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "IR_06",  message = "Refund amount exceeds the payment amount.")]
    RefundAmountExceedsPaymentAmount { param: String },

    #[error(error_type = StripeErrorType::ApiError, code = "payment_intent_authentication_failure", message = "Payment failed while processing with connector. Retry payment.")]
    PaymentIntentAuthenticationFailure { data: Option<serde_json::Value> },

    #[error(error_type = StripeErrorType::ApiError, code = "payment_intent_payment_attempt_failed", message = "Capture attempt failed while processing with connector.")]
    PaymentIntentPaymentAttemptFailed { data: Option<serde_json::Value> },

    #[error(error_type = StripeErrorType::CardError, code = "expired_card", message = "Card Expired. Please use another card")]
    ExpiredCard,

    #[error(error_type = StripeErrorType::CardError, code = "invalid_card_type", message = "Card data is invalid")]
    InvalidCardType,

    #[error(error_type = StripeErrorType::ApiError, code = "refund_failed", message = "refund has failed")]
    RefundFailed, // stripe error code

    #[error(error_type = StripeErrorType::ApiError, code = "internal_server_error", message = "Server is down")]
    InternalServerError,

    #[error(error_type = StripeErrorType::ApiError, code = "internal_server_error", message = "Server is down")]
    DuplicateRefundRequest,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such refund")]
    RefundNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such customer")]
    CustomerNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such payment")]
    PaymentNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such payment method")]
    PaymentMethodNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such merchant account")]
    MerchantAccountNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such resource ID")]
    ResourceIdNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such merchant connector account")]
    MerchantConnectorAccountNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such mandate")]
    MandateNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "parameter_missing", message = "Return url is not available")]
    ReturnUrlUnavailable,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "token_already_used", message = "duplicate merchant account")]
    DuplicateMerchantAccount,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "token_already_used", message = "duplicate merchant_connector_account")]
    DuplicateMerchantConnectorAccount,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "token_already_used", message = "duplicate payment method")]
    DuplicatePaymentMethod,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "" , message = "deserialization failed: {error_message}")]
    SerdeQsError {
        error_message: String,
        param: Option<String>,
    },

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "payment_intent_invalid_parameter" , message = "The client_secret provided does not match the client_secret associated with the PaymentIntent.")]
    PaymentIntentInvalidParameter { param: String },

    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "IR_05",
        message = "{message}"
    )]
    InvalidRequestData { message: String },

    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "IR_10",
        message = "{message}"
    )]
    PreconditionFailed { message: String },

    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "",
        message = "The payment has not succeeded yet"
    )]
    PaymentFailed,

    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "",
        message = "The verification did not succeeded"
    )]
    VerificationFailed { data: Option<serde_json::Value> },

    #[error(
        error_type = StripeErrorType::InvalidRequestError, code = "",
        message = "Reached maximum refund attempts"
    )]
    MaximumRefundCount,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "", message = "Duplicate mandate request. Mandate already attempted with the Mandate ID.")]
    DuplicateMandate,

    #[error(error_type= StripeErrorType::InvalidRequestError, code = "", message = "Successful payment not found for the given payment id")]
    SuccessfulPaymentNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "", message = "Address does not exist in our records.")]
    AddressNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "", message = "This PaymentIntent could not be {current_flow} because it has a {field_name} of {current_value}. The expected state is {states}.")]
    PaymentIntentUnexpectedState {
        current_flow: String,
        field_name: String,
        current_value: String,
        states: String,
    },
    // TODO: Some day implement all stripe error codes https://stripe.com/docs/error-codes
    // AccountCountryInvalidAddress,
    // AccountErrorCountryChangeRequiresAdditionalSteps,
    // AccountInformationMismatch,
    // AccountInvalid,
    // AccountNumberInvalid,
    // AcssDebitSessionIncomplete,
    // AlipayUpgradeRequired,
    // AmountTooLarge,
    // AmountTooSmall,
    // ApiKeyExpired,
    // AuthenticationRequired,
    // BalanceInsufficient,
    // BankAccountBadRoutingNumbers,
    // BankAccountDeclined,
    // BankAccountExists,
    // BankAccountUnusable,
    // BankAccountUnverified,
    // BankAccountVerificationFailed,
    // BillingInvalidMandate,
    // BitcoinUpgradeRequired,
    // CardDeclineRateLimitExceeded,
    // CardDeclined,
    // CardholderPhoneNumberRequired,
    // ChargeAlreadyCaptured,
    // ChargeAlreadyRefunded,
    // ChargeDisputed,
    // ChargeExceedsSourceLimit,
    // ChargeExpiredForCapture,
    // ChargeInvalidParameter,
    // ClearingCodeUnsupported,
    // CountryCodeInvalid,
    // CountryUnsupported,
    // CouponExpired,
    // CustomerMaxPaymentMethods,
    // CustomerMaxSubscriptions,
    // DebitNotAuthorized,
    // EmailInvalid,
    // ExpiredCard,
    // IdempotencyKeyInUse,
    // IncorrectAddress,
    // IncorrectCvc,
    // IncorrectNumber,
    // IncorrectZip,
    // InstantPayoutsConfigDisabled,
    // InstantPayoutsCurrencyDisabled,
    // InstantPayoutsLimitExceeded,
    // InstantPayoutsUnsupported,
    // InsufficientFunds,
    // IntentInvalidState,
    // IntentVerificationMethodMissing,
    // InvalidCardType,
    // InvalidCharacters,
    // InvalidChargeAmount,
    // InvalidCvc,
    // InvalidExpiryMonth,
    // InvalidExpiryYear,
    // InvalidNumber,
    // InvalidSourceUsage,
    // InvoiceNoCustomerLineItems,
    // InvoiceNoPaymentMethodTypes,
    // InvoiceNoSubscriptionLineItems,
    // InvoiceNotEditable,
    // InvoiceOnBehalfOfNotEditable,
    // InvoicePaymentIntentRequiresAction,
    // InvoiceUpcomingNone,
    // LivemodeMismatch,
    // LockTimeout,
    // Missing,
    // NoAccount,
    // NotAllowedOnStandardAccount,
    // OutOfInventory,
    // ParameterInvalidEmpty,
    // ParameterInvalidInteger,
    // ParameterInvalidStringBlank,
    // ParameterInvalidStringEmpty,
    // ParametersExclusive,
    // PaymentIntentActionRequired,
    // PaymentIntentIncompatiblePaymentMethod,
    // PaymentIntentInvalidParameter,
    // PaymentIntentKonbiniRejectedConfirmationNumber,
    // PaymentIntentMandateInvalid,
    // PaymentIntentPaymentAttemptExpired,
    // PaymentIntentUnexpectedState,
    // PaymentMethodBankAccountAlreadyVerified,
    // PaymentMethodBankAccountBlocked,
    // PaymentMethodBillingDetailsAddressMissing,
    // PaymentMethodCurrencyMismatch,
    // PaymentMethodInvalidParameter,
    // PaymentMethodInvalidParameterTestmode,
    // PaymentMethodMicrodepositFailed,
    // PaymentMethodMicrodepositVerificationAmountsInvalid,
    // PaymentMethodMicrodepositVerificationAmountsMismatch,
    // PaymentMethodMicrodepositVerificationAttemptsExceeded,
    // PaymentMethodMicrodepositVerificationDescriptorCodeMismatch,
    // PaymentMethodMicrodepositVerificationTimeout,
    // PaymentMethodProviderDecline,
    // PaymentMethodProviderTimeout,
    // PaymentMethodUnactivated,
    // PaymentMethodUnexpectedState,
    // PaymentMethodUnsupportedType,
    // PayoutsNotAllowed,
    // PlatformAccountRequired,
    // PlatformApiKeyExpired,
    // PostalCodeInvalid,
    // ProcessingError,
    // ProductInactive,
    // RateLimit,
    // ReferToCustomer,
    // RefundDisputedPayment,
    // ResourceAlreadyExists,
    // ResourceMissing,
    // ReturnIntentAlreadyProcessed,
    // RoutingNumberInvalid,
    // SecretKeyRequired,
    // SepaUnsupportedAccount,
    // SetupAttemptFailed,
    // SetupIntentAuthenticationFailure,
    // SetupIntentInvalidParameter,
    // SetupIntentSetupAttemptExpired,
    // SetupIntentUnexpectedState,
    // ShippingCalculationFailed,
    // SkuInactive,
    // StateUnsupported,
    // StatusTransitionInvalid,
    // TaxIdInvalid,
    // TaxesCalculationFailed,
    // TerminalLocationCountryUnsupported,
    // TestmodeChargesOnly,
    // TlsVersionUnsupported,
    // TokenInUse,
    // TransferSourceBalanceParametersMismatch,
    // TransfersNotAllowed,
}

impl ::core::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{\"error\": {}}}",
            serde_json::to_string(self).unwrap_or_else(|_| "API error response".to_string())
        )
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub(crate) enum StripeErrorType {
    ApiError,
    CardError,
    InvalidRequestError,
}

impl From<ApiErrorResponse> for ErrorCode {
    fn from(value: ApiErrorResponse) -> Self {
        match value {
            ApiErrorResponse::Unauthorized | ApiErrorResponse::InvalidEphermeralKey => {
                ErrorCode::Unauthorized
            }
            ApiErrorResponse::InvalidRequestUrl | ApiErrorResponse::InvalidHttpMethod => {
                ErrorCode::InvalidRequestUrl
            }
            ApiErrorResponse::MissingRequiredField { field_name } => ErrorCode::ParameterMissing {
                field_name: field_name.to_owned(),
                param: field_name,
            },
            // parameter unknown, invalid request error // actually if we type wrong values in address we get this error. Stripe throws parameter unknown. I don't know if stripe is validating email and stuff
            ApiErrorResponse::InvalidDataFormat {
                field_name,
                expected_format,
            } => ErrorCode::ParameterUnknown {
                field_name,
                expected_format,
            },
            ApiErrorResponse::RefundAmountExceedsPaymentAmount => {
                ErrorCode::RefundAmountExceedsPaymentAmount {
                    param: "amount".to_owned(),
                }
            }
            ApiErrorResponse::PaymentAuthorizationFailed { data }
            | ApiErrorResponse::PaymentAuthenticationFailed { data } => {
                ErrorCode::PaymentIntentAuthenticationFailure { data }
            }
            ApiErrorResponse::VerificationFailed { data } => ErrorCode::VerificationFailed { data },
            ApiErrorResponse::PaymentCaptureFailed { data } => {
                ErrorCode::PaymentIntentPaymentAttemptFailed { data }
            }
            ApiErrorResponse::InvalidCardData { data } => ErrorCode::InvalidCardType, // Maybe it is better to de generalize this router error
            ApiErrorResponse::CardExpired { data } => ErrorCode::ExpiredCard,
            ApiErrorResponse::RefundFailed { data } => ErrorCode::RefundFailed, // Nothing at stripe to map

            ApiErrorResponse::InternalServerError => ErrorCode::InternalServerError, // not a stripe code
            ApiErrorResponse::DuplicateRefundRequest => ErrorCode::DuplicateRefundRequest,
            ApiErrorResponse::RefundNotFound => ErrorCode::RefundNotFound,
            ApiErrorResponse::CustomerNotFound => ErrorCode::CustomerNotFound,
            ApiErrorResponse::PaymentNotFound => ErrorCode::PaymentNotFound,
            ApiErrorResponse::PaymentMethodNotFound => ErrorCode::PaymentMethodNotFound,
            ApiErrorResponse::MerchantAccountNotFound => ErrorCode::MerchantAccountNotFound,
            ApiErrorResponse::ResourceIdNotFound => ErrorCode::ResourceIdNotFound,
            ApiErrorResponse::MerchantConnectorAccountNotFound => {
                ErrorCode::MerchantConnectorAccountNotFound
            }
            ApiErrorResponse::MandateNotFound => ErrorCode::MandateNotFound,
            ApiErrorResponse::ReturnUrlUnavailable => ErrorCode::ReturnUrlUnavailable,
            ApiErrorResponse::DuplicateMerchantAccount => ErrorCode::DuplicateMerchantAccount,
            ApiErrorResponse::DuplicateMerchantConnectorAccount => {
                ErrorCode::DuplicateMerchantConnectorAccount
            }
            ApiErrorResponse::DuplicatePaymentMethod => ErrorCode::DuplicatePaymentMethod,
            ApiErrorResponse::ClientSecretInvalid => ErrorCode::PaymentIntentInvalidParameter {
                param: "client_secret".to_owned(),
            },
            ApiErrorResponse::InvalidRequestData { message } => {
                ErrorCode::InvalidRequestData { message }
            }
            ApiErrorResponse::PreconditionFailed { message } => {
                ErrorCode::PreconditionFailed { message }
            }
            ApiErrorResponse::BadCredentials => ErrorCode::Unauthorized,
            ApiErrorResponse::InvalidDataValue { field_name } => ErrorCode::ParameterMissing {
                field_name: field_name.to_owned(),
                param: field_name.to_owned(),
            },
            ApiErrorResponse::MaximumRefundCount => ErrorCode::MaximumRefundCount,
            ApiErrorResponse::PaymentNotSucceeded => ErrorCode::PaymentFailed,
            ApiErrorResponse::DuplicateMandate => ErrorCode::DuplicateMandate,
            ApiErrorResponse::SuccessfulPaymentNotFound => ErrorCode::SuccessfulPaymentNotFound,
            ApiErrorResponse::AddressNotFound => ErrorCode::AddressNotFound,
            ApiErrorResponse::NotImplemented => ErrorCode::Unauthorized,
            ApiErrorResponse::PaymentUnexpectedState {
                current_flow,
                field_name,
                current_value,
                states,
            } => ErrorCode::PaymentIntentUnexpectedState {
                current_flow,
                field_name,
                current_value,
                states,
            },
        }
    }
}

impl actix_web::ResponseError for ErrorCode {
    fn status_code(&self) -> reqwest::StatusCode {
        use reqwest::StatusCode;

        match self {
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::InvalidRequestUrl => StatusCode::NOT_FOUND,
            ErrorCode::ParameterUnknown { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::ParameterMissing { .. }
            | ErrorCode::RefundAmountExceedsPaymentAmount { .. }
            | ErrorCode::PaymentIntentAuthenticationFailure { .. }
            | ErrorCode::PaymentIntentPaymentAttemptFailed { .. }
            | ErrorCode::ExpiredCard
            | ErrorCode::InvalidCardType
            | ErrorCode::DuplicateRefundRequest
            | ErrorCode::RefundNotFound
            | ErrorCode::CustomerNotFound
            | ErrorCode::PaymentNotFound
            | ErrorCode::PaymentMethodNotFound
            | ErrorCode::MerchantAccountNotFound
            | ErrorCode::MerchantConnectorAccountNotFound
            | ErrorCode::MandateNotFound
            | ErrorCode::DuplicateMerchantAccount
            | ErrorCode::DuplicateMerchantConnectorAccount
            | ErrorCode::DuplicatePaymentMethod
            | ErrorCode::PaymentFailed
            | ErrorCode::VerificationFailed { .. }
            | ErrorCode::MaximumRefundCount
            | ErrorCode::PaymentIntentInvalidParameter { .. }
            | ErrorCode::SerdeQsError { .. }
            | ErrorCode::InvalidRequestData { .. }
            | ErrorCode::PreconditionFailed { .. }
            | ErrorCode::DuplicateMandate
            | ErrorCode::SuccessfulPaymentNotFound
            | ErrorCode::AddressNotFound
            | ErrorCode::ResourceIdNotFound
            | ErrorCode::PaymentIntentUnexpectedState { .. } => StatusCode::BAD_REQUEST,
            ErrorCode::RefundFailed | ErrorCode::InternalServerError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ErrorCode::ReturnUrlUnavailable => StatusCode::SERVICE_UNAVAILABLE,
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

impl From<serde_qs::Error> for ErrorCode {
    fn from(item: serde_qs::Error) -> Self {
        match item {
            serde_qs::Error::Custom(s) => ErrorCode::SerdeQsError {
                error_message: s,
                param: None,
            },
            serde_qs::Error::Parse(param, position) => ErrorCode::SerdeQsError {
                error_message: format!(
                    "parsing failed with error: '{param}' at position: {position}"
                ),
                param: Some(param),
            },
            serde_qs::Error::Unsupported => ErrorCode::SerdeQsError {
                error_message: "Given request format is not supported".to_owned(),
                param: None,
            },
            serde_qs::Error::FromUtf8(_) => ErrorCode::SerdeQsError {
                error_message: "Failed to parse request to from utf-8".to_owned(),
                param: None,
            },
            serde_qs::Error::Io(_) => ErrorCode::SerdeQsError {
                error_message: "Failed to parse request".to_owned(),
                param: None,
            },
            serde_qs::Error::ParseInt(_) => ErrorCode::SerdeQsError {
                error_message: "Failed to parse integer in request".to_owned(),
                param: None,
            },
            serde_qs::Error::Utf8(_) => ErrorCode::SerdeQsError {
                error_message: "Failed to convert utf8 to string".to_owned(),
                param: None,
            },
        }
    }
}
