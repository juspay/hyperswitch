//! Errors returned by this crate.

/// Errors that could occur during OCI Vault KMS operations.
///
/// A plain error type rather than an `error_stack::Report`, so consuming services can wrap it
/// in whichever `error-stack` version they use. Each variant carries a human-readable detail.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OciKmsError {
    /// The [`OciKmsConfig`](crate::OciKmsConfig) is unusable, such as an invalid endpoint URL.
    #[error("invalid OCI KMS configuration: {0}")]
    InvalidConfig(String),

    /// The HTTP client couldn't be built.
    #[error("failed to build the OCI KMS HTTP client: {0}")]
    ClientCreationFailed(String),

    /// Signing credentials couldn't be obtained, from OKE Workload Identity inside Kubernetes
    /// or from the `oci` CLI config file outside it.
    #[error("OCI signing credentials unavailable: {0}")]
    CredentialsUnavailable(String),

    /// Constructing the OCI Signature v1 `Authorization` header failed.
    #[error("failed to sign the OCI request: {0}")]
    SigningFailed(String),

    /// The request couldn't be sent, or its response couldn't be read, after all retries.
    #[error("OCI KMS request failed: {0}")]
    RequestFailed(String),

    /// OCI answered with a non-success status, after all retries for retryable ones.
    #[error("OCI KMS returned HTTP {status}: {body}")]
    UnexpectedStatus {
        /// The HTTP status code.
        status: u16,
        /// The response body: OCI's `{"code", "message"}` error document.
        body: String,
    },

    /// OCI answered successfully, but with a body this client couldn't interpret.
    #[error("unexpected OCI KMS response: {0}")]
    InvalidResponse(String),
}
