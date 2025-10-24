use api_models::gsm as gsm_api_types;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::gsm::GsmInterface,
    services,
    types::transformers::{ForeignFrom, ForeignInto},
    SessionState,
};

#[instrument(skip_all)]
pub async fn create_gsm_rule(
    state: SessionState,
    gsm_rule: gsm_api_types::GsmCreateRequest,
) -> RouterResponse<gsm_api_types::GsmResponse> {
    let db = state.store.as_ref();
    GsmInterface::add_gsm_rule(db, gsm_rule.foreign_into())
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "GSM with given key already exists in our records".to_string(),
        })
        .map(|gsm| services::ApplicationResponse::Json(gsm.foreign_into()))
}

#[instrument(skip_all)]
pub async fn retrieve_gsm_rule(
    state: SessionState,
    gsm_request: gsm_api_types::GsmRetrieveRequest,
) -> RouterResponse<gsm_api_types::GsmResponse> {
    let db = state.store.as_ref();
    let gsm_api_types::GsmRetrieveRequest {
        connector,
        flow,
        sub_flow,
        code,
        message,
    } = gsm_request;
    GsmInterface::find_gsm_rule(db, connector.to_string(), flow, sub_flow, code, message)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "GSM with given key does not exist in our records".to_string(),
        })
        .map(|gsm| services::ApplicationResponse::Json(gsm.foreign_into()))
}

#[instrument(skip_all)]
pub async fn update_gsm_rule(
    state: SessionState,
    gsm_request: gsm_api_types::GsmUpdateRequest,
) -> RouterResponse<gsm_api_types::GsmResponse> {
    let db = state.store.as_ref();
    let connector = gsm_request.connector.clone();
    let flow = gsm_request.flow.clone();
    let code = gsm_request.code.clone();
    let sub_flow = gsm_request.sub_flow.clone();
    let message = gsm_request.message.clone();

    let gsm_db_record =
        GsmInterface::find_gsm_rule(db, connector.to_string(), flow, sub_flow, code, message)
            .await
            .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                message: "GSM with given key does not exist in our records".to_string(),
            })?;

    let inferred_feature_info = <(
        common_enums::GsmFeature,
        common_types::domain::GsmFeatureData,
    )>::foreign_from((&gsm_request, gsm_db_record));

    let gsm_api_types::GsmUpdateRequest {
        connector,
        flow,
        sub_flow,
        code,
        message,
        decision,
        status,
        router_error,
        step_up_possible,
        unified_code,
        unified_message,
        error_category,
        clear_pan_possible,
        feature,
        feature_data,
    } = gsm_request;
    GsmInterface::update_gsm_rule(
        db,
        connector.to_string(),
        flow,
        sub_flow,
        code,
        message,
        hyperswitch_domain_models::gsm::GatewayStatusMappingUpdate {
            decision,
            status,
            router_error: Some(router_error),
            step_up_possible: feature_data
                .as_ref()
                .and_then(|feature_data| feature_data.get_retry_feature_data())
                .map(|retry_feature_data| retry_feature_data.is_step_up_possible())
                .or(step_up_possible),
            unified_code,
            unified_message,
            error_category,
            clear_pan_possible: feature_data
                .as_ref()
                .and_then(|feature_data| feature_data.get_retry_feature_data())
                .map(|retry_feature_data| retry_feature_data.is_clear_pan_possible())
                .or(clear_pan_possible),
            feature_data: feature_data.or(Some(inferred_feature_info.1)),
            feature: feature.or(Some(inferred_feature_info.0)),
        },
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "GSM with given key does not exist in our records".to_string(),
    })
    .attach_printable("Failed while updating Gsm rule")
    .map(|gsm| services::ApplicationResponse::Json(gsm.foreign_into()))
}

#[instrument(skip_all)]
pub async fn delete_gsm_rule(
    state: SessionState,
    gsm_request: gsm_api_types::GsmDeleteRequest,
) -> RouterResponse<gsm_api_types::GsmDeleteResponse> {
    let db = state.store.as_ref();
    let gsm_api_types::GsmDeleteRequest {
        connector,
        flow,
        sub_flow,
        code,
        message,
    } = gsm_request;
    match GsmInterface::delete_gsm_rule(
        db,
        connector.to_string(),
        flow.to_owned(),
        sub_flow.to_owned(),
        code.to_owned(),
        message.to_owned(),
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "GSM with given key does not exist in our records".to_string(),
    })
    .attach_printable("Failed while Deleting Gsm rule")
    {
        Ok(is_deleted) => {
            if is_deleted {
                Ok(services::ApplicationResponse::Json(
                    gsm_api_types::GsmDeleteResponse {
                        gsm_rule_delete: true,
                        connector,
                        flow,
                        sub_flow,
                        code,
                    },
                ))
            } else {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while Deleting Gsm rule, got response as false")
            }
        }
        Err(err) => Err(err),
    }
}
