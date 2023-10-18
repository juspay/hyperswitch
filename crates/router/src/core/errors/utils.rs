use common_utils::errors::CustomResult;

use crate::{core::errors, logger};

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
}

impl<T> StorageErrorExt<T, errors::CustomersErrorResponse>
    for error_stack::Result<T, errors::StorageError>
{
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: errors::CustomersErrorResponse,
    ) -> error_stack::Result<T, errors::CustomersErrorResponse> {
        self.map_err(|err| match err.current_context() {
            error if error.is_db_not_found() => err.change_context(not_found_response),
            errors::StorageError::CustomerRedacted => {
                err.change_context(errors::CustomersErrorResponse::CustomerRedacted)
            }
            _ => err.change_context(errors::CustomersErrorResponse::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        duplicate_response: errors::CustomersErrorResponse,
    ) -> error_stack::Result<T, errors::CustomersErrorResponse> {
        self.map_err(|err| {
            if err.current_context().is_db_unique_violation() {
                err.change_context(duplicate_response)
            } else {
                err.change_context(errors::CustomersErrorResponse::InternalServerError)
            }
        })
    }
}

impl<T> StorageErrorExt<T, errors::ApiErrorResponse>
    for error_stack::Result<T, data_models::errors::StorageError>
{
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: errors::ApiErrorResponse,
    ) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                data_models::errors::StorageError::ValueNotFound(_) => not_found_response,
                data_models::errors::StorageError::CustomerRedacted => {
                    errors::ApiErrorResponse::CustomerRedacted
                }
                _ => errors::ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: errors::ApiErrorResponse,
    ) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                data_models::errors::StorageError::DuplicateValue { .. } => duplicate_response,
                _ => errors::ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }
}

impl<T> StorageErrorExt<T, errors::ApiErrorResponse>
    for error_stack::Result<T, errors::StorageError>
{
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: errors::ApiErrorResponse,
    ) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            if err.current_context().is_db_not_found() {
                return err.change_context(not_found_response);
            };
            match err.current_context() {
                errors::StorageError::CustomerRedacted => {
                    err.change_context(errors::ApiErrorResponse::CustomerRedacted)
                }
                _ => err.change_context(errors::ApiErrorResponse::InternalServerError),
            }
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: errors::ApiErrorResponse,
    ) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            if err.current_context().is_db_unique_violation() {
                err.change_context(duplicate_response)
            } else {
                err.change_context(errors::ApiErrorResponse::InternalServerError)
            }
        })
    }
}

pub trait ConnectorErrorExt<T> {
    #[track_caller]
    fn to_refund_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse>;
    #[track_caller]
    fn to_payment_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse>;
    #[track_caller]
    fn to_setup_mandate_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse>;
    #[track_caller]
    fn to_dispute_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse>;
    #[cfg(feature = "payouts")]
    #[track_caller]
    fn to_payout_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse>;

    // Validates if the result, is Ok(..) or WebhookEventTypeNotFound all the other error variants
    // are cascaded while these two event types are handled via `Option`
    #[track_caller]
    fn allow_webhook_event_type_not_found(
        self,
        enabled: bool,
    ) -> error_stack::Result<Option<T>, errors::ConnectorError>;
}

