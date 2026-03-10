use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use api_models::{
    open_router as or_types,
    routing::{
        self as api_routing, ComparisonType, ConnectorSelection, ConnectorVolumeSplit,
        DeRoutableConnectorChoice, MetadataValue, NumberComparison, RoutableConnectorChoice,
        RoutingEvaluateRequest, RoutingEvaluateResponse, ValueType,
    },
};
use async_trait::async_trait;
use common_enums::{RoutableConnectors, TransactionType};
use common_utils::{
    ext_traits::{BytesExt, StringExt},
    id_type,
};
use diesel_models::{enums, routing_algorithm};
use error_stack::ResultExt;
use euclid::{
    backend::BackendInput,
    frontend::{
        ast::{self},
        dir::{self, transformers::IntoDirValue},
    },
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing as ir_client;
use hyperswitch_domain_models::business_profile;
use hyperswitch_interfaces::events::routing_api_logs as routing_events;
use router_env::RequestId;
use serde::{Deserialize, Serialize};

use super::RoutingResult;
use crate::{
    core::errors,
    db::domain,
    routes::{app::SessionStateInfo, SessionState},
    services::{self, logger},
    types::transformers::ForeignInto,
};

// New Trait for handling Euclid API calls
#[async_trait]
pub trait DecisionEngineApiHandler {
    async fn send_decision_engine_request<Req, Res>(
        state: &SessionState,
        http_method: services::Method,
        path: &str,
        request_body: Option<Req>, // Option to handle GET/DELETE requests without body
        timeout: Option<u64>,
        events_wrapper: Option<RoutingEventsWrapper<Req>>,
    ) -> RoutingResult<RoutingEventsResponse<Res>>
    where
        Req: Serialize + Send + Sync + 'static + Clone,
        Res: Serialize + serde::de::DeserializeOwned + Send + 'static + std::fmt::Debug + Clone;
}

// Struct to implement the DecisionEngineApiHandler trait
pub struct EuclidApiClient;

pub struct ConfigApiClient;

pub struct SRApiClient;

pub async fn build_and_send_decision_engine_http_request<Req, Res, ErrRes>(
    state: &SessionState,
    http_method: services::Method,
    path: &str,
    request_body: Option<Req>,
    _timeout: Option<u64>,
    context_message: &str,
    events_wrapper: Option<RoutingEventsWrapper<Req>>,
) -> RoutingResult<RoutingEventsResponse<Res>>
where
    Req: Serialize + Send + Sync + 'static + Clone,
    Res: Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Clone + 'static,
    ErrRes: serde::de::DeserializeOwned + std::fmt::Debug + Clone + DecisionEngineErrorsInterface,
{
    let decision_engine_base_url = &state.conf.open_router.url;
    let url = format!("{decision_engine_base_url}/{path}");
    logger::debug!(decision_engine_api_call_url = %url, decision_engine_request_path = %path, http_method = ?http_method, "decision_engine: Initiating decision_engine API call ({})", context_message);

    let mut request_builder = services::RequestBuilder::new()
        .method(http_method)
        .url(&url);

    if let Some(body_content) = request_body {
        let body = common_utils::request::RequestContent::Json(Box::new(body_content));
        request_builder = request_builder.set_body(body);
    }

    let http_request = request_builder.build();
    logger::info!(?http_request, decision_engine_request_path = %path, "decision_engine: Constructed Decision Engine API request details ({})", context_message);
    let should_parse_response = events_wrapper
        .as_ref()
        .map(|wrapper| wrapper.parse_response)
        .unwrap_or(true);

    let closure = || async {
        let response =
            services::call_connector_api(state, http_request, "Decision Engine API call")
                .await
                .change_context(errors::RoutingError::OpenRouterCallFailed)?;

        match response {
            Ok(resp) => {
                logger::debug!(
                    "decision_engine: Received response from Decision Engine API ({:?})",
                    String::from_utf8_lossy(&resp.response) // For logging
                );

                let resp = should_parse_response
                    .then(|| {
                        if std::any::TypeId::of::<Res>() == std::any::TypeId::of::<String>()
                            && resp.response.is_empty()
                        {
                            return serde_json::from_str::<Res>("\"\"").change_context(
                                errors::RoutingError::OpenRouterError(
                                    "Failed to parse empty response as String".into(),
                                ),
                            );
                        }
                        let response_type: Res = resp
                            .response
                            .parse_struct(std::any::type_name::<Res>())
                            .change_context(errors::RoutingError::OpenRouterError(
                                "Failed to parse the response from open_router".into(),
                            ))?;

                        Ok::<_, error_stack::Report<errors::RoutingError>>(response_type)
                    })
                    .transpose()?;

                logger::debug!("decision_engine_success_response: {:?}", resp);

                Ok(resp)
            }
            Err(err) => {
                logger::debug!(
                    "decision_engine: Received response from Decision Engine API ({:?})",
                    String::from_utf8_lossy(&err.response) // For logging
                );

                let err_resp: ErrRes = err
                    .response
                    .parse_struct(std::any::type_name::<ErrRes>())
                    .change_context(errors::RoutingError::OpenRouterError(
                    "Failed to parse the response from open_router".into(),
                ))?;

                logger::error!(
                    decision_engine_error_code = %err_resp.get_error_code(),
                    decision_engine_error_message = %err_resp.get_error_message(),
                    decision_engine_raw_response = ?err_resp.get_error_data(),
                );

                Err(error_stack::report!(
                    errors::RoutingError::RoutingEventsError {
                        message: err_resp.get_error_message(),
                        status_code: err.status_code,
                    }
                ))
            }
        }
    };

    let events_response = if let Some(wrapper) = events_wrapper {
        wrapper
            .construct_event_builder(
                url,
                routing_events::RoutingEngine::DecisionEngine,
                routing_events::ApiMethod::Rest(http_method),
            )?
            .trigger_event(state, closure)
            .await?
    } else {
        let resp = closure()
            .await
            .change_context(errors::RoutingError::OpenRouterCallFailed)?;

        RoutingEventsResponse::new(None, resp)
    };

    Ok(events_response)
}

#[async_trait]
impl DecisionEngineApiHandler for EuclidApiClient {
    async fn send_decision_engine_request<Req, Res>(
        state: &SessionState,
        http_method: services::Method,
        path: &str,
        request_body: Option<Req>, // Option to handle GET/DELETE requests without body
        timeout: Option<u64>,
        events_wrapper: Option<RoutingEventsWrapper<Req>>,
    ) -> RoutingResult<RoutingEventsResponse<Res>>
    where
        Req: Serialize + Send + Sync + 'static + Clone,
        Res: Serialize + serde::de::DeserializeOwned + Send + 'static + std::fmt::Debug + Clone,
    {
        let event_response = build_and_send_decision_engine_http_request::<_, _, DeErrorResponse>(
            state,
            http_method,
            path,
            request_body,
            timeout,
            "parsing response",
            events_wrapper,
        )
        .await?;

        let parsed_response =
            event_response
                .response
                .as_ref()
                .ok_or(errors::RoutingError::OpenRouterError(
                    "Response from decision engine API is empty".to_string(),
                ))?;

        logger::debug!(parsed_response = ?parsed_response, response_type = %std::any::type_name::<Res>(), euclid_request_path = %path, "decision_engine_euclid: Successfully parsed response from Euclid API");
        Ok(event_response)
    }
}

#[async_trait]
impl DecisionEngineApiHandler for ConfigApiClient {
    async fn send_decision_engine_request<Req, Res>(
        state: &SessionState,
        http_method: services::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
        events_wrapper: Option<RoutingEventsWrapper<Req>>,
    ) -> RoutingResult<RoutingEventsResponse<Res>>
    where
        Req: Serialize + Send + Sync + 'static + Clone,
        Res: Serialize + serde::de::DeserializeOwned + Send + 'static + std::fmt::Debug + Clone,
    {
        let events_response = build_and_send_decision_engine_http_request::<_, _, DeErrorResponse>(
            state,
            http_method,
            path,
            request_body,
            timeout,
            "parsing response",
            events_wrapper,
        )
        .await?;

        let parsed_response =
            events_response
                .response
                .as_ref()
                .ok_or(errors::RoutingError::OpenRouterError(
                    "Response from decision engine API is empty".to_string(),
                ))?;
        logger::debug!(parsed_response = ?parsed_response, response_type = %std::any::type_name::<Res>(), decision_engine_request_path = %path, "decision_engine_config: Successfully parsed response from Decision Engine config API");
        Ok(events_response)
    }
}

#[async_trait]
impl DecisionEngineApiHandler for SRApiClient {
    async fn send_decision_engine_request<Req, Res>(
        state: &SessionState,
        http_method: services::Method,
        path: &str,
        request_body: Option<Req>,
        timeout: Option<u64>,
        events_wrapper: Option<RoutingEventsWrapper<Req>>,
    ) -> RoutingResult<RoutingEventsResponse<Res>>
    where
        Req: Serialize + Send + Sync + 'static + Clone,
        Res: Serialize + serde::de::DeserializeOwned + Send + 'static + std::fmt::Debug + Clone,
    {
        let events_response =
            build_and_send_decision_engine_http_request::<_, _, or_types::ErrorResponse>(
                state,
                http_method,
                path,
                request_body,
                timeout,
                "parsing response",
                events_wrapper,
            )
            .await?;

        let parsed_response =
            events_response
                .response
                .as_ref()
                .ok_or(errors::RoutingError::OpenRouterError(
                    "Response from decision engine API is empty".to_string(),
                ))?;
        logger::debug!(parsed_response = ?parsed_response, response_type = %std::any::type_name::<Res>(), decision_engine_request_path = %path, "decision_engine_config: Successfully parsed response from Decision Engine config API");
        Ok(events_response)
    }
}

const EUCLID_API_TIMEOUT: u64 = 5;

pub async fn perform_decision_euclid_routing(
    state: &SessionState,
    input: BackendInput,
    created_by: String,
    events_wrapper: RoutingEventsWrapper<RoutingEvaluateRequest>,
    fallback_output: Vec<RoutableConnectorChoice>,
) -> RoutingResult<RoutingEvaluateResponse> {
    logger::debug!("decision_engine_euclid: evaluate api call for euclid routing evaluation");

    let mut events_wrapper = events_wrapper;
    let fallback_output = fallback_output
        .into_iter()
        .map(|c| DeRoutableConnectorChoice {
            gateway_name: c.connector,
            gateway_id: c.merchant_connector_id,
        })
        .collect::<Vec<_>>();

    let routing_request =
        convert_backend_input_to_routing_eval(created_by, input, fallback_output)?;
    events_wrapper.set_request_body(routing_request.clone());

    let event_response = EuclidApiClient::send_decision_engine_request(
        state,
        services::Method::Post,
        "routing/evaluate",
        Some(routing_request),
        Some(EUCLID_API_TIMEOUT),
        Some(events_wrapper),
    )
    .await?;

    let euclid_response: RoutingEvaluateResponse =
        event_response
            .response
            .ok_or(errors::RoutingError::OpenRouterError(
                "Response from decision engine API is empty".to_string(),
            ))?;

    let mut routing_event =
        event_response
            .event
            .ok_or(errors::RoutingError::RoutingEventsError {
                message: "Routing event not found in EventsResponse".to_string(),
                status_code: 500,
            })?;

    routing_event.set_routing_approach(RoutingApproach::StaticRouting.to_string());
    routing_event.set_routable_connectors(euclid_response.evaluated_output.clone());
    state.event_handler.log_event(&routing_event);

    logger::debug!(decision_engine_euclid_response=?euclid_response,"decision_engine_euclid");
    logger::debug!(decision_engine_euclid_selected_connector=?euclid_response.evaluated_output,"decision_engine_euclid");
    Ok(euclid_response)
}

/// This function transforms the decision_engine response in a way that's usable for further flows:
/// It places evaluated_output connectors first, followed by remaining output connectors (no duplicates).
pub fn transform_de_output_for_router(
    de_output: Vec<ConnectorInfo>,
    de_evaluated_output: Vec<RoutableConnectorChoice>,
) -> RoutingResult<Vec<RoutableConnectorChoice>> {
    let mut seen = HashSet::new();

    // evaluated connectors on top, to ensure the fallback is based on other connectors.
    let mut ordered = Vec::with_capacity(de_output.len() + de_evaluated_output.len());
    for eval_conn in de_evaluated_output {
        if seen.insert(eval_conn.connector) {
            ordered.push(eval_conn);
        }
    }

    // Add remaining connectors from de_output (only if not already seen), for fallback
    for conn in de_output {
        let key = RoutableConnectors::from_str(&conn.gateway_name).map_err(|_| {
            errors::RoutingError::GenericConversionError {
                from: "String".to_string(),
                to: "RoutableConnectors".to_string(),
            }
        })?;
        if seen.insert(key) {
            let de_choice = DeRoutableConnectorChoice::try_from(conn)?;
            ordered.push(RoutableConnectorChoice::from(de_choice));
        }
    }
    Ok(ordered)
}

pub async fn decision_engine_routing(
    state: &SessionState,
    backend_input: BackendInput,
    business_profile: &domain::Profile,
    payment_id: String,
    merchant_fallback_config: Vec<RoutableConnectorChoice>,
) -> RoutingResult<Vec<RoutableConnectorChoice>> {
    let routing_events_wrapper = RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_id,
        business_profile.get_id().to_owned(),
        business_profile.merchant_id.to_owned(),
        "DecisionEngine: Euclid Static Routing".to_string(),
        None,
        true,
        false,
    );

    let de_euclid_evaluate_response = perform_decision_euclid_routing(
        state,
        backend_input.clone(),
        business_profile.get_id().get_string_repr().to_string(),
        routing_events_wrapper,
        merchant_fallback_config,
    )
    .await;

    let Ok(de_euclid_response) = de_euclid_evaluate_response else {
        logger::error!("decision_engine_euclid_evaluation_error: error in evaluation of rule");
        return Ok(Vec::default());
    };

    let de_output_connector = extract_de_output_connectors(de_euclid_response.output)
            .map_err(|e| {
                logger::error!(error=?e, "decision_engine_euclid_evaluation_error: Failed to extract connector from Output");
                e
            })?;

    transform_de_output_for_router(
            de_output_connector.clone(),
            de_euclid_response.evaluated_output.clone(),
        )
        .map_err(|e| {
            logger::error!(error=?e, "decision_engine_euclid_evaluation_error: failed to transform connector from de-output");
            e
        })
}

