pub mod transformers;
pub mod types;
use api_models::payments::PaymentsRetrieveRequest;
use common_enums::{self, IntentStatus};
use common_utils::{self, ext_traits::OptionExt, id_type, types::keymanager::KeyManagerState};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response,
    merchant_key_store::MerchantKeyStore,
    payments::{PaymentIntent, PaymentStatusData},
};
use scheduler::errors;

use crate::{
    core::{
        errors::{RouterResult, StorageErrorExt},
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
    workflows::passive_churn_recovery_workflow::get_schedule_time_to_retry_mit_payments,
};

pub async fn perform_execute_task(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    tracking_data: &pcr::PCRWorkflowTrackingData,
    pcr_data: &pcr::PCRPaymentData,
    key_manager_state: &KeyManagerState,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let decision_task = pcr_types::Decision::get_decision_based_on_params(
        state,
        payment_intent.status,
        true,
        payment_intent.active_attempt_id.clone(),
        key_manager_state,
        &pcr_data.key_store,
        &pcr_data.merchant_account,
        &tracking_data.global_payment_id,
    )
    .await?;

    match decision_task {
        pcr_types::Decision::ExecuteTask => {
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
                    key_manager_state,
                    &pcr_data.key_store,
                    execute_task_process,
                    &pcr_data.profile,
                )
                .await?;
        }

        pcr_types::Decision::PsyncTask(payment_attempt) => {
            // find if a psync task is already present
            let task = "PSYNC_WORKFLOW";
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
            let process_tracker_id = format!(
                "{runner}_{task}_{}",
                payment_intent.get_id().get_string_repr()
            );
            let psync_task_process = db.find_process_by_id(&process_tracker_id).await?;

            match psync_task_process {
                Some(psync_process) => {
                    let pcr_status: pcr_types::PCRAttemptStatus =
                        payment_attempt.status.foreign_into();

                    pcr_status
                        .update_pt_status_based_on_attempt_status(
                            db,
                            pcr_data.merchant_account.get_id(),
                            psync_process,
                            execute_task_process,
                            key_manager_state,
                            payment_intent.clone(),
                            &pcr_data.key_store,
                            pcr_data.merchant_account.storage_scheme,
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
                        payment_intent.active_attempt_id.clone(),
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
        pcr_types::Decision::ReviewTaskSuccessfulPayment
        | pcr_types::Decision::ReviewTaskFailedPayment => {
            insert_review_task(
                db,
                tracking_data.clone(),
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
            )
            .await?;
            db.finish_process_with_business_status(
                execute_task_process.clone(),
                business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_REVIEW,
            )
            .await?;
        }
        pcr_types::Decision::InvalidTask => {
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

// This function would be converted to proxy_payments_core
fn call_proxy_api<F>(payment_intent: &PaymentIntent) -> RouterResult<PaymentConfirmData<F>>
where
    F: Send + Clone + Sync,
{
    // TODO: remove the commented code once the proxy api is available
    // let (payment_data, _req, connector_http_status_code, external_latency) =
    // proxy_for_payments_operation_core::<_, _, _, _, _>(
    //     &state,
    //     req_state,
    //     merchant_account.clone(),
    //     key_store,
    //     profile,
    //     operation.clone(),
    //     req,
    //     get_tracker_response,
    //     call_connector_action,
    //     header_payload.clone(),
    // )
    // .await?;

    let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
        payment_intent
            .shipping_address
            .clone()
            .map(|address| address.into_inner()),
        payment_intent
            .billing_address
            .clone()
            .map(|address| address.into_inner()),
        None,
        Some(true),
    );

    let response = PaymentConfirmData {
        flow: std::marker::PhantomData,
        payment_intent: payment_intent.clone(),
        payment_attempt: todo!(),
        payment_method_data: None,
        payment_address,
    };
    Ok(response)
}

async fn insert_psync_pcr_task(
    db: &dyn StorageInterface,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::GlobalPaymentId,
    profile_id: id_type::ProfileId,
    payment_attempt_id: Option<id_type::GlobalAttemptId>,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let task = "PSYNC_WORKFLOW";
    let process_tracker_id = format!("{runner}_{task}_{}", payment_id.get_string_repr());
    let schedule_time = common_utils::date_time::now();
    let psync_workflow_tracking_data = pcr::PCRWorkflowTrackingData {
        global_payment_id: payment_id,
        merchant_id,
        profile_id,
        platform_merchant_id: None,
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
    metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "PsyncPCR")));

    Ok(response)
}

pub async fn perform_psync_task(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::PCRWorkflowTrackingData,
    pcr_data: &pcr::PCRPaymentData,
    key_manager_state: &KeyManagerState,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    match &tracking_data.payment_attempt_id {
        Some(attempt_id) => {
            let psync_data =
                call_psync_api(state, &tracking_data.global_payment_id, pcr_data).await?;
            // If there is an active_attempt id then there will be a payment attempt
            let payment_attempt = psync_data
                .payment_attempt
                .get_required_value("Payment Attempt")?;

            let pcr_status: pcr_types::PCRAttemptStatus = payment_attempt.status.foreign_into();
            pcr_status
                .update_pt_status_based_on_attempt_status_for_psync_task(
                    state,
                    process.clone(),
                    pcr_data,
                    key_manager_state,
                    tracking_data,
                    payment_intent,
                )
                .await?;
        }
        None => {
            insert_review_task(
                db,
                tracking_data.clone(),
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
            )
            .await?;

            // finish the psync task as there is no active attempt
            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE_FOR_REVIEW,
                )
                .await?;
        }
    };
    Ok(())
}

pub async fn call_psync_api(
    state: &SessionState,
    global_payment_id: &id_type::GlobalPaymentId,
    pcr_data: &pcr::PCRPaymentData,
) -> RouterResult<PaymentStatusData<api::PSync>> {
    let operation = payments::operations::PaymentGet;
    let req = PaymentsRetrieveRequest {
        force_sync: false,
        param: None,
        expand_attempts: true,
    };

    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &global_payment_id,
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
        pcr_data.profile.clone(),
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
    tracking_data: pcr::PCRWorkflowTrackingData,
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
        schedule_time,
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
    _tracking_data: &pcr::PCRWorkflowTrackingData,
    pcr_data: &pcr::PCRPaymentData,
    key_manager_state: &KeyManagerState,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let decision_task = pcr_types::Decision::get_decision_based_on_params(
        state,
        payment_intent.status,
        true,
        payment_intent.active_attempt_id.clone(),
        key_manager_state,
        &pcr_data.key_store,
        &pcr_data.merchant_account,
        &payment_intent.get_id(),
    )
    .await?;
    let task = "EXECUTE_WORKFLOW";
    let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
    let process_tracker_id = format!(
        "{runner}_{task}_{}",
        payment_intent.get_id().get_string_repr()
    );
    let execute_task_process = db
        .as_scheduler()
        .find_process_by_id(&process_tracker_id)
        .await?
        .ok_or(errors::ProcessTrackerError::ProcessFetchingFailed)?;
    match decision_task {
        types::Decision::ExecuteTask => {
            // get a reschedule time , without increasing the retry cpunt
            let schedule_time = get_schedule_time_to_retry_mit_payments(
                db,
                pcr_data.merchant_account.get_id(),
                execute_task_process.retry_count,
            )
            .await;

            // check if retry is possible
            if let Some(schedule_time) = schedule_time {
                // schedule a requeue for execute_task
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
        types::Decision::PsyncTask(payment_attempt) => {
            // create a Psync task
            insert_psync_pcr_task(
                db,
                pcr_data.merchant_account.get_id().clone(),
                payment_intent.get_id().clone(),
                pcr_data.profile.get_id().clone(),
                Some(payment_attempt.get_id().clone()),
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

        types::Decision::ReviewTaskFailedPayment => {
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

        types::Decision::ReviewTaskSuccessfulPayment => {
            // finish current review task as the payment was already a success
            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::REVIEW_WORKFLOW_COMPLETE,
                )
                .await?;
        }

        types::Decision::InvalidTask => {
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
