pub mod api;
pub mod transformers;
pub mod types;
use api_models::{enums, process_tracker::revenue_recovery, webhooks};
use common_utils::{
    self,
    errors::CustomResult,
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use diesel_models::{enums as diesel_enum, process_tracker::business_status};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    payments::PaymentIntent, revenue_recovery as domain_revenue_recovery,
    ApiModelToDieselModelConvertor,
};
use scheduler::errors as sch_errors;

use crate::{
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    db::StorageInterface,
    logger,
    routes::{app::ReqState, metrics, SessionState},
    services::ApplicationResponse,
    types::{
        domain,
        storage::{self, revenue_recovery as pcr},
        transformers::{ForeignFrom, ForeignInto},
    },
    workflows,
};

pub const EXECUTE_WORKFLOW: &str = "EXECUTE_WORKFLOW";
pub const PSYNC_WORKFLOW: &str = "PSYNC_WORKFLOW";
pub const CALCULATE_WORKFLOW: &str = "CALCULATE_WORKFLOW";

#[allow(clippy::too_many_arguments)]
pub async fn upsert_calculate_pcr_task(
    billing_connector_account: &domain::MerchantConnectorAccount,
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    recovery_intent_from_payment_intent: &domain_revenue_recovery::RecoveryPaymentIntent,
    business_profile: &domain::Profile,
    intent_retry_count: u16,
    payment_attempt_id: Option<id_type::GlobalAttemptId>,
    runner: storage::ProcessTrackerRunner,
    revenue_recovery_retry: diesel_enum::RevenueRecoveryAlgorithmType,
) -> CustomResult<webhooks::WebhookResponseTracker, errors::RevenueRecoveryError> {
    router_env::logger::info!("Starting calculate_job...");

    let task = "CALCULATE_WORKFLOW";

    let db = &*state.store;
    let payment_id = &recovery_intent_from_payment_intent.payment_id;

    // Create process tracker ID in the format: CALCULATE_WORKFLOW_{payment_intent_id}
    let process_tracker_id = format!("{runner}_{task}_{}", payment_id.get_string_repr());

    // Set scheduled time to 1 hour from now
    let schedule_time = common_utils::date_time::now() + time::Duration::hours(1);

    let payment_attempt_id = payment_attempt_id
        .ok_or(error_stack::report!(
            errors::RevenueRecoveryError::PaymentAttemptIdNotFound
        ))
        .attach_printable("payment attempt id is required for calculate workflow tracking")?;

    // Check if a process tracker entry already exists for this payment intent
    let existing_entry = db
        .as_scheduler()
        .find_process_by_id(&process_tracker_id)
        .await
        .change_context(errors::RevenueRecoveryError::ProcessTrackerResponseError)
        .attach_printable(
            "Failed to check for existing calculate workflow process tracker entry",
        )?;

    match existing_entry {
        Some(existing_process) => {
            router_env::logger::error!(
                "Found existing CALCULATE_WORKFLOW task with  id: {}",
                existing_process.id
            );
        }
        None => {
            // No entry exists - create a new one
            router_env::logger::info!(
                "No existing CALCULATE_WORKFLOW task found for payment_intent_id: {}, creating new entry scheduled for 1 hour from now",
                payment_id.get_string_repr()
            );

            // Create tracking data
            let calculate_workflow_tracking_data = pcr::RevenueRecoveryWorkflowTrackingData {
                billing_mca_id: billing_connector_account.get_id(),
                global_payment_id: payment_id.clone(),
                merchant_id: merchant_context.get_merchant_account().get_id().to_owned(),
                profile_id: business_profile.get_id().to_owned(),
                payment_attempt_id,
                revenue_recovery_retry,
                invoice_scheduled_time: None,
            };

            let tag = ["PCR"];
            let task = "CALCULATE_WORKFLOW";
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;

            let process_tracker_entry = storage::ProcessTrackerNew::new(
                process_tracker_id,
                task,
                runner,
                tag,
                calculate_workflow_tracking_data,
                Some(1),
                schedule_time,
                common_types::consts::API_VERSION,
            )
            .change_context(errors::RevenueRecoveryError::ProcessTrackerCreationError)
            .attach_printable("Failed to construct calculate workflow process tracker entry")?;

            // Insert into process tracker with status New
            db.as_scheduler()
                .insert_process(process_tracker_entry)
                .await
                .change_context(errors::RevenueRecoveryError::ProcessTrackerResponseError)
                .attach_printable(
                    "Failed to enter calculate workflow process_tracker_entry in DB",
                )?;

            router_env::logger::info!(
                "Successfully created new CALCULATE_WORKFLOW task for payment_intent_id: {}",
                payment_id.get_string_repr()
            );

            metrics::TASKS_ADDED_COUNT.add(
                1,
                router_env::metric_attributes!(("flow", "CalculateWorkflow")),
            );
        }
    }

    Ok(webhooks::WebhookResponseTracker::Payment {
        payment_id: payment_id.clone(),
        status: recovery_intent_from_payment_intent.status,
    })
}

