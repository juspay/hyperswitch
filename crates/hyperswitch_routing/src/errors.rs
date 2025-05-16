use error_stack::Result;
use thiserror::Error;
use hyperswitch_domain_models::api::ApplicationResponse;
pub use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

// Define a generic error type for this crate
#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("DSL execution failed")]
    DslExecutionError,
    #[error("DSL parsing error")]
    DslParsingError,
    #[error("DSL backend initialization error")]
    DslBackendInitError,
    #[error("DSL missing in DB")]
    DslMissingInDb,
    #[error("DSL final connector selection failed")]
    DslFinalConnectorSelectionFailed,
    #[error("DSL incorrect selection algorithm")]
    DslIncorrectSelectionAlgorithm,
    #[error("Metadata parsing error")]
    MetadataParsingError,
    #[error("Fallback config fetch failed")]
    FallbackConfigFetchFailed,
    #[error("Connector selection failed")]
    ConnectorSelectionFailed,
    #[error("KGraph cache refresh failed")]
    KgraphCacheRefreshFailed,
    #[error("KGraph analysis error")]
    KgraphAnalysisError,
    #[error("Invalid connector name: {0}")]
    InvalidConnectorName(String),
    #[error("Generic not found error: {field}")]
    GenericNotFoundError { field: String },
    #[error("Deserialization error from {from} to {to}")]
    DeserializationError { from: String, to: String },
    #[error("Success rate client initialization error")]
    SuccessRateClientInitializationError,
    #[error("Success rate calculation error")]
    SuccessRateCalculationError,
    #[error("Success based routing config error")]
    SuccessBasedRoutingConfigError,
    #[error("Success based routing params not found")]
    SuccessBasedRoutingParamsNotFoundError,
    #[error("Elimination client initialization error")]
    EliminationClientInitializationError,
    #[error("Elimination routing config error")]
    EliminationRoutingConfigError,
    #[error("Elimination based routing params not found")]
    EliminationBasedRoutingParamsNotFoundError,
    #[error("Elimination routing calculation error")]
    EliminationRoutingCalculationError,
    #[error("Contract routing client initialization error")]
    ContractRoutingClientInitializationError,
    #[error("Contract based routing config error")]
    ContractBasedRoutingConfigError,
    #[error("Contract score calculation error: {err}")]
    ContractScoreCalculationError{ err: String },
    #[error("Contract score updation error")]
    ContractScoreUpdationError,
    #[error("Invalid success based connector label: {0}")]
    InvalidSuccessBasedConnectorLabel(String),
    #[error("Invalid elimination based connector label: {0}")]
    InvalidEliminationBasedConnectorLabel(String),
    #[error("Invalid contract based connector label: {0}")]
    InvalidContractBasedConnectorLabel(String),
    #[error("Open Router call failed for algorithm: {algo}")]
    OpenRouterCallFailed{ algo: String },
    #[error("Open Router error: {0}")]
    OpenRouterError(String),
    #[error("Generic conversion error from {from} to {to}")]
    GenericConversionError { from: String, to: String },
    #[error("Profile ID missing")]
    ProfileIdMissing,
    #[error("Invalid routing algorithm structure")]
    InvalidRoutingAlgorithmStructure,
    // Add other specific routing errors as needed
}

// Define a storage error type, possibly mirroring or wrapping storage_impl::errors::StorageError
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DbError(String), // Placeholder for more specific DB errors
    #[error("Value not found: {0}")]
    ValueNotFound(String),
    #[error("Mock DB error")]
    MockDbError,
    #[error("Deserialization failed")]
    DeserializationFailed,
    // Add other storage-related errors
}

// Define an API client error (if not using a shared one)
#[derive(Debug, Error)]
pub enum ApiClientError {
    #[error("API client request failed: {0}")]
    RequestFailed(String),
    #[error("API client response deserialization failed: {0}")]
    ResponseDeserializationFailed(String),
    #[error("Not Implemented")]
    NotImplemented,
    // Add other client-related errors
}

// Type alias for Results in this crate
pub type RouterResult<T> = common_utils::errors::CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = common_utils::errors::CustomResult<T, RoutingError>;
pub type AppResponse<T> = common_utils::errors::CustomResult<ApplicationResponse<T>, ApiErrorResponse>;
pub type StorageResult<T> = Result<T, StorageError>; // Specific for storage operations if needed



