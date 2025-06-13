use error_stack::Result;
use thiserror::Error;
use hyperswitch_domain_models::api as domain_api;
pub use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;

// Define a generic error type for this crate
#[derive(Debug, Clone, Error)]
pub enum RoutingError {
    #[error("Merchant routing algorithm not found in cache")]
    CacheMiss,
    #[error("Final connector selection failed")]
    ConnectorSelectionFailed,
    #[error("[DSL] Missing required field in payment data: '{field_name}'")]
    DslMissingRequiredField { field_name: String },
    #[error("The lock on the DSL cache is most probably poisoned")]
    DslCachePoisoned,
    #[error("Expected DSL to be saved in DB but did not find")]
    DslMissingInDb,
    #[error("Unable to parse DSL from JSON")]
    DslParsingError,
    #[error("Failed to initialize DSL backend")]
    DslBackendInitError,
    #[error("Error updating merchant with latest dsl cache contents")]
    DslMerchantUpdateError,
    #[error("Error executing the DSL")]
    DslExecutionError,
    #[error("Final connector selection failed")]
    DslFinalConnectorSelectionFailed,
    #[error("[DSL] Received incorrect selection algorithm as DSL output")]
    DslIncorrectSelectionAlgorithm,
    #[error("there was an error saving/retrieving values from the kgraph cache")]
    KgraphCacheFailure,
    #[error("failed to refresh the kgraph cache")]
    KgraphCacheRefreshFailed,
    #[error("there was an error during the kgraph analysis phase")]
    KgraphAnalysisError,
    #[error("'profile_id' was not provided")]
    ProfileIdMissing,
    #[error("the profile was not found in the database")]
    ProfileNotFound,
    #[error("failed to fetch the fallback config for the merchant")]
    FallbackConfigFetchFailed,
    #[error("Invalid connector name received: '{0}'")]
    InvalidConnectorName(String),
    #[error("The routing algorithm in merchant account had invalid structure")]
    InvalidRoutingAlgorithmStructure,
    #[error("Volume split failed")]
    VolumeSplitFailed,
    #[error("Unable to parse metadata")]
    MetadataParsingError,
    #[error("Unable to retrieve success based routing config")]
    SuccessBasedRoutingConfigError,
    #[error("Params not found in success based routing config")]
    SuccessBasedRoutingParamsNotFoundError,
    #[error("Unable to calculate success based routing config from dynamic routing service")]
    SuccessRateCalculationError,
    #[error("Success rate client from dynamic routing gRPC service not initialized")]
    SuccessRateClientInitializationError,
    #[error("Elimination client from dynamic routing gRPC service not initialized")]
    EliminationClientInitializationError,
    #[error("Unable to analyze elimination routing config from dynamic routing service")]
    EliminationRoutingCalculationError,
    #[error("Params not found in elimination based routing config")]
    EliminationBasedRoutingParamsNotFoundError,
    #[error("Unable to retrieve elimination based routing config")]
    EliminationRoutingConfigError,
    #[error(
        "Invalid elimination based connector label received from dynamic routing service: '{0}'"
    )]
    InvalidEliminationBasedConnectorLabel(String),
    #[error("Unable to convert from '{from}' to '{to}'")]
    GenericConversionError { from: String, to: String },
    #[error("Invalid success based connector label received from dynamic routing service: '{0}'")]
    InvalidSuccessBasedConnectorLabel(String),
    #[error("unable to find '{field}'")]
    GenericNotFoundError { field: String },
    #[error("Unable to deserialize from '{from}' to '{to}'")]
    DeserializationError { from: String, to: String },
    #[error("Unable to retrieve contract based routing config")]
    ContractBasedRoutingConfigError,
    #[error("Params not found in contract based routing config")]
    ContractBasedRoutingParamsNotFoundError,
    #[error("Unable to calculate contract score from dynamic routing service: '{err}'")]
    ContractScoreCalculationError { err: String },
    #[error("Unable to update contract score on dynamic routing service")]
    ContractScoreUpdationError,
    #[error("contract routing client from dynamic routing gRPC service not initialized")]
    ContractRoutingClientInitializationError,
    #[error("Invalid contract based connector label received from dynamic routing service: '{0}'")]
    InvalidContractBasedConnectorLabel(String),
    #[error("Failed to perform {algo} in open_router")]
    OpenRouterCallFailed { algo: String },
    #[error("Error from open_router: {0}")]
    OpenRouterError(String),
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
pub type AppResult<T> = common_utils::errors::CustomResult<domain_api::ApplicationResponse<T>, ApiErrorResponse>;
pub type ApplicationResult<T> = Result<T, common_enums::enums::ApplicationError>;
pub type ApplicationResponse<T> = ApplicationResult<domain_api::ApplicationResponse<T>>;
pub type StorageResult<T> = Result<T, StorageError>; // Specific for storage operations if needed

// Adding a type alias for RoutingResult for clarity
pub type RoutingResult<T> = common_utils::errors::CustomResult<T, RoutingError>;

// // Implementation of From trait for error conversion
// impl From<RoutingError> for ApiErrorResponse {
//     fn from(err: RoutingError) -> Self {
//         match err {
//             RoutingError::FallbackConfigFetchFailed => Self::InternalServerError,
//             RoutingError::ProfileIdMissing => Self::MissingRequiredField { field_name: "profile_id".to_string() },
//             RoutingError::ConnectorSelectionFailed => Self::InternalServerError,
//             // Map other errors as appropriate
//             _ => Self::InternalServerError,
//         }
//     }
// }

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
