use thiserror::Error;

#[derive(Debug, Error)]
pub enum MicroserviceClientErrorKind {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Upstream error: status={status}, body={body}")]
    Upstream { status: u16, body: String },
    #[error("Serde error: {0}")]
    Serde(String),
    #[error("Response transform error: {0}")]
    ResponseTransform(String),
    #[error("Client specific error: {0}")]
    ClientSpecific(String),
}

#[derive(Debug, Error)]
#[error("Microservice client error for {operation}: {kind}")]
pub struct MicroserviceClientError {
    pub operation: String,
    pub kind: MicroserviceClientErrorKind,
}
