use api_models::errors::types::Extra;
use common_utils::errors::ErrorSwitch;
use http::StatusCode;

use super::{ApiErrorResponse, ConnectorError, CustomersErrorResponse, StorageError};

impl ErrorSwitch<api_models::errors::types::ApiErrorResponse> for ApiErrorResponse {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};

        match self {
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
            Self::ClientSecretInvalid => {
                AER::BadRequest(ApiError::new("IR", 9, "The client_secret provided does not match the client_secret associated with the Payment", None))
            }
            Self::CurrencyNotSupported { message } => {
                AER::BadRequest(ApiError::new("IR", 9, message, None))
            }
            Self::MandateActive => {
                AER::BadRequest(ApiError::new("IR", 10, "Customer has active mandate/subsciption", None))
            }
            Self::CustomerRedacted => {
                AER::BadRequest(ApiError::new("IR", 11, "Customer has already been redacted", None))
            }
            Self::MaximumRefundCount => AER::BadRequest(ApiError::new("IR", 12, "Reached maximum refund attempts", None)),
            Self::RefundAmountExceedsPaymentAmount => {
                AER::BadRequest(ApiError::new("IR", 13, "Refund amount exceeds the payment amount", None))
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
            Self::ClientSecretExpired => AER::BadRequest(ApiError::new(
                "IR",
                19,
                "The provided client_secret has expired", None
            )),
            Self::MissingRequiredFields { field_names } => AER::BadRequest(
                ApiError::new("IR", 21, "Missing required params".to_string(), Some(Extra {data: Some(serde_json::json!(field_names)), ..Default::default() })),
            ),
            Self::AccessForbidden {resource} => {
                AER::ForbiddenCommonResource(ApiError::new("IR", 22, format!("Access forbidden. Not authorized to access this resource {resource}"), None))
            },
            Self::FileProviderNotSupported { message } => {
                AER::BadRequest(ApiError::new("IR", 23, message.to_string(), None))
            },
            Self::UnprocessableEntity {message} => AER::Unprocessable(ApiError::new("IR", 23, message.to_string(), None)),
            Self::ExternalConnectorError {
                code,
                message,
                connector,
                reason,
                status_code,
            } => AER::ConnectorError(ApiError::new("CE", 0, format!("{code}: {message}"), Some(Extra {connector: Some(connector.clone()), reason: reason.clone(), ..Default::default()})), StatusCode::from_u16(*status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)),
            Self::PaymentAuthorizationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 1, "Payment failed during authorization with connector. Retry payment", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::PaymentAuthenticationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 2, "Payment failed during authentication with connector. Retry payment", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::PaymentCaptureFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 3, "Capture attempt failed while processing with connector", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::DisputeFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 1, "Dispute operation failed while processing with connector. Retry operation", Some(Extra { data: data.clone(), ..Default::default()})))
            }
            Self::InvalidCardData { data } => AER::BadRequest(ApiError::new("CE", 4, "The card data is invalid", Some(Extra { data: data.clone(), ..Default::default()}))),
            Self::CardExpired { data } => AER::BadRequest(ApiError::new("CE", 5, "The card has expired", Some(Extra { data: data.clone(), ..Default::default()}))),
            Self::RefundFailed { data } => AER::BadRequest(ApiError::new("CE", 6, "Refund failed while processing with connector. Retry refund", Some(Extra { data: data.clone(), ..Default::default()}))),
            Self::VerificationFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 7, "Verification failed while processing with connector. Retry operation", Some(Extra { data: data.clone(), ..Default::default()})))
            },
            Self::MandateUpdateFailed | Self::MandateSerializationFailed | Self::MandateDeserializationFailed | Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, "Something went wrong", None))
            }
            Self::PayoutFailed { data } => {
                AER::BadRequest(ApiError::new("CE", 4, "Payout failed while processing with connector.", Some(Extra { data: data.clone(), ..Default::default()})))
            },
            Self::DuplicateRefundRequest => AER::BadRequest(ApiError::new("HE", 1, "Duplicate refund request. Refund already attempted with the refund ID", None)),
            Self::DuplicateMandate => AER::BadRequest(ApiError::new("HE", 1, "Duplicate mandate request. Mandate already attempted with the Mandate ID", None)),
            Self::DuplicateMerchantAccount => AER::BadRequest(ApiError::new("HE", 1, "The merchant account with the specified details already exists in our records", None)),
            Self::DuplicateMerchantConnectorAccount { profile_id, connector_name } => {
                AER::BadRequest(ApiError::new("HE", 1, format!("The merchant connector account with the specified profile_id '{profile_id}' and connector_name '{connector_name}' already exists in our records"), None))
            }
            Self::DuplicatePaymentMethod => AER::BadRequest(ApiError::new("HE", 1, "The payment method with the specified details already exists in our records", None)),
            Self::DuplicatePayment { payment_id } => {
                AER::BadRequest(ApiError::new("HE", 1, "The payment with the specified payment_id already exists in our records", Some(Extra {reason: Some(format!("{payment_id} already exists")), ..Default::default()})))
            }
            Self::DuplicatePayout { payout_id } => {
                AER::BadRequest(ApiError::new("HE", 1, format!("The payout with the specified payout_id '{payout_id}' already exists in our records"), None))
            }
            Self::GenericDuplicateError { message } => {
                AER::BadRequest(ApiError::new("HE", 1, message, None))
            }
            Self::RefundNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Refund does not exist in our records.", None))
            }
            Self::CustomerNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Customer does not exist in our records", None))
            }
            Self::ConfigNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Config key does not exist in our records.", None))
            },
            Self::DuplicateConfig => {
                AER::BadRequest(ApiError::new("HE", 1, "The config with the specified key already exists in our records", None))
            }
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
            Self::MerchantConnectorAccountDisabled => {
                AER::BadRequest(ApiError::new("HE", 3, "The selected merchant connector account is disabled", None))
            }
            Self::ResourceIdNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Resource ID does not exist in our records", None))
            }
            Self::MandateNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Mandate does not exist in our records", None))
            }
            Self::PayoutNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Payout does not exist in our records", None))
            }
            Self::ReturnUrlUnavailable => AER::NotFound(ApiError::new("HE", 3, "Return URL is not configured and not passed in payments request", None)),
            Self::RefundNotPossible { connector } => {
                AER::BadRequest(ApiError::new("HE", 3, format!("This refund is not possible through Hyperswitch. Please raise the refund through {connector} dashboard"), None))
            }
            Self::MandateValidationFailed { reason } => {
                AER::BadRequest(ApiError::new("HE", 3, "Mandate Validation Failed", Some(Extra { reason: Some(reason.clone()), ..Default::default() })))
            }
            Self::PaymentNotSucceeded => AER::BadRequest(ApiError::new("HE", 3, "The payment has not succeeded yet. Please pass a successful payment to initiate refund", None)),
            Self::SuccessfulPaymentNotFound => {
                AER::NotFound(ApiError::new("HE", 4, "Successful payment not found for the given payment id", None))
            }
            Self::IncorrectConnectorNameGiven => {
                AER::NotFound(ApiError::new("HE", 4, "The connector provided in the request is incorrect or not available", None))
            }
            Self::AddressNotFound => {
                AER::NotFound(ApiError::new("HE", 4, "Address does not exist in our records", None))
            },
            Self::GenericNotFoundError { message } => {
                AER::NotFound(ApiError::new("HE", 5, message, None))
            },
            Self::ApiKeyNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "API Key does not exist in our records", None))
            }
            Self::NotSupported { message } => {
                AER::BadRequest(ApiError::new("HE", 3, "Payment method type not supported", Some(Extra {reason: Some(message.to_owned()), ..Default::default()})))
            },
            Self::InvalidCardIin => AER::BadRequest(ApiError::new("HE", 3, "The provided card IIN does not exist", None)),
            Self::InvalidCardIinLength  => AER::BadRequest(ApiError::new("HE", 3, "The provided card IIN length is invalid, please provide an IIN with 6 digits", None)),
            Self::FlowNotSupported { flow, connector } => {
                AER::BadRequest(ApiError::new("IR", 20, format!("{flow} flow not supported"), Some(Extra {connector: Some(connector.to_owned()), ..Default::default()}))) //FIXME: error message
            }
            Self::DisputeNotFound { .. } => {
                AER::NotFound(ApiError::new("HE", 2, "Dispute does not exist in our records", None))
            },
            Self::BusinessProfileNotFound { id } => {
                AER::NotFound(ApiError::new("HE", 2, format!("Business profile with the given id {id} does not exist"), None))
            }
            Self::FileNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "File does not exist in our records", None))
            }
            Self::FileNotAvailable => {
                AER::NotFound(ApiError::new("HE", 2, "File not available", None))
            }
            Self::DisputeStatusValidationFailed { .. } => {
                AER::BadRequest(ApiError::new("HE", 2, "Dispute status validation failed", None))
            }
            Self::FileValidationFailed { reason } => {
                AER::BadRequest(ApiError::new("HE", 2, format!("File validation failed {reason}"), None))
            }
            Self::MissingFile => {
                AER::BadRequest(ApiError::new("HE", 2, "File not found in the request", None))
            }
            Self::MissingFilePurpose => {
                AER::BadRequest(ApiError::new("HE", 2, "File purpose not found in the request or is invalid", None))
            }
            Self::MissingFileContentType => {
                AER::BadRequest(ApiError::new("HE", 2, "File content type not found", None))
            }
            Self::MissingDisputeId => {
                AER::BadRequest(ApiError::new("HE", 2, "Dispute id not found in the request", None))
            }
            Self::WebhookAuthenticationFailed => {
                AER::Unauthorized(ApiError::new("WE", 1, "Webhook authentication failed", None))
            }
            Self::WebhookResourceNotFound => {
                AER::NotFound(ApiError::new("WE", 4, "Webhook resource was not found", None))
            }
            Self::WebhookBadRequest => {
                AER::BadRequest(ApiError::new("WE", 2, "Bad request body received", None))
            }
            Self::WebhookProcessingFailure => {
                AER::InternalServerError(ApiError::new("WE", 3, "There was an issue processing the webhook", None))
            },
            Self::WebhookInvalidMerchantSecret => {
                AER::BadRequest(ApiError::new("WE", 2, "Merchant Secret set for webhook source verificartion is invalid", None))
            }
            Self::IncorrectPaymentMethodConfiguration => {
                AER::BadRequest(ApiError::new("HE", 4, "No eligible connector was found for the current payment method configuration", None))
            }
            Self::WebhookUnprocessableEntity => {
                AER::Unprocessable(ApiError::new("WE", 5, "There was an issue processing the webhook body", None))
            },
            Self::ResourceBusy => {
                AER::Unprocessable(ApiError::new("WE", 5, "There was an issue processing the webhook body", None))
            }
            Self::PaymentLinkNotFound => {
                AER::NotFound(ApiError::new("HE", 2, "Payment Link does not exist in our records", None))
            }
            Self::InvalidConnectorConfiguration {config} => {
                AER::BadRequest(ApiError::new("IR", 24, format!("Merchant connector account is configured with invalid {config}"), None))
            }
        }
    }
}

