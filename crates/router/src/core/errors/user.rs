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
    #[error("RoleNotFound")]
    RoleNotFound,
    #[error("InvalidRoleOperationWithMessage")]
    InvalidRoleOperationWithMessage(String),
    #[error("RoleNameParsingError")]
    RoleNameParsingError,
    #[error("RoleNameAlreadyExists")]
    RoleNameAlreadyExists,
    #[error("TotpNotSetup")]
    TotpNotSetup,
    #[error("InvalidTotp")]
    InvalidTotp,
    #[error("TotpRequired")]
    TotpRequired,
    #[error("InvalidRecoveryCode")]
    InvalidRecoveryCode,
    #[error("TwoFactorAuthRequired")]
    TwoFactorAuthRequired,
    #[error("TwoFactorAuthNotSetup")]
    TwoFactorAuthNotSetup,
    #[error("TOTP secret not found")]
    TotpSecretNotFound,
    #[error("User auth method already exists")]
    UserAuthMethodAlreadyExists,
    #[error("Invalid user auth method operation")]
    InvalidUserAuthMethodOperation,
    #[error("Auth config parsing error")]
    AuthConfigParsingError,
    #[error("Invalid SSO request")]
    SSOFailed,
    #[error("profile_id missing in JWT")]
    JwtProfileIdMissing,
    #[error("Maximum attempts reached for TOTP")]
    MaxTotpAttemptsReached,
    #[error("Maximum attempts reached for Recovery Code")]
    MaxRecoveryCodeAttemptsReached,
    #[error("Forbidden tenant id")]
    ForbiddenTenantId,
    #[error("Error Uploading file to Theme Storage")]
    ErrorUploadingFile,
    #[error("Error Retrieving file from Theme Storage")]
    ErrorRetrievingFile,
    #[error("Theme not found")]
    ThemeNotFound,
    #[error("Theme with lineage already exists")]
    ThemeAlreadyExists,
    #[error("Invalid field: {0} in lineage")]
    InvalidThemeLineage(String),
    #[error("Missing required field: email_config")]
    MissingEmailConfig,
    #[error("Invalid Auth Method Operation: {0}")]
    InvalidAuthMethodOperationWithMessage(String),
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
            Self::MerchantAccountCreationError(_) => AER::InternalServerError(ApiError::new(
                sub_code,
                15,
                self.get_error_message(),
                None,
            )),
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
            Self::RoleNotFound => {
                AER::BadRequest(ApiError::new(sub_code, 32, self.get_error_message(), None))
            }
            Self::InvalidRoleOperationWithMessage(_) => {
                AER::BadRequest(ApiError::new(sub_code, 33, self.get_error_message(), None))
            }
            Self::RoleNameParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 34, self.get_error_message(), None))
            }
            Self::RoleNameAlreadyExists => {
                AER::BadRequest(ApiError::new(sub_code, 35, self.get_error_message(), None))
            }
            Self::TotpNotSetup => {
                AER::BadRequest(ApiError::new(sub_code, 36, self.get_error_message(), None))
            }
            Self::InvalidTotp => {
                AER::BadRequest(ApiError::new(sub_code, 37, self.get_error_message(), None))
            }
            Self::TotpRequired => {
                AER::BadRequest(ApiError::new(sub_code, 38, self.get_error_message(), None))
            }
            Self::InvalidRecoveryCode => {
                AER::BadRequest(ApiError::new(sub_code, 39, self.get_error_message(), None))
            }
            Self::TwoFactorAuthRequired => {
                AER::BadRequest(ApiError::new(sub_code, 40, self.get_error_message(), None))
            }
            Self::TwoFactorAuthNotSetup => {
                AER::BadRequest(ApiError::new(sub_code, 41, self.get_error_message(), None))
            }
            Self::TotpSecretNotFound => {
                AER::BadRequest(ApiError::new(sub_code, 42, self.get_error_message(), None))
            }
            Self::UserAuthMethodAlreadyExists => {
                AER::BadRequest(ApiError::new(sub_code, 43, self.get_error_message(), None))
            }
            Self::InvalidUserAuthMethodOperation => {
                AER::BadRequest(ApiError::new(sub_code, 44, self.get_error_message(), None))
            }
            Self::AuthConfigParsingError => {
                AER::BadRequest(ApiError::new(sub_code, 45, self.get_error_message(), None))
            }
            Self::SSOFailed => {
                AER::BadRequest(ApiError::new(sub_code, 46, self.get_error_message(), None))
            }
            Self::JwtProfileIdMissing => {
                AER::Unauthorized(ApiError::new(sub_code, 47, self.get_error_message(), None))
            }
            Self::MaxTotpAttemptsReached => {
                AER::BadRequest(ApiError::new(sub_code, 48, self.get_error_message(), None))
            }
            Self::MaxRecoveryCodeAttemptsReached => {
                AER::BadRequest(ApiError::new(sub_code, 49, self.get_error_message(), None))
            }
            Self::ForbiddenTenantId => {
                AER::BadRequest(ApiError::new(sub_code, 50, self.get_error_message(), None))
            }
            Self::ErrorUploadingFile => AER::InternalServerError(ApiError::new(
                sub_code,
                51,
                self.get_error_message(),
                None,
            )),
            Self::ErrorRetrievingFile => AER::InternalServerError(ApiError::new(
                sub_code,
                52,
                self.get_error_message(),
                None,
            )),
            Self::ThemeNotFound => {
                AER::NotFound(ApiError::new(sub_code, 53, self.get_error_message(), None))
            }
            Self::ThemeAlreadyExists => {
                AER::BadRequest(ApiError::new(sub_code, 54, self.get_error_message(), None))
            }
            Self::InvalidThemeLineage(_) => {
                AER::BadRequest(ApiError::new(sub_code, 55, self.get_error_message(), None))
            }
            Self::MissingEmailConfig => {
                AER::BadRequest(ApiError::new(sub_code, 56, self.get_error_message(), None))
            }
            Self::InvalidAuthMethodOperationWithMessage(_) => {
                AER::BadRequest(ApiError::new(sub_code, 57, self.get_error_message(), None))
            }
        }
    }
}

