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
    #[error("Unsupported response type")]
    UnsupportedResponseType,
    #[error("Invalid scope")]
    InvalidScope,
    #[error("OIDC server error")]
    ServerError,
    #[error("Service temporarily unavailable")]
    TemporarilyUnavailable,
    #[error("Invalid token request")]
    InvalidTokenRequest,
    #[error("Invalid client")]
    InvalidClient,
    #[error("Invalid grant")]
    InvalidGrant,
    #[error("Unauthorized client for grant type")]
    UnauthorizedClientForGrant,
    #[error("Unsupported grant type")]
    UnsupportedGrantType,
    #[error("Invalid token scope")]
    InvalidTokenScope,
}

impl OidcErrors {
    /// Returns the RFC 6749 compliant error code
    pub fn get_rfc_error_code(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnauthorizedClient => "unauthorized_client",
            Self::AccessDenied => "access_denied",
            Self::UnsupportedResponseType => "unsupported_response_type",
            Self::InvalidScope => "invalid_scope",
            Self::ServerError => "server_error",
            Self::TemporarilyUnavailable => "temporarily_unavailable",
            Self::InvalidTokenRequest => "invalid_request",
            Self::InvalidClient => "invalid_client",
            Self::InvalidGrant => "invalid_grant",
            Self::UnauthorizedClientForGrant => "unauthorized_client",
            Self::UnsupportedGrantType => "unsupported_grant_type",
            Self::InvalidTokenScope => "invalid_scope",
        }
    }

    /// Returns the RFC 6749 compliant error description
    pub fn get_error_message(&self) -> String {
        match self {
            Self::InvalidRequest => "The request is missing a required parameter, includes an invalid parameter value, includes a parameter more than once, or is otherwise malformed".to_string(),
            Self::UnauthorizedClient => "The client is not authorized to request an authorization code using this method".to_string(),
            Self::AccessDenied => "The resource owner or authorization server denied the request".to_string(),
            Self::UnsupportedResponseType => "The authorization server does not support obtaining an authorization code using this method".to_string(),
            Self::InvalidScope => "The requested scope is invalid, unknown, or malformed".to_string(),
            Self::ServerError => "The authorization server encountered an unexpected condition that prevented it from fulfilling the request".to_string(),
            Self::TemporarilyUnavailable => "The authorization server is currently unable to handle the request due to a temporary overloading or maintenance of the server".to_string(),
            Self::InvalidTokenRequest => "The request is missing a required parameter, includes an invalid parameter value, includes a parameter more than once, or is otherwise malformed".to_string(),
            Self::InvalidClient => "Client authentication failed".to_string(),
            Self::InvalidGrant => "The provided authorization grant is invalid, expired, revoked, or does not match the redirection URI used in the authorization request".to_string(),
            Self::UnauthorizedClientForGrant => "The authenticated client is not authorized to use this authorization grant type".to_string(),
            Self::UnsupportedGrantType => "The authorization grant type is not supported by the authorization server".to_string(),
            Self::InvalidTokenScope => "The requested scope is invalid, unknown, or malformed".to_string(),
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
            Self::UnsupportedResponseType => AER::BadRequest(ApiError::new(
                sub_code,
                4,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidScope => AER::BadRequest(ApiError::new(
                sub_code,
                5,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::ServerError => AER::InternalServerError(ApiError::new(
                sub_code,
                6,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::TemporarilyUnavailable => AER::InternalServerError(ApiError::new(
                sub_code,
                7,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidTokenRequest => AER::BadRequest(ApiError::new(
                sub_code,
                8,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidClient => AER::Unauthorized(ApiError::new(
                sub_code,
                9,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidGrant => AER::BadRequest(ApiError::new(
                sub_code,
                10,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::UnauthorizedClientForGrant => AER::Unauthorized(ApiError::new(
                sub_code,
                11,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::UnsupportedGrantType => AER::BadRequest(ApiError::new(
                sub_code,
                12,
                self.get_rfc_formatted_message(),
                None,
            )),
            Self::InvalidTokenScope => AER::BadRequest(ApiError::new(
                sub_code,
                13,
                self.get_rfc_formatted_message(),
                None,
            )),
        }
    }
}
