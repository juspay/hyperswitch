pub mod builder;
mod dense_map;
pub mod error;
pub mod graph;
pub mod types;

pub use builder::ConstraintGraphBuilder;
pub use error::{AnalysisTrace, GraphError};
pub use graph::ConstraintGraph;
pub use types::{
    CheckingContext, CycleCheck, DomainId, DomainIdentifier, Edge, EdgeId, KeyNode, Memoization,
    Node, NodeId, NodeValue, Relation, Strength, ValueNode,
};
