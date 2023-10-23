pub mod lowering;
#[cfg(feature = "ast_parser")]
pub mod parser;

#[cfg(feature = "connector_choice_bcompat")]
use std::hash;

use serde::{Deserialize, Serialize};

use crate::{
    enums::Connector,
    types::{DataType, Metadata},
};

#[cfg(feature = "connector_choice_bcompat")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConnectorChoiceKind {
    OnlyConnector,
    FullStruct,
}

#[cfg(feature = "connector_choice_bcompat")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConnectorChoiceSerde {
    OnlyConnector(Connector),
    FullStruct {
        connector: Connector,
        sub_label: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "connector_choice_bcompat",
    serde(from = "ConnectorChoiceSerde"),
    serde(into = "ConnectorChoiceSerde")
)]
#[cfg_attr(not(feature = "connector_choice_bcompat"), derive(PartialEq, Eq, Hash))]
pub struct ConnectorChoice {
    #[cfg(feature = "connector_choice_bcompat")]
    pub choice_kind: ConnectorChoiceKind,
    pub connector: Connector,
    pub sub_label: Option<String>,
}

#[cfg(feature = "connector_choice_bcompat")]
impl PartialEq for ConnectorChoice {
    fn eq(&self, other: &Self) -> bool {
        self.connector.eq(&other.connector) && self.sub_label.eq(&other.sub_label)
    }
}

#[cfg(feature = "connector_choice_bcompat")]
impl Eq for ConnectorChoice {}

#[cfg(feature = "connector_choice_bcompat")]
impl hash::Hash for ConnectorChoice {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.connector.hash(state);
        self.sub_label.hash(state);
    }
}

#[cfg(feature = "connector_choice_bcompat")]
impl From<ConnectorChoiceSerde> for ConnectorChoice {
    fn from(value: ConnectorChoiceSerde) -> Self {
        match value {
            ConnectorChoiceSerde::OnlyConnector(conn) => Self {
                choice_kind: ConnectorChoiceKind::OnlyConnector,
                connector: conn,
                sub_label: None,
            },

            ConnectorChoiceSerde::FullStruct {
                connector,
                sub_label,
            } => Self {
                choice_kind: ConnectorChoiceKind::FullStruct,
                connector,
                sub_label,
            },
        }
    }
}

#[cfg(feature = "connector_choice_bcompat")]
impl From<ConnectorChoice> for ConnectorChoiceSerde {
    fn from(value: ConnectorChoice) -> Self {
        match value.choice_kind {
            ConnectorChoiceKind::OnlyConnector => Self::OnlyConnector(value.connector),
            ConnectorChoiceKind::FullStruct => Self::FullStruct {
                connector: value.connector,
                sub_label: value.sub_label,
            },
        }
    }
}

/// Represents a connector volume split. This is basically a connector coupled with a percentage X
/// which denotes that X% of all requests should go through the given connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolSplit {
    pub connector: ConnectorChoice,
    pub split: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ConnectorSelection {
    Priority(Vec<ConnectorChoice>),
    VolumeSplit(Vec<VolSplit>),
}

impl ConnectorSelection {
    pub fn get_connector_list(&self) -> Vec<ConnectorChoice> {
        match self {
            Self::Priority(list) => list.clone(),
            Self::VolumeSplit(splits) => {
                splits.iter().map(|split| split.connector.clone()).collect()
            }
        }
    }
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
