pub mod transformers;
pub mod types;
use api_models::payments::PaymentsRetrieveRequest;
use common_utils::{self, id_type, types::keymanager::KeyManagerState};
use diesel_models::process_tracker::business_status;
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response,
    payments::{PaymentIntent, PaymentStatusData},
};
use scheduler::errors;

use crate::{
    core::{
        errors::RouterResult,
        passive_churn_recovery::types as pcr_types,
        payments::{self, operations::Operation},
    },
    db::StorageInterface,
    logger,
    routes::{metrics, SessionState},
    types::{
        api,
        storage::{self, passive_churn_recovery as pcr},
        transformers::ForeignInto,
    },
};

pub async fn perform_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    tracking_data: &pcr::PcrWorkflowTrackingData,
    pcr_data: &pcr::PcrPaymentData,
    _key_manager_state: &KeyManagerState,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let decision = pcr_types::Decision::get_decision_based_on_params(
        state,
        payment_intent.status,
        false,
        payment_intent.active_attempt_id.clone(),
        pcr_data,
        &tracking_data.global_payment_id,
    )
    .await?;
    // TODO decide if its a global failure or is it requeueable error
    match decision {
        pcr_types::Decision::Execute => {
            let action = pcr_types::Action::execute_payment(
                db,
                pcr_data.merchant_account.get_id(),
                payment_intent,
                execute_task_process,
            )
            .await?;
            action
                .execute_payment_task_response_handler(
                    db,
                    &pcr_data.merchant_account,
                    payment_intent,
                    execute_task_process,
                    &pcr_data.profile,
                )
                .await?;
        }

        pcr_types::Decision::Psync(attempt_status, attempt_id) => {
            // find if a psync task is already present
            let task = "PSYNC_WORKFLOW";
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
    let task = "PSYNC_WORKFLOW";
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
        schedule_time,
    )
    .change_context(api_error_response::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct delete tokenized data process tracker task")?;

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .change_context(api_error_response::ApiErrorResponse::InternalServerError)
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
