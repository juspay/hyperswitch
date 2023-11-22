use euclid::dssa::{graph::GraphError, types::AnalysisErrorType};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum KgraphError {
    #[error("Invalid connector name encountered: '{0}'")]
    InvalidConnectorName(String),
    #[error("There was an error constructing the graph: {0}")]
    GraphConstructionError(GraphError),
    #[error("There was an error constructing the context")]
    ContextConstructionError(AnalysisErrorType),
    #[error("there was an unprecedented indexing error")]
    IndexingError,
}
