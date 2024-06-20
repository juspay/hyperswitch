use common_utils::errors::ErrorSwitch;
use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

use super::{CustomersErrorResponse, StorageError};

impl ErrorSwitch<api_models::errors::types::ApiErrorResponse> for CustomersErrorResponse {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        match self {
            Self::CustomerRedacted => AER::BadRequest(ApiError::new(
                "IR",
                11,
                "Customer has already been redacted",
                None,
            )),
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, "Something went wrong", None))
            }
            Self::MandateActive => AER::BadRequest(ApiError::new(
                "IR",
                10,
                "Customer has active mandate/subsciption",
                None,
            )),
            Self::CustomerNotFound => AER::NotFound(ApiError::new(
                "HE",
                2,
                "Customer does not exist in our records",
                None,
            )),
            Self::CustomerAlreadyExists => AER::BadRequest(ApiError::new(
                "IR",
                12,
                "Customer with the given `customer_id` already exists",
                None,
            )),
        }
    }
}

impl ErrorSwitch<CustomersErrorResponse> for StorageError {
    fn switch(&self) -> CustomersErrorResponse {
        use CustomersErrorResponse as CER;
        match self {
            err if err.is_db_not_found() => CER::CustomerNotFound,
            Self::CustomerRedacted => CER::CustomerRedacted,
            _ => CER::InternalServerError,
        }
    }
}

impl ErrorSwitch<CustomersErrorResponse> for common_utils::errors::CryptoError {
    fn switch(&self) -> CustomersErrorResponse {
        CustomersErrorResponse::InternalServerError
    }
}

impl ErrorSwitch<CustomersErrorResponse> for ApiErrorResponse {
    fn switch(&self) -> CustomersErrorResponse {
        use CustomersErrorResponse as CER;
        match self {
            Self::InternalServerError => CER::InternalServerError,
            Self::MandateActive => CER::MandateActive,
            Self::CustomerNotFound => CER::CustomerNotFound,
            _ => CER::InternalServerError,
        }
    }
}
