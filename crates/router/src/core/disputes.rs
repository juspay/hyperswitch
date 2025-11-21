use std::{collections::HashMap, ops::Deref, str::FromStr};

use api_models::{
    admin::MerchantConnectorInfo, disputes as dispute_models, files as files_api_models,
};
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::ResultExt;
use router_env::{
    instrument, logger,
    tracing::{self, Instrument},
};
use strum::IntoEnumIterator;
pub mod transformers;

use super::{
    errors::{self, ConnectorErrorExt, RouterResponse, StorageErrorExt},
    metrics,
};
use crate::{
    core::{files, payments, utils as core_utils, webhooks},
    routes::{app::StorageInterface, metrics::TASKS_ADDED_COUNT, SessionState},
    services,
    types::{
        api::{self, disputes},
        domain,
        storage::enums as storage_enums,
        transformers::{ForeignFrom, ForeignInto},
        AcceptDisputeRequestData, AcceptDisputeResponse, DefendDisputeRequestData,
        DefendDisputeResponse, DisputePayload, DisputeSyncData, DisputeSyncResponse,
        FetchDisputesRequestData, FetchDisputesResponse, SubmitEvidenceRequestData,
        SubmitEvidenceResponse,
    },
    workflows::process_dispute,
};

pub(crate) fn should_call_connector_for_dispute_sync(
    force_sync: Option<bool>,
    dispute_status: storage_enums::DisputeStatus,
) -> bool {
    force_sync == Some(true)
        && matches!(
            dispute_status,
            common_enums::DisputeStatus::DisputeAccepted
                | common_enums::DisputeStatus::DisputeChallenged
                | common_enums::DisputeStatus::DisputeOpened
        )
}

#[instrument(skip(state))]
pub async fn retrieve_dispute(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    req: dispute_models::DisputeRetrieveRequest,
) -> RouterResponse<api_models::disputes::DisputeResponse> {
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(
            platform.get_processor().get_account().get_id(),
            &req.dispute_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id,
        })?;
    core_utils::validate_profile_id_from_auth_layer(profile_id.clone(), &dispute)?;

    #[cfg(feature = "v1")]
    let dispute_response =
        if should_call_connector_for_dispute_sync(req.force_sync, dispute.dispute_status) {
            let db = &state.store;
            core_utils::validate_profile_id_from_auth_layer(profile_id.clone(), &dispute)?;
            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &dispute.payment_id,
                    platform.get_processor().get_account().get_id(),
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            let payment_attempt = db
                .find_payment_attempt_by_attempt_id_merchant_id(
                    &dispute.attempt_id,
                    platform.get_processor().get_account().get_id(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &dispute.connector,
                api::GetToken::Connector,
                dispute.merchant_connector_id.clone(),
            )?;

            let connector_integration: services::BoxedDisputeConnectorIntegrationInterface<
                api::Dsync,
                DisputeSyncData,
                DisputeSyncResponse,
            > = connector_data.connector.get_connector_integration();
            let router_data = core_utils::construct_dispute_sync_router_data(
                &state,
                &payment_intent,
                &payment_attempt,
                &platform,
                &dispute,
            )
            .await?;
            let response = services::execute_connector_processing_step(
                &state,
                connector_integration,
                &router_data,
                payments::CallConnectorAction::Trigger,
                None,
                None,
            )
            .await
            .to_dispute_failed_response()
            .attach_printable("Failed while calling accept dispute connector api")?;

            let dispute_sync_response = response.response.map_err(|err| {
                errors::ApiErrorResponse::ExternalConnectorError {
                    code: err.code,
                    message: err.message,
                    connector: dispute.connector.clone(),
                    status_code: err.status_code,
                    reason: err.reason,
                }
            })?;

            let business_profile = state
                .store
                .find_business_profile_by_profile_id(
                    platform.get_processor().get_key_store(),
                    &payment_attempt.profile_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                    id: payment_attempt.profile_id.get_string_repr().to_owned(),
                })?;

            update_dispute_data(
                &state,
                platform,
                business_profile,
                Some(dispute.clone()),
                dispute_sync_response,
                payment_attempt,
                dispute.connector.as_str(),
            )
            .await
            .attach_printable("Dispute update failed")?
        } else {
            api_models::disputes::DisputeResponse::foreign_from(dispute)
        };

    #[cfg(not(feature = "v1"))]
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(dispute);

    Ok(services::ApplicationResponse::Json(dispute_response))
}

