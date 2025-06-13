use std::collections::{HashMap, HashSet};

use crate::errors::RouterResponse;
use crate::types;
use crate::{errors, state::RoutingState, transformers::ForeignInto};
use api_models::routing as api_routing;
use async_trait::async_trait;
use common_utils::id_type;
use error_stack::ResultExt;
use euclid::{backend::BackendInput, frontend::ast};
use router_env;
use serde::{Deserialize, Serialize};
use storage_impl::routing_algorithm::common_enums as enums;
use storage_impl::routing_algorithm::storage_models;

#[async_trait]
pub trait EuclidApiHandler {
    async fn send_euclid_request<Req, Res>(
        state: &RoutingState<'_>,
        http_method: common_utils::request::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
    ) -> RouterResponse<Res>
    where
        Req: Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static + std::fmt::Debug;

    async fn send_euclid_request_without_response_parsing<Req>(
        state: &RoutingState<'_>,
        http_method: common_utils::request::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
    ) -> RouterResponse<()>
    where
        Req: Serialize + Send + Sync + 'static;
}

pub struct EuclidApiClientStructure; // Renamed to avoid conflict with the field name

const EUCLID_BASE_URL: &str = "http://localhost:8082";
impl EuclidApiClientStructure {
    async fn build_and_send_euclid_http_request<Req>(
        state: &RoutingState<'_>,
        http_method: common_utils::request::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
        context_message: &str,
    ) -> RouterResponse<reqwest::Response>
    where
        Req: Serialize + Send + Sync + 'static,
    {
        let url = format!("{}/{}", EUCLID_BASE_URL, path);
        router_env::logger::debug!(euclid_api_call_url = %url, euclid_request_path = %path, http_method = ?http_method, "Initiating Euclid API call ({})", context_message);

        let mut request_builder = common_utils::request::RequestBuilder::new() // Assuming common_utils::request::RequestBuilder
            .method(http_method)
            .url(&url);

        if let Some(body_content) = request_body {
            let body = common_utils::request::RequestContent::Json(Box::new(body_content));
            request_builder = request_builder.set_body(body);
        }

        let http_request = request_builder
            .header(types::CONTENT_TYPE, "application/json".into())
            .header(
                common_utils::consts::TENANT_HEADER,
                state.tenant.tenant_id.get_string_repr(),
            )
            .build();

        router_env::logger::info!(?http_request, euclid_request_path = %path, "Constructed Euclid API request details ({})", context_message);

        state
            .api_client
            .send_request(state.conf.proxy.clone(), http_request, timeout, false)
            .await
            .change_context(errors::RoutingError::DslExecutionError)
            .attach_printable_lazy(|| {
                format!(
                    "Euclid API call to path '{}' unresponsive ({})",
                    path, context_message
                )
            })
    }
}

#[async_trait]
impl EuclidApiHandler for EuclidApiClientStructure {
    async fn send_euclid_request<Req, Res>(
        state: &RoutingState<'_>,
        http_method: common_utils::request::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
    ) -> RouterResponse<Res>
    where
        Req: Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static + std::fmt::Debug,
    {
        let response = Self::build_and_send_euclid_http_request(
            state,
            http_method,
            path,
            request_body,
            timeout,
            "parsing response",
        )
        .await?;
        router_env::logger::debug!(euclid_response = ?response, euclid_request_path = %path, "Received raw response from Euclid API");

        let parsed_response = response
            .json::<Res>()
            .await
            .change_context(errors::RoutingError::GenericConversionError {
                from: "ApiResponse".to_string(),
                to: std::any::type_name::<Res>().to_string(),
            })
            .attach_printable_lazy(|| {
                format!(
                    "Unable to parse response of type '{}' received from Euclid API path: {}",
                    std::any::type_name::<Res>(),
                    path
                )
            })?;
        router_env::logger::debug!(parsed_response = ?parsed_response, response_type = %std::any::type_name::<Res>(), euclid_request_path = %path, "Successfully parsed response from Euclid API");
        Ok(parsed_response)
    }

    async fn send_euclid_request_without_response_parsing<Req>(
        state: &RoutingState<'_>,
        http_method: common_utils::request::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
    ) -> RouterResponse<()>
    where
        Req: Serialize + Send + Sync + 'static,
    {
        let response = Self::build_and_send_euclid_http_request(
            state,
            http_method,
            path,
            request_body,
            timeout,
            "not parsing response",
        )
        .await?;

        router_env::logger::debug!(euclid_response = ?response, euclid_request_path = %path, "Received raw response from Euclid API");
        Ok(())
    }
}

