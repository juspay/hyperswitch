use std::collections::{HashMap, HashSet};

use error_stack::ResultExt;
use euclid::frontend::ast;
use serde::{Deserialize, Serialize};

use super::RoutingResult;
use crate::{
    core::errors,
    routes::SessionState,
    services::{self, logger},
};

//TODO: will be converted to configs
const EUCLID_API_TIMEOUT: u64 = 5;
const EUCLID_BASE_URL: &str = "http://localhost:8082";
pub async fn perform_decision_euclid_routing(
    state: &SessionState,
    routing_request: &RoutingEvaluateRequest,
) -> RoutingResult<()> {
    let decision_engine_evaluate_url =
        format!("{}/{}", EUCLID_BASE_URL.to_string(), "routing/evaluate");

    logger::debug!("decision_engine_euclid: evaluate api call for euclid routing evaluation");

    let body = common_utils::request::RequestContent::Json(Box::new(routing_request.clone()));
    let request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&decision_engine_evaluate_url)
        .set_body(body)
        .build();

    logger::info!(decision_engine_euclid_request=?request,"decision_engine_euclid: api call for evaluate decision engine routing evaluate");
    let response = state
        .api_client
        .send_request(&state.clone(), request, Some(EUCLID_API_TIMEOUT), false)
        .await
        .change_context(errors::RoutingError::DslExecutionError)
        .attach_printable("decision_engine_euclid: evaluate api unresponsive")?;

    let euclid_response = response
        .json::<RoutingEvaluateResponse>()
        .await
        .change_context(errors::RoutingError::GenericConversionError {
            from: "ApiResponse".to_string(),
            to: "RoutingEvaluateResponse".to_string(),
        })
        .attach_printable(
            "decision_engine_euclid: Unable to parse response received from evaluate api",
        )?;

    logger::debug!(decision_engine_euclid_response=?euclid_response,"decision_engine_euclid");

    Ok(())
}

pub async fn create_de_routing_algo(
    state: &SessionState,
    routing_request: &RoutingRule,
) -> RoutingResult<String> {
    let decision_engine_create_url =
        format!("{}/{}", EUCLID_BASE_URL.to_string(), "routing/create");

    logger::debug!("decision_engine_euclid: create api call for euclid routing rule creation");

    let body = common_utils::request::RequestContent::Json(Box::new(routing_request.clone()));
    let request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&decision_engine_create_url)
        .set_body(body)
        .build();

    logger::info!(decision_engine_euclid_request=?request,"decision_engine_euclid: api call for create decision engine routing rule");
    let response = state
        .api_client
        .send_request(&state.clone(), request, Some(EUCLID_API_TIMEOUT), false)
        .await
        .change_context(errors::RoutingError::DslExecutionError)
        .attach_printable("decision_engine_euclid: create api unresponsive")?;

    logger::debug!(decision_engine_euclid_response=?response,"decision_engine_euclid");
    let euclid_response = response
        .json::<RoutingDictionaryRecord>()
        .await
        .change_context(errors::RoutingError::GenericConversionError {
            from: "ApiResponse".to_string(),
            to: "RoutingDictionaryRecord".to_string(),
        })
        .attach_printable(
            "decision_engine_euclid: Unable to parse response received from create api",
        )?;
    Ok(euclid_response.rule_id)
}

