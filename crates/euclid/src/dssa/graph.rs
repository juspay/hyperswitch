use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use erased_serde::{self, Serialize as ErasedSerialize};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;

use crate::{
    dssa::types,
    frontend::dir,
    types::{DataType, Metadata},
    utils,
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash, strum::Display)]
pub enum Strength {
    Weak,
    Normal,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Positive,
    Negative,
}

impl From<Relation> for bool {
    fn from(value: Relation) -> Self {
        matches!(value, Relation::Positive)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl utils::EntityId for NodeId {
    #[inline]
    fn get_id(&self) -> usize {
        self.0
    }

    #[inline]
    fn with_id(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainInfo<'a> {
    pub domain_identifier: DomainIdentifier<'a>,
    pub domain_description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainIdentifier<'a>(&'a str);

impl<'a> DomainIdentifier<'a> {
    pub fn new(domain_identifier: &'a str) -> Self {
        Self(domain_identifier)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomainId(usize);

impl utils::EntityId for DomainId {
    #[inline]
    fn get_id(&self) -> usize {
        self.0
    }

    #[inline]
    fn with_id(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(usize);

impl utils::EntityId for EdgeId {
    #[inline]
    fn get_id(&self) -> usize {
        self.0
    }

    #[inline]
    fn with_id(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Memoization(FxHashMap<(NodeId, Relation, Strength), Result<(), Arc<AnalysisTrace>>>);

impl Memoization {
    pub fn new() -> Self {
        Self(FxHashMap::default())
    }
}

impl Default for Memoization {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Memoization {
    type Target = FxHashMap<(NodeId, Relation, Strength), Result<(), Arc<AnalysisTrace>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Memoization {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[derive(Debug, Clone)]
pub struct Edge {
    pub strength: Strength,
    pub relation: Relation,
    pub pred: NodeId,
    pub succ: NodeId,
}

#[derive(Debug)]
pub struct Node {
    pub node_type: NodeType,
    pub preds: Vec<EdgeId>,
    pub succs: Vec<EdgeId>,
    pub domain_ids: Vec<DomainId>,
}

impl Node {
    fn new(node_type: NodeType, domain_ids: Vec<DomainId>) -> Self {
        Self {
            node_type,
            preds: Vec::new(),
            succs: Vec::new(),
            domain_ids,
        }
    }
}

pub trait KgraphMetadata: ErasedSerialize + std::any::Any + Sync + Send + Debug {}
erased_serde::serialize_trait_object!(KgraphMetadata);

impl<M> KgraphMetadata for M where M: ErasedSerialize + std::any::Any + Sync + Send + Debug {}

#[derive(Debug)]
pub struct KnowledgeGraph<'a> {
    domain: utils::DenseMap<DomainId, DomainInfo<'a>>,
    nodes: utils::DenseMap<NodeId, Node>,
    edges: utils::DenseMap<EdgeId, Edge>,
    value_map: FxHashMap<NodeValue, NodeId>,
    node_info: utils::DenseMap<NodeId, Option<&'static str>>,
    node_metadata: utils::DenseMap<NodeId, Option<Arc<dyn KgraphMetadata>>>,
}

pub struct KnowledgeGraphBuilder<'a> {
    domain: utils::DenseMap<DomainId, DomainInfo<'a>>,
    nodes: utils::DenseMap<NodeId, Node>,
    edges: utils::DenseMap<EdgeId, Edge>,
    domain_identifier_map: FxHashMap<DomainIdentifier<'a>, DomainId>,
    value_map: FxHashMap<NodeValue, NodeId>,
    edges_map: FxHashMap<(NodeId, NodeId), EdgeId>,
    node_info: utils::DenseMap<NodeId, Option<&'static str>>,
    node_metadata: utils::DenseMap<NodeId, Option<Arc<dyn KgraphMetadata>>>,
}

impl<'a> Default for KnowledgeGraphBuilder<'a> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    AllAggregator,
    AnyAggregator,
    InAggregator(FxHashSet<dir::DirValue>),
    Value(NodeValue),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NodeValue {
    Key(dir::DirKey),
    Value(dir::DirValue),
}

impl From<dir::DirValue> for NodeValue {
    fn from(value: dir::DirValue) -> Self {
        Self::Value(value)
    }
}

impl From<dir::DirKey> for NodeValue {
    fn from(key: dir::DirKey) -> Self {
        Self::Key(key)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "predecessor", rename_all = "snake_case")]
pub enum ValueTracePredecessor {
    Mandatory(Box<Weak<AnalysisTrace>>),
    OneOf(Vec<Weak<AnalysisTrace>>),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "trace", rename_all = "snake_case")]
pub enum AnalysisTrace {
    Value {
        value: NodeValue,
        relation: Relation,
        predecessors: Option<ValueTracePredecessor>,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn KgraphMetadata>>,
    },

    AllAggregation {
        unsatisfied: Vec<Weak<AnalysisTrace>>,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn KgraphMetadata>>,
    },

    AnyAggregation {
        unsatisfied: Vec<Weak<AnalysisTrace>>,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn KgraphMetadata>>,
    },

    InAggregation {
        expected: Vec<dir::DirValue>,
        found: Option<dir::DirValue>,
        relation: Relation,
        info: Option<&'static str>,
        metadata: Option<Arc<dyn KgraphMetadata>>,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "details", rename_all = "snake_case")]
pub enum AnalysisError {
    Graph(GraphError),
    AssertionTrace {
        trace: Weak<AnalysisTrace>,
        metadata: Metadata,
    },
    NegationTrace {
        trace: Weak<AnalysisTrace>,
        metadata: Vec<Metadata>,
    },
}

impl AnalysisError {
    fn assertion_from_graph_error(metadata: &Metadata, graph_error: GraphError) -> Self {
        match graph_error {
            GraphError::AnalysisError(trace) => Self::AssertionTrace {
                trace,
                metadata: metadata.clone(),
            },

            other => Self::Graph(other),
        }
    }

    fn negation_from_graph_error(metadata: Vec<&Metadata>, graph_error: GraphError) -> Self {
        match graph_error {
            GraphError::AnalysisError(trace) => Self::NegationTrace {
                trace,
                metadata: metadata.iter().map(|m| (*m).clone()).collect(),
            },

            other => Self::Graph(other),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum GraphError {
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
    ValueNodeNotFound(dir::DirValue),
    #[error("No values provided for an 'in' aggregator node")]
    NoInAggregatorValues,
    #[error("Error during analysis: {0:#?}")]
    AnalysisError(Weak<AnalysisTrace>),
}

impl GraphError {
    fn get_analysis_trace(self) -> Result<Weak<AnalysisTrace>, Self> {
        match self {
            Self::AnalysisError(trace) => Ok(trace),
            _ => Err(self),
        }
    }
}

impl PartialEq<dir::DirValue> for NodeValue {
    fn eq(&self, other: &dir::DirValue) -> bool {
        match self {
            Self::Key(dir_key) => *dir_key == other.get_key(),
            Self::Value(dir_value) if dir_value.get_key() == other.get_key() => {
                if let (Some(left), Some(right)) =
                    (dir_value.get_num_value(), other.get_num_value())
                {
                    left.fits(&right)
                } else {
                    dir::DirValue::check_equality(dir_value, other)
                }
            }
            Self::Value(_) => false,
        }
    }
}

pub struct AnalysisContext {
    keywise_values: FxHashMap<dir::DirKey, FxHashSet<dir::DirValue>>,
}

impl AnalysisContext {
    pub fn from_dir_values(vals: impl IntoIterator<Item = dir::DirValue>) -> Self {
        let mut keywise_values: FxHashMap<dir::DirKey, FxHashSet<dir::DirValue>> =
            FxHashMap::default();

        for dir_val in vals {
            let key = dir_val.get_key();
            let set = keywise_values.entry(key).or_default();
            set.insert(dir_val);
        }

        Self { keywise_values }
    }

    fn check_presence(&self, value: &NodeValue, weak: bool) -> bool {
        match value {
            NodeValue::Key(k) => self.keywise_values.contains_key(k) || weak,
            NodeValue::Value(val) => {
                let key = val.get_key();
                let value_set = if let Some(set) = self.keywise_values.get(&key) {
                    set
                } else {
                    return weak;
                };

                match key.kind.get_type() {
                    DataType::EnumVariant | DataType::StrValue | DataType::MetadataValue => {
                        value_set.contains(val)
                    }
                    DataType::Number => val.get_num_value().map_or(false, |num_val| {
                        value_set.iter().any(|ctx_val| {
                            ctx_val
                                .get_num_value()
                                .map_or(false, |ctx_num_val| num_val.fits(&ctx_num_val))
                        })
                    }),
                }
            }
        }
    }

    pub fn insert(&mut self, value: dir::DirValue) {
        self.keywise_values
            .entry(value.get_key())
            .or_default()
            .insert(value);
    }

    pub fn remove(&mut self, value: dir::DirValue) {
        let set = self.keywise_values.entry(value.get_key()).or_default();

        set.remove(&value);

        if set.is_empty() {
            self.keywise_values.remove(&value.get_key());
        }
    }
}

impl<'a> KnowledgeGraphBuilder<'a> {
    pub fn new() -> Self {
        Self {
            domain: utils::DenseMap::new(),
            nodes: utils::DenseMap::new(),
            edges: utils::DenseMap::new(),
            domain_identifier_map: FxHashMap::default(),
            value_map: FxHashMap::default(),
            edges_map: FxHashMap::default(),
            node_info: utils::DenseMap::new(),
            node_metadata: utils::DenseMap::new(),
        }
    }

    pub fn build(self) -> KnowledgeGraph<'a> {
        KnowledgeGraph {
            domain: self.domain,
            nodes: self.nodes,
            edges: self.edges,
            value_map: self.value_map,
            node_info: self.node_info,
            node_metadata: self.node_metadata,
        }
    }

    pub fn make_domain(
        &mut self,
        domain_identifier: DomainIdentifier<'a>,
        domain_description: String,
    ) -> Result<DomainId, GraphError> {
        Ok(self
            .domain_identifier_map
            .clone()
            .get(&domain_identifier)
            .map_or_else(
                || {
                    let domain_id = self.domain.push(DomainInfo {
                        domain_identifier: domain_identifier.clone(),
                        domain_description,
                    });
                    self.domain_identifier_map
                        .insert(domain_identifier.clone(), domain_id);
                    domain_id
                },
                |domain_id| *domain_id,
            ))
    }

    pub fn make_value_node<M: KgraphMetadata>(
        &mut self,
        value: NodeValue,
        info: Option<&'static str>,
        domain_identifiers: Vec<DomainIdentifier<'_>>,
        metadata: Option<M>,
    ) -> Result<NodeId, GraphError> {
        match self.value_map.get(&value).copied() {
            Some(node_id) => Ok(node_id),
            None => {
                let mut domain_ids: Vec<DomainId> = Vec::new();
                domain_identifiers
                    .iter()
                    .try_for_each(|ident| {
                        self.domain_identifier_map
                            .get(ident)
                            .map(|id| domain_ids.push(*id))
                    })
                    .ok_or(GraphError::DomainNotFound)?;

                let node_id = self
                    .nodes
                    .push(Node::new(NodeType::Value(value.clone()), domain_ids));
                let _node_info_id = self.node_info.push(info);

                let _node_metadata_id = self
                    .node_metadata
                    .push(metadata.map(|meta| -> Arc<dyn KgraphMetadata> { Arc::new(meta) }));

                self.value_map.insert(value, node_id);
                Ok(node_id)
            }
        }
    }

    pub fn make_edge(
        &mut self,
        pred_id: NodeId,
        succ_id: NodeId,
        strength: Strength,
        relation: Relation,
    ) -> Result<EdgeId, GraphError> {
        self.ensure_node_exists(pred_id)?;
        self.ensure_node_exists(succ_id)?;
        self.edges_map
            .get(&(pred_id, succ_id))
            .copied()
            .and_then(|edge_id| self.edges.get(edge_id).cloned().map(|edge| (edge_id, edge)))
            .map_or_else(
                || {
                    let edge_id = self.edges.push(Edge {
                        strength,
                        relation,
                        pred: pred_id,
                        succ: succ_id,
                    });
                    self.edges_map.insert((pred_id, succ_id), edge_id);

                    let pred = self
                        .nodes
                        .get_mut(pred_id)
                        .ok_or(GraphError::NodeNotFound)?;
                    pred.succs.push(edge_id);

                    let succ = self
                        .nodes
                        .get_mut(succ_id)
                        .ok_or(GraphError::NodeNotFound)?;
                    succ.preds.push(edge_id);

                    Ok(edge_id)
                },
                |(edge_id, edge)| {
                    if edge.strength == strength && edge.relation == relation {
                        Ok(edge_id)
                    } else {
                        Err(GraphError::ConflictingEdgeCreated)
                    }
                },
            )
    }

    pub fn make_all_aggregator<M: KgraphMetadata>(
        &mut self,
        nodes: &[(NodeId, Relation, Strength)],
        info: Option<&'static str>,
        metadata: Option<M>,
        domain: Vec<DomainIdentifier<'_>>,
    ) -> Result<NodeId, GraphError> {
        nodes
            .iter()
            .try_for_each(|(node_id, _, _)| self.ensure_node_exists(*node_id))?;

        let mut domain_ids: Vec<DomainId> = Vec::new();
        domain
            .iter()
            .try_for_each(|ident| {
                self.domain_identifier_map
                    .get(ident)
                    .map(|id| domain_ids.push(*id))
            })
            .ok_or(GraphError::DomainNotFound)?;

        let aggregator_id = self
            .nodes
            .push(Node::new(NodeType::AllAggregator, domain_ids));
        let _aggregator_info_id = self.node_info.push(info);

        let _node_metadata_id = self
            .node_metadata
            .push(metadata.map(|meta| -> Arc<dyn KgraphMetadata> { Arc::new(meta) }));

        for (node_id, relation, strength) in nodes {
            self.make_edge(*node_id, aggregator_id, *strength, *relation)?;
        }

        Ok(aggregator_id)
    }

    pub fn make_any_aggregator<M: KgraphMetadata>(
        &mut self,
        nodes: &[(NodeId, Relation)],
        info: Option<&'static str>,
        metadata: Option<M>,
        domain: Vec<DomainIdentifier<'_>>,
    ) -> Result<NodeId, GraphError> {
        nodes
            .iter()
            .try_for_each(|(node_id, _)| self.ensure_node_exists(*node_id))?;

        let mut domain_ids: Vec<DomainId> = Vec::new();
        domain
            .iter()
            .try_for_each(|ident| {
                self.domain_identifier_map
                    .get(ident)
                    .map(|id| domain_ids.push(*id))
            })
            .ok_or(GraphError::DomainNotFound)?;

        let aggregator_id = self
            .nodes
            .push(Node::new(NodeType::AnyAggregator, domain_ids));
        let _aggregator_info_id = self.node_info.push(info);

        let _node_metadata_id = self
            .node_metadata
            .push(metadata.map(|meta| -> Arc<dyn KgraphMetadata> { Arc::new(meta) }));

        for (node_id, relation) in nodes {
            self.make_edge(*node_id, aggregator_id, Strength::Strong, *relation)?;
        }

        Ok(aggregator_id)
    }

    pub fn make_in_aggregator<M: KgraphMetadata>(
        &mut self,
        values: Vec<dir::DirValue>,
        info: Option<&'static str>,
        metadata: Option<M>,
        domain: Vec<DomainIdentifier<'_>>,
    ) -> Result<NodeId, GraphError> {
        let key = values
            .first()
            .ok_or(GraphError::NoInAggregatorValues)?
            .get_key();

        for val in &values {
            if val.get_key() != key {
                Err(GraphError::MalformedGraph {
                    reason: "Values for 'In' aggregator not of same key".to_string(),
                })?;
            }
        }

        let mut domain_ids: Vec<DomainId> = Vec::new();
        domain
            .iter()
            .try_for_each(|ident| {
                self.domain_identifier_map
                    .get(ident)
                    .map(|id| domain_ids.push(*id))
            })
            .ok_or(GraphError::DomainNotFound)?;

        let node_id = self.nodes.push(Node::new(
            NodeType::InAggregator(FxHashSet::from_iter(values)),
            domain_ids,
        ));
        let _aggregator_info_id = self.node_info.push(info);

        let _node_metadata_id = self
            .node_metadata
            .push(metadata.map(|meta| -> Arc<dyn KgraphMetadata> { Arc::new(meta) }));

        Ok(node_id)
    }

    fn ensure_node_exists(&self, id: NodeId) -> Result<(), GraphError> {
        if self.nodes.contains_key(id) {
            Ok(())
        } else {
            Err(GraphError::NodeNotFound)
        }
    }
}

impl<'a> KnowledgeGraph<'a> {
    fn check_node(
        &self,
        ctx: &AnalysisContext,
        node_id: NodeId,
        relation: Relation,
        strength: Strength,
        memo: &mut Memoization,
    ) -> Result<(), GraphError> {
        let node = self.nodes.get(node_id).ok_or(GraphError::NodeNotFound)?;
        if let Some(already_memo) = memo.get(&(node_id, relation, strength)) {
            already_memo
                .clone()
                .map_err(|err| GraphError::AnalysisError(Arc::downgrade(&err)))
        } else {
            match &node.node_type {
                NodeType::AllAggregator => {
                    let mut unsatisfied = Vec::<Weak<AnalysisTrace>>::new();

                    for edge_id in node.preds.iter().copied() {
                        let edge = self.edges.get(edge_id).ok_or(GraphError::EdgeNotFound)?;

                        if let Err(e) =
                            self.check_node(ctx, edge.pred, edge.relation, edge.strength, memo)
                        {
                            unsatisfied.push(e.get_analysis_trace()?);
                        }
                    }

                    if !unsatisfied.is_empty() {
                        let err = Arc::new(AnalysisTrace::AllAggregation {
                            unsatisfied,
                            info: self.node_info.get(node_id).cloned().flatten(),
                            metadata: self.node_metadata.get(node_id).cloned().flatten(),
                        });

                        memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                        Err(GraphError::AnalysisError(Arc::downgrade(&err)))
                    } else {
                        memo.insert((node_id, relation, strength), Ok(()));
                        Ok(())
                    }
                }

                NodeType::AnyAggregator => {
                    let mut unsatisfied = Vec::<Weak<AnalysisTrace>>::new();
                    let mut matched_one = false;

                    for edge_id in node.preds.iter().copied() {
                        let edge = self.edges.get(edge_id).ok_or(GraphError::EdgeNotFound)?;

                        if let Err(e) =
                            self.check_node(ctx, edge.pred, edge.relation, edge.strength, memo)
                        {
                            unsatisfied.push(e.get_analysis_trace()?);
                        } else {
                            matched_one = true;
                        }
                    }

                    if matched_one || node.preds.is_empty() {
                        memo.insert((node_id, relation, strength), Ok(()));
                        Ok(())
                    } else {
                        let err = Arc::new(AnalysisTrace::AnyAggregation {
                            unsatisfied: unsatisfied.clone(),
                            info: self.node_info.get(node_id).cloned().flatten(),
                            metadata: self.node_metadata.get(node_id).cloned().flatten(),
                        });

                        memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                        Err(GraphError::AnalysisError(Arc::downgrade(&err)))
                    }
                }

                NodeType::InAggregator(expected) => {
                    let the_key = expected
                        .iter()
                        .next()
                        .ok_or_else(|| GraphError::MalformedGraph {
                            reason:
                                "An OnlyIn aggregator node must have at least one expected value"
                                    .to_string(),
                        })?
                        .get_key();

                    let ctx_vals = if let Some(vals) = ctx.keywise_values.get(&the_key) {
                        vals
                    } else {
                        return if let Strength::Weak = strength {
                            memo.insert((node_id, relation, strength), Ok(()));
                            Ok(())
                        } else {
                            let err = Arc::new(AnalysisTrace::InAggregation {
                                expected: expected.iter().cloned().collect(),
                                found: None,
                                relation,
                                info: self.node_info.get(node_id).cloned().flatten(),
                                metadata: self.node_metadata.get(node_id).cloned().flatten(),
                            });

                            memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                            Err(GraphError::AnalysisError(Arc::downgrade(&err)))
                        };
                    };

                    let relation_bool: bool = relation.into();
                    for ctx_value in ctx_vals {
                        if expected.contains(ctx_value) != relation_bool {
                            let err = Arc::new(AnalysisTrace::InAggregation {
                                expected: expected.iter().cloned().collect(),
                                found: Some(ctx_value.clone()),
                                relation,
                                info: self.node_info.get(node_id).cloned().flatten(),
                                metadata: self.node_metadata.get(node_id).cloned().flatten(),
                            });

                            memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                            Err(GraphError::AnalysisError(Arc::downgrade(&err)))?;
                        }
                    }

                    memo.insert((node_id, relation, strength), Ok(()));
                    Ok(())
                }

                NodeType::Value(val) => {
                    let in_context = ctx.check_presence(val, matches!(strength, Strength::Weak));
                    let relation_bool: bool = relation.into();

                    if in_context != relation_bool {
                        let err = Arc::new(AnalysisTrace::Value {
                            value: val.clone(),
                            relation,
                            predecessors: None,
                            info: self.node_info.get(node_id).cloned().flatten(),
                            metadata: self.node_metadata.get(node_id).cloned().flatten(),
                        });

                        memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                        Err(GraphError::AnalysisError(Arc::downgrade(&err)))?;
                    }

                    if !relation_bool {
                        memo.insert((node_id, relation, strength), Ok(()));
                        return Ok(());
                    }

                    let mut errors = Vec::<Weak<AnalysisTrace>>::new();
                    let mut matched_one = false;

                    for edge_id in node.preds.iter().copied() {
                        let edge = self.edges.get(edge_id).ok_or(GraphError::EdgeNotFound)?;
                        let result =
                            self.check_node(ctx, edge.pred, edge.relation, edge.strength, memo);

                        match (edge.strength, result) {
                            (Strength::Strong, Err(trace)) => {
                                let err = Arc::new(AnalysisTrace::Value {
                                    value: val.clone(),
                                    relation,
                                    info: self.node_info.get(node_id).cloned().flatten(),
                                    metadata: self.node_metadata.get(node_id).cloned().flatten(),
                                    predecessors: Some(ValueTracePredecessor::Mandatory(Box::new(
                                        trace.get_analysis_trace()?,
                                    ))),
                                });
                                memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                                Err(GraphError::AnalysisError(Arc::downgrade(&err)))?;
                            }

                            (Strength::Strong, Ok(_)) => {
                                matched_one = true;
                            }

                            (Strength::Normal | Strength::Weak, Err(trace)) => {
                                errors.push(trace.get_analysis_trace()?);
                            }

                            (Strength::Normal | Strength::Weak, Ok(_)) => {
                                matched_one = true;
                            }
                        }
                    }

                    if matched_one || node.preds.is_empty() {
                        memo.insert((node_id, relation, strength), Ok(()));
                        Ok(())
                    } else {
                        let err = Arc::new(AnalysisTrace::Value {
                            value: val.clone(),
                            relation,
                            info: self.node_info.get(node_id).cloned().flatten(),
                            metadata: self.node_metadata.get(node_id).cloned().flatten(),
                            predecessors: Some(ValueTracePredecessor::OneOf(errors.clone())),
                        });

                        memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
                        Err(GraphError::AnalysisError(Arc::downgrade(&err)))
                    }
                }
            }
        }
    }

    fn key_analysis(
        &self,
        key: dir::DirKey,
        ctx: &AnalysisContext,
        memo: &mut Memoization,
    ) -> Result<(), GraphError> {
        self.value_map
            .get(&NodeValue::Key(key))
            .map_or(Ok(()), |node_id| {
                self.check_node(ctx, *node_id, Relation::Positive, Strength::Strong, memo)
            })
    }

    fn value_analysis(
        &self,
        val: dir::DirValue,
        ctx: &AnalysisContext,
        memo: &mut Memoization,
    ) -> Result<(), GraphError> {
        self.value_map
            .get(&NodeValue::Value(val))
            .map_or(Ok(()), |node_id| {
                self.check_node(ctx, *node_id, Relation::Positive, Strength::Strong, memo)
            })
    }

    pub fn check_value_validity(
        &self,
        val: dir::DirValue,
        analysis_ctx: &AnalysisContext,
        memo: &mut Memoization,
    ) -> Result<bool, GraphError> {
        let maybe_node_id = self.value_map.get(&NodeValue::Value(val));

        let node_id = if let Some(nid) = maybe_node_id {
            nid
        } else {
            return Ok(false);
        };

        let result = self.check_node(
            analysis_ctx,
            *node_id,
            Relation::Positive,
            Strength::Weak,
            memo,
        );

        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                e.get_analysis_trace()?;
                Ok(false)
            }
        }
    }

    pub fn key_value_analysis(
        &self,
        val: dir::DirValue,
        ctx: &AnalysisContext,
        memo: &mut Memoization,
    ) -> Result<(), GraphError> {
        self.key_analysis(val.get_key(), ctx, memo)
            .and_then(|_| self.value_analysis(val, ctx, memo))
    }

    fn assertion_analysis(
        &self,
        positive_ctx: &[(&dir::DirValue, &Metadata)],
        analysis_ctx: &AnalysisContext,
        memo: &mut Memoization,
    ) -> Result<(), AnalysisError> {
        positive_ctx.iter().try_for_each(|(value, metadata)| {
            self.key_value_analysis((*value).clone(), analysis_ctx, memo)
                .map_err(|e| AnalysisError::assertion_from_graph_error(metadata, e))
        })
    }

    fn negation_analysis(
        &self,
        negative_ctx: &[(&[dir::DirValue], &Metadata)],
        analysis_ctx: &mut AnalysisContext,
        memo: &mut Memoization,
    ) -> Result<(), AnalysisError> {
        let mut keywise_metadata: FxHashMap<dir::DirKey, Vec<&Metadata>> = FxHashMap::default();
        let mut keywise_negation: FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>> =
            FxHashMap::default();

        for (values, metadata) in negative_ctx {
            let mut metadata_added = false;

            for dir_value in *values {
                if !metadata_added {
                    keywise_metadata
                        .entry(dir_value.get_key())
                        .or_default()
                        .push(metadata);

                    metadata_added = true;
                }

                keywise_negation
                    .entry(dir_value.get_key())
                    .or_default()
                    .insert(dir_value);
            }
        }

        for (key, negation_set) in keywise_negation {
            let all_metadata = keywise_metadata.remove(&key).unwrap_or_default();
            let first_metadata = all_metadata.first().cloned().cloned().unwrap_or_default();

            self.key_analysis(key.clone(), analysis_ctx, memo)
                .map_err(|e| AnalysisError::assertion_from_graph_error(&first_metadata, e))?;

            let mut value_set = if let Some(set) = key.kind.get_value_set() {
                set
            } else {
                continue;
            };

            value_set.retain(|v| !negation_set.contains(v));

            for value in value_set {
                analysis_ctx.insert(value.clone());
                self.value_analysis(value.clone(), analysis_ctx, memo)
                    .map_err(|e| {
                        AnalysisError::negation_from_graph_error(all_metadata.clone(), e)
                    })?;
                analysis_ctx.remove(value);
            }
        }

        Ok(())
    }

    pub fn perform_context_analysis(
        &self,
        ctx: &types::ConjunctiveContext<'_>,
        memo: &mut Memoization,
    ) -> Result<(), AnalysisError> {
        let mut analysis_ctx = AnalysisContext::from_dir_values(
            ctx.iter()
                .filter_map(|ctx_val| ctx_val.value.get_assertion().cloned()),
        );

        let positive_ctx = ctx
            .iter()
            .filter_map(|ctx_val| {
                ctx_val
                    .value
                    .get_assertion()
                    .map(|val| (val, ctx_val.metadata))
            })
            .collect::<Vec<_>>();
        self.assertion_analysis(&positive_ctx, &analysis_ctx, memo)?;

        let negative_ctx = ctx
            .iter()
            .filter_map(|ctx_val| {
                ctx_val
                    .value
                    .get_negation()
                    .map(|vals| (vals, ctx_val.metadata))
            })
            .collect::<Vec<_>>();
        self.negation_analysis(&negative_ctx, &mut analysis_ctx, memo)?;

        Ok(())
    }

    pub fn combine<'b>(g1: &'b Self, g2: &'b Self) -> Result<Self, GraphError> {
        let mut node_builder = KnowledgeGraphBuilder::new();
        let mut g1_old2new_id = utils::DenseMap::<NodeId, NodeId>::new();
        let mut g2_old2new_id = utils::DenseMap::<NodeId, NodeId>::new();
        let mut g1_old2new_domain_id = utils::DenseMap::<DomainId, DomainId>::new();
        let mut g2_old2new_domain_id = utils::DenseMap::<DomainId, DomainId>::new();

        let add_domain = |node_builder: &mut KnowledgeGraphBuilder<'a>,
                          domain: DomainInfo<'a>|
         -> Result<DomainId, GraphError> {
            node_builder.make_domain(domain.domain_identifier, domain.domain_description)
        };

        let add_node = |node_builder: &mut KnowledgeGraphBuilder<'a>,
                        node: &Node,
                        domains: Vec<DomainIdentifier<'_>>|
         -> Result<NodeId, GraphError> {
            match &node.node_type {
                NodeType::Value(node_value) => {
                    node_builder.make_value_node(node_value.clone(), None, domains, None::<()>)
                }

                NodeType::AllAggregator => {
                    Ok(node_builder.make_all_aggregator(&[], None, None::<()>, domains)?)
                }

                NodeType::AnyAggregator => {
                    Ok(node_builder.make_any_aggregator(&[], None, None::<()>, Vec::new())?)
                }

                NodeType::InAggregator(expected) => Ok(node_builder.make_in_aggregator(
                    expected.iter().cloned().collect(),
                    None,
                    None::<()>,
                    Vec::new(),
                )?),
            }
        };

        for (_old_domain_id, domain) in g1.domain.iter() {
            let new_domain_id = add_domain(&mut node_builder, domain.clone())?;
            g1_old2new_domain_id.push(new_domain_id);
        }

        for (_old_domain_id, domain) in g2.domain.iter() {
            let new_domain_id = add_domain(&mut node_builder, domain.clone())?;
            g2_old2new_domain_id.push(new_domain_id);
        }

        for (_old_node_id, node) in g1.nodes.iter() {
            let mut domain_identifiers: Vec<DomainIdentifier<'_>> = Vec::new();
            for domain_id in &node.domain_ids {
                match g1.domain.get(*domain_id) {
                    Some(domain) => domain_identifiers.push(domain.domain_identifier.clone()),
                    None => return Err(GraphError::DomainNotFound),
                }
            }
            let new_node_id = add_node(&mut node_builder, node, domain_identifiers.clone())?;
            g1_old2new_id.push(new_node_id);
        }

        for (_old_node_id, node) in g2.nodes.iter() {
            let mut domain_identifiers: Vec<DomainIdentifier<'_>> = Vec::new();
            for domain_id in &node.domain_ids {
                match g2.domain.get(*domain_id) {
                    Some(domain) => domain_identifiers.push(domain.domain_identifier.clone()),
                    None => return Err(GraphError::DomainNotFound),
                }
            }
            let new_node_id = add_node(&mut node_builder, node, domain_identifiers.clone())?;
            g2_old2new_id.push(new_node_id);
        }

        for edge in g1.edges.values() {
            let new_pred_id = g1_old2new_id
                .get(edge.pred)
                .ok_or(GraphError::NodeNotFound)?;
            let new_succ_id = g1_old2new_id
                .get(edge.succ)
                .ok_or(GraphError::NodeNotFound)?;

            node_builder.make_edge(*new_pred_id, *new_succ_id, edge.strength, edge.relation)?;
        }

        for edge in g2.edges.values() {
            let new_pred_id = g2_old2new_id
                .get(edge.pred)
                .ok_or(GraphError::NodeNotFound)?;
            let new_succ_id = g2_old2new_id
                .get(edge.succ)
                .ok_or(GraphError::NodeNotFound)?;

            node_builder.make_edge(*new_pred_id, *new_succ_id, edge.strength, edge.relation)?;
        }

        Ok(node_builder.build())
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use euclid_macros::knowledge;

    use super::*;
    use crate::{dirval, frontend::dir::enums};

    #[test]
    fn test_strong_positive_relation_success() {
        let graph = knowledge! {crate
            PaymentMethod(Card) ->> CaptureMethod(Automatic);
            PaymentMethod(not Wallet)
                & PaymentMethod(not PayLater) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
            ]),
            memo,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_strong_positive_relation_failure() {
        let graph = knowledge! {crate
            PaymentMethod(Card) ->> CaptureMethod(Automatic);
            PaymentMethod(not Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([dirval!(CaptureMethod = Automatic)]),
            memo,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_strong_negative_relation_success() {
        let graph = knowledge! {crate
            PaymentMethod(Card) -> CaptureMethod(Automatic);
            PaymentMethod(not Wallet) ->> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
            ]),
            memo,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_strong_negative_relation_failure() {
        let graph = knowledge! {crate
            PaymentMethod(Card) -> CaptureMethod(Automatic);
            PaymentMethod(not Wallet) ->> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Wallet),
            ]),
            memo,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_normal_one_of_failure() {
        let graph = knowledge! {crate
            PaymentMethod(Card) -> CaptureMethod(Automatic);
            PaymentMethod(Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
        );
        assert!(matches!(
            *Weak::upgrade(&result.unwrap_err().get_analysis_trace().unwrap())
                .expect("Expected Arc"),
            AnalysisTrace::Value {
                predecessors: Some(ValueTracePredecessor::OneOf(_)),
                ..
            }
        ));
    }

    #[test]
    fn test_all_aggregator_success() {
        let graph = knowledge! {crate
            PaymentMethod(Card) & PaymentMethod(not Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentMethod = Card),
                dirval!(CaptureMethod = Automatic),
            ]),
            memo,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_all_aggregator_failure() {
        let graph = knowledge! {crate
            PaymentMethod(Card) & PaymentMethod(not Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_all_aggregator_mandatory_failure() {
        let graph = knowledge! {crate
            PaymentMethod(Card) & PaymentMethod(not Wallet) ->> CaptureMethod(Automatic);
        };
        let mut memo = Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
            ]),
            &mut memo,
        );

        assert!(matches!(
            *Weak::upgrade(&result.unwrap_err().get_analysis_trace().unwrap())
                .expect("Expected Arc"),
            AnalysisTrace::Value {
                predecessors: Some(ValueTracePredecessor::Mandatory(_)),
                ..
            }
        ));
    }

    #[test]
    fn test_in_aggregator_success() {
        let graph = knowledge! {crate
            PaymentMethod(in [Card, Wallet]) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
            ]),
            memo,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_in_aggregator_failure() {
        let graph = knowledge! {crate
            PaymentMethod(in [Card, Wallet]) -> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_not_in_aggregator_success() {
        let graph = knowledge! {crate
            PaymentMethod(not in [Card, Wallet]) ->> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
                dirval!(PaymentMethod = BankRedirect),
            ]),
            memo,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_not_in_aggregator_failure() {
        let graph = knowledge! {crate
            PaymentMethod(not in [Card, Wallet]) ->> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(PaymentMethod = Card),
            ]),
            memo,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_in_aggregator_failure_trace() {
        let graph = knowledge! {crate
            PaymentMethod(in [Card, Wallet]) ->> CaptureMethod(Automatic);
        };
        let memo = &mut Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
        );

        if let AnalysisTrace::Value {
            predecessors: Some(ValueTracePredecessor::Mandatory(agg_error)),
            ..
        } = Weak::upgrade(&result.unwrap_err().get_analysis_trace().unwrap())
            .expect("Expected arc")
            .deref()
        {
            assert!(matches!(
                *Weak::upgrade(agg_error.deref()).expect("Expected Arc"),
                AnalysisTrace::InAggregation {
                    found: Some(dir::DirValue::PaymentMethod(enums::PaymentMethod::PayLater)),
                    ..
                }
            ));
        } else {
            panic!("Failed unwrapping OnlyInAggregation trace from AnalysisTrace");
        }
    }

    #[test]
    fn _test_memoization_in_kgraph() {
        let mut builder = KnowledgeGraphBuilder::new();
        let _node_1 = builder.make_value_node(
            NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Wallet)),
            None,
            Vec::new(),
            None::<()>,
        );
        let _node_2 = builder.make_value_node(
            NodeValue::Value(dir::DirValue::BillingCountry(enums::BillingCountry::India)),
            None,
            Vec::new(),
            None::<()>,
        );
        let _node_3 = builder.make_value_node(
            NodeValue::Value(dir::DirValue::BusinessCountry(
                enums::BusinessCountry::UnitedStatesOfAmerica,
            )),
            None,
            Vec::new(),
            None::<()>,
        );
        let mut memo = Memoization::new();
        let _edge_1 = builder
            .make_edge(
                _node_1.expect("node1 constructtion failed"),
                _node_2.clone().expect("node2 construction failed"),
                Strength::Strong,
                Relation::Positive,
            )
            .expect("Failed to make an edge");
        let _edge_2 = builder
            .make_edge(
                _node_2.expect("node2 construction failed"),
                _node_3.clone().expect("node3 construction failed"),
                Strength::Strong,
                Relation::Positive,
            )
            .expect("Failed to an edge");
        let graph = builder.build();
        let _result = graph.key_value_analysis(
            dirval!(BusinessCountry = UnitedStatesOfAmerica),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentMethod = Wallet),
                dirval!(BillingCountry = India),
                dirval!(BusinessCountry = UnitedStatesOfAmerica),
            ]),
            &mut memo,
        );
        let _ans = memo
            .0
            .get(&(
                _node_3.expect("node3 construction failed"),
                Relation::Positive,
                Strength::Strong,
            ))
            .expect("Memoization not workng");
        matches!(_ans, Ok(()));
    }
}
