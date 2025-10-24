use common_types::{
    consts::{CUSTOMER_LIST_LOWER_LIMIT, CUSTOMER_LIST_UPPER_LIMIT},
    primitive_wrappers::CustomerListLimit,
};

use crate::core::errors::{self, CustomResult};

pub fn validate_customer_list_limit(
    limit: Option<u16>,
) -> CustomResult<CustomerListLimit, errors::ApiErrorResponse> {
    match limit {
        Some(l) => CustomerListLimit::new(l).map_err(|err| {
            errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    " limit should be between {} and {}: {}",
                    CUSTOMER_LIST_LOWER_LIMIT, CUSTOMER_LIST_UPPER_LIMIT, err
                ),
            }
            .into()
        }),
        None => Ok(CustomerListLimit::default()),
    }
}
