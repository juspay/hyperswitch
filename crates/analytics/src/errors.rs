use api_models::errors::types::{ApiError, ApiErrorResponse};
use common_utils::errors::{CustomResult, ErrorSwitch};

pub type AnalyticsResult<T> = CustomResult<T, AnalyticsError>;

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
pub enum AnalyticsError {
    #[allow(dead_code)]
    #[error("Not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("Unknown Analytics Error")]
    UnknownError,
    #[error("Access Forbidden Analytics Error")]
    AccessForbiddenError,
    #[error("Failed to fetch currency exchange rate")]
    ForexFetchFailed,
    #[error("Missing email")]
    MissingEmail,
    #[error("Invalid URL scheme: {0}")]
    InvalidReturnUrl(String),
}

// Read back by hand under `deja` so the lambda seam can replay a recorded
// failure as the same variant. The derive cannot be used: `NotImplemented`
// holds a `&'static str`, which no recording can supply, and serde ties the
// whole enum to `'static` for it. That variant is left out of the reader, so a
// tape holding one fails to load rather than inventing a string. The call that
// is seamed only produces `UnknownError`.
#[cfg(feature = "deja")]
impl<'de> serde::Deserialize<'de> for AnalyticsError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        enum Recorded {
            UnknownError,
            AccessForbiddenError,
            ForexFetchFailed,
            MissingEmail,
            InvalidReturnUrl(String),
        }
        Ok(match Recorded::deserialize(deserializer)? {
            Recorded::UnknownError => Self::UnknownError,
            Recorded::AccessForbiddenError => Self::AccessForbiddenError,
            Recorded::ForexFetchFailed => Self::ForexFetchFailed,
            Recorded::MissingEmail => Self::MissingEmail,
            Recorded::InvalidReturnUrl(url) => Self::InvalidReturnUrl(url),
        })
    }
}

impl ErrorSwitch<ApiErrorResponse> for AnalyticsError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::NotImplemented(feature) => ApiErrorResponse::NotImplemented(ApiError::new(
                "IR",
                0,
                format!("{feature} is not implemented."),
                None,
            )),
            Self::UnknownError => ApiErrorResponse::InternalServerError(ApiError::new(
                "HE",
                0,
                "Something went wrong",
                None,
            )),
            Self::AccessForbiddenError => {
                ApiErrorResponse::Unauthorized(ApiError::new("IR", 0, "Access Forbidden", None))
            }
            Self::ForexFetchFailed => ApiErrorResponse::InternalServerError(ApiError::new(
                "HE",
                0,
                "Failed to fetch currency exchange rate",
                None,
            )),
            Self::MissingEmail => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                6,
                "Missing or invalid merchant email address.",
                None,
            )),
            Self::InvalidReturnUrl(invalid_url_err) => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                6,
                format!("Invalid return URL: {invalid_url_err}"),
                None,
            )),
        }
    }
}
