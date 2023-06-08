use crate::{core::errors, logger};

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
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

pub trait ConnectorErrorExt {
    #[track_caller]
    fn to_refund_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
    #[track_caller]
    fn to_payment_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
    #[track_caller]
    fn to_verify_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
}

impl ConnectorErrorExt for error_stack::Report<errors::ConnectorError> {
    fn to_refund_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse> {
        let data = match self.current_context() {
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
        self.change_context(errors::ApiErrorResponse::RefundFailed { data })
    }

    fn to_payment_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse> {
        let error = match self.current_context() {
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
            errors::ConnectorError::NotSupported { message, connector, payment_experience } => {
                errors::ApiErrorResponse::NotSupported { message: format!("{message} is not supported by {connector} through payment experience {payment_experience}") }
            },
            errors::ConnectorError::FlowNotSupported{ flow, connector } => {
                errors::ApiErrorResponse::FlowNotSupported { flow: flow.to_owned(), connector: connector.to_owned() }
            }
            _ => errors::ApiErrorResponse::InternalServerError,
        };
        self.change_context(error)
    }

    fn to_verify_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse> {
        let error = self.current_context();
        let data = match error {
            errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                let response_str = std::str::from_utf8(bytes);
                let error_response = match response_str {
                    Ok(s) => serde_json::from_str(s)
                        .map_err(|err| logger::error!(%err, "Failed to convert response to JSON"))
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
            _ => {
                logger::error!(%error,"Verify flow failed");
                errors::ApiErrorResponse::PaymentAuthorizationFailed { data: None }
            }
        };
        self.change_context(data)
    }
}

pub trait RedisErrorExt {
    #[track_caller]
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<errors::StorageError>;
}

impl RedisErrorExt for error_stack::Report<errors::RedisError> {
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<errors::StorageError> {
        match self.current_context() {
            errors::RedisError::NotFound => self.change_context(
                errors::StorageError::ValueNotFound(format!("Data does not exist for key {key}",)),
            ),
            _ => self.change_context(errors::StorageError::KVError),
        }
    }
}
