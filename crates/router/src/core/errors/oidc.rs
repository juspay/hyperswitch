use common_utils::errors::CustomResult;

use crate::services::ApplicationResponse;

pub type OidcResult<T> = CustomResult<T, OidcErrors>;
pub type OidcResponse<T> = CustomResult<ApplicationResponse<T>, OidcErrors>;

#[derive(Debug, thiserror::Error)]
pub enum OidcErrors {
    #[error("Invalid OIDC request")]
    InvalidRequest,
    #[error("Unauthorized client")]
    UnauthorizedClient,
    #[error("Access denied")]
    AccessDenied,
    #[error("Invalid scope")]
    InvalidScope,
    #[error("OIDC server error")]
    ServerError,
    #[error("Invalid token request")]
    InvalidTokenRequest,
    #[error("Invalid client")]
    InvalidClient,
    #[error("Invalid grant")]
    InvalidGrant,
}

impl OidcErrors {
    /// Returns the RFC 6749 compliant error code
    fn get_rfc_error_code(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnauthorizedClient => "unauthorized_client",
            Self::AccessDenied => "access_denied",
            Self::InvalidScope => "invalid_scope",
            Self::ServerError => "server_error",
            Self::InvalidTokenRequest => "invalid_request",
            Self::InvalidClient => "invalid_client",
            Self::InvalidGrant => "invalid_grant",
        }
    }

    /// Returns the RFC 6749 compliant error description
    fn get_error_message(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "The request is missing a required parameter, includes an invalid parameter value, includes a parameter more than once, or is otherwise malformed",
            Self::UnauthorizedClient => "The client is not authorized to request an authorization code using this method",
            Self::AccessDenied => "The resource owner or authorization server denied the request",
            Self::InvalidScope => "The requested scope is invalid, unknown, or malformed",
            Self::ServerError => "The authorization server encountered an unexpected condition that prevented it from fulfilling the request",
            Self::InvalidTokenRequest => "The request is missing a required parameter, includes an invalid parameter value, includes a parameter more than once, or is otherwise malformed",
            Self::InvalidClient => "Client authentication failed",
            Self::InvalidGrant => "The provided authorization grant is invalid, expired, revoked, or does not match the redirection URI used in the authorization request",
        }
    }

    /// Returns a formatted RFC-compliant error message
    pub fn get_rfc_formatted_message(&self) -> String {
        format!(
            "{}: {}",
            self.get_rfc_error_code(),
            self.get_error_message()
        )
    }
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse> for OidcErrors {
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        let sub_code = "OI";

        match self {
            Self::InvalidRequest => AER::BadRequest(ApiError::new(
                sub_code,
                1,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::UnauthorizedClient => AER::Unauthorized(ApiError::new(
                sub_code,
                2,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::AccessDenied => AER::Unauthorized(ApiError::new(
                sub_code,
                3,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidScope => AER::BadRequest(ApiError::new(
                sub_code,
                4,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::ServerError => AER::InternalServerError(ApiError::new(
                sub_code,
                5,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidTokenRequest => AER::BadRequest(ApiError::new(
                sub_code,
                6,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidClient => AER::Unauthorized(ApiError::new(
                sub_code,
                7,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidGrant => AER::BadRequest(ApiError::new(
                sub_code,
                8,
                self.get_rfc_formatted_message(),
                None,
            )),
        }
    }
}
