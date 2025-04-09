use api_models::gsm as gsm_api_types;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    db::gsm::GsmInterface,
    services,
    types::transformers::ForeignInto,
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

    let gsm_retrieved_record =
        GsmInterface::find_gsm_rule(db, connector.to_string(), flow, sub_flow, code, message)
            .await
            .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                message: "GSM with given key does not exist in our records".to_string(),
            })?;

    let gsm_feature_data =
        gsm_retrieved_record
            .feature_data
            .map(|gsm_feature_data| match gsm_feature_data {
                hyperswitch_domain_models::gsm::FeatureData::Retry(retry_feature_data) => {
                    retry_feature_data
                }
            });

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
        alternate_network_possible,
        feature,
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
            step_up_possible,
            unified_code,
            unified_message,
            error_category,
            clear_pan_possible,
            feature_data: Some(hyperswitch_domain_models::gsm::FeatureData::Retry(
                hyperswitch_domain_models::gsm::RetryFeatureData {
                    step_up_possible: gsm_feature_data
                        .clone()
                        .map(|data| data.step_up_possible)
                        .or(Some(gsm_retrieved_record.step_up_possible))
                        .or(step_up_possible)
                        .unwrap_or_default(),
                    clear_pan_possible: gsm_feature_data
                        .clone()
                        .map(|data| data.clear_pan_possible)
                        .or(Some(gsm_retrieved_record.clear_pan_possible))
                        .or(clear_pan_possible)
                        .unwrap_or_default(),
                    alternate_network_possible: gsm_feature_data
                        .clone()
                        .map(|data| data.alternate_network_possible)
                        .or(alternate_network_possible)
                        .unwrap_or_default(),
                    decision: gsm_feature_data
                        .clone()
                        .map(|data| data.decision)
                        .or(Some(gsm_retrieved_record.decision))
                        .or(decision)
                        .unwrap_or_default(),
                },
            )),
            feature,
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
