use thiserror::Error;

/// Error categories for the microservice client pipeline.
#[derive(Debug, Error)]
pub enum MicroserviceClientErrorKind {
    /// Request validation or request-shape errors.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Transport or network failures.
    #[error("Transport error: {0}")]
    Transport(String),

    /// Non-2xx response from upstream service.
    #[error("Upstream error: status={status}, body={body}")]
    Upstream {
        /// HTTP status code returned by the upstream service.
        status: u16,
        /// Response body for debugging.
        body: String,
    },

    /// Request serialization failures.
    #[error("Serialize error: {0}")]
    Serialize(String),

    /// Response deserialization failures.
    #[error("Deserialize error: {0}")]
    Deserialize(String),

    /// Failures while transforming upstream response into v1 output.
    #[error("Response transform error: {0}")]
    ResponseTransform(String),

    /// Client-specific errors not covered by the generic categories.
    #[error("Client specific error: {0}")]
    ClientSpecific(String),
}

/// Error wrapper carrying the operation name and error category.
#[derive(Debug, Error)]
#[error("Microservice client error for {operation}: {kind}")]
pub struct MicroserviceClientError {
    /// Operation name for logging and error attribution.
    pub operation: String,
    /// Error category for the failure.
    pub kind: MicroserviceClientErrorKind,
}
