use super::DatabaseError;
use crate::logger;

pub(crate) trait StorageErrorExt {
    fn to_not_found_response(
        self,
        not_found_response: super::ApiErrorResponse,
    ) -> error_stack::Report<super::ApiErrorResponse>;

    fn to_duplicate_response(
        self,
        duplicate_response: super::ApiErrorResponse,
    ) -> error_stack::Report<super::ApiErrorResponse>;
}

impl StorageErrorExt for error_stack::Report<super::StorageError> {
    fn to_not_found_response(
        self,
        not_found_response: super::ApiErrorResponse,
    ) -> error_stack::Report<super::ApiErrorResponse> {
        match self.current_context() {
            super::StorageError::DatabaseError(DatabaseError::NotFound) => {
                self.change_context(not_found_response)
            }
            _ => self.change_context(super::ApiErrorResponse::InternalServerError),
        }
    }

    fn to_duplicate_response(
        self,
        duplicate_response: super::ApiErrorResponse,
    ) -> error_stack::Report<super::ApiErrorResponse> {
        match self.current_context() {
            super::StorageError::DatabaseError(DatabaseError::UniqueViolation) => {
                self.change_context(duplicate_response)
            }
            _ => self.change_context(super::ApiErrorResponse::InternalServerError),
        }
    }
}

pub(crate) trait ApiClientErrorExt {
    fn to_unsuccessful_processing_step_response(self)
        -> error_stack::Report<super::ConnectorError>;
}

impl ApiClientErrorExt for error_stack::Report<super::ApiClientError> {
    fn to_unsuccessful_processing_step_response(
        self,
    ) -> error_stack::Report<super::ConnectorError> {
        let data = match self.current_context() {
            super::ApiClientError::BadRequestReceived(bytes)
            | super::ApiClientError::UnauthorizedReceived(bytes)
            | super::ApiClientError::NotFoundReceived(bytes)
            | super::ApiClientError::UnprocessableEntityReceived(bytes) => Some(bytes.clone()),
            _ => None,
        };
        self.change_context(super::ConnectorError::ProcessingStepFailed(data))
    }
}

pub(crate) trait ConnectorErrorExt {
    fn to_refund_failed_response(self) -> error_stack::Report<super::ApiErrorResponse>;
    fn to_payment_failed_response(self) -> error_stack::Report<super::ApiErrorResponse>;
    fn to_verify_failed_response(self) -> error_stack::Report<super::ApiErrorResponse>;
}

// FIXME: The implementation can be improved by handling BOM maybe?
impl ConnectorErrorExt for error_stack::Report<super::ConnectorError> {
    fn to_refund_failed_response(self) -> error_stack::Report<super::ApiErrorResponse> {
        let data = match self.current_context() {
            super::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
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
        self.change_context(super::ApiErrorResponse::RefundFailed { data })
    }

    fn to_payment_failed_response(self) -> error_stack::Report<super::ApiErrorResponse> {
        let data = match self.current_context() {
            super::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
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
        self.change_context(super::ApiErrorResponse::PaymentAuthorizationFailed { data })
    }

    fn to_verify_failed_response(self) -> error_stack::Report<super::ApiErrorResponse> {
        let data = match self.current_context() {
            super::ConnectorError::ProcessingStepFailed(Some(bytes)) => {
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
        self.change_context(super::ApiErrorResponse::PaymentAuthorizationFailed { data })
    }
}
