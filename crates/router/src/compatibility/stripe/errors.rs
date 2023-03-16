#![allow(unused_variables)]
use crate::core::errors;

#[derive(Debug, router_derive::ApiError, Clone)]
#[error(error_type_enum = StripeErrorType)]
pub enum StripeErrorCode {
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
    ParameterMissing {
        field_name: &'static str,
        param: &'static str,
    },

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

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "active_mandate", message = "Customer has active mandate")]
    MandateActive,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "customer_redacted", message = "Customer has redacted")]
    CustomerRedacted,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such refund")]
    RefundNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "client_secret_invalid", message = "Expected client secret to be included in the request")]
    ClientSecretNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such customer")]
    CustomerNotFound,

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such config")]
    ConfigNotFound,

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

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "resource_missing", message = "No such API key")]
    ApiKeyNotFound,

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
    #[error(error_type = StripeErrorType::InvalidRequestError, code = "", message = "The mandate information is invalid. {message}")]
    PaymentIntentMandateInvalid { message: String },

    #[error(error_type = StripeErrorType::InvalidRequestError, code = "", message = "The payment with the specified payment_id '{payment_id}' already exists in our records.")]
    DuplicatePayment { payment_id: String },

    #[error(error_type = StripeErrorType::ConnectorError, code = "", message = "{code}: {message}")]
    ExternalConnectorError {
        code: String,
        message: String,
        connector: String,
        status_code: u16,
    },

    #[error(error_type = StripeErrorType::HyperswitchError, code = "", message = "The connector provided in the request is incorrect or not available")]
    IncorrectConnectorNameGiven,
    // [#216]: https://github.com/juspay/hyperswitch/issues/216
    // Implement the remaining stripe error codes

    /*
        AccountCountryInvalidAddress,
        AccountErrorCountryChangeRequiresAdditionalSteps,
        AccountInformationMismatch,
        AccountInvalid,
        AccountNumberInvalid,
        AcssDebitSessionIncomplete,
        AlipayUpgradeRequired,
        AmountTooLarge,
        AmountTooSmall,
        ApiKeyExpired,
        AuthenticationRequired,
        BalanceInsufficient,
        BankAccountBadRoutingNumbers,
        BankAccountDeclined,
        BankAccountExists,
        BankAccountUnusable,
        BankAccountUnverified,
        BankAccountVerificationFailed,
        BillingInvalidMandate,
        BitcoinUpgradeRequired,
        CardDeclineRateLimitExceeded,
        CardDeclined,
        CardholderPhoneNumberRequired,
        ChargeAlreadyCaptured,
        ChargeAlreadyRefunded,
        ChargeDisputed,
        ChargeExceedsSourceLimit,
        ChargeExpiredForCapture,
        ChargeInvalidParameter,
        ClearingCodeUnsupported,
        CountryCodeInvalid,
        CountryUnsupported,
        CouponExpired,
        CustomerMaxPaymentMethods,
        CustomerMaxSubscriptions,
        DebitNotAuthorized,
        EmailInvalid,
        ExpiredCard,
        IdempotencyKeyInUse,
        IncorrectAddress,
        IncorrectCvc,
        IncorrectNumber,
        IncorrectZip,
        InstantPayoutsConfigDisabled,
        InstantPayoutsCurrencyDisabled,
        InstantPayoutsLimitExceeded,
        InstantPayoutsUnsupported,
        InsufficientFunds,
        IntentInvalidState,
        IntentVerificationMethodMissing,
        InvalidCardType,
        InvalidCharacters,
        InvalidChargeAmount,
        InvalidCvc,
        InvalidExpiryMonth,
        InvalidExpiryYear,
        InvalidNumber,
        InvalidSourceUsage,
        InvoiceNoCustomerLineItems,
        InvoiceNoPaymentMethodTypes,
        InvoiceNoSubscriptionLineItems,
        InvoiceNotEditable,
        InvoiceOnBehalfOfNotEditable,
        InvoicePaymentIntentRequiresAction,
        InvoiceUpcomingNone,
        LivemodeMismatch,
        LockTimeout,
        Missing,
        NoAccount,
        NotAllowedOnStandardAccount,
        OutOfInventory,
        ParameterInvalidEmpty,
        ParameterInvalidInteger,
        ParameterInvalidStringBlank,
        ParameterInvalidStringEmpty,
        ParametersExclusive,
        PaymentIntentActionRequired,
        PaymentIntentIncompatiblePaymentMethod,
        PaymentIntentInvalidParameter,
        PaymentIntentKonbiniRejectedConfirmationNumber,
        PaymentIntentPaymentAttemptExpired,
        PaymentIntentUnexpectedState,
        PaymentMethodBankAccountAlreadyVerified,
        PaymentMethodBankAccountBlocked,
        PaymentMethodBillingDetailsAddressMissing,
        PaymentMethodCurrencyMismatch,
        PaymentMethodInvalidParameter,
        PaymentMethodInvalidParameterTestmode,
        PaymentMethodMicrodepositFailed,
        PaymentMethodMicrodepositVerificationAmountsInvalid,
        PaymentMethodMicrodepositVerificationAmountsMismatch,
        PaymentMethodMicrodepositVerificationAttemptsExceeded,
        PaymentMethodMicrodepositVerificationDescriptorCodeMismatch,
        PaymentMethodMicrodepositVerificationTimeout,
        PaymentMethodProviderDecline,
        PaymentMethodProviderTimeout,
        PaymentMethodUnactivated,
        PaymentMethodUnexpectedState,
        PaymentMethodUnsupportedType,
        PayoutsNotAllowed,
        PlatformAccountRequired,
        PlatformApiKeyExpired,
        PostalCodeInvalid,
        ProcessingError,
        ProductInactive,
        RateLimit,
        ReferToCustomer,
        RefundDisputedPayment,
        ResourceAlreadyExists,
        ResourceMissing,
        ReturnIntentAlreadyProcessed,
        RoutingNumberInvalid,
        SecretKeyRequired,
        SepaUnsupportedAccount,
        SetupAttemptFailed,
        SetupIntentAuthenticationFailure,
        SetupIntentInvalidParameter,
        SetupIntentSetupAttemptExpired,
        SetupIntentUnexpectedState,
        ShippingCalculationFailed,
        SkuInactive,
        StateUnsupported,
        StatusTransitionInvalid,
        TaxIdInvalid,
        TaxesCalculationFailed,
        TerminalLocationCountryUnsupported,
        TestmodeChargesOnly,
        TlsVersionUnsupported,
        TokenInUse,
        TransferSourceBalanceParametersMismatch,
        TransfersNotAllowed,
    */
}

