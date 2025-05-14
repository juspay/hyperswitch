pub use hyperswitch_domain_models::ext_traits::OptionExt;

use crate::core::errors::{self, CustomResult};

pub trait ValidateCall<T, F> {
    fn validate_opt(self, func: F) -> CustomResult<(), errors::ValidationError>;
}

impl<T, F> ValidateCall<T, F> for Option<&T>
where
    F: Fn(&T) -> CustomResult<(), errors::ValidationError>,
{
    fn validate_opt(self, func: F) -> CustomResult<(), errors::ValidationError> {
        match self {
            Some(val) => func(val),
            None => Ok(()),
        }
    }
}