//TODO: will be converted to configs
pub const EUCLID_API_TIMEOUT: u64 = 5;
// const EUCLID_BASE_URL: &str = "http://localhost:8082"; // This should come from config

// These functions will need to accept http_client and euclid_base_url
pub async fn perform_decision_euclid_routing(
    state: &RoutingState<'_>,
    input: BackendInput,
    created_by: String,
) -> RouterResponse<()> {
    router_env::logger::debug!(
        "decision_engine_euclid: evaluate api call for euclid routing evaluation"
    );

    let routing_request = convert_backend_input_to_routing_eval(created_by, input)?;

    let euclid_response: RoutingEvaluateResponse = EuclidApiClientStructure::send_euclid_request(
        state,
        common_utils::request::Method::Post,
        "routing/evaluate",
        Some(routing_request),
        Some(EUCLID_API_TIMEOUT),
    )
    .await?;

    router_env::logger::debug!(decision_engine_euclid_response=?euclid_response,"decision_engine_euclid");
    router_env::logger::debug!(decision_engine_euclid_selected_connector=?euclid_response.evaluated_output,"decision_engine_euclid");

    Ok(())
}

// ... (Similar refactoring for other functions: create_de_euclid_routing_algo, link_de_euclid_routing_algorithm, list_de_euclid_routing_algorithms)
// ... They will need to accept http_client and euclid_base_url ...

pub async fn create_de_euclid_routing_algo(
    state: &RoutingState<'_>,
    routing_request: &RoutingRule,
) -> RouterResponse<String> {
    router_env::logger::debug!(
        "decision_engine_euclid: create api call for euclid routing rule creation"
    );

    let euclid_response: RoutingDictionaryRecord = EuclidApiClientStructure::send_euclid_request(
        state,
        common_utils::request::Method::Post,
        "routing/create",
        Some(routing_request.clone()),
        Some(EUCLID_API_TIMEOUT),
    )
    .await?;

    router_env::logger::debug!(decision_engine_euclid_parsed_response=?euclid_response,"decision_engine_euclid");
    Ok(euclid_response.rule_id)
}

pub async fn link_de_euclid_routing_algorithm(
    state: &RoutingState<'_>,
    routing_request: ActivateRoutingConfigRequest,
) -> RouterResponse<()> {
    router_env::logger::debug!(
        "decision_engine_euclid: link api call for euclid routing algorithm"
    );

    EuclidApiClientStructure::send_euclid_request_without_response_parsing(
        state,
        common_utils::request::Method::Post,
        "routing/activate",
        Some(routing_request.clone()),
        Some(EUCLID_API_TIMEOUT),
    )
    .await?;

    router_env::logger::debug!(decision_engine_euclid_activated=?routing_request, "decision_engine_euclid: link_de_euclid_routing_algorithm completed");
    Ok(())
}

pub async fn list_de_euclid_routing_algorithms(
    state: &RoutingState<'_>,
    routing_list_request: ListRountingAlgorithmsRequest,
) -> RouterResponse<Vec<api_routing::RoutingDictionaryRecord>> {
    router_env::logger::debug!(
        "decision_engine_euclid: list api call for euclid routing algorithms"
    );
    let created_by = routing_list_request.created_by;
    let response: Vec<RoutingAlgorithmRecord> = EuclidApiClientStructure::send_euclid_request(
        state,
        common_utils::request::Method::Post, // Should this be GET if no body? Or POST if created_by is in body?
        format!("routing/list/{created_by}").as_str(),
        None::<()>, // Assuming no body for list
        Some(EUCLID_API_TIMEOUT),
    )
    .await?;

    Ok(response
        .into_iter()
        .map(storage_models::RoutingProfileMetadata::from) // diesel_models::routing_algorithm
        .map(ForeignInto::foreign_into) // This ForeignInto needs to be defined or available
        .collect::<Vec<_>>())
}

pub fn compare_and_log_result<T: RoutingEq<T> + Serialize>(
    de_result: Vec<T>,
    result: Vec<T>,
    flow: String,
) {
    let is_equal = de_result.len() == result.len()
        && de_result
            .iter()
            .zip(result.iter())
            .all(|(a, b)| T::is_equal(a, b));

    if is_equal {
        router_env::logger::info!(routing_flow=?flow, is_equal=?is_equal, "decision_engine_euclid");
    } else {
        router_env::logger::debug!(routing_flow=?flow, is_equal=?is_equal, de_response=?to_json_string(&de_result), hs_response=?to_json_string(&result), "decision_engine_euclid");
    }
}