impl ErrorSwitch<ApiErrorResponse> for ConnectorError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::WebhookSourceVerificationFailed => ApiErrorResponse::WebhookAuthenticationFailed,
            Self::WebhookSignatureNotFound
            | Self::WebhookReferenceIdNotFound
            | Self::WebhookResourceObjectNotFound
            | Self::WebhookBodyDecodingFailed
            | Self::WebhooksNotImplemented => ApiErrorResponse::WebhookBadRequest,
            Self::WebhookEventTypeNotFound => ApiErrorResponse::WebhookUnprocessableEntity,
            Self::WebhookVerificationSecretInvalid => {
                ApiErrorResponse::WebhookInvalidMerchantSecret
            }
            _ => ApiErrorResponse::InternalServerError,
        }
    }
}

impl ErrorSwitch<api_models::errors::types::ApiErrorResponse> for CustomersErrorResponse {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        match self {
            Self::CustomerRedacted => AER::BadRequest(ApiError::new(
                "IR",
                11,
                "Customer has already been redacted",
                None,
            )),
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, "Something went wrong", None))
            }
            Self::MandateActive => AER::BadRequest(ApiError::new(
                "IR",
                10,
                "Customer has active mandate/subsciption",
                None,
            )),
            Self::CustomerNotFound => AER::NotFound(ApiError::new(
                "HE",
                2,
                "Customer does not exist in our records",
                None,
            )),
            Self::CustomerAlreadyExists => AER::BadRequest(ApiError::new(
                "IR",
                12,
                "Customer with the given `customer_id` already exists",
                None,
            )),
        }
    }
}

impl ErrorSwitch<CustomersErrorResponse> for StorageError {
    fn switch(&self) -> CustomersErrorResponse {
        use CustomersErrorResponse as CER;
        match self {
            err if err.is_db_not_found() => CER::CustomerNotFound,
            Self::CustomerRedacted => CER::CustomerRedacted,
            _ => CER::InternalServerError,
        }
    }
}

impl ErrorSwitch<CustomersErrorResponse> for common_utils::errors::CryptoError {
    fn switch(&self) -> CustomersErrorResponse {
        CustomersErrorResponse::InternalServerError
    }
}

impl ErrorSwitch<CustomersErrorResponse> for ApiErrorResponse {
    fn switch(&self) -> CustomersErrorResponse {
        use CustomersErrorResponse as CER;
        match self {
            Self::InternalServerError => CER::InternalServerError,
            Self::MandateActive => CER::MandateActive,
            Self::CustomerNotFound => CER::CustomerNotFound,
            _ => CER::InternalServerError,
        }
    }
}