#[instrument(skip(state))]
pub async fn retrieve_disputes_list(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    constraints: api_models::disputes::DisputeListGetConstraints,
) -> RouterResponse<Vec<api_models::disputes::DisputeResponse>> {
    let dispute_list_constraints = &(constraints.clone(), profile_id_list.clone()).try_into()?;
    let disputes = state
        .store
        .find_disputes_by_constraints(
            platform.get_processor().get_account().get_id(),
            dispute_list_constraints,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve disputes")?;
    let disputes_list = disputes
        .into_iter()
        .map(api_models::disputes::DisputeResponse::foreign_from)
        .collect();
    Ok(services::ApplicationResponse::Json(disputes_list))
}

#[cfg(feature = "v2")]
#[instrument(skip(state))]
pub async fn accept_dispute(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    req: disputes::DisputeId,
) -> RouterResponse<dispute_models::DisputeResponse> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip(state))]
pub async fn get_filters_for_disputes(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
) -> RouterResponse<api_models::disputes::DisputeListFilters> {
    let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
        super::admin::list_payment_connectors(
            state,
            platform.get_processor().get_account().get_id().to_owned(),
            profile_id_list,
        )
        .await?
    {
        data
    } else {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InternalServerError
        ))
        .attach_printable(
            "Failed to retrieve merchant connector accounts while fetching dispute list filters.",
        );
    };

    let connector_map = merchant_connector_accounts
        .into_iter()
        .filter_map(|merchant_connector_account| {
            merchant_connector_account
                .connector_label
                .clone()
                .map(|label| {
                    let info = merchant_connector_account.to_merchant_connector_info(&label);
                    (merchant_connector_account.connector_name, info)
                })
        })
        .fold(
            HashMap::new(),
            |mut map: HashMap<String, Vec<MerchantConnectorInfo>>, (connector_name, info)| {
                map.entry(connector_name).or_default().push(info);
                map
            },
        );

    Ok(services::ApplicationResponse::Json(
        api_models::disputes::DisputeListFilters {
            connector: connector_map,
            currency: storage_enums::Currency::iter().collect(),
            dispute_status: storage_enums::DisputeStatus::iter().collect(),
            dispute_stage: storage_enums::DisputeStage::iter().collect(),
        },
    ))
}

#[cfg(feature = "v1")]
#[instrument(skip(state))]
pub async fn accept_dispute(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    req: disputes::DisputeId,
) -> RouterResponse<dispute_models::DisputeResponse> {
    let db = &state.store;
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(
            platform.get_processor().get_account().get_id(),
            &req.dispute_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id,
        })?;
    core_utils::validate_profile_id_from_auth_layer(profile_id, &dispute)?;
    let dispute_id = dispute.dispute_id.clone();
    common_utils::fp_utils::when(
        !core_utils::should_proceed_with_accept_dispute(
            dispute.dispute_stage,
            dispute.dispute_status,
        ),
        || {
            metrics::ACCEPT_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(1, &[]);
            Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
            reason: format!(
                "This dispute cannot be accepted because the dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
        })
        },
    )?;

    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &dispute.payment_id,
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &dispute.attempt_id,
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &dispute.connector,
        api::GetToken::Connector,
        dispute.merchant_connector_id.clone(),
    )?;
    let connector_integration: services::BoxedDisputeConnectorIntegrationInterface<
        api::Accept,
        AcceptDisputeRequestData,
        AcceptDisputeResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = core_utils::construct_accept_dispute_router_data(
        &state,
        &payment_intent,
        &payment_attempt,
        &platform,
        &dispute,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_dispute_failed_response()
    .attach_printable("Failed while calling accept dispute connector api")?;
    let accept_dispute_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: dispute.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;
    let update_dispute = diesel_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_status: accept_dispute_response.dispute_status,
        connector_status: accept_dispute_response.connector_status.clone(),
    };
    let updated_dispute = db
        .update_dispute(dispute.clone(), update_dispute)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(updated_dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}

