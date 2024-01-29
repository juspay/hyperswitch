use common_utils::errors::CustomResult;

use crate::services::ApplicationResponse;

pub type UserResult<T> = CustomResult<T, UserErrors>;
pub type UserResponse<T> = CustomResult<ApplicationResponse<T>, UserErrors>;
pub mod sample_data;

#[derive(Debug, thiserror::Error)]
pub enum UserErrors {
    #[error("User InternalServerError")]
    InternalServerError,
    #[error("InvalidCredentials")]
    InvalidCredentials,
    #[error("UserNotFound")]
    UserNotFound,
    #[error("UserExists")]
    UserExists,
    #[error("LinkInvalid")]
    LinkInvalid,
    #[error("UnverifiedUser")]
    UnverifiedUser,
    #[error("InvalidOldPassword")]
    InvalidOldPassword,
    #[error("EmailParsingError")]
    EmailParsingError,
    #[error("NameParsingError")]
    NameParsingError,
    #[error("PasswordParsingError")]
    PasswordParsingError,
    #[error("UserAlreadyVerified")]
    UserAlreadyVerified,
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
    #[error("MetadataAlreadySet")]
    MetadataAlreadySet,
    #[error("InvalidRoleId")]
    InvalidRoleId,
    #[error("InvalidRoleOperation")]
    InvalidRoleOperation,
    #[error("IpAddressParsingFailed")]
    IpAddressParsingFailed,
    #[error("InvalidMetadataRequest")]
    InvalidMetadataRequest,
    #[error("MerchantIdParsingError")]
    MerchantIdParsingError,
    #[error("ChangePasswordError")]
    ChangePasswordError,
    #[error("InvalidDeleteOperation")]
    InvalidDeleteOperation,
    #[error("MaxInvitationsError")]
    MaxInvitationsError,
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse> for UserErrors {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        let sub_code = "UR";
        match self {
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, self.get_error_message(), None))
            }
            Self::InvalidCredentials => {
                AER::Unauthorized(ApiError::new(sub_code, 1, self.get_error_message(), None))
            }
            Self::UserNotFound => {
                AER::Unauthorized(ApiError::new(sub_code, 2, self.get_error_message(), None))
            }
            Self::UserExists => {
                AER::BadRequest(ApiError::new(sub_code, 3, self.get_error_message(), None))
            }
            Self::LinkInvalid => {
                AER::Unauthorized(ApiError::new(sub_code, 4, self.get_error_message(), None))
            }
            Self::UnverifiedUser => {
                AER::Unauthorized(ApiError::new(sub_code, 5, self.get_error_message(), None))
            }
            Self::InvalidOldPassword => {
                AER::BadRequest(ApiError::new(sub_code, 6, self.get_error_message(), None))
            }
            Self::EmailParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 7, self.get_error_message(), None))
            }
            Self::NameParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 8, self.get_error_message(), None))
            }
            Self::PasswordParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 9, self.get_error_message(), None))
            }
            Self::UserAlreadyVerified => {
                AER::Unauthorized(ApiError::new(sub_code, 11, self.get_error_message(), None))
            }
            Self::CompanyNameParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 14, self.get_error_message(), None))
            }
            Self::MerchantAccountCreationError(error_message) => {
                AER::InternalServerError(ApiError::new(sub_code, 15, error_message, None))
            }
            Self::InvalidEmailError => {
                AER::BadRequest(ApiError::new(sub_code, 16, self.get_error_message(), None))
            }
            Self::MerchantIdNotFound => {
                AER::BadRequest(ApiError::new(sub_code, 18, self.get_error_message(), None))
            }
            Self::MetadataAlreadySet => {
                AER::BadRequest(ApiError::new(sub_code, 19, self.get_error_message(), None))
            }
            Self::DuplicateOrganizationId => AER::InternalServerError(ApiError::new(
                sub_code,
                21,
                self.get_error_message(),
                None,
            )),
            Self::InvalidRoleId => {
                AER::BadRequest(ApiError::new(sub_code, 22, self.get_error_message(), None))
            }
            Self::InvalidRoleOperation => {
                AER::BadRequest(ApiError::new(sub_code, 23, self.get_error_message(), None))
            }
            Self::IpAddressParsingFailed => AER::InternalServerError(ApiError::new(
                sub_code,
                24,
                self.get_error_message(),
                None,
            )),
            Self::InvalidMetadataRequest => {
                AER::BadRequest(ApiError::new(sub_code, 26, self.get_error_message(), None))
            }
            Self::MerchantIdParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 28, self.get_error_message(), None))
            }
            Self::ChangePasswordError => {
                AER::BadRequest(ApiError::new(sub_code, 29, self.get_error_message(), None))
            }
            Self::InvalidDeleteOperation => {
                AER::BadRequest(ApiError::new(sub_code, 30, self.get_error_message(), None))
            }
            Self::MaxInvitationsError => {
                AER::BadRequest(ApiError::new(sub_code, 31, self.get_error_message(), None))
            }
        }
    }
}

impl UserErrors {
    pub fn get_error_message(&self) -> &str {
        match self {
            Self::InternalServerError => "Something went wrong",
            Self::InvalidCredentials => "Incorrect email or password",
            Self::UserNotFound => "Email doesn’t exist. Register",
            Self::UserExists => "An account already exists with this email",
            Self::LinkInvalid => "Invalid or expired link",
            Self::UnverifiedUser => "Kindly verify your account",
            Self::InvalidOldPassword => "Old password incorrect. Please enter the correct password",
            Self::EmailParsingError => "Invalid Email",
            Self::NameParsingError => "Invalid Name",
            Self::PasswordParsingError => "Invalid Password",
            Self::UserAlreadyVerified => "User already verified",
            Self::CompanyNameParsingError => "Invalid Company Name",
            Self::MerchantAccountCreationError(error_message) => error_message,
            Self::InvalidEmailError => "Invalid Email",
            Self::MerchantIdNotFound => "Invalid Merchant ID",
            Self::MetadataAlreadySet => "Metadata already set",
            Self::DuplicateOrganizationId => "An Organization with the id already exists",
            Self::InvalidRoleId => "Invalid Role ID",
            Self::InvalidRoleOperation => "User Role Operation Not Supported",
            Self::IpAddressParsingFailed => "Something went wrong",
            Self::InvalidMetadataRequest => "Invalid Metadata Request",
            Self::MerchantIdParsingError => "Invalid Merchant Id",
            Self::ChangePasswordError => "Old and new password cannot be the same",
            Self::InvalidDeleteOperation => "Delete Operation Not Supported",
            Self::MaxInvitationsError => "Maximum invite count per request exceeded",
        }
    }
}
