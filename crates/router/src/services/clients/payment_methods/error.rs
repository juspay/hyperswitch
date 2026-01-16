use thiserror::Error;

#[derive(Debug, Error)]
pub enum PaymentMethodClientError {
    #[error("Invalid request for {operation}: {message}")]
    InvalidRequest { operation: String, message: String },

    #[error("Transport error for {operation}: {message}")]
    TransportError { operation: String, message: String },

    #[error("Upstream error for {operation}: status={status}, body={body}")]
    UpstreamError {
        operation: String,
        status: u16,
        body: String,
    },

    #[error("Serialization error for {operation}: {message}")]
    SerdeError { operation: String, message: String },

    #[error("Response transform error for {operation}: {message}")]
    ResponseTransformError { operation: String, message: String },
}
