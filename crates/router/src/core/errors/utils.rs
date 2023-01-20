use crate::{core::errors, logger};

pub trait StorageErrorExt {
    fn to_not_found_response(
        self,
        not_found_response: errors::ApiErrorResponse,
    ) -> error_stack::Report<errors::ApiErrorResponse>;

    fn to_duplicate_response(
        self,
        duplicate_response: errors::ApiErrorResponse,
    ) -> error_stack::Report<errors::ApiErrorResponse>;
}

impl StorageErrorExt for error_stack::Report<errors::StorageError> {
    fn to_not_found_response(
        self,
        not_found_response: errors::ApiErrorResponse,
    ) -> error_stack::Report<errors::ApiErrorResponse> {
        if self.current_context().is_db_not_found() {
            return self.change_context(not_found_response);
        }
        match self.current_context() {
            errors::StorageError::CustomerRedacted => {
                self.change_context(errors::ApiErrorResponse::CustomerRedacted)
            }
            _ => self.change_context(errors::ApiErrorResponse::InternalServerError),
        }
    }

    fn to_duplicate_response(
        self,
        duplicate_response: errors::ApiErrorResponse,
    ) -> error_stack::Report<errors::ApiErrorResponse> {
        if self.current_context().is_db_unique_violation() {
            self.change_context(duplicate_response)
        } else {
            self.change_context(errors::ApiErrorResponse::InternalServerError)
        }
    }
}

pub trait ConnectorErrorExt {
    fn to_refund_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
    fn to_payment_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
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
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: field_name.clone(),
                }
            }
            errors::ConnectorError::NotImplemented(reason) => {
                errors::ApiErrorResponse::NotImplemented {
                    message: errors::api_error_response::NotImplementedMessage::Reason(
                        reason.to_string(),
                    ),
                }
            }
            _ => errors::ApiErrorResponse::InternalServerError,
        };
        self.change_context(error)
    }

    fn to_verify_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse> {
        let data = match self.current_context() {
            errors::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
                let response_str = std::str::from_utf8(bytes);
                match response_str {
                    Ok(s) => serde_json::from_str(s)
                        .map_err(|err| logger::error!(%err, "Failed to convert response to JSON"))
                        .ok(),
                    Err(err) => {
                        logger::error!(%err, "Failed to convert response to UTF8 string");
                        None
                    }
                }
            }
            _ => None,
        };
        self.change_context(errors::ApiErrorResponse::PaymentAuthorizationFailed { data })
    }
}

pub trait RedisErrorExt {
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