#[cfg(feature = "v2")]
#[instrument(skip(state))]
pub async fn submit_evidence(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    req: dispute_models::SubmitEvidenceRequest,
) -> RouterResponse<dispute_models::DisputeResponse> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip(state))]
pub async fn submit_evidence(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    req: dispute_models::SubmitEvidenceRequest,
) -> RouterResponse<dispute_models::DisputeResponse> {
    let db = &state.store;
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(
            platform.get_processor().get_account().get_id(),
            &req.dispute_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id.clone(),
        })?;
    core_utils::validate_profile_id_from_auth_layer(profile_id, &dispute)?;
    let dispute_id = dispute.dispute_id.clone();
    common_utils::fp_utils::when(
        !core_utils::should_proceed_with_submit_evidence(
            dispute.dispute_stage,
            dispute.dispute_status,
        ),
        || {
            metrics::EVIDENCE_SUBMISSION_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(1, &[]);
            Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
                reason: format!(
                "Evidence cannot be submitted because the dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
            })
        },
    )?;
    let submit_evidence_request_data =
        transformers::get_evidence_request_data(&state, &platform, req, &dispute).await?;

    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &dispute.payment_id,
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &dispute.attempt_id,
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &dispute.connector,
        api::GetToken::Connector,
        dispute.merchant_connector_id.clone(),
    )?;

    let connector_integration: services::BoxedDisputeConnectorIntegrationInterface<
        api::Evidence,
        SubmitEvidenceRequestData,
        SubmitEvidenceResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = core_utils::construct_submit_evidence_router_data(
        &state,
        &payment_intent,
        &payment_attempt,
        &platform,
        &dispute,
        submit_evidence_request_data,
    )
    .await?;
    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_dispute_failed_response()
    .attach_printable("Failed while calling submit evidence connector api")?;
    let submit_evidence_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: dispute.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;
    //Defend Dispute Optionally if connector expects to defend / submit evidence in a separate api call
    let (dispute_status, connector_status) = if connector_data
        .connector_name
        .requires_defend_dispute()
    {
        let connector_integration_defend_dispute: services::BoxedDisputeConnectorIntegrationInterface<
                api::Defend,
                DefendDisputeRequestData,
                DefendDisputeResponse,
            > = connector_data.connector.get_connector_integration();
        let defend_dispute_router_data = core_utils::construct_defend_dispute_router_data(
            &state,
            &payment_intent,
            &payment_attempt,
            &platform,
            &dispute,
        )
        .await?;
        let defend_response = services::execute_connector_processing_step(
            &state,
            connector_integration_defend_dispute,
            &defend_dispute_router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_dispute_failed_response()
        .attach_printable("Failed while calling defend dispute connector api")?;
        let defend_dispute_response = defend_response.response.map_err(|err| {
            errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: dispute.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            }
        })?;
        (
            defend_dispute_response.dispute_status,
            defend_dispute_response.connector_status,
        )
    } else {
        (
            submit_evidence_response.dispute_status,
            submit_evidence_response.connector_status,
        )
    };
    let update_dispute = diesel_models::dispute::DisputeUpdate::StatusUpdate {
        dispute_status,
        connector_status,
    };
    let updated_dispute = db
        .update_dispute(dispute.clone(), update_dispute)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.to_owned(),
        })
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    let dispute_response = api_models::disputes::DisputeResponse::foreign_from(updated_dispute);
    Ok(services::ApplicationResponse::Json(dispute_response))
}

pub async fn attach_evidence(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    attach_evidence_request: api::AttachEvidenceRequest,
) -> RouterResponse<files_api_models::CreateFileResponse> {
    let db = &state.store;
    let dispute_id = attach_evidence_request
        .create_file_request
        .dispute_id
        .clone()
        .ok_or(errors::ApiErrorResponse::MissingDisputeId)?;
    let dispute = db
        .find_dispute_by_merchant_id_dispute_id(
            platform.get_processor().get_account().get_id(),
            &dispute_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.clone(),
        })?;
    core_utils::validate_profile_id_from_auth_layer(profile_id, &dispute)?;
    common_utils::fp_utils::when(
        !(dispute.dispute_stage == storage_enums::DisputeStage::Dispute
            && dispute.dispute_status == storage_enums::DisputeStatus::DisputeOpened),
        || {
            metrics::ATTACH_EVIDENCE_DISPUTE_STATUS_VALIDATION_FAILURE_METRIC.add(1, &[]);
            Err(errors::ApiErrorResponse::DisputeStatusValidationFailed {
                reason: format!(
                "Evidence cannot be attached because the dispute is in {} stage and has {} status",
                dispute.dispute_stage, dispute.dispute_status
            ),
            })
        },
    )?;
    let create_file_response = Box::pin(files::files_create_core(
        state.clone(),
        platform,
        attach_evidence_request.create_file_request,
    ))
    .await?;
    let file_id = match &create_file_response {
        services::ApplicationResponse::Json(res) => res.file_id.clone(),
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unexpected response received from files create core")?,
    };
    let dispute_evidence: api::DisputeEvidence = dispute
        .evidence
        .clone()
        .parse_value("DisputeEvidence")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing dispute evidence record")?;
    let updated_dispute_evidence = transformers::update_dispute_evidence(
        dispute_evidence,
        attach_evidence_request.evidence_type,
        file_id,
    );
    let update_dispute = diesel_models::dispute::DisputeUpdate::EvidenceUpdate {
        evidence: updated_dispute_evidence
            .encode_to_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while encoding dispute evidence")?
            .into(),
    };
    db.update_dispute(dispute, update_dispute)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.to_owned(),
        })
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    Ok(create_file_response)
}

