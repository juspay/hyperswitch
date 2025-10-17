use crate::core::errors::{self, CustomResult};

pub const CUSTOMER_LIST_LOWER_LIMIT: u16 = 1;
pub const CUSTOMER_LIST_UPPER_LIMIT: u16 = 100;
pub const CUSTOMER_LIST_DEFAULT_LIMIT: u16 = 10;

pub fn validate_customer_list_limit(
    limit: Option<u16>,
) -> CustomResult<u16, errors::ApiErrorResponse> {
    match limit {
        Some(l) if (CUSTOMER_LIST_LOWER_LIMIT..=CUSTOMER_LIST_UPPER_LIMIT).contains(&l) => Ok(l),
        Some(_) => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "limit should be between {} and {}",
                CUSTOMER_LIST_LOWER_LIMIT, CUSTOMER_LIST_UPPER_LIMIT
            ),
        }
        .into()),
        None => Ok(CUSTOMER_LIST_DEFAULT_LIMIT),
    }
}
