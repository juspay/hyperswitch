pub mod transformers;
pub mod types;
use api_models::payments::PaymentsRetrieveRequest;
use common_utils::{self, ext_traits::OptionExt, id_type};
use diesel_models::process_tracker::business_status;
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response,
    payments::{PaymentIntent, PaymentStatusData},
    ApiModelToDieselModelConvertor,
};
use scheduler::errors;

use crate::{
    core::{
        errors::RouterResult,
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
    workflows::passive_churn_recovery_workflow::get_schedule_time_to_retry_mit_payments,
};

pub async fn perform_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    tracking_data: &pcr::PcrWorkflowTrackingData,
    pcr_data: &pcr::PcrPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;

    let mut pcr_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();

    let decision = types::Decision::get_decision_based_on_params(
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
        types::Decision::Execute => {
            let action = types::Action::execute_payment(
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

        types::Decision::Psync(attempt_status, attempt_id) => {
            // find if a psync task is already present
            let task = "PSYNC_WORKFLOW";
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
            let process_tracker_id = format!("{runner}_{task}_{}", attempt_id.get_string_repr());
            let psync_process = db.find_process_by_id(&process_tracker_id).await?;

            match psync_process {
                Some(_) => {
                    let pcr_status: types::PcrAttemptStatus = attempt_status.foreign_into();

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
        types::Decision::ReviewForSuccessfulPayment | types::Decision::ReviewForFailedPayment => {
            insert_review_task(
                db,
                tracking_data,
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
            )
            .await?;
            db.finish_process_with_business_status(
                execute_task_process.clone(),
                business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_REVIEW,
            )
            .await?;
        }
        types::Decision::InvalidDecision => {
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
        None,
        schedule_time,
        hyperswitch_domain_models::consts::API_VERSION,
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

pub async fn perform_payments_sync(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::PcrWorkflowTrackingData,
    pcr_data: &pcr::PcrPaymentData,
) -> Result<(), errors::ProcessTrackerError> {
    let psync_data = call_psync_api(state, &tracking_data.global_payment_id, pcr_data).await?;
    // If there is an active_attempt id then there will be a payment attempt
    let payment_attempt = psync_data
        .payment_attempt
        .get_required_value("Payment Attempt")?;

    let pcr_status: types::PcrAttemptStatus = payment_attempt.status.foreign_into();
    pcr_status
        .update_pt_status_based_on_attempt_status_for_payments_sync(
            state,
            process.clone(),
            pcr_data,
            tracking_data,
            payment_attempt,
        )
        .await?;

    Ok(())
}

pub async fn call_psync_api(
    state: &SessionState,
    global_payment_id: &id_type::GlobalPaymentId,
    revenue_recovery_data: &pcr::PcrPaymentData,
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
            &revenue_recovery_data.merchant_account,
            &revenue_recovery_data.profile,
            &revenue_recovery_data.key_store,
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
        revenue_recovery_data.merchant_account.clone(),
        revenue_recovery_data.key_store.clone(),
        &revenue_recovery_data.profile,
        operation,
        req,
        get_tracker_response,
        payments::CallConnectorAction::Trigger,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data)
}

async fn insert_review_task(
    db: &dyn StorageInterface,
    tracking_data: &pcr::PcrWorkflowTrackingData,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let task = "REVIEW_WORKFLOW";
    let process_tracker_id = format!(
        "{runner}_{task}_{}",
        tracking_data.global_payment_id.get_string_repr()
    );
    let schedule_time = common_utils::date_time::now();
    let tag = ["PCR"];
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        None,
        schedule_time,
        hyperswitch_domain_models::consts::API_VERSION,
    )
    .change_context(api_error_response::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct delete tokenized data process tracker task")?;

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .change_context(api_error_response::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to construct delete tokenized data process tracker task")?;
    metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "REVIEW_TASK")));

    Ok(response)
}

pub async fn perform_review_task(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::PcrWorkflowTrackingData,
    pcr_data: &pcr::PcrPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let mut pcr_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();

    let decision = types::Decision::get_decision_based_on_params(
        state,
        payment_intent.status,
        pcr_metadata.payment_connector_transmission,
        payment_intent.active_attempt_id.clone(),
        pcr_data,
        &tracking_data.global_payment_id,
    )
    .await?;

    let task = "EXECUTE_WORKFLOW";
    let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
    let process_tracker_id = format!(
        "{runner}_{task}_{}",
        payment_intent.get_id().get_string_repr()
    );
    let mut execute_task_process = db
        .as_scheduler()
        .find_process_by_id(&process_tracker_id)
        .await?
        .ok_or(errors::ProcessTrackerError::ProcessFetchingFailed)?;

    match decision {
        types::Decision::Execute => {
            // get a reschedule time , without increasing the retry count
            let schedule_time = get_schedule_time_to_retry_mit_payments(
                db,
                pcr_data.merchant_account.get_id(),
                execute_task_process.retry_count,
            )
            .await;

            // check if retry is possible
            if let Some(schedule_time) = schedule_time {
                // schedule a requeue for execute_task with a new schedule time
                execute_task_process.schedule_time = Some(schedule_time);

                let pt_task_update = diesel_models::ProcessTrackerUpdate::StatusUpdate {
                    status: storage::enums::ProcessTrackerStatus::Pending,
                    business_status: Some(business_status::PENDING.to_owned()),
                };
                db.as_scheduler()
                    .update_process(execute_task_process.clone(), pt_task_update)
                    .await?;
            } else {
                // TODO: send back the failure webhook
            }

            // finish current review task as the payment was a success
            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::REVIEW_WORKFLOW_COMPLETE,
                )
                .await?;
        }

        types::Decision::Psync(_attempt_status, attempt_id) => {
            // create a Psync task
            insert_psync_pcr_task(
                db,
                pcr_data.merchant_account.get_id().clone(),
                payment_intent.get_id().clone(),
                pcr_data.profile.get_id().clone(),
                attempt_id.clone(),
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
            )
            .await?;
            // finish current review task as the payment was a success
            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::REVIEW_WORKFLOW_COMPLETE,
                )
                .await?;
        }

        types::Decision::ReviewForFailedPayment => {
            // get a reschedule time for the next retry
            let schedule_time = get_schedule_time_to_retry_mit_payments(
                db,
                pcr_data.merchant_account.get_id(),
                process.retry_count + 1,
            )
            .await;

            // check if retry is possible
            if let Some(schedule_time) = schedule_time {
                // schedule a retry for execute_task
                db.as_scheduler()
                    .retry_process(execute_task_process.clone(), schedule_time)
                    .await?;
            } else {
                // TODO: send back the failure webhook
            }
            // a retry has been scheduled
            // TODO: set the connector called as false and active attempt id field None
        }

        types::Decision::ReviewForSuccessfulPayment => {
            // finish current review task as the payment was already a success
            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::REVIEW_WORKFLOW_COMPLETE,
                )
                .await?;
        }

        types::Decision::InvalidDecision => {
            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::REVIEW_WORKFLOW_COMPLETE,
                )
                .await?;
            logger::warn!("Abnormal State Identified")
        }
    }
    Ok(())
}
