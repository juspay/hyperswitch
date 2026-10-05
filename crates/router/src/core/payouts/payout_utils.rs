use error_stack::report;

use crate::core::errors::{self, RouterResult};

pub(super) fn invalid_payout_request(
    message: &str,
) -> error_stack::Report<errors::ApiErrorResponse> {
    report!(errors::ApiErrorResponse::InvalidRequestData {
        message: message.to_owned(),
    })
}

pub(super) fn validate_payout_condition(invalid: bool, message: &str) -> RouterResult<()> {
    match invalid {
        true => Err(invalid_payout_request(message)),
        false => Ok(()),
    }
}
