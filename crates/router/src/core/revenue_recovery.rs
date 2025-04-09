pub mod api;
pub mod transformers;
pub mod types;
use api_models::{payments::PaymentsRetrieveRequest, process_tracker::revenue_recovery};
use common_utils::{
    self,
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use diesel_models::process_tracker::business_status;
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response,
    merchant_connector_account,
    payments::{PaymentIntent, PaymentStatusData},
    ApiModelToDieselModelConvertor,
};
use scheduler::errors as sch_errors;

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::{self, operations::Operation},
    },
    db::StorageInterface,
    logger,
    routes::{metrics, SessionState},
    services::ApplicationResponse,
    types::{
        storage::{self, revenue_recovery as pcr},
        transformers::ForeignInto,
    },
    workflows::revenue_recovery::get_schedule_time_to_retry_mit_payments,
};

pub const EXECUTE_WORKFLOW: &str = "EXECUTE_WORKFLOW";
pub const PSYNC_WORKFLOW: &str = "PSYNC_WORKFLOW";

pub async fn perform_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), sch_errors::ProcessTrackerError> {
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
        revenue_recovery_payment_data,
        &tracking_data.global_payment_id,
    )
    .await?;

    // TODO decide if its a global failure or is it requeueable error
    match decision {
        types::Decision::Execute => {
            let action = types::Action::execute_payment(
                state,
                revenue_recovery_payment_data.merchant_account.get_id(),
                payment_intent,
                execute_task_process,
                revenue_recovery_payment_data,
                &pcr_metadata,
            )
            .await?;
            Box::pin(action.execute_payment_task_response_handler(
                state,
                payment_intent,
                execute_task_process,
                revenue_recovery_payment_data,
                &mut pcr_metadata,
            ))
            .await?;
        }

        types::Decision::Psync(attempt_status, attempt_id) => {
            // find if a psync task is already present
            let task = PSYNC_WORKFLOW;
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
            let process_tracker_id = attempt_id.get_psync_revenue_recovery_id(task, runner);
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
                    insert_psync_pcr_task_to_pt(
                        db,
                        revenue_recovery_payment_data
                            .merchant_account
                            .get_id()
                            .clone(),
                        payment_intent.get_id().clone(),
                        revenue_recovery_payment_data.profile.get_id().clone(),
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
            insert_review_task_to_pt(
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

async fn insert_psync_pcr_task_to_pt(
    db: &dyn StorageInterface,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::GlobalPaymentId,
    profile_id: id_type::ProfileId,
    payment_attempt_id: id_type::GlobalAttemptId,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let task = PSYNC_WORKFLOW;
    let process_tracker_id = payment_attempt_id.get_psync_revenue_recovery_id(task, runner);
    let schedule_time = common_utils::date_time::now();
    let psync_workflow_tracking_data = pcr::RevenueRecoveryWorkflowTrackingData {
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
        common_types::consts::API_VERSION,
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

pub async fn perform_payments_sync(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    pcr_data: &pcr::RevenueRecoveryPaymentData,
) -> Result<(), errors::ProcessTrackerError> {
    let psync_data = api::call_psync_api(state, &tracking_data.global_payment_id, pcr_data).await?;
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

async fn insert_review_task_to_pt(
    db: &dyn StorageInterface,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
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
        common_types::consts::API_VERSION,
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
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    pcr_data: &pcr::RevenueRecoveryPaymentData,
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
    let process_tracker_id = payment_intent
        .get_id()
        .get_execute_revenue_recovery_id(task, runner);
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
        // if active attempt id is there do a psync check for the already present psync task for that attempt
        types::Decision::Psync(_attempt_status, attempt_id) => {
            // create a Psync task
            insert_psync_pcr_task_to_pt(
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
        .parse_value::<pcr::RevenueRecoveryWorkflowTrackingData>("PCRWorkflowTrackingData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to deserialize  Pcr Workflow Tracking Data")?;

    let psync_task = PSYNC_WORKFLOW;

    let process_tracker_id_for_psync = tracking_data
        .payment_attempt_id
        .get_psync_revenue_recovery_id(psync_task, runner);

    let process_tracker_for_psync = db
        .find_process_by_id(&process_tracker_id_for_psync)
        .await
        .map_err(|e| {
            logger::error!("Error while retrieving psync task : {:?}", e);
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
