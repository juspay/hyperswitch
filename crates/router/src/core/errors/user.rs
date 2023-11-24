use common_utils::errors::CustomResult;

use crate::services::ApplicationResponse;

pub type UserResult<T> = CustomResult<T, UserErrors>;
pub type UserResponse<T> = CustomResult<ApplicationResponse<T>, UserErrors>;

#[derive(Debug, thiserror::Error)]
pub enum UserErrors {
    #[error("User InternalServerError")]
    InternalServerError,
    #[error("InvalidCredentials")]
    InvalidCredentials,
    #[error("UserExists")]
    UserExists,
    #[error("InvalidOldPassword")]
    InvalidOldPassword,
    #[error("EmailParsingError")]
    EmailParsingError,
    #[error("NameParsingError")]
    NameParsingError,
    #[error("PasswordParsingError")]
    PasswordParsingError,
    #[error("CompanyNameParsingError")]
    CompanyNameParsingError,
    #[error("MerchantAccountCreationError: {0}")]
    MerchantAccountCreationError(String),
    #[error("InvalidEmailError")]
    InvalidEmailError,
    #[error("DuplicateOrganizationId")]
    DuplicateOrganizationId,
    #[error("MerchantIdNotFound")]
    MerchantIdNotFound,
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse> for UserErrors {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        let sub_code = "UR";
        match self {
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, "Something Went Wrong", None))
            }
            Self::InvalidCredentials => AER::Unauthorized(ApiError::new(
                sub_code,
                1,
                "Incorrect email or password",
                None,
            )),
            Self::UserExists => AER::BadRequest(ApiError::new(
                sub_code,
                3,
                "An account already exists with this email",
                None,
            )),
            Self::InvalidOldPassword => AER::BadRequest(ApiError::new(
                sub_code,
                6,
                "Old password incorrect. Please enter the correct password",
                None,
            )),
            Self::EmailParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 7, "Invalid Email", None))
            }
            Self::NameParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 8, "Invalid Name", None))
            }
            Self::PasswordParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 9, "Invalid Password", None))
            }
            Self::CompanyNameParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 14, "Invalid Company Name", None))
            }
            Self::MerchantAccountCreationError(error_message) => {
                AER::InternalServerError(ApiError::new(sub_code, 15, error_message, None))
            }
            Self::InvalidEmailError => {
                AER::BadRequest(ApiError::new(sub_code, 16, "Invalid Email", None))
            }
            Self::DuplicateOrganizationId => AER::InternalServerError(ApiError::new(
                sub_code,
                21,
                "An Organization with the id already exists",
                None,
            )),
            Self::MerchantIdNotFound => {
                AER::BadRequest(ApiError::new(sub_code, 18, "Invalid Merchant ID", None))
            }
        }
    }
}