#[instrument(skip(state))]
pub async fn retrieve_dispute_evidence(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    req: disputes::DisputeId,
) -> RouterResponse<Vec<api_models::disputes::DisputeEvidenceBlock>> {
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(
            platform.get_processor().get_account().get_id(),
            &req.dispute_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: req.dispute_id,
        })?;
    core_utils::validate_profile_id_from_auth_layer(profile_id, &dispute)?;
    let dispute_evidence: api::DisputeEvidence = dispute
        .evidence
        .clone()
        .parse_value("DisputeEvidence")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing dispute evidence record")?;
    let dispute_evidence_vec =
        transformers::get_dispute_evidence_vec(&state, platform, dispute_evidence).await?;
    Ok(services::ApplicationResponse::Json(dispute_evidence_vec))
}

pub async fn delete_evidence(
    state: SessionState,
    platform: domain::Platform,
    delete_evidence_request: dispute_models::DeleteEvidenceRequest,
) -> RouterResponse<serde_json::Value> {
    let dispute_id = delete_evidence_request.dispute_id.clone();
    let dispute = state
        .store
        .find_dispute_by_merchant_id_dispute_id(
            platform.get_processor().get_account().get_id(),
            &dispute_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.clone(),
        })?;
    let dispute_evidence: api::DisputeEvidence = dispute
        .evidence
        .clone()
        .parse_value("DisputeEvidence")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing dispute evidence record")?;
    let updated_dispute_evidence =
        transformers::delete_evidence_file(dispute_evidence, delete_evidence_request.evidence_type);
    let update_dispute = diesel_models::dispute::DisputeUpdate::EvidenceUpdate {
        evidence: updated_dispute_evidence
            .encode_to_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while encoding dispute evidence")?
            .into(),
    };
    state
        .store
        .update_dispute(dispute, update_dispute)
        .await
        .to_not_found_response(errors::ApiErrorResponse::DisputeNotFound {
            dispute_id: dispute_id.to_owned(),
        })
        .attach_printable_lazy(|| {
            format!("Unable to update dispute with dispute_id: {dispute_id}")
        })?;
    Ok(services::ApplicationResponse::StatusOk)
}

#[instrument(skip(state))]
pub async fn get_aggregates_for_disputes(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<dispute_models::DisputesAggregateResponse> {
    let db = state.store.as_ref();
    let dispute_status_with_count = db
        .get_dispute_status_with_count(
            platform.get_processor().get_account().get_id(),
            profile_id_list,
            &time_range,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve disputes aggregate")?;

    let mut status_map: HashMap<storage_enums::DisputeStatus, i64> =
        dispute_status_with_count.into_iter().collect();

    for status in storage_enums::DisputeStatus::iter() {
        status_map.entry(status).or_default();
    }

    Ok(services::ApplicationResponse::Json(
        dispute_models::DisputesAggregateResponse {
            status_with_count: status_map,
        },
    ))
}

#[cfg(feature = "v1")]
#[instrument(skip(state))]
pub async fn connector_sync_disputes(
    state: SessionState,
    platform: domain::Platform,
    merchant_connector_id: String,
    payload: disputes::DisputeFetchQueryData,
) -> RouterResponse<FetchDisputesResponse> {
    let connector_id =
        common_utils::id_type::MerchantConnectorAccountId::wrap(merchant_connector_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse merchant connector account id format")?;
    let format = time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse the date-time format")?;
    let created_from = time::PrimitiveDateTime::parse(&payload.fetch_from, &format)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "fetch_from".to_string(),
            expected_format: "YYYY-MM-DDTHH:MM:SS".to_string(),
        })?;
    let created_till = time::PrimitiveDateTime::parse(&payload.fetch_till, &format)
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "fetch_till".to_string(),
            expected_format: "YYYY-MM-DDTHH:MM:SS".to_string(),
        })?;
    let fetch_dispute_request = FetchDisputesRequestData {
        created_from,
        created_till,
    };
    Box::pin(fetch_disputes_from_connector(
        state,
        platform,
        connector_id,
        fetch_dispute_request,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip(state))]
