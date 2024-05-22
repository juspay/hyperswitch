use std::{
    any::Any,
    fmt, hash, iter,
    ops::{self, Deref, DerefMut},
    sync::Arc,
};

use rustc_hash::{FxHashMap, FxHashSet};

macro_rules! default_get_size_impl {
    ($the_type:ty) => {
        impl GetSize for $the_type {
            fn get_size(&self) -> Size {
                Size(std::mem::size_of::<Self>())
            }
        }
    };
}

use crate::{dense_map::impl_entity, error::AnalysisTrace};

pub trait KeyNode: fmt::Debug + Clone + hash::Hash + serde::Serialize + PartialEq + Eq {}

pub trait ValueNode: fmt::Debug + Clone + hash::Hash + serde::Serialize + PartialEq + Eq {
    type Key: KeyNode;

    fn get_key(&self) -> Self::Key;
}

#[cfg(feature = "viz")]
pub trait NodeViz {
    fn viz(&self) -> String;
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NodeId(usize);

impl_entity!(NodeId);
default_get_size_impl!(NodeId);

#[derive(Debug)]
pub struct Node<V: ValueNode> {
    pub node_type: NodeType<V>,
    pub preds: Vec<EdgeId>,
    pub succs: Vec<EdgeId>,
}

impl<V: ValueNode> Node<V> {
    pub(crate) fn new(node_type: NodeType<V>) -> Self {
        Self {
            node_type,
            preds: Vec::new(),
            succs: Vec::new(),
        }
    }
}

impl<V> GetSize for Node<V>
where
    V: ValueNode + GetSize,
{
    fn get_size(&self) -> Size {
        self.node_type.get_size() + self.preds.get_size() + self.succs.get_size()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeType<V: ValueNode> {
    AllAggregator,
    AnyAggregator,
    InAggregator(FxHashSet<V>),
    Value(NodeValue<V>),
}

impl<V> GetSize for NodeType<V>
where
    V: ValueNode + GetSize,
{
    fn get_size(&self) -> Size {
        match self {
            Self::AllAggregator | Self::AnyAggregator | Self::Value(_) => {
                Size(std::mem::size_of::<Self>())
            }

            Self::InAggregator(set) => {
                Size(std::mem::size_of::<Self>()) + set.iter().map(|v| v.get_size()).sum()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NodeValue<V: ValueNode> {
    Key(<V as ValueNode>::Key),
    Value(V),
}

impl<V: ValueNode> From<V> for NodeValue<V> {
    fn from(value: V) -> Self {
        Self::Value(value)
    }
}

impl<V> GetSize for NodeValue<V>
where
    V: ValueNode,
{
    fn get_size(&self) -> Size {
        Size(std::mem::size_of::<Self>())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(usize);

impl_entity!(EdgeId);
default_get_size_impl!(EdgeId);

#[derive(
    Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, Hash, strum::Display, PartialOrd, Ord,
)]
pub enum Strength {
    Weak,
    Normal,
    Strong,
}

impl Strength {
    pub fn get_resolved_strength(prev_strength: Self, curr_strength: Self) -> Self {
        std::cmp::max(prev_strength, curr_strength)
    }
}

default_get_size_impl!(Strength);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, serde::Serialize)]
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

default_get_size_impl!(Relation);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, serde::Serialize)]
pub enum RelationResolution {
    Positive,
    Negative,
    Contradiction,
}

impl From<Relation> for RelationResolution {
    fn from(value: Relation) -> Self {
        match value {
            Relation::Positive => Self::Positive,
            Relation::Negative => Self::Negative,
        }
    }
}

impl RelationResolution {
    pub fn get_resolved_relation(prev_relation: Self, curr_relation: Self) -> Self {
        if prev_relation != curr_relation {
            Self::Contradiction
        } else {
            curr_relation
        }
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub strength: Strength,
    pub relation: Relation,
    pub pred: NodeId,
    pub succ: NodeId,
    pub domain: Option<DomainId>,
}

impl GetSize for Edge {
    fn get_size(&self) -> Size {
        self.strength.get_size()
            + self.relation.get_size()
            + self.pred.get_size()
            + self.succ.get_size()
            + self.domain.get_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomainId(usize);

impl_entity!(DomainId);
default_get_size_impl!(DomainId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomainIdentifier<'a>(&'a str);

impl<'a> DomainIdentifier<'a> {
    pub fn new(identifier: &'a str) -> Self {
        Self(identifier)
    }

    pub fn into_inner(&self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for DomainIdentifier<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> Deref for DomainIdentifier<'a> {
    type Target = str;

    fn deref(&self) -> &'a Self::Target {
        self.0
    }
}

impl<'a> GetSize for DomainIdentifier<'a> {
    fn get_size(&self) -> Size {
        Size(std::mem::size_of::<Self>())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainInfo<'a> {
    pub domain_identifier: DomainIdentifier<'a>,
    pub domain_description: String,
}

impl<'a> GetSize for DomainInfo<'a> {
    fn get_size(&self) -> Size {
        self.domain_identifier.get_size() + self.domain_description.get_size()
    }
}

pub trait CheckingContext {
    type Value: ValueNode;

    fn from_node_values<L>(vals: impl IntoIterator<Item = L>) -> Self
    where
        L: Into<Self::Value>;

    fn check_presence(&self, value: &NodeValue<Self::Value>, strength: Strength) -> bool;

    fn get_values_by_key(
        &self,
        expected: &<Self::Value as ValueNode>::Key,
    ) -> Option<Vec<Self::Value>>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Memoization<V: ValueNode>(
    #[allow(clippy::type_complexity)]
    FxHashMap<(NodeId, Relation, Strength), Result<(), Arc<AnalysisTrace<V>>>>,
);

impl<V: ValueNode> Memoization<V> {
    pub fn new() -> Self {
        Self(FxHashMap::default())
    }
}

impl<V: ValueNode> Default for Memoization<V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<V: ValueNode> Deref for Memoization<V> {
    type Target = FxHashMap<(NodeId, Relation, Strength), Result<(), Arc<AnalysisTrace<V>>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: ValueNode> DerefMut for Memoization<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub struct CycleCheck(FxHashMap<NodeId, (Strength, RelationResolution)>);
impl CycleCheck {
    pub fn new() -> Self {
        Self(FxHashMap::default())
    }
}

impl Default for CycleCheck {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for CycleCheck {
    type Target = FxHashMap<NodeId, (Strength, RelationResolution)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CycleCheck {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait Metadata: erased_serde::Serialize + Any + Send + Sync + fmt::Debug {}
erased_serde::serialize_trait_object!(Metadata);

impl<M> Metadata for M where M: erased_serde::Serialize + Any + Send + Sync + fmt::Debug {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size(usize);

impl ops::Add for Size {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Size {
    pub fn in_bytes(self) -> usize {
        self.0
    }

    pub fn in_kilobytes(self) -> usize {
        self.0 / 1000
    }
}

impl iter::Sum for Size {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(0), |a, b| a + b)
    }
}

impl<'a> iter::Sum<&'a Self> for Size {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self(0), |a, b| a + *b)
    }
}

pub trait GetSize {
    fn get_size(&self) -> Size;
}

impl<V> GetSize for Vec<V>
where
    V: GetSize,
{
    fn get_size(&self) -> Size {
        Size(std::mem::size_of::<Self>()) + self.iter().map(|v| v.get_size()).sum()
    }
}

impl<T> GetSize for Option<T>
where
    T: GetSize,
{
    fn get_size(&self) -> Size {
        match self {
            Some(v) => v.get_size(),
            None => Size(std::mem::size_of::<Self>()),
        }
    }
}

impl<K, V, S> GetSize for std::collections::HashMap<K, V, S>
where
    K: GetSize,
    V: GetSize,
{
    fn get_size(&self) -> Size {
        Size(std::mem::size_of::<Self>())
            + self.iter().map(|(k, v)| k.get_size() + v.get_size()).sum()
    }
}

impl GetSize for String {
    fn get_size(&self) -> Size {
        Size(std::mem::size_of::<Self>()) + Size(self.len())
    }
}

impl<'a> GetSize for &'a str {
    fn get_size(&self) -> Size {
        Size(std::mem::size_of::<Self>())
    }
}