pub async fn perform_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    profile: &domain::Profile,
    merchant_context: domain::MerchantContext,
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
            let connector_customer_id = revenue_recovery_metadata.get_connector_customer_id();

            let last_token_used = payment_intent
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.payment_revenue_recovery_metadata.as_ref())
                .map(|rr| {
                    rr.billing_connector_payment_details
                        .payment_processor_token
                        .clone()
                });

            let processor_token = storage::revenue_recovery_redis_operation::RedisTokenManager::get_token_based_on_retry_type(
                state,
                &connector_customer_id,
                tracking_data.revenue_recovery_retry,
                last_token_used.as_deref(),
                )
                .await
                .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "Failed to fetch token details from redis".to_string(),
                })?
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "Failed to fetch token details from redis".to_string(),
            })?;
            logger::info!("Token fetched from redis success");
            let card_info =
                api_models::payments::AdditionalCardInfo::foreign_from(&processor_token);
            // record attempt call
            let record_attempt = api::record_internal_attempt_api(
                state,
                payment_intent,
                revenue_recovery_payment_data,
                &revenue_recovery_metadata,
                card_info,
                &processor_token
                    .payment_processor_token_details
                    .payment_processor_token,
            )
            .await;

            match record_attempt {
                Ok(_) => {
                    let action = Box::pin(types::Action::execute_payment(
                        state,
                        revenue_recovery_payment_data.merchant_account.get_id(),
                        payment_intent,
                        execute_task_process,
                        profile,
                        merchant_context,
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
    profile: &domain::Profile,
    merchant_context: domain::MerchantContext,
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
            profile,
            merchant_context,
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
    profile: &domain::Profile,
    merchant_context: domain::MerchantContext,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let db = &*state.store;
    let merchant_id = revenue_recovery_payment_data.merchant_account.get_id();
    let profile_id = revenue_recovery_payment_data.profile.get_id();
    let billing_mca_id = revenue_recovery_payment_data.billing_mca.get_id();

    logger::info!(
        process_id = %process.id,
        payment_id = %tracking_data.global_payment_id.get_string_repr(),
        "Starting CALCULATE_WORKFLOW..."
    );

    // 1. Extract connector_customer_id and token_list from tracking_data
    let connector_customer_id = payment_intent
        .extract_connector_customer_id_from_payment_intent()
        .change_context(errors::RecoveryError::ValueNotFound)
        .attach_printable("Failed to extract customer ID from payment intent")?;

    let merchant_context_from_revenue_recovery_payment_data =
        domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
            revenue_recovery_payment_data.merchant_account.clone(),
            revenue_recovery_payment_data.key_store.clone(),
        )));

    let retry_algorithm_type = match profile
        .revenue_recovery_retry_algorithm_type
        .filter(|retry_type|
             *retry_type != common_enums::RevenueRecoveryAlgorithmType::Monitoring) // ignore Monitoring in profile
        .unwrap_or(tracking_data.revenue_recovery_retry)                                                                  // fallback to tracking_data
    {
        common_enums::RevenueRecoveryAlgorithmType::Smart => common_enums::RevenueRecoveryAlgorithmType::Smart,
        common_enums::RevenueRecoveryAlgorithmType::Cascading => common_enums::RevenueRecoveryAlgorithmType::Cascading,
        common_enums::RevenueRecoveryAlgorithmType::Monitoring => {
            return Err(sch_errors::ProcessTrackerError::ProcessUpdateFailed);
        }
    };

    // 2. Get best available token
    let best_time_to_schedule = match workflows::revenue_recovery::get_token_with_schedule_time_based_on_retry_algorithm_type(
        state,
        &connector_customer_id,
        payment_intent,
        retry_algorithm_type,
        process.retry_count,
    )
    .await
    {
        Ok(token_opt) => token_opt,
        Err(e) => {
            logger::error!(
                error = ?e,
                connector_customer_id = %connector_customer_id,
                "Failed to get best PSP token"
            );
            None
        }
    };

    match best_time_to_schedule {
        Some(scheduled_time) => {
            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "Found best available token, creating EXECUTE_WORKFLOW task"
            );

            // 3. If token found: create EXECUTE_WORKFLOW task and finish CALCULATE_WORKFLOW
            insert_execute_pcr_task_to_pt(
                &tracking_data.billing_mca_id,
                state,
                &tracking_data.merchant_id,
                payment_intent,
                &tracking_data.profile_id,
                &tracking_data.payment_attempt_id,
                storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                retry_algorithm_type,
                scheduled_time,
            )
            .await?;

            db.as_scheduler()
                .finish_process_with_business_status(
                    process.clone(),
                    business_status::CALCULATE_WORKFLOW_SCHEDULED,
                )
                .await
                .map_err(|e| {
                    logger::error!(
                        process_id = %process.id,
                        error = ?e,
                        "Failed to update CALCULATE_WORKFLOW status to complete"
                    );
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;

            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "CALCULATE_WORKFLOW completed successfully"
            );
        }

        None => {
            let scheduled_token = match storage::revenue_recovery_redis_operation::
                RedisTokenManager::get_payment_processor_token_with_schedule_time(state, &connector_customer_id)
                .await {
                    Ok(scheduled_token_opt) => scheduled_token_opt,
                    Err(e) => {
                        logger::error!(
                            error = ?e,
                            connector_customer_id = %connector_customer_id,
                            "Failed to get PSP token status"
                        );
                        None
                    }
                };

            match scheduled_token {
                Some(scheduled_token) => {
                    // Update scheduled time to scheduled time + 15 minutes
                    // here scheduled_time is the wait time 15 minutes is a buffer time that we are adding
                    logger::info!(
                        process_id = %process.id,
                        connector_customer_id = %connector_customer_id,
                        "No token but time available, rescheduling for scheduled time + 15 mins"
                    );

                    update_calculate_job_schedule_time(
                        db,
                        process,
                        time::Duration::minutes(15),
                        scheduled_token.scheduled_at,
                        &connector_customer_id,
                    )
                    .await?;
                }
                None => {
                    let hard_decline_flag = storage::revenue_recovery_redis_operation::
                        RedisTokenManager::are_all_tokens_hard_declined(
                            state,
                            &connector_customer_id
                        )
                        .await
                        .ok()
                        .unwrap_or(false);

                    match hard_decline_flag {
                        false => {
                            logger::info!(
                                process_id = %process.id,
                                connector_customer_id = %connector_customer_id,
                                "Hard decline flag is false, rescheduling for scheduled time + 15 mins"
                            );

                            update_calculate_job_schedule_time(
                                db,
                                process,
                                time::Duration::minutes(15),
                                Some(common_utils::date_time::now()),
                                &connector_customer_id,
                            )
                            .await?;
                        }
                        true => {
                            // Finish calculate workflow with CALCULATE_WORKFLOW_FINISH
                            logger::info!(
                                process_id = %process.id,
                                connector_customer_id = %connector_customer_id,
                                "No token available, finishing CALCULATE_WORKFLOW"
                            );

                            db.as_scheduler()
                                .finish_process_with_business_status(
                                    process.clone(),
                                    business_status::CALCULATE_WORKFLOW_FINISH,
                                )
                                .await
                                .map_err(|e| {
                                    logger::error!(
                                        process_id = %process.id,
                                        error = ?e,
                                        "Failed to finish CALCULATE_WORKFLOW"
                                    );
                                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                                })?;

                            logger::info!(
                                process_id = %process.id,
                                connector_customer_id = %connector_customer_id,
                                "CALCULATE_WORKFLOW finished successfully"
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Update the schedule time for a CALCULATE_WORKFLOW process tracker
async fn update_calculate_job_schedule_time(
    db: &dyn StorageInterface,
    process: &storage::ProcessTracker,
    additional_time: time::Duration,
    base_time: Option<time::PrimitiveDateTime>,
    connector_customer_id: &str,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let new_schedule_time =
        base_time.unwrap_or_else(common_utils::date_time::now) + additional_time;
    logger::info!(
        new_schedule_time = %new_schedule_time,
        process_id = %process.id,
        connector_customer_id = %connector_customer_id,
        "Rescheduling Calculate Job at "
    );
    let pt_update = storage::ProcessTrackerUpdate::Update {
        name: Some("CALCULATE_WORKFLOW".to_string()),
        retry_count: Some(process.clone().retry_count),
        schedule_time: Some(new_schedule_time),
        tracking_data: Some(process.clone().tracking_data),
        business_status: Some(String::from(business_status::PENDING)),
        status: Some(common_enums::ProcessTrackerStatus::Pending),
        updated_at: Some(common_utils::date_time::now()),
    };

    db.as_scheduler()
        .update_process(process.clone(), pt_update)
        .await
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
        connector_customer_id = %connector_customer_id,
        new_schedule_time = %new_schedule_time,
        additional_time = ?additional_time,
        "CALCULATE_WORKFLOW rescheduled successfully"
    );

    Ok(())
}

/// Insert Execute PCR Task to Process Tracker
#[allow(clippy::too_many_arguments)]
async fn insert_execute_pcr_task_to_pt(
    billing_mca_id: &id_type::MerchantConnectorAccountId,
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    payment_intent: &PaymentIntent,
    profile_id: &id_type::ProfileId,
    payment_attempt_id: &id_type::GlobalAttemptId,
    runner: storage::ProcessTrackerRunner,
    revenue_recovery_retry: diesel_enum::RevenueRecoveryAlgorithmType,
    schedule_time: time::PrimitiveDateTime,
) -> Result<storage::ProcessTracker, sch_errors::ProcessTrackerError> {
    let task = "EXECUTE_WORKFLOW";

    let payment_id = payment_intent.id.clone();

    let process_tracker_id = format!("{runner}_{task}_{}", payment_id.get_string_repr());

    // Check if a process tracker entry already exists for this payment intent
    let existing_entry = state
        .store
        .find_process_by_id(&process_tracker_id)
        .await
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
        Some(existing_process)
            if existing_process.business_status == business_status::EXECUTE_WORKFLOW_FAILURE
                || existing_process.business_status
                    == business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC =>
        {
            // Entry exists with EXECUTE_WORKFLOW_COMPLETE status - update it
            logger::info!(
                payment_id = %payment_id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                current_retry_count = %existing_process.retry_count,
                "Found existing EXECUTE_WORKFLOW task with COMPLETE status, updating to PENDING with incremented retry count"
            );

            let mut tracking_data: pcr::RevenueRecoveryWorkflowTrackingData =
                serde_json::from_value(existing_process.tracking_data.clone())
                    .change_context(errors::RecoveryError::ValueNotFound)
                    .attach_printable(
                        "Failed to deserialize the tracking data from process tracker",
                    )?;

            tracking_data.revenue_recovery_retry = revenue_recovery_retry;

            let tracking_data_json = serde_json::to_value(&tracking_data)
                .change_context(errors::RecoveryError::ValueNotFound)
                .attach_printable("Failed to serialize the tracking data to json")?;

            let pt_update = storage::ProcessTrackerUpdate::Update {
                name: Some(task.to_string()),
                retry_count: Some(existing_process.clone().retry_count + 1),
                schedule_time: Some(schedule_time),
                tracking_data: Some(tracking_data_json),
                business_status: Some(String::from(business_status::PENDING)),
                status: Some(enums::ProcessTrackerStatus::Pending),
                updated_at: Some(common_utils::date_time::now()),
            };

            let updated_process = state
                .store
                .update_process(existing_process, pt_update)
                .await
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
                invoice_scheduled_time: Some(schedule_time),
            };

            let tag = ["PCR"];
            let process_tracker_entry = storage::ProcessTrackerNew::new(
                process_tracker_id.clone(),
                task,
                runner,
                tag,
                execute_workflow_tracking_data,
                Some(1),
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

            let response = state
                .store
                .insert_process(process_tracker_entry)
                .await
                .map_err(|e| {
                    logger::error!(
                        payment_id = %payment_id.get_string_repr(),
                        error = ?e,
                        "Failed to insert execute workflow process tracker entry"
                    );
                    sch_errors::ProcessTrackerError::ProcessUpdateFailed
                })?;

            metrics::TASKS_ADDED_COUNT.add(
                1,
                router_env::metric_attributes!(("flow", "RevenueRecoveryExecute")),
            );

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