impl ::core::fmt::Display for StripeErrorCode {
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
pub enum StripeErrorType {
    ApiError,
    CardError,
    InvalidRequestError,
    ConnectorError,
    HyperswitchError,
}

impl From<errors::ApiErrorResponse> for StripeErrorCode {
    fn from(value: errors::ApiErrorResponse) -> Self {
        match value {
            errors::ApiErrorResponse::Unauthorized
            | errors::ApiErrorResponse::InvalidJwtToken
            | errors::ApiErrorResponse::GenericUnauthorized { .. }
            | errors::ApiErrorResponse::InvalidEphemeralKey => Self::Unauthorized,
            errors::ApiErrorResponse::InvalidRequestUrl
            | errors::ApiErrorResponse::InvalidHttpMethod => Self::InvalidRequestUrl,
            errors::ApiErrorResponse::MissingRequiredField { field_name } => {
                Self::ParameterMissing {
                    field_name,
                    param: field_name,
                }
            }
            // parameter unknown, invalid request error // actually if we type wrong values in address we get this error. Stripe throws parameter unknown. I don't know if stripe is validating email and stuff
            errors::ApiErrorResponse::InvalidDataFormat {
                field_name,
                expected_format,
            } => Self::ParameterUnknown {
                field_name,
                expected_format,
            },
            errors::ApiErrorResponse::RefundAmountExceedsPaymentAmount => {
                Self::RefundAmountExceedsPaymentAmount {
                    param: "amount".to_owned(),
                }
            }
            errors::ApiErrorResponse::PaymentAuthorizationFailed { data }
            | errors::ApiErrorResponse::PaymentAuthenticationFailed { data } => {
                Self::PaymentIntentAuthenticationFailure { data }
            }
            errors::ApiErrorResponse::VerificationFailed { data } => {
                Self::VerificationFailed { data }
            }
            errors::ApiErrorResponse::PaymentCaptureFailed { data } => {
                Self::PaymentIntentPaymentAttemptFailed { data }
            }
            errors::ApiErrorResponse::InvalidCardData { data } => Self::InvalidCardType, // Maybe it is better to de generalize this router error
            errors::ApiErrorResponse::CardExpired { data } => Self::ExpiredCard,
            errors::ApiErrorResponse::RefundNotPossible { connector } => Self::RefundFailed,
            errors::ApiErrorResponse::RefundFailed { data } => Self::RefundFailed, // Nothing at stripe to map

            errors::ApiErrorResponse::InternalServerError => Self::InternalServerError, // not a stripe code
            errors::ApiErrorResponse::ExternalConnectorError {
                code,
                message,
                connector,
                status_code,
                ..
            } => Self::ExternalConnectorError {
                code,
                message,
                connector,
                status_code,
            },
            errors::ApiErrorResponse::IncorrectConnectorNameGiven => {
                Self::IncorrectConnectorNameGiven
            }
            errors::ApiErrorResponse::MandateActive => Self::MandateActive, //not a stripe code
            errors::ApiErrorResponse::CustomerRedacted => Self::CustomerRedacted, //not a stripe code
            errors::ApiErrorResponse::ConfigNotFound => Self::ConfigNotFound, // not a stripe code
            errors::ApiErrorResponse::DuplicateRefundRequest => Self::DuplicateRefundRequest,
            errors::ApiErrorResponse::RefundNotFound => Self::RefundNotFound,
            errors::ApiErrorResponse::CustomerNotFound => Self::CustomerNotFound,
            errors::ApiErrorResponse::PaymentNotFound => Self::PaymentNotFound,
            errors::ApiErrorResponse::PaymentMethodNotFound => Self::PaymentMethodNotFound,
            errors::ApiErrorResponse::ClientSecretNotGiven => Self::ClientSecretNotFound,
            errors::ApiErrorResponse::MerchantAccountNotFound => Self::MerchantAccountNotFound,
            errors::ApiErrorResponse::ResourceIdNotFound => Self::ResourceIdNotFound,
            errors::ApiErrorResponse::MerchantConnectorAccountNotFound => {
                Self::MerchantConnectorAccountNotFound
            }
            errors::ApiErrorResponse::MandateNotFound => Self::MandateNotFound,
            errors::ApiErrorResponse::ApiKeyNotFound => Self::ApiKeyNotFound,
            errors::ApiErrorResponse::MandateValidationFailed { reason } => {
                Self::PaymentIntentMandateInvalid { message: reason }
            }
            errors::ApiErrorResponse::ReturnUrlUnavailable => Self::ReturnUrlUnavailable,
            errors::ApiErrorResponse::DuplicateMerchantAccount => Self::DuplicateMerchantAccount,
            errors::ApiErrorResponse::DuplicateMerchantConnectorAccount => {
                Self::DuplicateMerchantConnectorAccount
            }
            errors::ApiErrorResponse::DuplicatePaymentMethod => Self::DuplicatePaymentMethod,
            errors::ApiErrorResponse::ClientSecretInvalid => Self::PaymentIntentInvalidParameter {
                param: "client_secret".to_owned(),
            },
            errors::ApiErrorResponse::InvalidRequestData { message } => {
                Self::InvalidRequestData { message }
            }
            errors::ApiErrorResponse::PreconditionFailed { message } => {
                Self::PreconditionFailed { message }
            }
            errors::ApiErrorResponse::InvalidDataValue { field_name } => Self::ParameterMissing {
                field_name,
                param: field_name,
            },
            errors::ApiErrorResponse::MaximumRefundCount => Self::MaximumRefundCount,
            errors::ApiErrorResponse::PaymentNotSucceeded => Self::PaymentFailed,
            errors::ApiErrorResponse::DuplicateMandate => Self::DuplicateMandate,
            errors::ApiErrorResponse::SuccessfulPaymentNotFound => Self::SuccessfulPaymentNotFound,
            errors::ApiErrorResponse::AddressNotFound => Self::AddressNotFound,
            errors::ApiErrorResponse::NotImplemented { .. } => Self::Unauthorized,
            errors::ApiErrorResponse::FlowNotSupported { .. } => Self::InternalServerError,
            errors::ApiErrorResponse::PaymentUnexpectedState {
                current_flow,
                field_name,
                current_value,
                states,
            } => Self::PaymentIntentUnexpectedState {
                current_flow,
                field_name,
                current_value,
                states,
            },
            errors::ApiErrorResponse::DuplicatePayment { payment_id } => {
                Self::DuplicatePayment { payment_id }
            }
            errors::ApiErrorResponse::NotSupported { .. } => Self::InternalServerError,
        }
    }
}

impl actix_web::ResponseError for StripeErrorCode {
    fn status_code(&self) -> reqwest::StatusCode {
        use reqwest::StatusCode;

        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::InvalidRequestUrl => StatusCode::NOT_FOUND,
            Self::ParameterUnknown { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ParameterMissing { .. }
            | Self::RefundAmountExceedsPaymentAmount { .. }
            | Self::PaymentIntentAuthenticationFailure { .. }
            | Self::PaymentIntentPaymentAttemptFailed { .. }
            | Self::ExpiredCard
            | Self::InvalidCardType
            | Self::DuplicateRefundRequest
            | Self::RefundNotFound
            | Self::CustomerNotFound
            | Self::ConfigNotFound
            | Self::ClientSecretNotFound
            | Self::PaymentNotFound
            | Self::PaymentMethodNotFound
            | Self::MerchantAccountNotFound
            | Self::MerchantConnectorAccountNotFound
            | Self::MandateNotFound
            | Self::ApiKeyNotFound
            | Self::DuplicateMerchantAccount
            | Self::DuplicateMerchantConnectorAccount
            | Self::DuplicatePaymentMethod
            | Self::PaymentFailed
            | Self::VerificationFailed { .. }
            | Self::MaximumRefundCount
            | Self::PaymentIntentInvalidParameter { .. }
            | Self::SerdeQsError { .. }
            | Self::InvalidRequestData { .. }
            | Self::PreconditionFailed { .. }
            | Self::DuplicateMandate
            | Self::SuccessfulPaymentNotFound
            | Self::AddressNotFound
            | Self::ResourceIdNotFound
            | Self::PaymentIntentMandateInvalid { .. }
            | Self::PaymentIntentUnexpectedState { .. }
            | Self::DuplicatePayment { .. }
            | Self::IncorrectConnectorNameGiven => StatusCode::BAD_REQUEST,
            Self::RefundFailed
            | Self::InternalServerError
            | Self::MandateActive
            | Self::CustomerRedacted => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ReturnUrlUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::ExternalConnectorError { status_code, .. } => {
                StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::http::header;

        use crate::consts;

        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .insert_header((header::STRICT_TRANSPORT_SECURITY, consts::HSTS_HEADER_VALUE))
            .insert_header((header::VIA, "Juspay_Router"))
            .body(self.to_string())
    }
}

impl From<serde_qs::Error> for StripeErrorCode {
    fn from(item: serde_qs::Error) -> Self {
        match item {
            serde_qs::Error::Custom(s) => Self::SerdeQsError {
                error_message: s,
                param: None,
            },
            serde_qs::Error::Parse(param, position) => Self::SerdeQsError {
                error_message: format!(
                    "parsing failed with error: '{param}' at position: {position}"
                ),
                param: Some(param),
            },
            serde_qs::Error::Unsupported => Self::SerdeQsError {
                error_message: "Given request format is not supported".to_owned(),
                param: None,
            },
            serde_qs::Error::FromUtf8(_) => Self::SerdeQsError {
                error_message: "Failed to parse request to from utf-8".to_owned(),
                param: None,
            },
            serde_qs::Error::Io(_) => Self::SerdeQsError {
                error_message: "Failed to parse request".to_owned(),
                param: None,
            },
            serde_qs::Error::ParseInt(_) => Self::SerdeQsError {
                error_message: "Failed to parse integer in request".to_owned(),
                param: None,
            },
            serde_qs::Error::Utf8(_) => Self::SerdeQsError {
                error_message: "Failed to convert utf8 to string".to_owned(),
                param: None,
            },
        }
    }
}

impl common_utils::errors::ErrorSwitch<StripeErrorCode> for errors::ApiErrorResponse {
    fn switch(&self) -> StripeErrorCode {
        self.clone().into()
    }
}