impl<T> ConnectorErrorExt<T> for error_stack::Result<T, errors::ConnectorError> {
    fn to_refund_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let data = match err.current_context() {
                errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                    let response_str = std::str::from_utf8(bytes);
                    match response_str {
                        Ok(s) => serde_json::from_str(s)
                            .map_err(
                                |error| logger::error!(%error,"Failed to convert response to JSON"),
                            )
                            .ok(),
                        Err(error) => {
                            logger::error!(%error,"Failed to convert response to UTF8 string");
                            None
                        }
                    }
                }
                _ => None,
            };
            err.change_context(errors::ApiErrorResponse::RefundFailed { data })
        })
    }

    fn to_payment_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let error = match err.current_context() {
                errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                    let response_str = std::str::from_utf8(bytes);
                    let data = match response_str {
                        Ok(s) => serde_json::from_str(s)
                            .map_err(
                                |error| logger::error!(%error,"Failed to convert response to JSON"),
                            )
                            .ok(),
                        Err(error) => {
                            logger::error!(%error,"Failed to convert response to UTF8 string");
                            None
                        }
                    };
                    errors::ApiErrorResponse::PaymentAuthorizationFailed { data }
                }
                errors::ConnectorError::MissingRequiredField { field_name } => {
                    errors::ApiErrorResponse::MissingRequiredField { field_name }
                }
                errors::ConnectorError::MissingRequiredFields { field_names } => {
                    errors::ApiErrorResponse::MissingRequiredFields { field_names: field_names.to_vec() }
                }
                errors::ConnectorError::NotImplemented(reason) => {
                    errors::ApiErrorResponse::NotImplemented {
                        message: errors::api_error_response::NotImplementedMessage::Reason(
                            reason.to_string(),
                        ),
                    }
                }
                errors::ConnectorError::MismatchedPaymentData => {
                    errors::ApiErrorResponse::InvalidDataValue {
                        field_name:
                            "payment_method_data, payment_method_type and payment_experience does not match",
                    }
                },
                errors::ConnectorError::NotSupported { message, connector } => {
                    errors::ApiErrorResponse::NotSupported { message: format!("{message} is not supported by {connector}") }
                },
                errors::ConnectorError::FlowNotSupported{ flow, connector } => {
                    errors::ApiErrorResponse::FlowNotSupported { flow: flow.to_owned(), connector: connector.to_owned() }
                },
                errors::ConnectorError::InvalidDataFormat { field_name } => {
                    errors::ApiErrorResponse::InvalidDataValue { field_name }
                },
                errors::ConnectorError::CurrencyNotSupported { message, connector} => errors::ApiErrorResponse::CurrencyNotSupported { message: format!("Credentials for the currency {message} are not configured with the connector {connector}/hyperswitch") },
                errors::ConnectorError::FailedToObtainAuthType =>  errors::ApiErrorResponse::InvalidConnectorConfiguration {config: "connector_account_details".to_string()},
                errors::ConnectorError::InvalidConnectorConfig { config }  => errors::ApiErrorResponse::InvalidConnectorConfiguration { config: config.to_string() },
                errors::ConnectorError::FailedToObtainIntegrationUrl |
                errors::ConnectorError::RequestEncodingFailed |
                errors::ConnectorError::RequestEncodingFailedWithReason(_) |
                errors::ConnectorError::ParsingFailed |
                errors::ConnectorError::ResponseDeserializationFailed |
                errors::ConnectorError::UnexpectedResponseError(_) |
                errors::ConnectorError::RoutingRulesParsingError |
                errors::ConnectorError::FailedToObtainPreferredConnector |
                errors::ConnectorError::InvalidConnectorName |
                errors::ConnectorError::InvalidWallet |
                errors::ConnectorError::ResponseHandlingFailed |
                errors::ConnectorError::FailedToObtainCertificate |
                errors::ConnectorError::NoConnectorMetaData |
                errors::ConnectorError::FailedToObtainCertificateKey |
                errors::ConnectorError::CaptureMethodNotSupported |
                errors::ConnectorError::MissingConnectorMandateID |
                errors::ConnectorError::MissingConnectorTransactionID |
                errors::ConnectorError::MissingConnectorRefundID |
                errors::ConnectorError::MissingApplePayTokenData |
                errors::ConnectorError::WebhooksNotImplemented |
                errors::ConnectorError::WebhookBodyDecodingFailed |
                errors::ConnectorError::WebhookSignatureNotFound |
                errors::ConnectorError::WebhookSourceVerificationFailed |
                errors::ConnectorError::WebhookVerificationSecretNotFound |
                errors::ConnectorError::WebhookVerificationSecretInvalid |
                errors::ConnectorError::WebhookReferenceIdNotFound |
                errors::ConnectorError::WebhookEventTypeNotFound |
                errors::ConnectorError::WebhookResourceObjectNotFound |
                errors::ConnectorError::WebhookResponseEncodingFailed |
                errors::ConnectorError::InvalidDateFormat |
                errors::ConnectorError::DateFormattingFailed |
                errors::ConnectorError::InvalidWalletToken |
                errors::ConnectorError::MissingConnectorRelatedTransactionID { .. } |
                errors::ConnectorError::FileValidationFailed { .. } |
                errors::ConnectorError::MissingConnectorRedirectionPayload { .. } |
                errors::ConnectorError::FailedAtConnector { .. } |
                errors::ConnectorError::MissingPaymentMethodType |
                errors::ConnectorError::InSufficientBalanceInPaymentMethod |
                errors::ConnectorError::RequestTimeoutReceived |
                errors::ConnectorError::ProcessingStepFailed(None) => errors::ApiErrorResponse::InternalServerError
            };
            err.change_context(error)
        })
    }

    fn to_setup_mandate_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let error = err.current_context();
            let data = match error {
                errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                    let response_str = std::str::from_utf8(bytes);
                    let error_response = match response_str {
                        Ok(s) => serde_json::from_str(s)
                            .map_err(
                                |err| logger::error!(%err, "Failed to convert response to JSON"),
                            )
                            .ok(),
                        Err(err) => {
                            logger::error!(%err, "Failed to convert response to UTF8 string");
                            None
                        }
                    };
                    errors::ApiErrorResponse::PaymentAuthorizationFailed {
                        data: error_response,
                    }
                }
                errors::ConnectorError::MissingRequiredField { field_name } => {
                    errors::ApiErrorResponse::MissingRequiredField { field_name }
                }
                errors::ConnectorError::FailedToObtainIntegrationUrl => {
                    errors::ApiErrorResponse::InvalidConnectorConfiguration {
                        config: "connector_account_details".to_string(),
                    }
                }
                errors::ConnectorError::InvalidConnectorConfig { config: field_name } => {
                    errors::ApiErrorResponse::InvalidConnectorConfiguration {
                        config: field_name.to_string(),
                    }
                }
                errors::ConnectorError::RequestEncodingFailed
                | errors::ConnectorError::RequestEncodingFailedWithReason(_)
                | errors::ConnectorError::ParsingFailed
                | errors::ConnectorError::ResponseDeserializationFailed
                | errors::ConnectorError::UnexpectedResponseError(_)
                | errors::ConnectorError::RoutingRulesParsingError
                | errors::ConnectorError::FailedToObtainPreferredConnector
                | errors::ConnectorError::InvalidConnectorName
                | errors::ConnectorError::InvalidWallet
                | errors::ConnectorError::ResponseHandlingFailed
                | errors::ConnectorError::MissingRequiredFields { .. }
                | errors::ConnectorError::FailedToObtainAuthType
                | errors::ConnectorError::FailedToObtainCertificate
                | errors::ConnectorError::NoConnectorMetaData
                | errors::ConnectorError::FailedToObtainCertificateKey
                | errors::ConnectorError::NotImplemented(_)
                | errors::ConnectorError::NotSupported { .. }
                | errors::ConnectorError::FlowNotSupported { .. }
                | errors::ConnectorError::CaptureMethodNotSupported
                | errors::ConnectorError::MissingConnectorMandateID
                | errors::ConnectorError::MissingConnectorTransactionID
                | errors::ConnectorError::MissingConnectorRefundID
                | errors::ConnectorError::MissingApplePayTokenData
                | errors::ConnectorError::WebhooksNotImplemented
                | errors::ConnectorError::WebhookBodyDecodingFailed
                | errors::ConnectorError::WebhookSignatureNotFound
                | errors::ConnectorError::WebhookSourceVerificationFailed
                | errors::ConnectorError::WebhookVerificationSecretNotFound
                | errors::ConnectorError::WebhookVerificationSecretInvalid
                | errors::ConnectorError::WebhookReferenceIdNotFound
                | errors::ConnectorError::WebhookEventTypeNotFound
                | errors::ConnectorError::WebhookResourceObjectNotFound
                | errors::ConnectorError::WebhookResponseEncodingFailed
                | errors::ConnectorError::InvalidDateFormat
                | errors::ConnectorError::DateFormattingFailed
                | errors::ConnectorError::InvalidDataFormat { .. }
                | errors::ConnectorError::MismatchedPaymentData
                | errors::ConnectorError::InvalidWalletToken
                | errors::ConnectorError::MissingConnectorRelatedTransactionID { .. }
                | errors::ConnectorError::FileValidationFailed { .. }
                | errors::ConnectorError::MissingConnectorRedirectionPayload { .. }
                | errors::ConnectorError::FailedAtConnector { .. }
                | errors::ConnectorError::MissingPaymentMethodType
                | errors::ConnectorError::InSufficientBalanceInPaymentMethod
                | errors::ConnectorError::RequestTimeoutReceived
                | errors::ConnectorError::CurrencyNotSupported { .. }
                | errors::ConnectorError::ProcessingStepFailed(None) => {
                    logger::error!(%error,"Setup Mandate flow failed");
                    errors::ApiErrorResponse::PaymentAuthorizationFailed { data: None }
                }
            };
            err.change_context(data)
        })
    }

    fn to_dispute_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let error = match err.current_context() {
                errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                    let response_str = std::str::from_utf8(bytes);
                    let data = match response_str {
                        Ok(s) => serde_json::from_str(s)
                            .map_err(
                                |error| logger::error!(%error,"Failed to convert response to JSON"),
                            )
                            .ok(),
                        Err(error) => {
                            logger::error!(%error,"Failed to convert response to UTF8 string");
                            None
                        }
                    };
                    errors::ApiErrorResponse::DisputeFailed { data }
                }
                errors::ConnectorError::MissingRequiredField { field_name } => {
                    errors::ApiErrorResponse::MissingRequiredField { field_name }
                }
                errors::ConnectorError::MissingRequiredFields { field_names } => {
                    errors::ApiErrorResponse::MissingRequiredFields {
                        field_names: field_names.to_vec(),
                    }
                }
                _ => errors::ApiErrorResponse::InternalServerError,
            };
            err.change_context(error)
        })
    }

    #[cfg(feature = "payouts")]
    fn to_payout_failed_response(self) -> error_stack::Result<T, errors::ApiErrorResponse> {
        self.map_err(|err| {
            let error = match err.current_context() {
                errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                    let response_str = std::str::from_utf8(bytes);
                    let data = match response_str {
                        Ok(s) => serde_json::from_str(s)
                            .map_err(
                                |error| logger::error!(%error,"Failed to convert response to JSON"),
                            )
                            .ok(),
                        Err(error) => {
                            logger::error!(%error,"Failed to convert response to UTF8 string");
                            None
                        }
                    };
                    errors::ApiErrorResponse::PayoutFailed { data }
                }
                errors::ConnectorError::MissingRequiredField { field_name } => {
                    errors::ApiErrorResponse::MissingRequiredField { field_name }
                }
                errors::ConnectorError::MissingRequiredFields { field_names } => {
                    errors::ApiErrorResponse::MissingRequiredFields {
                        field_names: field_names.to_vec(),
                    }
                }
                _ => errors::ApiErrorResponse::InternalServerError,
            };
            err.change_context(error)
        })
    }

    fn allow_webhook_event_type_not_found(
        self,
        enabled: bool,
    ) -> CustomResult<Option<T>, errors::ConnectorError> {
        match self {
            Ok(event_type) => Ok(Some(event_type)),
            Err(error) => match error.current_context() {
                errors::ConnectorError::WebhookEventTypeNotFound if enabled => Ok(None),
                _ => Err(error),
            },
        }
    }
}
