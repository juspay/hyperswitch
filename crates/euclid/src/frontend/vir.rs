//! Valued Intermediate Representation
use serde::{Deserialize, Serialize};

use crate::types::{EuclidValue, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValuedComparisonLogic {
    NegativeConjunction,
    PositiveDisjunction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuedComparison {
    pub values: Vec<EuclidValue>,
    pub logic: ValuedComparisonLogic,
    pub metadata: Metadata,
}

pub type ValuedIfCondition = Vec<ValuedComparison>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuedIfStatement {
    pub condition: ValuedIfCondition,
    pub nested: Option<Vec<ValuedIfStatement>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuedRule<O> {
    pub name: String,
    pub connector_selection: O,
    pub statements: Vec<ValuedIfStatement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuedProgram<O> {
    pub default_selection: O,
    pub rules: Vec<ValuedRule<O>>,
    pub metadata: Metadata,
}
