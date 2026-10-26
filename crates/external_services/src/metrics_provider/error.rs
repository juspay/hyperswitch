#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("failed to send the metric request")]
    Transport,
    #[error("the metric request timed out")]
    Timeout,
    #[error("metric provider rejected the credential")]
    Unauthorized,
    #[error("metric provider responded with HTTP status {status}")]
    HttpStatus { status: u16 },
    #[error("metric provider refused the query: {reason}")]
    QueryRefused { reason: String },
    #[error("could not interpret the metric response")]
    MalformedResponse,
}
