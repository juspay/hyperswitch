use api_models::gsm as gsm_api_types;
use diesel_models::gsm as storage;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors,
        errors::{RouterResponse, StorageErrorExt},
    },
    db::gsm::GsmInterface,
    services,
    types::transformers::ForeignInto,
    AppState,
};

#[instrument(skip_all)]
pub async fn create_gsm_rule(
    state: AppState,
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
    state: AppState,
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
    state: AppState,
    gsm_request: gsm_api_types::GsmUpdateRequest,
) -> RouterResponse<gsm_api_types::GsmResponse> {
    let db = state.store.as_ref();
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
    } = gsm_request;
    GsmInterface::update_gsm_rule(
        db,
        connector.to_string(),
        flow,
        sub_flow,
        code,
        message,
        storage::GatewayStatusMappingUpdate {
            decision: decision.map(|d| d.to_string()),
            status,
            router_error: Some(router_error),
            step_up_possible,
            unified_code,
            unified_message,
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
    state: AppState,
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
                    .into_report()
                    .attach_printable("Failed while Deleting Gsm rule, got response as false")
            }
        }
        Err(err) => Err(err),
    }
}
