pub mod utils;

use common_types::three_ds_decision_rule_engine::ThreeDSDecisionRule;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    frontend::ast,
};
use hyperswitch_domain_models::platform::Platform;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors,
        errors::{RouterResponse, StorageErrorExt},
    },
    services,
    types::transformers::ForeignFrom,
    SessionState,
};

#[instrument(skip_all)]
pub async fn execute_three_ds_decision_rule(
    state: SessionState,
    platform: Platform,
    request: api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> RouterResponse<api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteResponse> {
    let decision = get_three_ds_decision_rule_output(
        &state,
        platform.get_processor().get_account().get_id(),
        request.clone(),
    )
    .await?;
    // Construct response
    let response =
        api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteResponse { decision };
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn get_three_ds_decision_rule_output(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    request: api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> errors::RouterResult<common_types::three_ds_decision_rule_engine::ThreeDSDecision> {
    let db = state.store.as_ref();
    // Retrieve the rule from database
    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(&request.routing_id, merchant_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
    let algorithm: Algorithm = routing_algorithm
        .algorithm_data
        .parse_value("Algorithm")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error parsing program from three_ds_decision rule algorithm")?;
    let program: ast::Program<ThreeDSDecisionRule> = algorithm
        .data
        .parse_value("Program<ThreeDSDecisionRule>")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error parsing program from three_ds_decision rule algorithm")?;
    // Construct backend input from request
    let backend_input = dsl_inputs::BackendInput::foreign_from(request.clone());
    // Initialize interpreter with the rule program
    let interpreter = backend::VirInterpreterBackend::with_program(program)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error initializing DSL interpreter backend")?;
    // Execute the rule
    let result = interpreter
        .execute(backend_input)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error executing 3DS decision rule")?;
    // Apply PSD2 validations to the decision
    let final_decision =
        utils::apply_psd2_validations_during_execute(result.get_output().get_decision(), &request);
    Ok(final_decision)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Algorithm {
    data: serde_json::Value,
}
