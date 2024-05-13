mod transformers;

use api_models::{
    conditional_configs::{ConditionalConfigs, DecisionManagerRecord},
    routing,
};
use common_utils::{ext_traits::StringExt, static_cache::StaticCache};
use error_stack::ResultExt;
use euclid::backend::{self, inputs as dsl_inputs, EuclidBackend};
use router_env::{instrument, tracing};

use super::routing::make_dsl_input;
use crate::{
    core::{errors, errors::ConditionalConfigError as ConfigError, payments},
    routes,
};

static CONF_CACHE: StaticCache<backend::VirInterpreterBackend<ConditionalConfigs>> =
    StaticCache::new();
pub type ConditionalConfigResult<O> = errors::CustomResult<O, ConfigError>;

#[instrument(skip_all)]
pub async fn perform_decision_management<F: Clone>(
    state: &routes::SessionState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    merchant_id: &str,
    payment_data: &mut payments::PaymentData<F>,
) -> ConditionalConfigResult<ConditionalConfigs> {
    let algorithm_id = if let Some(id) = algorithm_ref.config_algo_id {
        id
    } else {
        return Ok(ConditionalConfigs::default());
    };

    let key = ensure_algorithm_cached(
        state,
        merchant_id,
        algorithm_ref.timestamp,
        algorithm_id.as_str(),
    )
    .await?;
    let cached_algo = CONF_CACHE
        .retrieve(&key)
        .change_context(ConfigError::CacheMiss)
        .attach_printable("Unable to retrieve cached routing algorithm even after refresh")?;
    let backend_input =
        make_dsl_input(payment_data).change_context(ConfigError::InputConstructionError)?;
    let interpreter = cached_algo.as_ref();
    execute_dsl_and_get_conditional_config(backend_input, interpreter).await
}

#[instrument(skip_all)]
pub async fn ensure_algorithm_cached(
    state: &routes::SessionState,
    merchant_id: &str,
    timestamp: i64,
    algorithm_id: &str,
) -> ConditionalConfigResult<String> {
    let key = format!("dsl_{merchant_id}");
    let present = CONF_CACHE
        .present(&key)
        .change_context(ConfigError::DslCachePoisoned)
        .attach_printable("Error checking presece of DSL")?;
    let expired = CONF_CACHE
        .expired(&key, timestamp)
        .change_context(ConfigError::DslCachePoisoned)
        .attach_printable("Error checking presence of DSL")?;
    if !present || expired {
        refresh_routing_cache(state, key.clone(), algorithm_id, timestamp).await?;
    };
    Ok(key)
}

#[instrument(skip_all)]
pub async fn refresh_routing_cache(
    state: &routes::SessionState,
    key: String,
    algorithm_id: &str,
    timestamp: i64,
) -> ConditionalConfigResult<()> {
    let config = state
        .store
        .find_config_by_key(algorithm_id)
        .await
        .change_context(ConfigError::DslMissingInDb)
        .attach_printable("Error parsing DSL from config")?;
    let rec: DecisionManagerRecord = config
        .config
        .parse_struct("Program")
        .change_context(ConfigError::DslParsingError)
        .attach_printable("Error parsing routing algorithm from configs")?;
    let interpreter: backend::VirInterpreterBackend<ConditionalConfigs> =
        backend::VirInterpreterBackend::with_program(rec.program)
            .change_context(ConfigError::DslBackendInitError)
            .attach_printable("Error initializing DSL interpreter backend")?;
    CONF_CACHE
        .save(key, interpreter, timestamp)
        .change_context(ConfigError::DslCachePoisoned)
        .attach_printable("Error saving DSL to cache")?;
    Ok(())
}

pub async fn execute_dsl_and_get_conditional_config(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<ConditionalConfigs>,
) -> ConditionalConfigResult<ConditionalConfigs> {
    let routing_output = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection)
        .change_context(ConfigError::DslExecutionError)?;
    Ok(routing_output)
}
