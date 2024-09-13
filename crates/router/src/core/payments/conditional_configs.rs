mod transformers;

use api_models::{
    conditional_configs::{ConditionalConfigs, DecisionManagerRecord},
    routing,
};
use common_utils::ext_traits::StringExt;
use error_stack::ResultExt;
use euclid::backend::{self, inputs as dsl_inputs, EuclidBackend};
use router_env::{instrument, tracing};
use storage_impl::redis::cache::{self, DECISION_MANAGER_CACHE};

use super::routing::make_dsl_input;
use crate::{
    core::{errors, errors::ConditionalConfigError as ConfigError, routing as core_routing},
    routes,
};
pub type ConditionalConfigResult<O> = errors::CustomResult<O, ConfigError>;

#[instrument(skip_all)]
pub async fn perform_decision_management(
    state: &routes::SessionState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_data: &core_routing::PaymentsDslInput<'_>,
) -> ConditionalConfigResult<ConditionalConfigs> {
    let algorithm_id = if let Some(id) = algorithm_ref.config_algo_id {
        id
    } else {
        return Ok(ConditionalConfigs::default());
    };
    let db = &*state.store;

    let key = merchant_id.get_dsl_config();

    let find_key_from_db = || async {
        let config = db.find_config_by_key(&algorithm_id).await?;

        let rec: DecisionManagerRecord = config
            .config
            .parse_struct("Program")
            .change_context(errors::StorageError::DeserializationFailed)
            .attach_printable("Error parsing routing algorithm from configs")?;

        backend::VirInterpreterBackend::with_program(rec.program)
            .change_context(errors::StorageError::ValueNotFound("Program".to_string()))
            .attach_printable("Error initializing DSL interpreter backend")
    };

    let interpreter = cache::get_or_populate_in_memory(
        db.get_cache_store().as_ref(),
        &key,
        find_key_from_db,
        &DECISION_MANAGER_CACHE,
    )
    .await
    .change_context(ConfigError::DslCachePoisoned)?;

    let backend_input =
        make_dsl_input(payment_data).change_context(ConfigError::InputConstructionError)?;

    execute_dsl_and_get_conditional_config(backend_input, &interpreter)
}

pub fn execute_dsl_and_get_conditional_config(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<ConditionalConfigs>,
) -> ConditionalConfigResult<ConditionalConfigs> {
    let routing_output = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection)
        .change_context(ConfigError::DslExecutionError)?;
    Ok(routing_output)
}