// Implement Clone for error types if needed for error_stack contexts
impl Clone for RoutingError {
    fn clone(&self) -> Self {
        match self {
            Self::DslExecutionError => Self::DslExecutionError,
            Self::DslParsingError => Self::DslParsingError,
            Self::DslBackendInitError => Self::DslBackendInitError,
            Self::DslMissingInDb => Self::DslMissingInDb,
            Self::DslFinalConnectorSelectionFailed => Self::DslFinalConnectorSelectionFailed,
            Self::DslIncorrectSelectionAlgorithm => Self::DslIncorrectSelectionAlgorithm,
            Self::MetadataParsingError => Self::MetadataParsingError,
            Self::FallbackConfigFetchFailed => Self::FallbackConfigFetchFailed,
            Self::ConnectorSelectionFailed => Self::ConnectorSelectionFailed,
            Self::KgraphCacheRefreshFailed => Self::KgraphCacheRefreshFailed,
            Self::KgraphAnalysisError => Self::KgraphAnalysisError,
            Self::InvalidConnectorName(s) => Self::InvalidConnectorName(s.clone()),
            Self::GenericNotFoundError{field} => Self::GenericNotFoundError{field: field.clone()},
            Self::DeserializationError{from, to} => Self::DeserializationError{from: from.clone(), to: to.clone()},
            Self::SuccessRateClientInitializationError => Self::SuccessRateClientInitializationError,
            Self::SuccessRateCalculationError => Self::SuccessRateCalculationError,
            Self::SuccessBasedRoutingConfigError => Self::SuccessBasedRoutingConfigError,
            Self::SuccessBasedRoutingParamsNotFoundError => Self::SuccessBasedRoutingParamsNotFoundError,
            Self::EliminationClientInitializationError => Self::EliminationClientInitializationError,
            Self::EliminationRoutingConfigError => Self::EliminationRoutingConfigError,
            Self::EliminationBasedRoutingParamsNotFoundError => Self::EliminationBasedRoutingParamsNotFoundError,
            Self::EliminationRoutingCalculationError => Self::EliminationRoutingCalculationError,
            Self::ContractRoutingClientInitializationError => Self::ContractRoutingClientInitializationError,
            Self::ContractBasedRoutingConfigError => Self::ContractBasedRoutingConfigError,
            Self::ContractScoreCalculationError{err} => Self::ContractScoreCalculationError{err: err.clone()},
            Self::ContractScoreUpdationError => Self::ContractScoreUpdationError,
            Self::InvalidSuccessBasedConnectorLabel(s) => Self::InvalidSuccessBasedConnectorLabel(s.clone()),
            Self::InvalidEliminationBasedConnectorLabel(s) => Self::InvalidEliminationBasedConnectorLabel(s.clone()),
            Self::InvalidContractBasedConnectorLabel(s) => Self::InvalidContractBasedConnectorLabel(s.clone()),
            Self::OpenRouterCallFailed{algo} => Self::OpenRouterCallFailed{algo: algo.clone()},
            Self::OpenRouterError(s) => Self::OpenRouterError(s.clone()),
            Self::GenericConversionError{from, to} => Self::GenericConversionError{from: from.clone(), to: to.clone()},
            Self::ProfileIdMissing => Self::ProfileIdMissing,
            Self::InvalidRoutingAlgorithmStructure => Self::InvalidRoutingAlgorithmStructure,
        }
    }
}
impl Clone for StorageError {
     fn clone(&self) -> Self {
        match self {
            Self::DbError(s) => Self::DbError(s.clone()),
            Self::ValueNotFound(s) => Self::ValueNotFound(s.clone()),
            Self::MockDbError => Self::MockDbError,
            Self::DeserializationFailed => Self::DeserializationFailed,
        }
    }
}
impl Clone for ApiClientError {
    fn clone(&self) -> Self {
        match self {
            Self::RequestFailed(s) => Self::RequestFailed(s.clone()),
            Self::ResponseDeserializationFailed(s) => Self::ResponseDeserializationFailed(s.clone()),
            Self::NotImplemented => Self::NotImplemented,
        }
    }
}
