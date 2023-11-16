use std::fmt;

use serde::Serialize;

use crate::{
    dssa::{self, graph},
    frontend::{ast, dir},
    types::{DataType, EuclidValue, Metadata},
};

pub trait EuclidAnalysable: Sized {
    fn get_dir_value_for_analysis(&self, rule_name: String) -> Vec<(dir::DirValue, Metadata)>;
}

#[derive(Debug, Clone)]
pub enum CtxValueKind<'a> {
    Assertion(&'a dir::DirValue),
    Negation(&'a [dir::DirValue]),
}

impl<'a> CtxValueKind<'a> {
    pub fn get_assertion(&self) -> Option<&dir::DirValue> {
        if let Self::Assertion(val) = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn get_negation(&self) -> Option<&[dir::DirValue]> {
        if let Self::Negation(vals) = self {
            Some(vals)
        } else {
            None
        }
    }

    pub fn get_key(&self) -> Option<dir::DirKey> {
        match self {
            Self::Assertion(val) => Some(val.get_key()),
            Self::Negation(vals) => vals.first().map(|v| (*v).get_key()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextValue<'a> {
    pub value: CtxValueKind<'a>,
    pub metadata: &'a Metadata,
}

impl<'a> ContextValue<'a> {
    #[inline]
    pub fn assertion(value: &'a dir::DirValue, metadata: &'a Metadata) -> Self {
        Self {
            value: CtxValueKind::Assertion(value),
            metadata,
        }
    }

    #[inline]
    pub fn negation(values: &'a [dir::DirValue], metadata: &'a Metadata) -> Self {
        Self {
            value: CtxValueKind::Negation(values),
            metadata,
        }
    }
}

pub type ConjunctiveContext<'a> = Vec<ContextValue<'a>>;

#[derive(Clone, Serialize)]
pub enum AnalyzeResult {
    AllOk,
}

#[derive(Debug, Clone, Serialize, thiserror::Error)]
pub struct AnalysisError {
    #[serde(flatten)]
    pub error_type: AnalysisErrorType,
    pub metadata: Metadata,
}
impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error_type.fmt(f)
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ValueData {
    pub value: dir::DirValue,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum AnalysisErrorType {
    #[error("Invalid program key given: '{0}'")]
    InvalidKey(String),
    #[error("Invalid variant '{got}' received for key '{key}'")]
    InvalidVariant {
        key: String,
        expected: Vec<String>,
        got: String,
    },
    #[error(
        "Invalid data type for value '{}' (expected {expected}, got {got})",
        key
    )]
    InvalidType {
        key: String,
        expected: DataType,
        got: DataType,
    },
    #[error("Invalid comparison '{operator:?}' for value type {value_type}")]
    InvalidComparison {
        operator: ast::ComparisonType,
        value_type: DataType,
    },
    #[error("Invalid value received for length as '{value}: {:?}'", message)]
    InvalidValue {
        key: dir::DirKeyKind,
        value: String,
        message: Option<String>,
    },
    #[error("Conflicting assertions received for key '{}'", .key.kind)]
    ConflictingAssertions {
        key: dir::DirKey,
        values: Vec<ValueData>,
    },

    #[error("Key '{}' exhaustively negated", .key.kind)]
    ExhaustiveNegation {
        key: dir::DirKey,
        metadata: Vec<Metadata>,
    },
    #[error("The condition '{value}' was asserted and negated in the same condition")]
    NegatedAssertion {
        value: dir::DirValue,
        assertion_metadata: Metadata,
        negation_metadata: Metadata,
    },
    #[error("Graph analysis error: {0:#?}")]
    GraphAnalysis(graph::AnalysisError, graph::Memoization),
    #[error("State machine error")]
    StateMachine(dssa::state_machine::StateMachineError),
    #[error("Unsupported program key '{0}'")]
    UnsupportedProgramKey(dir::DirKeyKind),
    #[error("Ran into an unimplemented feature")]
    NotImplemented,
    #[error("The payment method type is not supported under the payment method")]
    NotSupported,
}

#[derive(Debug, Clone)]
pub enum ValueType {
    EnumVariants(Vec<EuclidValue>),
    Number,
}
