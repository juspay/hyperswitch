#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConnectorError {
    #[error("Failed to obtain authentication type")]
    FailedToObtainAuthType,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: &'static str },
    #[error("Failed to execute a processing step: {0:?}")]
    ProcessingStepFailed(Option<bytes::Bytes>),
    #[error("Failed to deserialize connector response")]
    ResponseDeserializationFailed,
    #[error("Failed to encode connector request")]
    RequestEncodingFailed,
}

pub type CustomResult<T, E> = error_stack::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    #[error("Failed to parse enum: {0}")]
    EnumParseFailure(&'static str),
    #[error("Failed to parse struct: {0}")]
    StructParseFailure(&'static str),
    #[error("Failed to serialize to {0} format")]
    EncodeError(&'static str),
    #[error("Unknown error while parsing")]
    UnknownError,
}