pub trait RoutingEq<T> {
    fn is_equal(a: &T, b: &T) -> bool;
}

impl RoutingEq<api_routing::RoutingDictionaryRecord> for api_routing::RoutingDictionaryRecord {
    fn is_equal(
        a: &api_routing::RoutingDictionaryRecord,
        b: &api_routing::RoutingDictionaryRecord,
    ) -> bool {
        a.name == b.name
            && a.profile_id == b.profile_id
            && a.description == b.description
            && a.kind == b.kind
            && a.algorithm_for == b.algorithm_for
    }
}

pub fn to_json_string<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map_err(|_| errors::RoutingError::GenericConversionError {
            // errors local
            from: "T".to_string(),
            to: "JsonValue".to_string(),
        })
        .unwrap_or_default()
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ActivateRoutingConfigRequest {
    pub created_by: String,
    pub routing_algorithm_id: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ListRountingAlgorithmsRequest {
    pub created_by: String,
}

// Maps Hyperswitch `BackendInput` to a `RoutingEvaluateRequest` compatible with Decision Engine
pub fn convert_backend_input_to_routing_eval(
    created_by: String,
    input: BackendInput,
) -> RouterResponse<RoutingEvaluateRequest> {
    let mut params: HashMap<String, Option<ValueType>> = HashMap::new();

    // Payment
    params.insert(
        "amount".to_string(),
        Some(ValueType::Number(
            input
                .payment
                .amount
                .get_amount_as_i64()
                .try_into()
                .unwrap_or_default(),
        )),
    );
    params.insert(
        "currency".to_string(),
        Some(ValueType::EnumVariant(input.payment.currency.to_string())),
    );

    if let Some(auth_type) = input.payment.authentication_type {
        params.insert(
            "authentication_type".to_string(),
            Some(ValueType::EnumVariant(auth_type.to_string())),
        );
    }
    if let Some(bin) = input.payment.card_bin {
        params.insert("card_bin".to_string(), Some(ValueType::StrValue(bin)));
    }
    if let Some(capture_method) = input.payment.capture_method {
        params.insert(
            "capture_method".to_string(),
            Some(ValueType::EnumVariant(capture_method.to_string())),
        );
    }
    if let Some(country) = input.payment.business_country {
        params.insert(
            "business_country".to_string(),
            Some(ValueType::EnumVariant(country.to_string())),
        );
    }
    if let Some(country) = input.payment.billing_country {
        params.insert(
            "billing_country".to_string(),
            Some(ValueType::EnumVariant(country.to_string())),
        );
    }
    if let Some(label) = input.payment.business_label {
        params.insert(
            "business_label".to_string(),
            Some(ValueType::StrValue(label)),
        );
    }
    if let Some(sfu) = input.payment.setup_future_usage {
        params.insert(
            "setup_future_usage".to_string(),
            Some(ValueType::EnumVariant(sfu.to_string())),
        );
    }

    // PaymentMethod
    if let Some(pm) = input.payment_method.payment_method {
        params.insert(
            "payment_method".to_string(),
            Some(ValueType::EnumVariant(pm.to_string())),
        );
    }
    if let Some(pmt) = input.payment_method.payment_method_type {
        params.insert(
            "payment_method_type".to_string(),
            Some(ValueType::EnumVariant(pmt.to_string())),
        );
    }
    if let Some(network) = input.payment_method.card_network {
        params.insert(
            "card_network".to_string(),
            Some(ValueType::EnumVariant(network.to_string())),
        );
    }

    // Mandate
    if let Some(pt) = input.mandate.payment_type {
        params.insert(
            "payment_type".to_string(),
            Some(ValueType::EnumVariant(pt.to_string())),
        );
    }
    if let Some(mt) = input.mandate.mandate_type {
        params.insert(
            "mandate_type".to_string(),
            Some(ValueType::EnumVariant(mt.to_string())),
        );
    }
    if let Some(mat) = input.mandate.mandate_acceptance_type {
        params.insert(
            "mandate_acceptance_type".to_string(),
            Some(ValueType::EnumVariant(mat.to_string())),
        );
    }

    // Metadata
    if let Some(meta) = input.metadata {
        for (k, v) in meta.into_iter() {
            params.insert(
                k.clone(),
                Some(ValueType::MetadataVariant(MetadataValue {
                    key: k,
                    value: v,
                })),
            );
        }
    }

    Ok(RoutingEvaluateRequest {
        created_by,
        parameters: params,
    })
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RoutingEvaluateRequest {
    pub created_by: String,
    pub parameters: HashMap<String, Option<ValueType>>,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ValueType {
    Number(u64),
    EnumVariant(String),
    MetadataVariant(MetadataValue),
    StrValue(String),
    GlobalRef(String),
}

pub type Metadata = HashMap<String, serde_json::Value>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberComparison {
    pub comparison_type: ComparisonType,
    pub number: u64,
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Comparison {
    pub lhs: String,
    pub comparison: ComparisonType,
    pub value: ValueType,
    pub metadata: Metadata,
}

pub type IfCondition = Vec<Comparison>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfStatement {
    pub condition: IfCondition,
    pub nested: Option<Vec<IfStatement>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    pub globals: Globals,
    pub default_selection: Output,
    pub rules: Vec<Rule>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingRule {
    pub name: String,
    pub description: Option<String>,
    pub metadata: Option<RoutingMetadata>,
    pub created_by: String,
    pub algorithm: Program,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingMetadata {
    pub kind: enums::RoutingAlgorithmKind,
    pub algorithm_for: common_enums::enums::TransactionType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingDictionaryRecord {
    pub rule_id: String,
    pub name: String,
    pub created_at: time::PrimitiveDateTime,  // time crate
    pub modified_at: time::PrimitiveDateTime, // time crate
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateGatewayScoreResponse {
    pub response: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingAlgorithmRecord {
    pub id: id_type::RoutingId, // common_utils
    pub name: String,
    pub description: Option<String>,
    pub created_by: id_type::ProfileId, // common_utils
    pub algorithm_data: Program,
    pub metadata: Option<RoutingMetadata>,
    pub created_at: time::PrimitiveDateTime,  // time
    pub modified_at: time::PrimitiveDateTime, // time
}

impl From<RoutingAlgorithmRecord> for storage_models::RoutingProfileMetadata {
    // diesel_models
    fn from(record: RoutingAlgorithmRecord) -> Self {
        let (kind, algorithm_for) = match record.metadata {
            Some(metadata) => (metadata.kind, metadata.algorithm_for),
            None => (
                enums::RoutingAlgorithmKind::Advanced,
                common_enums::enums::TransactionType::default(),
            ),
        };
        Self {
            profile_id: record.created_by,
            algorithm_id: record.id,
            name: record.name,
            description: record.description,
            kind,
            created_at: record.created_at,
            modified_at: record.modified_at,
            algorithm_for,
        }
    }
}

// These From impls are for converting euclid AST to local Euclid-specific types.
// They should be fine as long as euclid crate is a dependency.
impl From<ast::Program<api_routing::ConnectorSelection>> for Program {
    // api_routing from api_models
    fn from(p: ast::Program<api_routing::ConnectorSelection>) -> Self {
        Program {
            globals: HashMap::new(),
            default_selection: convert_output(p.default_selection),
            rules: p.rules.into_iter().map(convert_rule).collect(),
            metadata: Some(p.metadata),
        }
    }
}

fn convert_rule(rule: ast::Rule<api_routing::ConnectorSelection>) -> Rule {
    Rule {
        name: rule.name,
        routing_type: RoutingType::Priority, // Defaulting, might need more logic
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
        Number(n) => ValueType::Number(n.get_amount_as_i64().try_into().unwrap_or_default()), // Added unwrap_or_default
        EnumVariant(e) => ValueType::EnumVariant(e),
        MetadataVariant(m) => ValueType::MetadataVariant(MetadataValue {
            key: m.key,
            value: m.value,
        }),
        StrValue(s) => ValueType::StrValue(s),
        _ => unimplemented!(), // GlobalRef(r) => ValueType::GlobalRef(r),
    }
}

fn convert_output(sel: api_routing::ConnectorSelection) -> Output {
    // api_routing from api_models
    match sel {
        api_routing::ConnectorSelection::Priority(choices) => {
            Output::Priority(choices.into_iter().map(stringify_choice).collect())
        }
        api_routing::ConnectorSelection::VolumeSplit(vs) => Output::VolumeSplit(
            vs.into_iter()
                .map(|v| VolumeSplit {
                    split: v.split,
                    output: stringify_choice(v.connector),
                })
                .collect(),
        ),
    }
}

fn stringify_choice(c: api_routing::RoutableConnectorChoice) -> String {
    // api_routing from api_models
    c.connector.to_string()
}