impl UserErrors {
    pub fn get_error_message(&self) -> String {
        match self {
            Self::InternalServerError => "Something went wrong".to_string(),
            Self::InvalidCredentials => "Incorrect email or password".to_string(),
            Self::UserNotFound => "Email doesn’t exist. Register".to_string(),
            Self::UserExists => "An account already exists with this email".to_string(),
            Self::LinkInvalid => "Invalid or expired link".to_string(),
            Self::UnverifiedUser => "Kindly verify your account".to_string(),
            Self::InvalidOldPassword => {
                "Old password incorrect. Please enter the correct password".to_string()
            }
            Self::EmailParsingError => "Invalid Email".to_string(),
            Self::NameParsingError => "Invalid Name".to_string(),
            Self::PasswordParsingError => "Invalid Password".to_string(),
            Self::UserAlreadyVerified => "User already verified".to_string(),
            Self::CompanyNameParsingError => "Invalid Company Name".to_string(),
            Self::MerchantAccountCreationError(error_message) => error_message.to_string(),
            Self::InvalidEmailError => "Invalid Email".to_string(),
            Self::MerchantIdNotFound => "Invalid Merchant ID".to_string(),
            Self::MetadataAlreadySet => "Metadata already set".to_string(),
            Self::DuplicateOrganizationId => {
                "An Organization with the id already exists".to_string()
            }
            Self::InvalidRoleId => "Invalid Role ID".to_string(),
            Self::InvalidRoleOperation => "User Role Operation Not Supported".to_string(),
            Self::IpAddressParsingFailed => "Something went wrong".to_string(),
            Self::InvalidMetadataRequest => "Invalid Metadata Request".to_string(),
            Self::MerchantIdParsingError => "Invalid Merchant Id".to_string(),
            Self::ChangePasswordError => "Old and new password cannot be same".to_string(),
            Self::InvalidDeleteOperation => "Delete Operation Not Supported".to_string(),
            Self::MaxInvitationsError => "Maximum invite count per request exceeded".to_string(),
            Self::RoleNotFound => "Role Not Found".to_string(),
            Self::InvalidRoleOperationWithMessage(error_message) => error_message.to_string(),
            Self::RoleNameParsingError => "Invalid Role Name".to_string(),
            Self::RoleNameAlreadyExists => "Role name already exists".to_string(),
            Self::TotpNotSetup => "TOTP not setup".to_string(),
            Self::InvalidTotp => "Invalid TOTP".to_string(),
            Self::TotpRequired => "TOTP required".to_string(),
            Self::InvalidRecoveryCode => "Invalid Recovery Code".to_string(),
            Self::MaxTotpAttemptsReached => "Maximum attempts reached for TOTP".to_string(),
            Self::MaxRecoveryCodeAttemptsReached => {
                "Maximum attempts reached for Recovery Code".to_string()
            }
            Self::TwoFactorAuthRequired => "Two factor auth required".to_string(),
            Self::TwoFactorAuthNotSetup => "Two factor auth not setup".to_string(),
            Self::TotpSecretNotFound => "TOTP secret not found".to_string(),
            Self::UserAuthMethodAlreadyExists => "User auth method already exists".to_string(),
            Self::InvalidUserAuthMethodOperation => {
                "Invalid user auth method operation".to_string()
            }
            Self::AuthConfigParsingError => "Auth config parsing error".to_string(),
            Self::SSOFailed => "Invalid SSO request".to_string(),
            Self::JwtProfileIdMissing => "profile_id missing in JWT".to_string(),
            Self::ForbiddenTenantId => "Forbidden tenant id".to_string(),
            Self::ErrorUploadingFile => "Error Uploading file to Theme Storage".to_string(),
            Self::ErrorRetrievingFile => "Error Retrieving file from Theme Storage".to_string(),
            Self::ThemeNotFound => "Theme not found".to_string(),
            Self::ThemeAlreadyExists => "Theme with lineage already exists".to_string(),
            Self::InvalidThemeLineage(field_name) => {
                format!("Invalid field: {} in lineage", field_name)
            }
            Self::MissingEmailConfig => "Missing required field: email_config".to_string(),
            Self::InvalidAuthMethodOperationWithMessage(operation) => {
                format!("Invalid Auth Method Operation: {}", operation)
            }
        }
    }
}
