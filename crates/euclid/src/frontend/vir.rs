//! Valued Intermediate Representation
use crate::types::{EuclidValue, Metadata};

#[derive(Debug, Clone)]
pub enum ValuedComparisonLogic {
    NegativeConjunction,
    PositiveDisjunction,
}

#[derive(Clone, Debug)]
pub struct ValuedComparison {
    pub values: Vec<EuclidValue>,
    pub logic: ValuedComparisonLogic,
    pub metadata: Metadata,
}

pub type ValuedIfCondition = Vec<ValuedComparison>;

#[derive(Clone, Debug)]
pub struct ValuedIfStatement {
    pub condition: ValuedIfCondition,
    pub nested: Option<Vec<ValuedIfStatement>>,
}

#[derive(Clone, Debug)]
pub struct ValuedRule<O> {
    pub name: String,
    pub connector_selection: O,
    pub statements: Vec<ValuedIfStatement>,
}

#[derive(Clone, Debug)]
pub struct ValuedProgram<O> {
    pub default_selection: O,
    pub rules: Vec<ValuedRule<O>>,
    pub metadata: Metadata,
}
