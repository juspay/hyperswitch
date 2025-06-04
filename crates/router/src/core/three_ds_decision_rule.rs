pub mod utils;

use common_types::three_ds_decision_rule_engine::ThreeDSDecisionRule;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    frontend::ast,
};
use hyperswitch_domain_models::merchant_context::MerchantContext;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors,
        errors::{RouterResponse, StorageErrorExt},
    },
    services, SessionState,
};

#[instrument(skip_all)]
pub async fn execute_three_ds_decision_rule(
    state: SessionState,
    merchant_context: MerchantContext,
    request: api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> RouterResponse<api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteResponse> {
    let db = state.store.as_ref();
    // Retrieve the rule from database
    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &request.routing_id,
            merchant_context.get_merchant_account().get_id(),
        )
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
    let backend_input = construct_backend_input(request.clone())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to construct backend input for 3DS decision rule execution")?;
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
    // Construct response
    let response = api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteResponse {
        decision: final_decision,
    };
    Ok(services::ApplicationResponse::Json(response))
}

// Helper function to construct backend input from request
fn construct_backend_input(
    request: api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> Result<dsl_inputs::BackendInput, errors::ApiErrorResponse> {
    // Construct payment input
    let payment_input = dsl_inputs::PaymentInput {
        amount: request.payment.amount,
        currency: request.payment.currency,
        authentication_type: None,
        capture_method: None,
        business_country: None,
        billing_country: None,
        business_label: None,
        setup_future_usage: None,
        card_bin: None,
    };

    // Construct payment method input
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: request
            .payment_method
            .as_ref()
            .and_then(|pm| pm.card_network.clone()),
    };

    // Construct mandate data (empty for now as it's not used in 3DS decision rules)
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: None,
        mandate_type: None,
        payment_type: None,
    };

    // Construct acquirer data
    let acquirer_data = request.acquirer.map(|data| dsl_inputs::AcquirerDataInput {
        country: data.country,
        fraud_rate: data.fraud_rate,
    });

    // Construct customer device data
    let customer_device_data =
        request
            .customer_device
            .map(|data| dsl_inputs::CustomerDeviceDataInput {
                platform: data.platform,
                device_type: data.device_type,
                display_size: data.display_size,
            });

    // Construct issuer data
    let issuer_data = request.issuer.map(|data| dsl_inputs::IssuerDataInput {
        name: data.name,
        country: data.country,
    });

    // Construct final backend input
    let backend_input = dsl_inputs::BackendInput {
        metadata: None,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
        acquirer_data,
        customer_device_data,
        issuer_data,
    };

    Ok(backend_input)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Algorithm {
    data: serde_json::Value,
}
