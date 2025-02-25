pub type UserResult<T> = error_stack::Result<T, UserErrors>;

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