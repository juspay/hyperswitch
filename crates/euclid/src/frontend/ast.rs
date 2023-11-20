pub mod lowering;
#[cfg(feature = "ast_parser")]
pub mod parser;

use serde::{Deserialize, Serialize};

use crate::{
    enums::Connector,
    types::{DataType, Metadata},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConnectorChoice {
    pub connector: Connector,
    #[cfg(not(feature = "connector_choice_mca_id"))]
    pub sub_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetadataValue {
    pub key: String,
    pub value: String,
}

/// Represents a value in the DSL
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ValueType {
    /// Represents a number literal
    Number(i64),
    /// Represents an enum variant
    EnumVariant(String),
    /// Represents a Metadata variant
    MetadataVariant(MetadataValue),
    /// Represents a arbitrary String value
    StrValue(String),
    /// Represents an array of numbers. This is basically used for
    /// "one of the given numbers" operations
    /// eg: payment.method.amount = (1, 2, 3)
    NumberArray(Vec<i64>),
    /// Similar to NumberArray but for enum variants
    /// eg: payment.method.cardtype = (debit, credit)
    EnumVariantArray(Vec<String>),
    /// Like a number array but can include comparisons. Useful for
    /// conditions like "500 < amount < 1000"
    /// eg: payment.amount = (> 500, < 1000)
    NumberComparisonArray(Vec<NumberComparison>),
}

impl ValueType {
    pub fn get_type(&self) -> DataType {
        match self {
            Self::Number(_) => DataType::Number,
            Self::StrValue(_) => DataType::StrValue,
            Self::MetadataVariant(_) => DataType::MetadataValue,
            Self::EnumVariant(_) => DataType::EnumVariant,
            Self::NumberComparisonArray(_) => DataType::Number,
            Self::NumberArray(_) => DataType::Number,
            Self::EnumVariantArray(_) => DataType::EnumVariant,
        }
    }
}

/// Represents a number comparison for "NumberComparisonArrayValue"
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberComparison {
    pub comparison_type: ComparisonType,
    pub number: i64,
}

/// Conditional comparison type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonType {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
}

/// Represents a single comparison condition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    /// The left hand side which will always be a domain input identifier like "payment.method.cardtype"
    pub lhs: String,
    /// The comparison operator
    pub comparison: ComparisonType,
    /// The value to compare against
    pub value: ValueType,
    /// Additional metadata that the Static Analyzer and Backend does not touch.
    /// This can be used to store useful information for the frontend and is required for communication
    /// between the static analyzer and the frontend.
    pub metadata: Metadata,
}

/// Represents all the conditions of an IF statement
/// eg:
///
/// ```text
/// payment.method = card & payment.method.cardtype = debit & payment.method.network = diners
/// ```
pub type IfCondition = Vec<Comparison>;

/// Represents an IF statement with conditions and optional nested IF statements
///
/// ```text
/// payment.method = card {
///     payment.method.cardtype = (credit, debit) {
///         payment.method.network = (amex, rupay, diners)
///     }
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfStatement {
    pub condition: IfCondition,
    pub nested: Option<Vec<IfStatement>>,
}

/// Represents a rule
///
/// ```text
/// rule_name: [stripe, adyen, checkout]
/// {
///     payment.method = card {
///         payment.method.cardtype = (credit, debit) {
///             payment.method.network = (amex, rupay, diners)
///         }
///
///         payment.method.cardtype = credit
///     }
/// }
/// ```

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule<O> {
    pub name: String,
    #[serde(alias = "routingOutput")]
    pub connector_selection: O,
    pub statements: Vec<IfStatement>,
}

/// The program, having a default connector selection and
/// a bunch of rules. Also can hold arbitrary metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program<O> {
    pub default_selection: O,
    pub rules: Vec<Rule<O>>,
    pub metadata: Metadata,
}
