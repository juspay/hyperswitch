use std::sync::{Arc, Weak};

use crate::types::{Metadata, NodeValue, Relation, RelationResolution, ValueNode};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "predecessor", rename_all = "snake_case")]
pub enum ValueTracePredecessor<V: ValueNode> {
    Mandatory(Box<Weak<AnalysisTrace<V>>>),
    OneOf(Vec<Weak<AnalysisTrace<V>>>),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "trace", rename_all = "snake_case")]
pub enum AnalysisTrace<V: ValueNode> {
    Value {
        value: NodeValue<V>,
        relation: Relation,
        predecessors: Option<ValueTracePredecessor<V>>,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn Metadata>>,
    },

    AllAggregation {
        unsatisfied: Vec<Weak<AnalysisTrace<V>>>,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn Metadata>>,
    },

    AnyAggregation {
        unsatisfied: Vec<Weak<AnalysisTrace<V>>>,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn Metadata>>,
    },

    InAggregation {
        expected: Vec<V>,
        found: Option<V>,
        relation: Relation,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn Metadata>>,
    },
    Contradiction {
        relation: RelationResolution,
    },
}

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum GraphError<V: ValueNode> {
    #[error("An edge was not found in the graph")]
    EdgeNotFound,
    #[error("Attempted to create a conflicting edge between two nodes")]
    ConflictingEdgeCreated,
    #[error("Cycle detected in graph")]
    CycleDetected,
    #[error("Domain wasn't found in the Graph")]
    DomainNotFound,
    #[error("Malformed Graph: {reason}")]
    MalformedGraph { reason: String },
    #[error("A node was not found in the graph")]
    NodeNotFound,
    #[error("A value node was not found: {0:#?}")]
    ValueNodeNotFound(V),
    #[error("No values provided for an 'in' aggregator node")]
    NoInAggregatorValues,
    #[error("Error during analysis: {0:#?}")]
    AnalysisError(Weak<AnalysisTrace<V>>),
}

impl<V: ValueNode> GraphError<V> {
    pub fn get_analysis_trace(self) -> Result<Weak<AnalysisTrace<V>>, Self> {
        match self {
            Self::AnalysisError(trace) => Ok(trace),
            _ => Err(self),
        }
    }
}