/// Custom deserializer for output from decision_engine, this is required as untagged enum is
/// stored but the enum requires tagged deserialization, hence deserializing it into specific
/// variants
pub fn extract_de_output_connectors(
    output_value: serde_json::Value,
) -> RoutingResult<Vec<ConnectorInfo>> {
    const SINGLE: &str = "straight_through";
    const PRIORITY: &str = "priority";
    const VOLUME_SPLIT: &str = "volume_split";
    const VOLUME_SPLIT_PRIORITY: &str = "volume_split_priority";

    let obj = output_value.as_object().ok_or_else(|| {
        logger::error!("decision_engine_euclid_error: output is not a JSON object");
        errors::RoutingError::OpenRouterError("Expected output to be a JSON object".into())
    })?;

    let type_str = obj.get("type").and_then(|v| v.as_str()).ok_or_else(|| {
        logger::error!("decision_engine_euclid_error: missing or invalid 'type' in output");
        errors::RoutingError::OpenRouterError("Missing or invalid 'type' field in output".into())
    })?;

    match type_str {
        SINGLE => {
            let connector_value = obj.get("connector").ok_or_else(|| {
                logger::error!(
                    "decision_engine_euclid_error: missing 'connector' field for type=single"
                );
                errors::RoutingError::OpenRouterError(
                    "Missing 'connector' field for single output".into(),
                )
            })?;
            let connector: ConnectorInfo = serde_json::from_value(connector_value.clone())
                .map_err(|e| {
                    logger::error!(
                        ?e,
                        "decision_engine_euclid_error: Failed to parse single connector"
                    );
                    errors::RoutingError::OpenRouterError(
                        "Failed to deserialize single connector".into(),
                    )
                })?;
            Ok(vec![connector])
        }

        PRIORITY => {
            let connectors_value = obj.get("connectors").ok_or_else(|| {
                logger::error!(
                    "decision_engine_euclid_error: missing 'connectors' field for type=priority"
                );
                errors::RoutingError::OpenRouterError(
                    "Missing 'connectors' field for priority output".into(),
                )
            })?;
            let connectors: Vec<ConnectorInfo> = serde_json::from_value(connectors_value.clone())
                .map_err(|e| {
                logger::error!(
                    ?e,
                    "decision_engine_euclid_error: Failed to parse connectors for priority"
                );
                errors::RoutingError::OpenRouterError(
                    "Failed to deserialize priority connectors".into(),
                )
            })?;
            Ok(connectors)
        }

        VOLUME_SPLIT => {
            let splits_value = obj.get("splits").ok_or_else(|| {
                logger::error!(
                    "decision_engine_euclid_error: missing 'splits' field for type=volume_split"
                );
                errors::RoutingError::OpenRouterError(
                    "Missing 'splits' field for volume_split output".into(),
                )
            })?;

            // Transform each {connector, split} into {output, split}
            let fixed_splits: Vec<_> = splits_value
                .as_array()
                .ok_or_else(|| {
                    logger::error!("decision_engine_euclid_error: 'splits' is not an array");
                    errors::RoutingError::OpenRouterError("'splits' field must be an array".into())
                })?
                .iter()
                .map(|entry| {
                    let mut entry_map = entry.as_object().cloned().ok_or_else(|| {
                        logger::error!(
                            "decision_engine_euclid_error: invalid split entry in volume_split"
                        );
                        errors::RoutingError::OpenRouterError(
                            "Invalid entry in splits array".into(),
                        )
                    })?;
                    if let Some(connector) = entry_map.remove("connector") {
                        entry_map.insert("output".to_string(), connector);
                    }
                    Ok::<_, error_stack::Report<errors::RoutingError>>(serde_json::Value::Object(
                        entry_map,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let splits: Vec<VolumeSplit<ConnectorInfo>> =
                serde_json::from_value(serde_json::Value::Array(fixed_splits)).map_err(|e| {
                    logger::error!(
                        ?e,
                        "decision_engine_euclid_error: Failed to parse volume_split"
                    );
                    errors::RoutingError::OpenRouterError(
                        "Failed to deserialize volume_split connectors".into(),
                    )
                })?;

            Ok(splits.into_iter().map(|s| s.output).collect())
        }

        VOLUME_SPLIT_PRIORITY => {
            let splits_value = obj.get("splits").ok_or_else(|| {
                logger::error!("decision_engine_euclid_error: missing 'splits' field for type=volume_split_priority");
                errors::RoutingError::OpenRouterError("Missing 'splits' field for volume_split_priority output".into())
            })?;

            // Transform each {connector: [...], split} into {output: [...], split}
            let fixed_splits: Vec<_> = splits_value
                .as_array()
                .ok_or_else(|| {
                    logger::error!("decision_engine_euclid_error: 'splits' is not an array");
                    errors::RoutingError::OpenRouterError("'splits' field must be an array".into())
                })?
                .iter()
                .map(|entry| {
                    let mut entry_map = entry.as_object().cloned().ok_or_else(|| {
                        logger::error!("decision_engine_euclid_error: invalid split entry in volume_split_priority");
                        errors::RoutingError::OpenRouterError("Invalid entry in splits array".into())
                    })?;
                    if let Some(connector) = entry_map.remove("connector") {
                        entry_map.insert("output".to_string(), connector);
                    }
                    Ok::<_, error_stack::Report<errors::RoutingError>>(serde_json::Value::Object(entry_map))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let splits: Vec<VolumeSplit<Vec<ConnectorInfo>>> =
                serde_json::from_value(serde_json::Value::Array(fixed_splits)).map_err(|e| {
                    logger::error!(
                        ?e,
                        "decision_engine_euclid_error: Failed to parse volume_split_priority"
                    );
                    errors::RoutingError::OpenRouterError(
                        "Failed to deserialize volume_split_priority connectors".into(),
                    )
                })?;

            Ok(splits.into_iter().flat_map(|s| s.output).collect())
        }

        other => {
            logger::error!(type_str=%other, "decision_engine_euclid_error: unknown output type");
            Err(
                errors::RoutingError::OpenRouterError(format!("Unknown output type: {other}"))
                    .into(),
            )
        }
    }
}

pub async fn create_de_euclid_routing_algo(
    state: &SessionState,
    routing_request: &RoutingRule,
) -> RoutingResult<String> {
    logger::debug!("decision_engine_euclid: create api call for euclid routing rule creation");

    logger::debug!(decision_engine_euclid_request=?routing_request,"decision_engine_euclid");
    let events_response = EuclidApiClient::send_decision_engine_request(
        state,
        services::Method::Post,
        "routing/create",
        Some(routing_request.clone()),
        Some(EUCLID_API_TIMEOUT),
        None,
    )
    .await?;

    let euclid_response: RoutingDictionaryRecord =
        events_response
            .response
            .ok_or(errors::RoutingError::OpenRouterError(
                "Response from decision engine API is empty".to_string(),
            ))?;

    logger::debug!(decision_engine_euclid_parsed_response=?euclid_response,"decision_engine_euclid");
    Ok(euclid_response.rule_id)
}

pub async fn link_de_euclid_routing_algorithm(
    state: &SessionState,
    routing_request: ActivateRoutingConfigRequest,
) -> RoutingResult<()> {
    logger::debug!("decision_engine_euclid: link api call for euclid routing algorithm");

    EuclidApiClient::send_decision_engine_request::<_, String>(
        state,
        services::Method::Post,
        "routing/activate",
        Some(routing_request.clone()),
        Some(EUCLID_API_TIMEOUT),
        None,
    )
    .await?;

    logger::debug!(decision_engine_euclid_activated=?routing_request, "decision_engine_euclid: link_de_euclid_routing_algorithm completed");
    Ok(())
}

pub async fn list_de_euclid_routing_algorithms(
    state: &SessionState,
    routing_list_request: ListRountingAlgorithmsRequest,
) -> RoutingResult<Vec<api_routing::RoutingDictionaryRecord>> {
    logger::debug!("decision_engine_euclid: list api call for euclid routing algorithms");
    let created_by = routing_list_request.created_by;
    let events_response = EuclidApiClient::send_decision_engine_request(
        state,
        services::Method::Post,
        format!("routing/list/{created_by}").as_str(),
        None::<()>,
        Some(EUCLID_API_TIMEOUT),
        None,
    )
    .await?;

    let euclid_response: Vec<RoutingAlgorithmRecord> =
        events_response
            .response
            .ok_or(errors::RoutingError::OpenRouterError(
                "Response from decision engine API is empty".to_string(),
            ))?;

    Ok(euclid_response
        .into_iter()
        .map(routing_algorithm::RoutingProfileMetadata::from)
        .map(ForeignInto::foreign_into)
        .collect::<Vec<_>>())
}

pub async fn list_de_euclid_active_routing_algorithm(
    state: &SessionState,
    created_by: String,
) -> RoutingResult<Vec<api_routing::RoutingDictionaryRecord>> {
    logger::debug!("decision_engine_euclid: list api call for euclid active routing algorithm");
    let response: Vec<RoutingAlgorithmRecord> = EuclidApiClient::send_decision_engine_request(
        state,
        services::Method::Post,
        format!("routing/list/active/{created_by}").as_str(),
        None::<()>,
        Some(EUCLID_API_TIMEOUT),
        None,
    )
    .await?
    .response
    .ok_or(errors::RoutingError::OpenRouterError(
        "Response from decision engine API is empty".to_string(),
    ))?;

    Ok(response
        .into_iter()
        .map(|record| routing_algorithm::RoutingProfileMetadata::from(record).foreign_into())
        .collect())
}

pub fn compare_and_log_result<T: RoutingEq<T> + Serialize>(
    de_result: Vec<T>,
    result: Vec<T>,
    flow: String,
) {
    let is_equal = if de_result.is_empty() && result.is_empty() {
        true
    } else {
        de_result
            .iter()
            .zip(result.iter())
            .all(|(a, b)| T::is_equal(a, b))
    };

    let is_equal_in_length = de_result.len() == result.len();

    router_env::logger::debug!(
        routing_flow=?flow,
        is_equal=?is_equal,
        is_equal_length=?is_equal_in_length,
        de_response=?to_json_string(&de_result),
        hs_response=?to_json_string(&result),
        "decision_engine_euclid"
    );
}

pub trait RoutingEq<T> {
    fn is_equal(a: &T, b: &T) -> bool;
}

impl RoutingEq<Self> for api_routing::RoutingDictionaryRecord {
    fn is_equal(a: &Self, b: &Self) -> bool {
        a.id == b.id
            && a.name == b.name
            && a.profile_id == b.profile_id
            && a.description == b.description
            && a.kind == b.kind
            && a.algorithm_for == b.algorithm_for
    }
}

impl RoutingEq<Self> for String {
    fn is_equal(a: &Self, b: &Self) -> bool {
        a.to_lowercase() == b.to_lowercase()
    }
}

impl RoutingEq<Self> for RoutableConnectorChoice {
    fn is_equal(a: &Self, b: &Self) -> bool {
        a.connector.eq(&b.connector)
            && a.choice_kind.eq(&b.choice_kind)
            && a.merchant_connector_id.eq(&b.merchant_connector_id)
    }
}

pub fn to_json_string<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map_err(|_| errors::RoutingError::GenericConversionError {
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
    fallback_output: Vec<DeRoutableConnectorChoice>,
) -> RoutingResult<RoutingEvaluateRequest> {
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
    if let Some(extended_bin) = input.payment.extended_card_bin {
        params.insert(
            "extended_card_bin".to_string(),
            Some(ValueType::StrValue(extended_bin)),
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
        if let Some(pmt) = input.payment_method.payment_method_type {
            match (pmt, pm).into_dir_value() {
                Ok(dv) => insert_dirvalue_param(&mut params, dv),
                Err(e) => logger::debug!(
                    ?e,
                    ?pmt,
                    ?pm,
                    "decision_engine_euclid: into_dir_value failed; skipping subset param"
                ),
            }
        }
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
        fallback_output,
    })
}

// All the independent variants of payment method types, configured via dashboard
fn insert_dirvalue_param(params: &mut HashMap<String, Option<ValueType>>, dv: dir::DirValue) {
    match dv {
        dir::DirValue::RewardType(v) => {
            params.insert(
                "reward".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::CardType(v) => {
            params.insert(
                "card".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::PayLaterType(v) => {
            params.insert(
                "pay_later".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::WalletType(v) => {
            params.insert(
                "wallet".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::VoucherType(v) => {
            params.insert(
                "voucher".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::BankRedirectType(v) => {
            params.insert(
                "bank_redirect".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::BankDebitType(v) => {
            params.insert(
                "bank_debit".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::BankTransferType(v) => {
            params.insert(
                "bank_transfer".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::RealTimePaymentType(v) => {
            params.insert(
                "real_time_payment".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::UpiType(v) => {
            params.insert(
                "upi".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::GiftCardType(v) => {
            params.insert(
                "gift_card".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::CardRedirectType(v) => {
            params.insert(
                "card_redirect".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::OpenBankingType(v) => {
            params.insert(
                "open_banking".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::MobilePaymentType(v) => {
            params.insert(
                "mobile_payment".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        dir::DirValue::CryptoType(v) => {
            params.insert(
                "crypto".to_string(),
                Some(ValueType::EnumVariant(v.to_string())),
            );
        }
        other => {
            // all other values can be ignored for now as they don't converge with
            // payment method type
            logger::warn!(
                ?other,
                "decision_engine_euclid: unmapped dir::DirValue; add a mapping here"
            );
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DeErrorResponse {
    code: String,
    message: String,
    data: Option<serde_json::Value>,
}

impl DecisionEngineErrorsInterface for DeErrorResponse {
    fn get_error_message(&self) -> String {
        self.message.clone()
    }

    fn get_error_code(&self) -> String {
        self.code.clone()
    }

    fn get_error_data(&self) -> Option<String> {
        self.data.as_ref().map(|data| data.to_string())
    }
}

impl DecisionEngineErrorsInterface for or_types::ErrorResponse {
    fn get_error_message(&self) -> String {
        self.error_message.clone()
    }

    fn get_error_code(&self) -> String {
        self.error_code.clone()
    }

    fn get_error_data(&self) -> Option<String> {
        Some(format!(
            "decision_engine Error: {}",
            self.error_message.clone()
        ))
    }
}

pub type Metadata = HashMap<String, serde_json::Value>;

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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum RoutingType {
    Priority,
    VolumeSplit,
    VolumeSplitPriority,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VolumeSplit<T> {
    pub split: u8,
    pub output: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConnectorInfo {
    pub gateway_name: String,
    pub gateway_id: Option<String>,
}

impl TryFrom<ConnectorInfo> for DeRoutableConnectorChoice {
    type Error = error_stack::Report<errors::RoutingError>;

    fn try_from(c: ConnectorInfo) -> Result<Self, Self::Error> {
        let gateway_id = c
            .gateway_id
            .map(|mca| {
                id_type::MerchantConnectorAccountId::wrap(mca)
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "MerchantConnectorAccountId".to_string(),
                    })
                    .attach_printable("unable to convert MerchantConnectorAccountId from string")
            })
            .transpose()?;

        let gateway_name = RoutableConnectors::from_str(&c.gateway_name)
            .map_err(|_| errors::RoutingError::GenericConversionError {
                from: "String".to_string(),
                to: "RoutableConnectors".to_string(),
            })
            .attach_printable("unable to convert connector name to RoutableConnectors")?;

        Ok(Self {
            gateway_name,
            gateway_id,
        })
    }
}

impl ConnectorInfo {
    pub fn new(gateway_name: String, gateway_id: Option<String>) -> Self {
        Self {
            gateway_name,
            gateway_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Output {
    Single(ConnectorInfo),
    Priority(Vec<ConnectorInfo>),
    VolumeSplit(Vec<VolumeSplit<ConnectorInfo>>),
    VolumeSplitPriority(Vec<VolumeSplit<Vec<ConnectorInfo>>>),
}

pub type Globals = HashMap<String, HashSet<ValueType>>;

/// The program, having a default connector selection and
/// a bunch of rules. Also can hold arbitrary metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    pub rule_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub metadata: Option<RoutingMetadata>,
    pub created_by: String,
    #[serde(default)]
    pub algorithm_for: AlgorithmType,
    pub algorithm: StaticRoutingAlgorithm,
}
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AlgorithmType {
    #[default]
    Payment,
    Payout,
    ThreeDsAuthentication,
}

impl From<TransactionType> for AlgorithmType {
    fn from(transaction_type: TransactionType) -> Self {
        match transaction_type {
            TransactionType::Payment => Self::Payment,
            TransactionType::Payout => Self::Payout,
            TransactionType::ThreeDsAuthentication => Self::ThreeDsAuthentication,
        }
    }
}

impl From<RoutableConnectorChoice> for ConnectorInfo {
    fn from(c: RoutableConnectorChoice) -> Self {
        Self {
            gateway_name: c.connector.to_string(),
            gateway_id: c
                .merchant_connector_id
                .map(|mca_id| mca_id.get_string_repr().to_string()),
        }
    }
}

impl From<Box<RoutableConnectorChoice>> for ConnectorInfo {
    fn from(c: Box<RoutableConnectorChoice>) -> Self {
        Self {
            gateway_name: c.connector.to_string(),
            gateway_id: c
                .merchant_connector_id
                .map(|mca_id| mca_id.get_string_repr().to_string()),
        }
    }
}

impl From<ConnectorVolumeSplit> for VolumeSplit<ConnectorInfo> {
    fn from(v: ConnectorVolumeSplit) -> Self {
        Self {
            split: v.split,
            output: ConnectorInfo {
                gateway_name: v.connector.connector.to_string(),
                gateway_id: v
                    .connector
                    .merchant_connector_id
                    .map(|mca_id| mca_id.get_string_repr().to_string()),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum StaticRoutingAlgorithm {
    Single(Box<ConnectorInfo>),
    Priority(Vec<ConnectorInfo>),
    VolumeSplit(Vec<VolumeSplit<ConnectorInfo>>),
    Advanced(Program),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingMetadata {
    pub kind: enums::RoutingAlgorithmKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingDictionaryRecord {
    pub rule_id: String,
    pub name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutingAlgorithmRecord {
    pub id: id_type::RoutingId,
    pub name: String,
    pub description: Option<String>,
    pub created_by: id_type::ProfileId,
    pub algorithm_data: StaticRoutingAlgorithm,
    pub algorithm_for: TransactionType,
    pub metadata: Option<RoutingMetadata>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

impl From<RoutingAlgorithmRecord> for routing_algorithm::RoutingProfileMetadata {
    fn from(record: RoutingAlgorithmRecord) -> Self {
        let kind = match record.algorithm_data {
            StaticRoutingAlgorithm::Single(_) => enums::RoutingAlgorithmKind::Single,
            StaticRoutingAlgorithm::Priority(_) => enums::RoutingAlgorithmKind::Priority,
            StaticRoutingAlgorithm::VolumeSplit(_) => enums::RoutingAlgorithmKind::VolumeSplit,
            StaticRoutingAlgorithm::Advanced(_) => enums::RoutingAlgorithmKind::Advanced,
        };
        Self {
            profile_id: record.created_by,
            algorithm_id: record.id,
            name: record.name,
            description: record.description,
            kind,
            created_at: record.created_at,
            modified_at: record.modified_at,
            algorithm_for: record.algorithm_for,
        }
    }
}

impl TryFrom<ast::Program<ConnectorSelection>> for Program {
    type Error = error_stack::Report<errors::RoutingError>;

    fn try_from(p: ast::Program<ConnectorSelection>) -> Result<Self, Self::Error> {
        let rules = p
            .rules
            .into_iter()
            .map(convert_rule)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            globals: HashMap::new(),
            default_selection: convert_output(p.default_selection),
            rules,
            metadata: Some(p.metadata),
        })
    }
}

impl TryFrom<ast::Program<ConnectorSelection>> for StaticRoutingAlgorithm {
    type Error = error_stack::Report<errors::RoutingError>;

    fn try_from(p: ast::Program<ConnectorSelection>) -> Result<Self, Self::Error> {
        let internal_program: Program = p.try_into()?;
        Ok(Self::Advanced(internal_program))
    }
}

fn convert_rule(rule: ast::Rule<ConnectorSelection>) -> RoutingResult<Rule> {
    let routing_type = match &rule.connector_selection {
        ConnectorSelection::Priority(_) => RoutingType::Priority,
        ConnectorSelection::VolumeSplit(_) => RoutingType::VolumeSplit,
    };

    Ok(Rule {
        name: rule.name,
        routing_type,
        output: convert_output(rule.connector_selection),
        statements: rule
            .statements
            .into_iter()
            .map(convert_if_stmt)
            .collect::<RoutingResult<Vec<IfStatement>>>()?,
    })
}

fn convert_if_stmt(stmt: ast::IfStatement) -> RoutingResult<IfStatement> {
    Ok(IfStatement {
        condition: stmt
            .condition
            .into_iter()
            .map(convert_comparison)
            .collect::<RoutingResult<Vec<Comparison>>>()?,

        nested: stmt
            .nested
            .map(|v| {
                v.into_iter()
                    .map(convert_if_stmt)
                    .collect::<RoutingResult<Vec<IfStatement>>>()
            })
            .transpose()?,
    })
}

fn convert_comparison(c: ast::Comparison) -> RoutingResult<Comparison> {
    Ok(Comparison {
        lhs: c.lhs,
        comparison: convert_comparison_type(c.comparison),
        value: convert_value(c.value)?,
        metadata: c.metadata,
    })
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

fn convert_value(v: ast::ValueType) -> RoutingResult<ValueType> {
    use ast::ValueType::*;
    match v {
        Number(n) => Ok(ValueType::Number(
            n.get_amount_as_i64().try_into().unwrap_or_default(),
        )),
        EnumVariant(e) => Ok(ValueType::EnumVariant(e)),
        MetadataVariant(m) => Ok(ValueType::MetadataVariant(MetadataValue {
            key: m.key,
            value: m.value,
        })),
        StrValue(s) => Ok(ValueType::StrValue(s)),

        NumberArray(arr) => Ok(ValueType::NumberArray(
            arr.into_iter()
                .map(|n| n.get_amount_as_i64().try_into().unwrap_or_default())
                .collect(),
        )),
        EnumVariantArray(arr) => Ok(ValueType::EnumVariantArray(arr)),
        NumberComparisonArray(arr) => Ok(ValueType::NumberComparisonArray(
            arr.into_iter()
                .map(|nc| NumberComparison {
                    comparison_type: convert_comparison_type(nc.comparison_type),
                    number: nc.number.get_amount_as_i64().try_into().unwrap_or_default(),
                })
                .collect(),
        )),
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

fn stringify_choice(c: RoutableConnectorChoice) -> ConnectorInfo {
    ConnectorInfo::new(
        c.connector.to_string(),
        c.merchant_connector_id
            .map(|mca_id| mca_id.get_string_repr().to_string()),
    )
}

pub async fn select_routing_result<T>(
    state: &SessionState,
    business_profile: &business_profile::Profile,
    hyperswitch_result: T,
    de_result: T,
) -> T
where
    T: Clone + IntoIterator,
{
    let routing_result_source: Option<api_routing::RoutingResultSource> = state
        .store
        .find_config_by_key(&format!(
            "routing_result_source_{0}",
            business_profile.get_id().get_string_repr()
        ))
        .await
        .map(|c| c.config.parse_enum("RoutingResultSource").ok())
        .unwrap_or(None);

    if let Some(api_routing::RoutingResultSource::DecisionEngine) = routing_result_source {
        logger::debug!(
            business_profile_id=?business_profile.get_id(),
            "decision_engine_euclid: Using Decision Engine routing result"
        );

        let is_de_result_empty = de_result.clone().into_iter().next().is_none();
        if is_de_result_empty {
            logger::debug!(
                business_profile_id=?business_profile.get_id(),
                "decision_engine_euclid: DE result empty, falling back to Hyperswitch result"
            );
            hyperswitch_result
        } else {
            de_result
        }
    } else {
        logger::debug!(
            business_profile_id=?business_profile.get_id(),
            "decision_engine_euclid: Using Hyperswitch routing result"
        );
        hyperswitch_result
    }
}

pub trait DecisionEngineErrorsInterface {
    fn get_error_message(&self) -> String;
    fn get_error_code(&self) -> String;
    fn get_error_data(&self) -> Option<String>;
}

#[derive(Debug)]
pub struct RoutingEventsWrapper<Req>
where
    Req: Serialize + Clone,
{
    pub tenant_id: id_type::TenantId,
    pub request_id: Option<RequestId>,
    pub payment_id: String,
    pub profile_id: id_type::ProfileId,
    pub merchant_id: id_type::MerchantId,
    pub flow: String,
    pub request: Option<Req>,
    pub parse_response: bool,
    pub log_event: bool,
    pub routing_event: Option<routing_events::RoutingEvent>,
}

#[derive(Debug)]
pub enum EventResponseType<Res>
where
    Res: Serialize + serde::de::DeserializeOwned + Clone,
{
    Structured(Res),
    String(String),
}

#[derive(Debug, Serialize)]
pub struct RoutingEventsResponse<Res>
where
    Res: Serialize + serde::de::DeserializeOwned + Clone,
{
    pub event: Option<routing_events::RoutingEvent>,
    pub response: Option<Res>,
}

impl<Res> RoutingEventsResponse<Res>
where
    Res: Serialize + serde::de::DeserializeOwned + Clone,
{
    pub fn new(event: Option<routing_events::RoutingEvent>, response: Option<Res>) -> Self {
        Self { event, response }
    }

    pub fn set_response(&mut self, response: Res) {
        self.response = Some(response);
    }

    pub fn set_event(&mut self, event: routing_events::RoutingEvent) {
        self.event = Some(event);
    }
}

impl<Req> RoutingEventsWrapper<Req>
where
    Req: Serialize + Clone,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: id_type::TenantId,
        request_id: Option<RequestId>,
        payment_id: String,
        profile_id: id_type::ProfileId,
        merchant_id: id_type::MerchantId,
        flow: String,
        request: Option<Req>,
        parse_response: bool,
        log_event: bool,
    ) -> Self {
        Self {
            tenant_id,
            request_id,
            payment_id,
            profile_id,
            merchant_id,
            flow,
            request,
            parse_response,
            log_event,
            routing_event: None,
        }
    }

    pub fn construct_event_builder(
        self,
        url: String,
        routing_engine: routing_events::RoutingEngine,
        method: routing_events::ApiMethod,
    ) -> RoutingResult<Self> {
        let mut wrapper = self;
        let request = wrapper
            .request
            .clone()
            .ok_or(errors::RoutingError::RoutingEventsError {
                message: "Request body is missing".to_string(),
                status_code: 400,
            })?;

        let serialized_request = serde_json::to_value(&request)
            .change_context(errors::RoutingError::RoutingEventsError {
                message: "Failed to serialize RoutingRequest".to_string(),
                status_code: 500,
            })
            .attach_printable("Failed to serialize request body")?;

        let routing_event = routing_events::RoutingEvent::new(
            wrapper.tenant_id.clone(),
            "".to_string(),
            &wrapper.flow,
            serialized_request,
            url,
            method,
            wrapper.payment_id.clone(),
            wrapper.profile_id.clone(),
            wrapper.merchant_id.clone(),
            wrapper.request_id.clone(),
            routing_engine,
        );

        wrapper.set_routing_event(routing_event);

        Ok(wrapper)
    }

    pub async fn trigger_event<Res, F, Fut>(
        self,
        state: &SessionState,
        func: F,
    ) -> RoutingResult<RoutingEventsResponse<Res>>
    where
        F: FnOnce() -> Fut + Send,
        Res: Serialize + serde::de::DeserializeOwned + Clone,
        Fut: futures::Future<Output = RoutingResult<Option<Res>>> + Send,
    {
        let mut routing_event =
            self.routing_event
                .ok_or(errors::RoutingError::RoutingEventsError {
                    message: "Routing event is missing".to_string(),
                    status_code: 500,
                })?;

        let mut response = RoutingEventsResponse::new(None, None);

        let resp = func().await;
        match resp {
            Ok(ok_resp) => {
                if let Some(resp) = ok_resp {
                    routing_event.set_response_body(&resp);
                    // routing_event
                    //     .set_routable_connectors(ok_resp.get_routable_connectors().unwrap_or_default());
                    // routing_event.set_payment_connector(ok_resp.get_payment_connector());
                    routing_event.set_status_code(200);

                    response.set_response(resp.clone());
                    self.log_event
                        .then(|| state.event_handler().log_event(&routing_event));
                }
            }
            Err(err) => {
                // Need to figure out a generic way to log errors
                routing_event
                    .set_error(serde_json::json!({"error": err.current_context().to_string()}));

                match err.current_context() {
                    errors::RoutingError::RoutingEventsError { status_code, .. } => {
                        routing_event.set_status_code(*status_code);
                    }
                    _ => {
                        routing_event.set_status_code(500);
                    }
                }
                state.event_handler().log_event(&routing_event)
            }
        }

        response.set_event(routing_event);

        Ok(response)
    }

    pub fn set_log_event(&mut self, log_event: bool) {
        self.log_event = log_event;
    }

    pub fn set_request_body(&mut self, request: Req) {
        self.request = Some(request);
    }

    pub fn set_routing_event(&mut self, routing_event: routing_events::RoutingEvent) {
        self.routing_event = Some(routing_event);
    }
}

pub trait RoutingEventsInterface {
    fn get_routable_connectors(&self) -> Option<Vec<RoutableConnectorChoice>>;
    fn get_payment_connector(&self) -> Option<RoutableConnectorChoice>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalSuccessRateConfigEventRequest {
    pub min_aggregates_size: Option<u32>,
    pub default_success_rate: Option<f64>,
    pub specificity_level: api_routing::SuccessRateSpecificityLevel,
    pub exploration_percent: Option<f64>,
}

impl From<&api_routing::SuccessBasedRoutingConfigBody> for CalSuccessRateConfigEventRequest {
    fn from(value: &api_routing::SuccessBasedRoutingConfigBody) -> Self {
        Self {
            min_aggregates_size: value.min_aggregates_size,
            default_success_rate: value.default_success_rate,
            specificity_level: value.specificity_level,
            exploration_percent: value.exploration_percent,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalSuccessRateEventRequest {
    pub id: String,
    pub params: String,
    pub labels: Vec<String>,
    pub config: Option<CalSuccessRateConfigEventRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EliminationRoutingEventBucketConfig {
    pub bucket_size: Option<u64>,
    pub bucket_leak_interval_in_secs: Option<u64>,
}

impl From<&api_routing::EliminationAnalyserConfig> for EliminationRoutingEventBucketConfig {
    fn from(value: &api_routing::EliminationAnalyserConfig) -> Self {
        Self {
            bucket_size: value.bucket_size,
            bucket_leak_interval_in_secs: value.bucket_leak_interval_in_secs,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EliminationRoutingEventRequest {
    pub id: String,
    pub params: String,
    pub labels: Vec<String>,
    pub config: Option<EliminationRoutingEventBucketConfig>,
}

/// API-1 types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalContractScoreEventRequest {
    pub id: String,
    pub params: String,
    pub labels: Vec<String>,
    pub config: Option<api_routing::ContractBasedRoutingConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LabelWithScoreEventResponse {
    pub score: f64,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalSuccessRateEventResponse {
    pub labels_with_score: Vec<LabelWithScoreEventResponse>,
    pub routing_approach: RoutingApproach,
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl TryFrom<&ir_client::success_rate_client::CalSuccessRateResponse>
    for CalSuccessRateEventResponse
{
    type Error = errors::RoutingError;

    fn try_from(
        value: &ir_client::success_rate_client::CalSuccessRateResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            labels_with_score: value
                .labels_with_score
                .iter()
                .map(|l| LabelWithScoreEventResponse {
                    score: l.score,
                    label: l.label.clone(),
                })
                .collect(),
            routing_approach: match value.routing_approach {
                0 => RoutingApproach::Exploration,
                1 => RoutingApproach::Exploitation,
                _ => {
                    return Err(errors::RoutingError::GenericNotFoundError {
                        field: "unknown routing approach from dynamic routing service".to_string(),
                    })
                }
            },
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingApproach {
    Exploitation,
    Exploration,
    Elimination,
    ContractBased,
    StaticRouting,
    Default,
}

impl RoutingApproach {
    pub fn from_decision_engine_approach(approach: &str) -> Self {
        match approach {
            "SR_SELECTION_V3_ROUTING" => Self::Exploitation,
            "SR_V3_HEDGING" => Self::Exploration,
            _ => Self::Default,
        }
    }
}

impl From<RoutingApproach> for common_enums::RoutingApproach {
    fn from(approach: RoutingApproach) -> Self {
        match approach {
            RoutingApproach::Exploitation => Self::SuccessRateExploitation,
            RoutingApproach::Exploration => Self::SuccessRateExploration,
            RoutingApproach::ContractBased => Self::ContractBasedRouting,
            RoutingApproach::StaticRouting => Self::RuleBasedRouting,
            _ => Self::DefaultFallback,
        }
    }
}

impl std::fmt::Display for RoutingApproach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exploitation => write!(f, "Exploitation"),
            Self::Exploration => write!(f, "Exploration"),
            Self::Elimination => write!(f, "Elimination"),
            Self::ContractBased => write!(f, "ContractBased"),
            Self::StaticRouting => write!(f, "StaticRouting"),
            Self::Default => write!(f, "Default"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BucketInformationEventResponse {
    pub is_eliminated: bool,
    pub bucket_name: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EliminationInformationEventResponse {
    pub entity: Option<BucketInformationEventResponse>,
    pub global: Option<BucketInformationEventResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LabelWithStatusEliminationEventResponse {
    pub label: String,
    pub elimination_information: Option<EliminationInformationEventResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EliminationEventResponse {
    pub labels_with_status: Vec<LabelWithStatusEliminationEventResponse>,
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl From<&ir_client::elimination_based_client::EliminationResponse> for EliminationEventResponse {
    fn from(value: &ir_client::elimination_based_client::EliminationResponse) -> Self {
        Self {
            labels_with_status: value
                .labels_with_status
                .iter()
                .map(
                    |label_with_status| LabelWithStatusEliminationEventResponse {
                        label: label_with_status.label.clone(),
                        elimination_information: label_with_status
                            .elimination_information
                            .as_ref()
                            .map(|info| EliminationInformationEventResponse {
                                entity: info.entity.as_ref().map(|entity_info| {
                                    BucketInformationEventResponse {
                                        is_eliminated: entity_info.is_eliminated,
                                        bucket_name: entity_info.bucket_name.clone(),
                                    }
                                }),
                                global: info.global.as_ref().map(|global_info| {
                                    BucketInformationEventResponse {
                                        is_eliminated: global_info.is_eliminated,
                                        bucket_name: global_info.bucket_name.clone(),
                                    }
                                }),
                            }),
                    },
                )
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScoreDataEventResponse {
    pub score: f64,
    pub label: String,
    pub current_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalContractScoreEventResponse {
    pub labels_with_score: Vec<ScoreDataEventResponse>,
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl From<&ir_client::contract_routing_client::CalContractScoreResponse>
    for CalContractScoreEventResponse
{
    fn from(value: &ir_client::contract_routing_client::CalContractScoreResponse) -> Self {
        Self {
            labels_with_score: value
                .labels_with_score
                .iter()
                .map(|label_with_score| ScoreDataEventResponse {
                    score: label_with_score.score,
                    label: label_with_score.label.clone(),
                    current_count: label_with_score.current_count,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalGlobalSuccessRateConfigEventRequest {
    pub entity_min_aggregates_size: u32,
    pub entity_default_success_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CalGlobalSuccessRateEventRequest {
    pub entity_id: String,
    pub entity_params: String,
    pub entity_labels: Vec<String>,
    pub global_labels: Vec<String>,
    pub config: Option<CalGlobalSuccessRateConfigEventRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateSuccessRateWindowConfig {
    pub max_aggregates_size: Option<u32>,
    pub current_block_threshold: Option<api_routing::CurrentBlockThreshold>,
}

impl From<&api_routing::SuccessBasedRoutingConfigBody> for UpdateSuccessRateWindowConfig {
    fn from(value: &api_routing::SuccessBasedRoutingConfigBody) -> Self {
        Self {
            max_aggregates_size: value.max_aggregates_size,
            current_block_threshold: value.current_block_threshold.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateLabelWithStatusEventRequest {
    pub label: String,
    pub status: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateSuccessRateWindowEventRequest {
    pub id: String,
    pub params: String,
    pub labels_with_status: Vec<UpdateLabelWithStatusEventRequest>,
    pub config: Option<UpdateSuccessRateWindowConfig>,
    pub global_labels_with_status: Vec<UpdateLabelWithStatusEventRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateSuccessRateWindowEventResponse {
    pub status: UpdationStatusEventResponse,
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl TryFrom<&ir_client::success_rate_client::UpdateSuccessRateWindowResponse>
    for UpdateSuccessRateWindowEventResponse
{
    type Error = errors::RoutingError;

    fn try_from(
        value: &ir_client::success_rate_client::UpdateSuccessRateWindowResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: match value.status {
                0 => UpdationStatusEventResponse::WindowUpdationSucceeded,
                1 => UpdationStatusEventResponse::WindowUpdationFailed,
                _ => {
                    return Err(errors::RoutingError::GenericNotFoundError {
                        field: "unknown updation status from dynamic routing service".to_string(),
                    })
                }
            },
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdationStatusEventResponse {
    WindowUpdationSucceeded,
    WindowUpdationFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LabelWithBucketNameEventRequest {
    pub label: String,
    pub bucket_name: String,
}

impl From<&api_routing::RoutableConnectorChoiceWithBucketName> for LabelWithBucketNameEventRequest {
    fn from(value: &api_routing::RoutableConnectorChoiceWithBucketName) -> Self {
        Self {
            label: value.routable_connector_choice.to_string(),
            bucket_name: value.bucket_name.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateEliminationBucketEventRequest {
    pub id: String,
    pub params: String,
    pub labels_with_bucket_name: Vec<LabelWithBucketNameEventRequest>,
    pub config: Option<EliminationRoutingEventBucketConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateEliminationBucketEventResponse {
    pub status: EliminationUpdationStatusEventResponse,
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl TryFrom<&ir_client::elimination_based_client::UpdateEliminationBucketResponse>
    for UpdateEliminationBucketEventResponse
{
    type Error = errors::RoutingError;

    fn try_from(
        value: &ir_client::elimination_based_client::UpdateEliminationBucketResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: match value.status {
                0 => EliminationUpdationStatusEventResponse::BucketUpdationSucceeded,
                1 => EliminationUpdationStatusEventResponse::BucketUpdationFailed,
                _ => {
                    return Err(errors::RoutingError::GenericNotFoundError {
                        field: "unknown updation status from dynamic routing service".to_string(),
                    })
                }
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EliminationUpdationStatusEventResponse {
    BucketUpdationSucceeded,
    BucketUpdationFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContractLabelInformationEventRequest {
    pub label: String,
    pub target_count: u64,
    pub target_time: u64,
    pub current_count: u64,
}

impl From<&api_routing::LabelInformation> for ContractLabelInformationEventRequest {
    fn from(value: &api_routing::LabelInformation) -> Self {
        Self {
            label: value.label.clone(),
            target_count: value.target_count,
            target_time: value.target_time,
            current_count: 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateContractRequestEventRequest {
    pub id: String,
    pub params: String,
    pub labels_information: Vec<ContractLabelInformationEventRequest>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateContractEventResponse {
    pub status: ContractUpdationStatusEventResponse,
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl TryFrom<&ir_client::contract_routing_client::UpdateContractResponse>
    for UpdateContractEventResponse
{
    type Error = errors::RoutingError;

    fn try_from(
        value: &ir_client::contract_routing_client::UpdateContractResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: match value.status {
                0 => ContractUpdationStatusEventResponse::ContractUpdationSucceeded,
                1 => ContractUpdationStatusEventResponse::ContractUpdationFailed,
                _ => {
                    return Err(errors::RoutingError::GenericNotFoundError {
                        field: "unknown updation status from dynamic routing service".to_string(),
                    })
                }
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractUpdationStatusEventResponse {
    ContractUpdationSucceeded,
    ContractUpdationFailed,
}
