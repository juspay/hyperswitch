pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
pub use hyperswitch_domain_models::{
    api,
    errors::api_error_response::{self, *},
};

pub type SubscriptionResult<T> = CustomResult<T, ApiErrorResponse>;
