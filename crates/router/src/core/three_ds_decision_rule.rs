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
