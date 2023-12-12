use api_models::errors::types::{ApiError, ApiErrorResponse};
use common_utils::errors::{CustomResult, ErrorSwitch, ErrorSwitchFrom};
use data_models::errors::StorageError;

pub type SampleDataResult<T> = CustomResult<T, SampleDataError>;

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
pub enum SampleDataError {
    #[error["Internal Server Error"]]
    InternalServerError,
    #[error("Data Does Not Exist")]
    DataDoesNotExist,
    #[error("Invalid Parameters")]
    InvalidParameters,
    #[error["Invalid Records"]]
    InvalidRange,
}

impl ErrorSwitch<ApiErrorResponse> for SampleDataError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::InternalServerError => ApiErrorResponse::InternalServerError(ApiError::new(
                "SD",
                0,
                "Something went wrong",
                None,
            )),
            Self::DataDoesNotExist => ApiErrorResponse::NotFound(ApiError::new(
                "SD",
                1,
                "Sample Data not present for given request",
                None,
            )),
            Self::InvalidParameters => ApiErrorResponse::BadRequest(ApiError::new(
                "SD",
                2,
                "Invalid parameters to generate Sample Data",
                None,
            )),
            Self::InvalidRange => ApiErrorResponse::BadRequest(ApiError::new(
                "SD",
                3,
                "Records to be generated should be between range 10 and 100",
                None,
            )),
        }
    }
}

impl ErrorSwitchFrom<StorageError> for SampleDataError {
    fn switch_from(error: &StorageError) -> Self {
        match matches!(error, StorageError::ValueNotFound(_)) {
            true => Self::DataDoesNotExist,
            false => Self::InternalServerError,
        }
    }
}
