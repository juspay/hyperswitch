use thiserror::Error;

#[derive(Debug, Error)]
pub enum DrainerError {
    #[error("Error in parsing config : {0}")]
    ConfigParsingError(String),
    #[error("Error fetching stream length for stream : {0}")]
    StreamGetLengthError(String),
    #[error("Error reading from stream : {0}")]
    StreamReadError(String),
    #[error("Error triming from stream: {0}")]
    StreamTrimFailed(String),
    #[error("No entries found for stream: {0}")]
    NoStreamEntry(String),
    #[error("Error in making stream: {0} available")]
    DeleteKeyFailed(String),
    #[error("Error in messing up: {0} available")]
    MessudUp(String),
}
