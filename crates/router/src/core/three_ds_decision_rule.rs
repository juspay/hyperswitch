use common_utils::transformers::ForeignTryFrom;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_context::MerchantContext;
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors,
        errors::{RouterResponse, StorageErrorExt},
    },
    db::three_ds_decision_rule::ThreeDSDecisionRuleInterface,
    services, SessionState,
};

#[instrument(skip_all)]
pub async fn create_three_ds_decision_rule(
    state: SessionState,
    merchant_context: MerchantContext,
    three_ds_decision_rule: api_models::three_ds_decision_rule::ThreeDsDecisionRuleRecord,
) -> RouterResponse<api_models::three_ds_decision_rule::ThreeDsDecisionRuleResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();
    let three_ds_decision_rule_domain_model =
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule::foreign_try_from(
            three_ds_decision_rule,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert ThreeDSDecisionRule to Domain Model")?;
    let db_record = ThreeDSDecisionRuleInterface::insert_three_ds_decision_rule(
        db,
        key_manager_state,
        merchant_key_store,
        three_ds_decision_rule_domain_model,
    )
    .await
    .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
        message: "ThreeDS Decision Rule with given Id already exists in our records".to_string(),
    })?;
    Ok(services::ApplicationResponse::Json(
        api_models::three_ds_decision_rule::ThreeDsDecisionRuleResponse::foreign_try_from(
            db_record,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert ThreeDSDecisionRule to API Model")?,
    ))
}

#[instrument(skip_all)]
pub async fn retrieve_three_ds_decision_rule(
    state: SessionState,
    merchant_context: MerchantContext,
    rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
) -> RouterResponse<api_models::three_ds_decision_rule::ThreeDsDecisionRuleResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();
    let db_record = ThreeDSDecisionRuleInterface::find_three_ds_decision_rule_by_id(
        db,
        key_manager_state,
        merchant_key_store,
        rule_id,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "ThreeDS Decision Rule with given Id does not exist in our records".to_string(),
    })?;
    Ok(services::ApplicationResponse::Json(
        api_models::three_ds_decision_rule::ThreeDsDecisionRuleResponse::foreign_try_from(
            db_record,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert ThreeDSDecisionRule to API Model")?,
    ))
}

#[instrument(skip_all)]
pub async fn update_three_ds_decision_rule(
    state: SessionState,
    merchant_context: MerchantContext,
    rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    update_request: api_models::three_ds_decision_rule::ThreeDsDecisionRuleUpdateRequest,
) -> RouterResponse<api_models::three_ds_decision_rule::ThreeDsDecisionRuleResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();
    let existing_rule = ThreeDSDecisionRuleInterface::find_three_ds_decision_rule_by_id(
        db,
        key_manager_state,
        merchant_key_store,
        rule_id,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "ThreeDS Decision Rule with given Id does not exist in our records".to_string(),
    })?;
    let rule = update_request.program.map(|program| {
        common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord { program }
    });
    let update =
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate::Update {
            rule,
            name: update_request.name,
            description: update_request.description,
        };
    let updated_rule = ThreeDSDecisionRuleInterface::update_three_ds_decision_rule(
        db,
        key_manager_state,
        merchant_key_store,
        existing_rule,
        update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update 3DS decision rule")?;
    Ok(services::ApplicationResponse::Json(
        api_models::three_ds_decision_rule::ThreeDsDecisionRuleResponse::foreign_try_from(
            updated_rule,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert ThreeDSDecisionRule to API Model")?,
    ))
}

#[instrument(skip_all)]
pub async fn delete_three_ds_decision_rule(
    state: SessionState,
    _merchant_context: MerchantContext,
    rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
) -> RouterResponse<()> {
    let db = state.store.as_ref();
    let deleted = ThreeDSDecisionRuleInterface::delete_three_ds_decision_rule(db, rule_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete 3DS decision rule")?;
    if !deleted {
        Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to delete 3DS decision rule")?
    } else {
        Ok(services::ApplicationResponse::StatusOk)
    }
}

use euclid::backend::{self, inputs as dsl_inputs, EuclidBackend};

#[instrument(skip_all)]
pub async fn execute_three_ds_decision_rule(
    state: SessionState,
    merchant_context: MerchantContext,
    rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    request: api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> RouterResponse<api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();
    // Retrieve the rule from database
    let rule = ThreeDSDecisionRuleInterface::find_three_ds_decision_rule_by_id(
        db,
        key_manager_state,
        merchant_key_store,
        rule_id,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "ThreeDS Decision Rule with given Id does not exist in our records".to_string(),
    })?;
    // Construct backend input from request
    let backend_input = construct_backend_input(request)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to construct backend input for 3DS decision rule execution")?;
    // Initialize interpreter with the rule program
    let interpreter = backend::VirInterpreterBackend::with_program(rule.rule.program)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error initializing DSL interpreter backend")?;
    // Execute the rule
    let result = interpreter
        .execute(backend_input)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error executing 3DS decision rule")?;
    // Construct response
    let response = api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteResponse {
        decision: result.connector_selection.decision,
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
