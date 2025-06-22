pub mod api;
pub mod transformers;
pub mod types;
use api_models::{enums, process_tracker::revenue_recovery};
use common_utils::{
    self,
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use diesel_models::{enums as diesel_enum, process_tracker::business_status};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{payments::PaymentIntent, ApiModelToDieselModelConvertor};
use scheduler::errors as sch_errors;

use crate::{
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    db::StorageInterface,
    logger,
    routes::{metrics, SessionState},
    services::ApplicationResponse,
    types::{
        storage::{self, revenue_recovery as pcr},
        transformers::ForeignInto,
    },
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

    let mut revenue_recovery_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();

    let decision = types::Decision::get_decision_based_on_params(
        state,
        payment_intent.status,
        revenue_recovery_metadata
            .payment_connector_transmission
            .unwrap_or_default(),
        payment_intent.active_attempt_id.clone(),
        revenue_recovery_payment_data,
        &tracking_data.global_payment_id,
    )
    .await?;

    // TODO decide if its a global failure or is it requeueable error
    match decision {
        types::Decision::Execute => {
            // record attempt call
            let record_attempt = api::record_internal_attempt_api(
                state,
                payment_intent,
                revenue_recovery_payment_data,
                &revenue_recovery_metadata,
            )
            .await;

            match record_attempt {
                Ok(_) => {
                    let action = Box::pin(types::Action::execute_payment(
                        state,
                        revenue_recovery_payment_data.merchant_account.get_id(),
                        payment_intent,
                        execute_task_process,
                        revenue_recovery_payment_data,
                        &revenue_recovery_metadata,
                    ))
                    .await?;
                    Box::pin(action.execute_payment_task_response_handler(
                        state,
                        payment_intent,
                        execute_task_process,
                        revenue_recovery_payment_data,
                        &mut revenue_recovery_metadata,
                    ))
                    .await?;
                }
                Err(err) => {
                    logger::error!("Error while recording attempt: {:?}", err);
                    let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                        status: enums::ProcessTrackerStatus::Pending,
                        business_status: Some(String::from(
                            business_status::EXECUTE_WORKFLOW_REQUEUE,
                        )),
                    };
                    db.as_scheduler()
                        .update_process(execute_task_process.clone(), pt_update)
                        .await?;
                }
            }
        }

        types::Decision::Psync(attempt_status, attempt_id) => {
            // find if a psync task is already present
            let task = PSYNC_WORKFLOW;
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
            let process_tracker_id = attempt_id.get_psync_revenue_recovery_id(task, runner);
            let psync_process = db.find_process_by_id(&process_tracker_id).await?;

            match psync_process {
                Some(_) => {
                    let pcr_status: types::RevenueRecoveryPaymentsAttemptStatus =
                        attempt_status.foreign_into();

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
                        revenue_recovery_payment_data.billing_mca.get_id().clone(),
                        db,
                        revenue_recovery_payment_data
                            .merchant_account
                            .get_id()
                            .clone(),
                        payment_intent.get_id().clone(),
                        revenue_recovery_payment_data.profile.get_id().clone(),
                        attempt_id.clone(),
                        storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                        tracking_data.revenue_recovery_retry,
                    )
                    .await?;

                    // finish the current task
                    db.as_scheduler()
                        .finish_process_with_business_status(
                            execute_task_process.clone(),
                            business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                        )
                        .await?;
                }
            };
        }
        types::Decision::ReviewForSuccessfulPayment => {
            // Finish the current task since the payment was a success
            // And mark it as review as it might have happened through the external system
            db.finish_process_with_business_status(
                execute_task_process.clone(),
                business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_REVIEW,
            )
            .await?;
        }
        types::Decision::ReviewForFailedPayment(triggered_by) => {
            match triggered_by {
                enums::TriggeredBy::Internal => {
                    // requeue the current tasks to update the fields for rescheduling a payment
                    let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                        status: enums::ProcessTrackerStatus::Pending,
                        business_status: Some(String::from(
                            business_status::EXECUTE_WORKFLOW_REQUEUE,
                        )),
                    };
                    db.as_scheduler()
                        .update_process(execute_task_process.clone(), pt_update)
                        .await?;
                }
                enums::TriggeredBy::External => {
                    logger::debug!("Failed Payment Attempt Triggered By External");
                    // Finish the current task since the payment was a failure by an external source
                    db.as_scheduler()
                        .finish_process_with_business_status(
                            execute_task_process.clone(),
                            business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_REVIEW,
                        )
                        .await?;
                }
            };
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

#[allow(clippy::too_many_arguments)]
async fn insert_psync_pcr_task_to_pt(
    billing_mca_id: id_type::MerchantConnectorAccountId,
    db: &dyn StorageInterface,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::GlobalPaymentId,
    profile_id: id_type::ProfileId,
    payment_attempt_id: id_type::GlobalAttemptId,
    runner: storage::ProcessTrackerRunner,
    revenue_recovery_retry: diesel_enum::RevenueRecoveryAlgorithmType,
) -> RouterResult<storage::ProcessTracker> {
    let task = PSYNC_WORKFLOW;
    let process_tracker_id = payment_attempt_id.get_psync_revenue_recovery_id(task, runner);
    let schedule_time = common_utils::date_time::now();
    let psync_workflow_tracking_data = pcr::RevenueRecoveryWorkflowTrackingData {
        billing_mca_id,
        global_payment_id: payment_id,
        merchant_id,
        profile_id,
        payment_attempt_id,
        revenue_recovery_retry,
    };
    let tag = ["REVENUE_RECOVERY"];
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
    metrics::TASKS_ADDED_COUNT.add(
        1,
        router_env::metric_attributes!(("flow", "RevenueRecoveryPsync")),
    );

    Ok(response)
}

pub async fn perform_payments_sync(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let psync_data = api::call_psync_api(
        state,
        &tracking_data.global_payment_id,
        revenue_recovery_payment_data,
    )
    .await?;

    let payment_attempt = psync_data.payment_attempt;
    let mut revenue_recovery_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();
    let pcr_status: types::RevenueRecoveryPaymentsAttemptStatus =
        payment_attempt.status.foreign_into();
    Box::pin(
        pcr_status.update_pt_status_based_on_attempt_status_for_payments_sync(
            state,
            payment_intent,
            process.clone(),
            revenue_recovery_payment_data,
            payment_attempt,
            &mut revenue_recovery_metadata,
        ),
    )
    .await?;

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
