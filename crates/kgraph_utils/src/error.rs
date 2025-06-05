#[cfg(feature = "v2")]
use common_enums::connector_enums;
use euclid::{dssa::types::AnalysisErrorType, frontend::dir};
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum KgraphError {
    #[error("Invalid connector name encountered: '{0}'")]
    InvalidConnectorName(
        #[cfg(feature = "v1")] String,
        #[cfg(feature = "v2")] connector_enums::Connector,
    ),
    #[error("Error in domain creation")]
    DomainCreationError,
    #[error("There was an error constructing the graph: {0}")]
    GraphConstructionError(hyperswitch_constraint_graph::GraphError<dir::DirValue>),
    #[error("There was an error constructing the context")]
    ContextConstructionError(Box<AnalysisErrorType>),
    #[error("there was an unprecedented indexing error")]
    IndexingError,
}
