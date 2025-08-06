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
pub const CALCULATE_WORKFLOW: &str = "CALCULATE_WORKFLOW";

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
                        tracking_data.active_token.ok_or(errors::RecoveryError::ValueNotFound)?,
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
        payment_attempt_id: Some(payment_attempt_id),
        revenue_recovery_retry,
        token_list: Vec::new(), // Empty for psync workflow
        active_token: None,
        invoice_scheduled_time: Some(schedule_time),
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

pub async fn perform_calculate_workflow(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let db = &*state.store;
    let merchant_id = revenue_recovery_payment_data
        .merchant_account
        .get_id();
    let profile_id = revenue_recovery_payment_data
        .profile
        .get_id();
    let billing_mca_id = revenue_recovery_payment_data
        .billing_mca
        .get_id();
    
    logger::info!(
        process_id = %process.id,
        payment_id = %tracking_data.global_payment_id.get_string_repr(),
        "Starting CALCULATE_WORKFLOW..."
    );

    // 1. Extract customer_id and token_list from tracking_data
    let customer_id = extract_customer_id_from_payment_intent(payment_intent)?;
    let token_list = tracking_data.token_list.clone(); // it will be passed in best_token() fn
    
    // TODO:- call redis fn to insert of the tokens

    // 3. Get best available token
    // TODO:- call redis to get best_token() from available tokens and a scheduled time, it will be Optional
    // so from best_token() fn we will be getting Option<{some_token,invoice_scheduled_time_variable}>
    let best_token= Some("some_token".to_string()); //here we will call that fn

    match best_token {
        Some(token) => {
            logger::info!(
                process_id = %process.id,
                customer_id = %customer_id,
                "Found best available token, creating EXECUTE_WORKFLOW task"
            );

            // Mark CALCULATE_WORKFLOW as complete

            let invoice_scheduled_time = common_utils::date_time::now(); // TODO: Replace with actual scheduled time from best_token() function
            
            let updated_tracking_data = pcr::RevenueRecoveryWorkflowTrackingData {
                merchant_id: merchant_id.clone(),
                profile_id: profile_id.clone(),
                global_payment_id: payment_intent.id,
                payment_attempt_id: None,
                billing_mca_id,
                revenue_recovery_retry: common_enums::RevenueRecoveryAlgorithmType::Smart,
                token_list: token_list, //  token list variable
                active_token: best_token.clone(), //  active token variable  
                invoice_scheduled_time: Some(invoice_scheduled_time), //  scheduled time variable for best_token()
            };

            let updated_tracking_data_json = serde_json::to_value(updated_tracking_data)
                .map_err(|e| {
                    logger::error!("Failed to serialize tracking data: {:?}", e);
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;

            let pt_update = storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: Some(process.retry_count),
                schedule_time: None,
                tracking_data: Some(updated_tracking_data_json),
                business_status: Some(String::from(business_status::CALCULATE_WORKFLOW_SCHEDULED)),
                status: Some(common_enums::ProcessTrackerStatus::Finish),
                updated_at: Some(common_utils::date_time::now()),
            };

            db.as_scheduler().update_process(process.clone(), pt_update).await
                .map_err(|e| {
                    logger::error!(
                        process_id = %process.id,
                        error = ?e,
                        "Failed to update CALCULATE_WORKFLOW status to complete"
                    );
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;

            // 4a. If token found: create EXECUTE_WORKFLOW task and finish CALCULATE_WORKFLOW
            insert_execute_pcr_task_to_pt(
                &tracking_data.billing_mca_id,
                db,
                &tracking_data.merchant_id,
                &tracking_data.global_payment_id,
                &tracking_data.profile_id,
                &tracking_data.payment_attempt_id,
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                tracking_data.revenue_recovery_retry,
                common_utils::date_time::now(), // change this to the scheduled returned from the get_best_psp_token_available
                token, // use the token given by the best_token() fn from redis for payment
            ).await?;

            logger::info!(
                process_id = %process.id,
                customer_id = %customer_id,
                "CALCULATE_WORKFLOW completed successfully"
            );
        }
        None => {
            logger::info!(
                process_id = %process.id,
                customer_id = %customer_id,
                "No best token found, rescheduling CALCULATE_WORKFLOW for 1 hour later"
            );

            // 4b. If no token found: reschedule CALCULATE_WORKFLOW for 1 hour later
            let new_schedule_time = common_utils::date_time::now() + time::Duration::hours(1);
           
            let pt_update = storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: None,
                schedule_time: Some(new_schedule_time),
                tracking_data: None,
                business_status: Some(String::from(business_status::CALCULATE_WORKFLOW_QUEUED)),
                status: Some(common_enums::ProcessTrackerStatus::New),
                updated_at: Some(common_utils::date_time::now()),
            };
            db.as_scheduler().update_process(process.clone(), pt_update).await
                .map_err(|e| {
                    logger::error!(
                        process_id = %process.id,
                        error = ?e,
                        "Failed to reschedule CALCULATE_WORKFLOW"
                    );
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;

            logger::info!(
                process_id = %process.id,
                customer_id = %customer_id,
                new_schedule_time = %new_schedule_time,
                "CALCULATE_WORKFLOW rescheduled successfully"
            );
        }
    }
    
    Ok(())
}

// Helper functions for the new CALCULATE_WORKFLOW implementation

/// Extract customer_id from payment intent feature metadata
fn extract_customer_id_from_payment_intent(
    payment_intent: &PaymentIntent,
) -> Result<String, sch_errors::ProcessTrackerError> {
    payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.payment_revenue_recovery_metadata.as_ref())
        .and_then(|recovery| recovery.billing_connector_payment_details.as_ref())
        .map(|details| details.connector_customer_id.get_string_repr())
        .ok_or_else(|| {
            logger::error!("Customer ID not found in payment intent feature metadata");
            sch_errors::ProcessTrackerError::MissingRequiredField
        })
}

/// Insert Execute PCR Task to Process Tracker
#[allow(clippy::too_many_arguments)]
async fn insert_execute_pcr_task_to_pt(
    billing_mca_id: &id_type::MerchantConnectorAccountId,
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    payment_id: &id_type::GlobalPaymentId,
    profile_id: &id_type::ProfileId,
    payment_attempt_id: &Option<id_type::GlobalAttemptId>,
    runner: storage::ProcessTrackerRunner,
    revenue_recovery_retry: diesel_enum::RevenueRecoveryAlgorithmType,
    schedule_time: time::PrimitiveDateTime,
    active_token: String,
) -> Result<storage::ProcessTracker, sch_errors::ProcessTrackerError> {
    let task = "EXECUTE_WORKFLOW";

    let payment_id = payment_id.clone();

    let process_tracker_id = format!("{runner}_{task}_{}", payment_id.get_string_repr());
    
    // Check if a process tracker entry already exists for this payment intent
    let existing_entry = db.find_process_by_id(&process_tracker_id).await
        .map_err(|e| {
            logger::error!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                error = ?e,
                "Failed to check for existing execute workflow process tracker entry"
            );
            sch_errors::ProcessTrackerError::ProcessUpdateFailed
        })?;

    match existing_entry {
        Some(existing_process) if existing_process.business_status == business_status::EXECUTE_WORKFLOW_FINISH => {
            // Entry exists with EXECUTE_WORKFLOW_COMPLETE status - update it
            logger::info!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                current_retry_count = %existing_process.retry_count,
                "Found existing EXECUTE_WORKFLOW task with COMPLETE status, updating to PENDING with incremented retry count"
            );

            let pt_update = storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: Some(existing_process.clone().retry_count + 1),
                schedule_time: Some(schedule_time),
                tracking_data: Some(existing_process.clone().tracking_data),
                business_status: Some(String::from(business_status::PENDING)),
                status: Some(enums::ProcessTrackerStatus::New),
                updated_at: Some(common_utils::date_time::now()),
            };

            let updated_process = db.update_process(existing_process, pt_update).await
                .map_err(|e| {
                    logger::error!(
                        payment_id = %payment_id.get_string_repr(),
                        process_tracker_id = %process_tracker_id,
                        error = ?e,
                        "Failed to update existing execute workflow process tracker entry"
                    );
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;

            logger::info!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                new_retry_count = %updated_process.retry_count,
                "Successfully updated existing EXECUTE_WORKFLOW task"
            );

            Ok(updated_process)
        }
        Some(existing_process) => {
            // Entry exists but business status is not EXECUTE_WORKFLOW_COMPLETE
            logger::info!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                current_business_status = %existing_process.business_status,
                "Found existing EXECUTE_WORKFLOW task but business status is not COMPLETE, returning existing entry"
            );

            Ok(existing_process)
        }
        None => {
            // No entry exists - create a new one
            logger::info!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                "No existing EXECUTE_WORKFLOW task found, creating new entry"
            );

            let execute_workflow_tracking_data = pcr::RevenueRecoveryWorkflowTrackingData {
                billing_mca_id: billing_mca_id.clone(),
                global_payment_id: payment_id.clone(),
                merchant_id: merchant_id.clone(),
                profile_id: profile_id.clone(),
                payment_attempt_id: payment_attempt_id.clone(),
                revenue_recovery_retry,
                token_list: Vec::new(), // Empty for execute workflow
                active_token: Some(active_token),
                invoice_scheduled_time: Some(schedule_time),
            };
            
            let tag = ["PCR"];
            let process_tracker_entry = storage::ProcessTrackerNew::new(
                process_tracker_id.clone(),
                task,
                runner,
                tag,
                execute_workflow_tracking_data,
                None,
                schedule_time,
                common_types::consts::API_VERSION,
            )
            .map_err(|e| {
                logger::error!(
                    payment_id = %payment_id.get_string_repr(),
                    error = ?e,
                    "Failed to construct execute workflow process tracker entry"
                );
                sch_errors::ProcessTrackerError::ProcessUpdateFailed
            })?;

            let response = db.insert_process(process_tracker_entry).await
                .map_err(|e| {
                    logger::error!(
                        payment_id = %payment_id.get_string_repr(),
                        error = ?e,
                        "Failed to insert execute workflow process tracker entry"
                    );
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;
            
            metrics::TASKS_ADDED_COUNT.add(1, router_env::metric_attributes!(("flow", "RevenueRecoveryExecute")));
            
            logger::info!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %response.id,
                "Successfully created new EXECUTE_WORKFLOW task"
            );
            
            Ok(response)
        }
    }
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
        .as_ref()
        .map(|attempt_id| attempt_id.get_psync_revenue_recovery_id(psync_task, runner));

    let process_tracker_for_psync = match process_tracker_id_for_psync {
        Some(ref psync_id) => db
            .find_process_by_id(psync_id)
            .await
            .map_err(|e| {
                logger::error!("Error while retrieving psync task : {:?}", e);
            })
            .ok()
            .flatten(),
        None => None,
    };

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