pub async fn fetch_disputes_from_connector(
    state: SessionState,
    platform: domain::Platform,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    req: FetchDisputesRequestData,
) -> RouterResponse<FetchDisputesResponse> {
    let db = &*state.store;
    let merchant_id = platform.get_processor().get_account().get_id();
    let merchant_connector_account = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            &merchant_connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.get_string_repr().to_string(),
        })?;
    let connector_name = merchant_connector_account.connector_name.clone();
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_id.clone()),
    )?;
    let connector_integration: services::BoxedDisputeConnectorIntegrationInterface<
        api::Fetch,
        FetchDisputesRequestData,
        FetchDisputesResponse,
    > = connector_data.connector.get_connector_integration();

    let router_data =
        core_utils::construct_dispute_list_router_data(&state, merchant_connector_account, req)
            .await?;

    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_dispute_failed_response()
    .attach_printable("Failed while calling accept dispute connector api")?;
    let fetch_dispute_response =
        response
            .response
            .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: connector_name.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

    for dispute in &fetch_dispute_response {
        // check if payment already exist
        let payment_attempt = webhooks::incoming::get_payment_attempt_from_object_reference_id(
            &state,
            dispute.object_reference_id.clone(),
            &platform,
        )
        .await;

        if payment_attempt.is_ok() {
            let schedule_time = process_dispute::get_sync_process_schedule_time(
                &*state.store,
                &connector_name,
                merchant_id,
                0,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while getting process schedule time")?;

            let response = add_process_dispute_task_to_pt(
                db,
                &connector_name,
                dispute,
                merchant_id.clone(),
                schedule_time,
            )
            .await;

            match response {
                Err(report)
                    if report
                        .downcast_ref::<errors::StorageError>()
                        .is_some_and(|error| {
                            matches!(error, errors::StorageError::DuplicateValue { .. })
                        }) =>
                {
                    Ok(())
                }
                Ok(_) => Ok(()),
                Err(_) => Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while adding task to process tracker"),
            }?;
        } else {
            router_env::logger::info!("Disputed payment does not exist in our records");
        }
    }

    Ok(services::ApplicationResponse::Json(fetch_dispute_response))
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn update_dispute_data(
    state: &SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    option_dispute: Option<diesel_models::dispute::Dispute>,
    dispute_details: DisputeSyncResponse,
    payment_attempt: domain::PaymentAttempt,
    connector_name: &str,
) -> errors::CustomResult<api_models::disputes::DisputeResponse, errors::ApiErrorResponse> {
    let dispute_data = DisputePayload::from(dispute_details.clone());
    let dispute_object = webhooks::incoming::get_or_update_dispute_object(
        state.clone(),
        option_dispute,
        dispute_data,
        platform.get_processor().get_account().get_id(),
        &platform.get_processor().get_account().organization_id,
        &payment_attempt,
        dispute_details.dispute_status,
        &business_profile,
        connector_name,
    )
    .await?;
    let disputes_response: dispute_models::DisputeResponse = dispute_object.clone().foreign_into();
    let event_type: storage_enums::EventType = dispute_details.dispute_status.into();

    Box::pin(webhooks::create_event_and_trigger_outgoing_webhook(
        state.clone(),
        platform,
        business_profile,
        event_type,
        storage_enums::EventClass::Disputes,
        dispute_object.dispute_id.clone(),
        storage_enums::EventObjectType::DisputeDetails,
        api::OutgoingWebhookContent::DisputeDetails(Box::new(disputes_response.clone())),
        Some(dispute_object.created_at),
    ))
    .await?;
    Ok(disputes_response)
}

#[cfg(feature = "v1")]
pub async fn add_process_dispute_task_to_pt(
    db: &dyn StorageInterface,
    connector_name: &str,
    dispute_payload: &DisputeSyncResponse,
    merchant_id: common_utils::id_type::MerchantId,
    schedule_time: Option<time::PrimitiveDateTime>,
) -> common_utils::errors::CustomResult<(), errors::StorageError> {
    match schedule_time {
        Some(time) => {
            TASKS_ADDED_COUNT.add(
                1,
                router_env::metric_attributes!(("flow", "dispute_process")),
            );
            let tracking_data = disputes::ProcessDisputePTData {
                connector_name: connector_name.to_string(),
                dispute_payload: dispute_payload.clone(),
                merchant_id: merchant_id.clone(),
            };
            let runner = common_enums::ProcessTrackerRunner::ProcessDisputeWorkflow;
            let task = "DISPUTE_PROCESS";
            let tag = ["PROCESS", "DISPUTE"];
            let process_tracker_id = scheduler::utils::get_process_tracker_id(
                runner,
                task,
                &dispute_payload.connector_dispute_id.clone(),
                &merchant_id,
            );
            let process_tracker_entry = diesel_models::ProcessTrackerNew::new(
                process_tracker_id,
                task,
                runner,
                tag,
                tracking_data,
                None,
                time,
                common_types::consts::API_VERSION,
            )
            .map_err(errors::StorageError::from)?;
            db.insert_process(process_tracker_entry).await?;
            Ok(())
        }
        None => Ok(()),
    }
}

#[cfg(feature = "v1")]
pub async fn add_dispute_list_task_to_pt(
    db: &dyn StorageInterface,
    connector_name: &str,
    merchant_id: common_utils::id_type::MerchantId,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    profile_id: common_utils::id_type::ProfileId,
    fetch_request: FetchDisputesRequestData,
) -> common_utils::errors::CustomResult<(), errors::StorageError> {
    TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "dispute_list")));
    let tracking_data = disputes::DisputeListPTData {
        connector_name: connector_name.to_string(),
        merchant_id: merchant_id.clone(),
        merchant_connector_id: merchant_connector_id.clone(),
        created_from: fetch_request.created_from,
        created_till: fetch_request.created_till,
        profile_id,
    };
    let runner = common_enums::ProcessTrackerRunner::DisputeListWorkflow;
    let task = "DISPUTE_LIST";
    let tag = ["LIST", "DISPUTE"];
    let process_tracker_id = scheduler::utils::get_process_tracker_id_for_dispute_list(
        runner,
        &merchant_connector_id,
        fetch_request.created_from,
        &merchant_id,
    );
    let process_tracker_entry = diesel_models::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        None,
        fetch_request.created_from,
        common_types::consts::API_VERSION,
    )
    .map_err(errors::StorageError::from)?;
    db.insert_process(process_tracker_entry).await?;
    Ok(())
}

