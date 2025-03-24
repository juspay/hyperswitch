pub mod transformers;
pub mod types;
use api_models::{payments::PaymentsRetrieveRequest, process_tracker::revenue_recovery};
use common_utils::{
    self,
    ext_traits::{OptionExt, ValueExt},
    id_type,
    types::keymanager::KeyManagerState,
};
use diesel_models::process_tracker::business_status;
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    payments::{PaymentIntent, PaymentStatusData},
    ApiModelToDieselModelConvertor,
};
use scheduler::errors as sch_errors;

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::{self, operations::Operation},
        revenue_recovery::types as pcr_types,
    },
    db::StorageInterface,
    logger,
    routes::{metrics, SessionState},
    types::{
        api,
        storage::{self, revenue_recovery as pcr},
        transformers::ForeignInto,
    },
};

pub const EXECUTE_WORKFLOW: &str = "EXECUTE_WORKFLOW";
pub const PSYNC_WORKFLOW: &str = "PSYNC_WORKFLOW";

pub async fn perform_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    tracking_data: &pcr::PcrWorkflowTrackingData,
    pcr_data: &pcr::PcrPaymentData,
    _key_manager_state: &KeyManagerState,
    payment_intent: &PaymentIntent,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let db = &*state.store;

    let mut pcr_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();

    let decision = pcr_types::Decision::get_decision_based_on_params(
        state,
        payment_intent.status,
        pcr_metadata.payment_connector_transmission,
        payment_intent.active_attempt_id.clone(),
        pcr_data,
        &tracking_data.global_payment_id,
    )
    .await?;

    // TODO decide if its a global failure or is it requeueable error
    match decision {
        pcr_types::Decision::Execute => {
            let action = pcr_types::Action::execute_payment(
                state,
                pcr_data.merchant_account.get_id(),
                payment_intent,
                execute_task_process,
                pcr_data,
                &pcr_metadata,
            )
            .await?;
            Box::pin(action.execute_payment_task_response_handler(
                state,
                payment_intent,
                execute_task_process,
                pcr_data,
                &mut pcr_metadata,
            ))
            .await?;
        }

        pcr_types::Decision::Psync(attempt_status, attempt_id) => {
            // find if a psync task is already present
            let task = PSYNC_WORKFLOW;
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
            let process_tracker_id = format!("{runner}_{task}_{}", attempt_id.get_string_repr());
            let psync_process = db.find_process_by_id(&process_tracker_id).await?;

            match psync_process {
                Some(_) => {
                    let pcr_status: pcr_types::PcrAttemptStatus = attempt_status.foreign_into();

                    pcr_status
                        .update_pt_status_based_on_attempt_status_for_execute_payment(
                            db,
                            execute_task_process,
                        )
                        .await?;
                }

                None => {
                    // insert new psync task
                    insert_psync_pcr_task(
                        db,
                        pcr_data.merchant_account.get_id().clone(),
                        payment_intent.get_id().clone(),
                        pcr_data.profile.get_id().clone(),
                        attempt_id.clone(),
                        storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                    )
                    .await?;

                    // finish the current task
                    db.finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                    )
                    .await?;
                }
            };
        }
        pcr_types::Decision::InvalidDecision => {
            db.finish_process_with_business_status(
                execute_task_process.clone(),
                business_status::EXECUTE_WORKFLOW_COMPLETE,
            )
            .await?;
            logger::warn!("Abnormal State Identified")
        }
    }

    Ok(())
}

async fn insert_psync_pcr_task(
    db: &dyn StorageInterface,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::GlobalPaymentId,
    profile_id: id_type::ProfileId,
    payment_attempt_id: id_type::GlobalAttemptId,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let task = PSYNC_WORKFLOW;
    let process_tracker_id = format!("{runner}_{task}_{}", payment_attempt_id.get_string_repr());
    let schedule_time = common_utils::date_time::now();
    let psync_workflow_tracking_data = pcr::PcrWorkflowTrackingData {
        global_payment_id: payment_id,
        merchant_id,
        profile_id,
        payment_attempt_id,
    };
    let tag = ["PCR"];
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        psync_workflow_tracking_data,
        None,
        schedule_time,
        hyperswitch_domain_models::consts::API_VERSION,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct delete tokenized data process tracker task")?;

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to construct delete tokenized data process tracker task")?;
    metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "PsyncPcr")));

    Ok(response)
}

pub async fn call_psync_api(
    state: &SessionState,
    global_payment_id: &id_type::GlobalPaymentId,
    pcr_data: &pcr::PcrPaymentData,
) -> RouterResult<PaymentStatusData<api::PSync>> {
    let operation = payments::operations::PaymentGet;
    let req = PaymentsRetrieveRequest {
        force_sync: false,
        param: None,
        expand_attempts: true,
    };
    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            global_payment_id,
            &req,
            &pcr_data.merchant_account,
            &pcr_data.profile,
            &pcr_data.key_store,
            &hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        )
        .await?;

    let (payment_data, _req, _, _, _) = Box::pin(payments::payments_operation_core::<
        api::PSync,
        _,
        _,
        _,
        PaymentStatusData<api::PSync>,
    >(
        state,
        state.get_req_state(),
        pcr_data.merchant_account.clone(),
        pcr_data.key_store.clone(),
        &pcr_data.profile,
        operation,
        req,
        get_tracker_response,
        payments::CallConnectorAction::Trigger,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data)
}

pub async fn retrieve_revenue_recovery_process_tracker(
    state: SessionState,
    id: id_type::GlobalPaymentId,
) -> RouterResponse<revenue_recovery::RevenueRecoveryResponse> {
    let db = &*state.store;
    let task = EXECUTE_WORKFLOW;
    let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
    let process_tracker_id = id.get_execute_revenue_recovery_id(task, runner);

    let process_tracker = db
        .find_process_by_id(&process_tracker_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("error retrieving the process tracker id")?
        .get_required_value("Process Tracker")
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Entry For the following id doesn't exists".to_owned(),
        })?;

    let tracking_data = process_tracker
        .tracking_data
        .clone()
        .parse_value::<pcr::PcrWorkflowTrackingData>("PCRWorkflowTrackingData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to deserialize  Pcr Workflow Tracking Data")?;

    let psync_task = PSYNC_WORKFLOW;
    tracking_data
        .payment_attempt_id
        .get_psync_revenue_recovery_id(task, runner);
    let process_tracker_id_for_psync = format!(
        "{runner}_{psync_task}_{}",
        tracking_data.payment_attempt_id.get_string_repr()
    );

    let process_tracker_for_psync = db
        .find_process_by_id(&process_tracker_id_for_psync)
        .await
        .map_err(|e| {
            logger::error!("Error while retreiving psync task : {:?}", e);
        })
        .ok()
        .flatten();

    let schedule_time_for_psync = process_tracker_for_psync.and_then(|pt| pt.schedule_time);

    let response = revenue_recovery::RevenueRecoveryResponse {
        id: process_tracker.id,
        name: process_tracker.name,
        schedule_time_for_payment: process_tracker.schedule_time,
        schedule_time_for_psync,
        status: process_tracker.status,
        business_status: process_tracker.business_status,
    };
    Ok(ApplicationResponse::Json(response))
}