//TODO: temporary change will be refactored afterwards
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RoutingEvaluateRequest {
    pub created_by: String,
    pub parameters: HashMap<String, Option<ast::ValueType>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RoutingEvaluateResponse {
    pub status: String,
    pub output: serde_json::Value,
    pub evaluated_output: Vec<String>,
    pub eligible_connectors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetadataValue {
    pub key: String,
    pub value: String,
}

/// Represents a value in the DSL
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ValueType {
    /// Represents a number literal
    Number(u64),
    /// Represents an enum variant
    EnumVariant(String),
    /// Represents a Metadata variant
    MetadataVariant(MetadataValue),
    /// Represents a arbitrary String value
    StrValue(String),
    GlobalRef(String),
}

// impl ValueType {
//     pub fn get_type(&self) -> DataType {
//         match self {
//             Self::Number(_) => DataType::Number,
//             Self::StrValue(_) => DataType::StrValue,
//             Self::MetadataVariant(_) => DataType::MetadataValue,
//             Self::EnumVariant(_) => DataType::EnumVariant,
//             Self::GlobalRef(_) => DataType::GlobalRef,
//         }
//     }
// }

pub type Metadata = HashMap<String, serde_json::Value>;
/// Represents a number comparison for "NumberComparisonArrayValue"
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberComparison {
    pub comparison_type: ComparisonType,
    pub number: u64,
}

/// Conditional comparison type
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "snake_case")]
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
    // #[schema(value_type=HashMap<String, serde_json::Value>)]
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
    // #[schema(value_type=Vec<Comparison>)]
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
// #[aliases(RuleConnectorSelection = Rule<ConnectorSelection>)]
pub struct Rule {
    pub name: String,
    #[serde(alias = "routingType")]
    pub routing_type: RoutingType,
    #[serde(alias = "routingOutput")]
    pub output: Output,
    pub statements: Vec<IfStatement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoutingType {
    Priority,
    VolumeSplit,
    VolumeSplitPriority,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSplit<T> {
    pub split: u8,
    pub output: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Output {
    Priority(Vec<String>),
    VolumeSplit(Vec<VolumeSplit<String>>),
    VolumeSplitPriority(Vec<VolumeSplit<Vec<String>>>),
}

pub type Globals = HashMap<String, HashSet<ValueType>>;

/// The program, having a default connector selection and
/// a bunch of rules. Also can hold arbitrary metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// #[aliases(ProgramConnectorSelection = Program<ConnectorSelection>)]
pub struct Program {
    pub globals: Globals,
    pub default_selection: Output,
    // #[schema(value_type=RuleConnectorSelection)]
    pub rules: Vec<Rule>,
    // #[schema(value_type=HashMap<String, serde_json::Value>)]
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingRule {
    pub name: String,
    pub created_by: String,
    pub algorithm: Program,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingDictionaryRecord {
    pub rule_id: String,
    pub name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

use api_models::routing::{ConnectorSelection, RoutableConnectorChoice};
impl From<ast::Program<ConnectorSelection>> for Program {
    fn from(p: ast::Program<ConnectorSelection>) -> Self {
        Program {
            globals: HashMap::new(),
            default_selection: convert_output(p.default_selection),
            rules: p.rules.into_iter().map(convert_rule).collect(),
            metadata: Some(p.metadata),
        }
    }
}

fn convert_rule(rule: ast::Rule<ConnectorSelection>) -> Rule {
    Rule {
        name: rule.name,
        routing_type: RoutingType::Priority,
        output: convert_output(rule.connector_selection),
        statements: rule.statements.into_iter().map(convert_if_stmt).collect(),
    }
}

fn convert_if_stmt(stmt: ast::IfStatement) -> IfStatement {
    IfStatement {
        condition: stmt.condition.into_iter().map(convert_comparison).collect(),
        nested: stmt
            .nested
            .map(|v| v.into_iter().map(convert_if_stmt).collect()),
    }
}

fn convert_comparison(c: ast::Comparison) -> Comparison {
    Comparison {
        lhs: c.lhs,
        comparison: convert_comparison_type(c.comparison),
        value: convert_value(c.value),
        metadata: c.metadata,
    }
}

fn convert_comparison_type(ct: ast::ComparisonType) -> ComparisonType {
    match ct {
        ast::ComparisonType::Equal => ComparisonType::Equal,
        ast::ComparisonType::NotEqual => ComparisonType::NotEqual,
        ast::ComparisonType::LessThan => ComparisonType::LessThan,
        ast::ComparisonType::LessThanEqual => ComparisonType::LessThanEqual,
        ast::ComparisonType::GreaterThan => ComparisonType::GreaterThan,
        ast::ComparisonType::GreaterThanEqual => ComparisonType::GreaterThanEqual,
    }
}

fn convert_value(v: ast::ValueType) -> ValueType {
    use ast::ValueType::*;
    match v {
        Number(n) => ValueType::Number(n.get_amount_as_i64().try_into().unwrap()),
        EnumVariant(e) => ValueType::EnumVariant(e),
        MetadataVariant(m) => ValueType::MetadataVariant(MetadataValue {
            key: m.key,
            value: m.value,
        }),
        StrValue(s) => ValueType::StrValue(s),
        _ => unimplemented!(), // GlobalRef(r) => ValueType::GlobalRef(r),
    }
}

fn convert_output(sel: ConnectorSelection) -> Output {
    match sel {
        ConnectorSelection::Priority(choices) => {
            Output::Priority(choices.into_iter().map(stringify_choice).collect())
        }
        ConnectorSelection::VolumeSplit(vs) => Output::VolumeSplit(
            vs.into_iter()
                .map(|v| VolumeSplit {
                    split: v.split,
                    output: stringify_choice(v.connector),
                })
                .collect(),
        ),
    }
}

fn stringify_choice(c: RoutableConnectorChoice) -> String {
    c.connector.to_string()
}