#[cfg(feature = "v1")]
pub async fn schedule_dispute_sync_task(
    state: &SessionState,
    business_profile: &domain::Profile,
    mca: &domain::MerchantConnectorAccount,
) -> common_utils::errors::CustomResult<(), errors::ApiErrorResponse> {
    let connector = api::enums::Connector::from_str(&mca.connector_name).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        },
    )?;

    if core_utils::should_add_dispute_sync_task_to_pt(state, connector) {
        let offset_date_time = time::OffsetDateTime::now_utc();
        let created_from =
            time::PrimitiveDateTime::new(offset_date_time.date(), offset_date_time.time());
        let dispute_polling_interval = *business_profile
            .dispute_polling_interval
            .unwrap_or_default()
            .deref();

        let created_till = created_from
            .checked_add(time::Duration::hours(i64::from(dispute_polling_interval)))
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        let m_db = state.clone().store;
        let connector_name = mca.connector_name.clone();
        let merchant_id = mca.merchant_id.clone();
        let merchant_connector_id = mca.merchant_connector_id.clone();
        let business_profile_id = business_profile.get_id().clone();

        tokio::spawn(
            async move {
                add_dispute_list_task_to_pt(
                    &*m_db,
                    &connector_name,
                    merchant_id.clone(),
                    merchant_connector_id.clone(),
                    business_profile_id,
                    FetchDisputesRequestData {
                        created_from,
                        created_till,
                    },
                )
                .await
                .map_err(|error| {
                    logger::error!("Failed to add dispute list task to process tracker: {error}")
                })
            }
            .in_current_span(),
        );
    }
    Ok(())
}
