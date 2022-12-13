use crate::{core::errors, logger};

pub(crate) trait StorageErrorExt {
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
            self.change_context(not_found_response)
        } else {
            self.change_context(errors::ApiErrorResponse::InternalServerError)
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

pub(crate) trait ApiClientErrorExt {
    fn to_unsuccessful_processing_step_response(
        self,
    ) -> error_stack::Report<errors::ConnectorError>;
}

impl ApiClientErrorExt for error_stack::Report<errors::ApiClientError> {
    fn to_unsuccessful_processing_step_response(
        self,
    ) -> error_stack::Report<errors::ConnectorError> {
        let data = match self.current_context() {
            errors::ApiClientError::BadRequestReceived(bytes)
            | errors::ApiClientError::UnauthorizedReceived(bytes)
            | errors::ApiClientError::NotFoundReceived(bytes)
            | errors::ApiClientError::UnprocessableEntityReceived(bytes) => Some(bytes.clone()),
            _ => None,
        };
        self.change_context(errors::ConnectorError::ProcessingStepFailed(data))
    }
}

pub(crate) trait ConnectorErrorExt {
    fn to_refund_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
    fn to_payment_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
    fn to_verify_failed_response(self) -> error_stack::Report<errors::ApiErrorResponse>;
}

// FIXME: The implementation can be improved by handling BOM maybe?
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
        self.change_context(errors::ApiErrorResponse::PaymentAuthorizationFailed { data })
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
