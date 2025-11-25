pub mod api;
pub mod transformers;
pub mod types;
use std::marker::PhantomData;

use api_models::{
    enums,
    payments::{
        self as api_payments, PaymentsGetIntentRequest, PaymentsResponse,
        RecoveryPaymentsListResponseItem,
    },
    process_tracker::revenue_recovery,
    webhooks,
};
use common_enums::enums::{IntentStatus, RecoveryStatus};
use common_utils::{
    self,
    errors::CustomResult,
    ext_traits::{AsyncExt, OptionExt, ValueExt},
    id_type,
    id_type::GlobalPaymentId,
};
use diesel_models::{enums as diesel_enum, process_tracker::business_status};
use error_stack::{self, report, ResultExt};
use hyperswitch_domain_models::{
    payments::{PaymentIntent, PaymentIntentData, PaymentStatusData},
    platform, revenue_recovery as domain_revenue_recovery, ApiModelToDieselModelConvertor,
};
use scheduler::errors as sch_errors;

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::{
            self,
            operations::{GetTrackerResponse, Operation},
            transformers::GenerateResponse,
        },
        revenue_recovery::types::{
            reopen_calculate_workflow_on_payment_failure, RevenueRecoveryOutgoingWebhook,
        },
        revenue_recovery_data_backfill::unlock_connector_customer_status_handler,
    },
    db::StorageInterface,
    logger,
    routes::{app::ReqState, metrics, SessionState},
    services::ApplicationResponse,
    types::{
        api as router_api_types, domain,
        storage::{
            self, revenue_recovery as pcr, PaymentAttempt, ProcessTracker as ProcessTrackerStorage,
        },
        transformers::{ForeignFrom, ForeignInto},
    },
    workflows::revenue_recovery as revenue_recovery_workflow,
};
pub const CALCULATE_WORKFLOW: &str = "CALCULATE_WORKFLOW";
pub const PSYNC_WORKFLOW: &str = "PSYNC_WORKFLOW";
pub const EXECUTE_WORKFLOW: &str = "EXECUTE_WORKFLOW";

use common_enums::enums::ProcessTrackerStatus;

#[allow(clippy::too_many_arguments)]
pub async fn upsert_calculate_pcr_task(
    billing_connector_account: &domain::MerchantConnectorAccount,
    state: &SessionState,
    platform: &domain::Platform,
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

    // Scheduled time is now because this will be the first entry in
    // process tracker and we dont want to wait
    let schedule_time = common_utils::date_time::now();

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
                merchant_id: platform.get_processor().get_account().get_id().to_owned(),
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

#[allow(clippy::too_many_arguments)]
pub async fn record_internal_attempt_and_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    profile: &domain::Profile,
    platform: domain::Platform,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
    payment_processor_token: &storage::revenue_recovery_redis_operation::PaymentProcessorTokenStatus,
    revenue_recovery_metadata: &mut api_models::payments::PaymentRevenueRecoveryMetadata,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let db = &*state.store;

    let card_info = api_models::payments::AdditionalCardInfo::foreign_from(payment_processor_token);

    // record attempt call
    let record_attempt = api::record_internal_attempt_api(
        state,
        payment_intent,
        revenue_recovery_payment_data,
        revenue_recovery_metadata,
        card_info,
        &payment_processor_token
            .payment_processor_token_details
            .payment_processor_token,
    )
    .await;

    match record_attempt {
        Ok(record_attempt_response) => {
            let action = Box::pin(types::Action::execute_payment(
                state,
                revenue_recovery_payment_data.merchant_account.get_id(),
                payment_intent,
                execute_task_process,
                profile,
                platform,
                revenue_recovery_payment_data,
                revenue_recovery_metadata,
                &record_attempt_response.id,
                payment_processor_token,
            ))
            .await?;
            Box::pin(action.execute_payment_task_response_handler(
                state,
                payment_intent,
                execute_task_process,
                revenue_recovery_payment_data,
                revenue_recovery_metadata,
            ))
            .await?;
        }
        Err(err) => {
            logger::error!("Error while recording attempt: {:?}", err);
            let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                status: ProcessTrackerStatus::Pending,
                business_status: Some(String::from(business_status::EXECUTE_WORKFLOW_REQUEUE)),
            };
            db.as_scheduler()
                .update_process(execute_task_process.clone(), pt_update)
                .await?;
        }
    }
    Ok(())
}

pub async fn perform_execute_payment(
    state: &SessionState,
    execute_task_process: &storage::ProcessTracker,
    profile: &domain::Profile,
    platform: domain::Platform,
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
            })?;

            match processor_token {
                None => {
                    logger::info!("No Token fetched from redis");

                    // Close the job if there is no token available
                    db.as_scheduler()
                        .finish_process_with_business_status(
                            execute_task_process.clone(),
                            business_status::EXECUTE_WORKFLOW_FAILURE,
                        )
                        .await?;

                    Box::pin(reopen_calculate_workflow_on_payment_failure(
                        state,
                        execute_task_process,
                        profile,
                        platform,
                        payment_intent,
                        revenue_recovery_payment_data,
                        &tracking_data.payment_attempt_id,
                    ))
                    .await?;

                    storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(state, &connector_customer_id, &payment_intent.id).await?;
                }

                Some(payment_processor_token) => {
                    logger::info!("Token fetched from redis success");

                    record_internal_attempt_and_execute_payment(
                        state,
                        execute_task_process,
                        profile,
                        platform,
                        tracking_data,
                        revenue_recovery_payment_data,
                        payment_intent,
                        &payment_processor_token,
                        &mut revenue_recovery_metadata,
                    )
                    .await?;
                }
            };
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
                        status: ProcessTrackerStatus::Pending,
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
    payment_id: GlobalPaymentId,
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
    platform: domain::Platform,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), errors::ProcessTrackerError> {
    let psync_data = api::call_psync_api(
        state,
        &tracking_data.global_payment_id,
        revenue_recovery_payment_data,
        true,
        true,
    )
    .await?;

    let payment_attempt = psync_data.payment_attempt.clone();
    let mut revenue_recovery_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();
    let pcr_status: types::RevenueRecoveryPaymentsAttemptStatus =
        payment_attempt.status.foreign_into();

    let new_revenue_recovery_payment_data = &pcr::RevenueRecoveryPaymentData {
        psync_data: Some(psync_data),
        ..revenue_recovery_payment_data.clone()
    };

    Box::pin(
        pcr_status.update_pt_status_based_on_attempt_status_for_payments_sync(
            state,
            payment_intent,
            process.clone(),
            profile,
            platform,
            new_revenue_recovery_payment_data,
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
    platform: domain::Platform,
    tracking_data: &pcr::RevenueRecoveryWorkflowTrackingData,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    payment_intent: &PaymentIntent,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let db = &*state.store;
    let merchant_id = revenue_recovery_payment_data.merchant_account.get_id();
    let profile_id = revenue_recovery_payment_data.profile.get_id();
    let billing_mca_id = revenue_recovery_payment_data.billing_mca.get_id();

    let mut event_type: Option<common_enums::EventType> = None;

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

    let platform_from_revenue_recovery_payment_data = domain::Platform::new(
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
    );

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

    // External Payments which enter the calculate workflow for the first time will have active attempt id as None
    // Then we dont need to send an webhook to the merchant as its not a failure from our side.
    // Thus we dont need to a payment get call for such payments.
    let active_payment_attempt_id = payment_intent.active_attempt_id.as_ref();

    let payments_response = get_payment_response_using_payment_get_operation(
        state,
        &tracking_data.global_payment_id,
        revenue_recovery_payment_data,
        &platform_from_revenue_recovery_payment_data,
        active_payment_attempt_id,
    )
    .await?;

    // 2. Get best available token
    let payment_processor_token_response =
        match revenue_recovery_workflow::get_token_with_schedule_time_based_on_retry_algorithm_type(
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
                revenue_recovery_workflow::PaymentProcessorTokenResponse::None
            }
        };

    match payment_processor_token_response {
        revenue_recovery_workflow::PaymentProcessorTokenResponse::ScheduledTime {
            scheduled_time,
        } => {
            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "Found best available token, creating EXECUTE_WORKFLOW task"
            );

            // reset active attmept id and payment connector transmission before going to execute workflow
            let  _ = Box::pin(reset_connector_transmission_and_active_attempt_id_before_pushing_to_execute_workflow(
                state,
                payment_intent,
                revenue_recovery_payment_data,
                active_payment_attempt_id
            )).await?;

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

        revenue_recovery_workflow::PaymentProcessorTokenResponse::NextAvailableTime {
            next_available_time,
        } => {
            // Update scheduled time to next_available_time + Buffer
            // here next_available_time is the wait time
            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "No token but time available, rescheduling for scheduled time "
            );

            update_calculate_job_schedule_time(
                db,
                process,
                time::Duration::seconds(
                    state
                        .conf
                        .revenue_recovery
                        .recovery_timestamp
                        .job_schedule_buffer_time_in_seconds,
                ),
                Some(next_available_time),
                &connector_customer_id,
                retry_algorithm_type,
            )
            .await?;
        }
        revenue_recovery_workflow::PaymentProcessorTokenResponse::None => {
            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "Hard decline flag is false, rescheduling for scheduled time + 15 mins"
            );

            update_calculate_job_schedule_time(
                db,
                process,
                time::Duration::seconds(
                    state
                        .conf
                        .revenue_recovery
                        .recovery_timestamp
                        .job_schedule_buffer_time_in_seconds,
                ),
                Some(common_utils::date_time::now()),
                &connector_customer_id,
                retry_algorithm_type,
            )
            .await?;
        }
        revenue_recovery_workflow::PaymentProcessorTokenResponse::HardDecline => {
            // Finish calculate workflow with CALCULATE_WORKFLOW_FINISH
            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "Token/Tokens is/are Hard decline, finishing CALCULATE_WORKFLOW"
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

            event_type = Some(common_enums::EventType::PaymentFailed);

            logger::info!(
                process_id = %process.id,
                connector_customer_id = %connector_customer_id,
                "CALCULATE_WORKFLOW finished successfully"
            );
        }
    }

    let _outgoing_webhook = event_type.and_then(|event_kind| {
        payments_response.map(|resp| Some((event_kind, resp)))
    })
    .flatten()
    .async_map(|(event_kind, response)| async move {
        let _ = RevenueRecoveryOutgoingWebhook::send_outgoing_webhook_based_on_revenue_recovery_status(
            state,
            common_enums::EventClass::Payments,
            event_kind,
            payment_intent,
            &platform,
            profile,
            tracking_data.payment_attempt_id.get_string_repr().to_string(),
            response
        )
        .await
        .map_err(|e| {
            logger::error!(
                error = ?e,
                "Failed to send outgoing webhook"
            );
            e
        })
        .ok();
    }
    ).await;

    Ok(())
}

/// Update the schedule time for a CALCULATE_WORKFLOW process tracker
async fn update_calculate_job_schedule_time(
    db: &dyn StorageInterface,
    process: &storage::ProcessTracker,
    additional_time: time::Duration,
    base_time: Option<time::PrimitiveDateTime>,
    connector_customer_id: &str,
    retry_algorithm_type: common_enums::RevenueRecoveryAlgorithmType,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let now = common_utils::date_time::now();

    let new_schedule_time = base_time.filter(|&t| t > now).unwrap_or(now) + additional_time;
    logger::info!(
        new_schedule_time = %new_schedule_time,
        process_id = %process.id,
        connector_customer_id = %connector_customer_id,
        "Rescheduling Calculate Job at "
    );
    let mut old_tracking_data: pcr::RevenueRecoveryWorkflowTrackingData =
        serde_json::from_value(process.tracking_data.clone())
            .change_context(errors::RecoveryError::ValueNotFound)
            .attach_printable("Failed to deserialize the tracking data from process tracker")?;

    old_tracking_data.revenue_recovery_retry = retry_algorithm_type;

    let tracking_data = serde_json::to_value(old_tracking_data)
        .change_context(errors::RecoveryError::ValueNotFound)
        .attach_printable("Failed to serialize the tracking data for process tracker")?;

    let pt_update = storage::ProcessTrackerUpdate::Update {
        name: Some("CALCULATE_WORKFLOW".to_string()),
        retry_count: Some(process.clone().retry_count),
        schedule_time: Some(new_schedule_time),
        tracking_data: Some(tracking_data),
        business_status: Some(String::from(business_status::PENDING)),
        status: Some(ProcessTrackerStatus::Pending),
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
                status: Some(ProcessTrackerStatus::Pending),
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
    id: GlobalPaymentId,
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

pub async fn resume_revenue_recovery_process_tracker(
    state: SessionState,
    id: GlobalPaymentId,
    request_retrigger: revenue_recovery::RevenueRecoveryRetriggerRequest,
) -> RouterResponse<revenue_recovery::RevenueRecoveryResponse> {
    let db = &*state.store;
    let task = request_retrigger.revenue_recovery_task;
    let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
    let process_tracker_id = id.get_execute_revenue_recovery_id(&task, runner);

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

    //Call payment intent to check the status
    let request = PaymentsGetIntentRequest { id: id.clone() };
    let revenue_recovery_payment_data =
        revenue_recovery_workflow::extract_data_and_perform_action(&state, &tracking_data)
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: "Failed to extract the revenue recovery data".to_owned(),
            })?;
    let platform_from_revenue_recovery_payment_data = domain::Platform::new(
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
        revenue_recovery_payment_data.merchant_account.clone(),
        revenue_recovery_payment_data.key_store.clone(),
    );
    let create_intent_response = payments::payments_intent_core::<
        router_api_types::PaymentGetIntent,
        router_api_types::payments::PaymentsIntentResponse,
        _,
        _,
        PaymentIntentData<router_api_types::PaymentGetIntent>,
    >(
        state.clone(),
        state.get_req_state(),
        platform_from_revenue_recovery_payment_data,
        revenue_recovery_payment_data.profile.clone(),
        payments::operations::PaymentGetIntent,
        request,
        tracking_data.global_payment_id.clone(),
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    )
    .await?;

    let response = create_intent_response
        .get_json_body()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unexpected response from payments core")?;

    match response.status {
        IntentStatus::Failed => {
            let pt_update = storage::ProcessTrackerUpdate::Update {
                name: process_tracker.name.clone(),
                tracking_data: Some(process_tracker.tracking_data.clone()),
                business_status: Some(request_retrigger.business_status.clone()),
                status: Some(request_retrigger.status),
                updated_at: Some(common_utils::date_time::now()),
                retry_count: Some(process_tracker.retry_count + 1),
                schedule_time: Some(request_retrigger.schedule_time.unwrap_or(
                    common_utils::date_time::now().saturating_add(time::Duration::seconds(600)),
                )),
            };
            let updated_pt = db
                .update_process(process_tracker, pt_update)
                .await
                .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "Failed to update the process tracker".to_owned(),
                })?;
            let response = revenue_recovery::RevenueRecoveryResponse {
                id: updated_pt.id,
                name: updated_pt.name,
                schedule_time_for_payment: updated_pt.schedule_time,
                schedule_time_for_psync: None,
                status: updated_pt.status,
                business_status: updated_pt.business_status,
            };
            Ok(ApplicationResponse::Json(response))
        }
        IntentStatus::Succeeded
        | IntentStatus::Cancelled
        | IntentStatus::CancelledPostCapture
        | IntentStatus::Processing
        | IntentStatus::RequiresCustomerAction
        | IntentStatus::RequiresMerchantAction
        | IntentStatus::RequiresPaymentMethod
        | IntentStatus::RequiresConfirmation
        | IntentStatus::RequiresCapture
        | IntentStatus::PartiallyCaptured
        | IntentStatus::PartiallyCapturedAndCapturable
        | IntentStatus::PartiallyAuthorizedAndRequiresCapture
        | IntentStatus::Conflicted
        | IntentStatus::Expired => Err(report!(errors::ApiErrorResponse::NotSupported {
            message: "Invalid Payment Status ".to_owned(),
        })),
    }
}
pub async fn get_payment_response_using_payment_get_operation(
    state: &SessionState,
    payment_intent_id: &GlobalPaymentId,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    platform: &domain::Platform,
    active_payment_attempt_id: Option<&id_type::GlobalAttemptId>,
) -> Result<Option<ApplicationResponse<PaymentsResponse>>, sch_errors::ProcessTrackerError> {
    match active_payment_attempt_id {
        Some(_) => {
            let payment_response = api::call_psync_api(
                state,
                payment_intent_id,
                revenue_recovery_payment_data,
                false,
                false,
            )
            .await?;
            let payments_response = payment_response.generate_response(
                state,
                None,
                None,
                None,
                platform,
                &revenue_recovery_payment_data.profile,
                None,
            )?;

            Ok(Some(payments_response))
        }
        None => Ok(None),
    }
}

// This function can be implemented to reset the connector transmission and active attempt ID
// before pushing to the execute workflow.
pub async fn reset_connector_transmission_and_active_attempt_id_before_pushing_to_execute_workflow(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    revenue_recovery_payment_data: &pcr::RevenueRecoveryPaymentData,
    active_payment_attempt_id: Option<&id_type::GlobalAttemptId>,
) -> Result<Option<()>, sch_errors::ProcessTrackerError> {
    let mut revenue_recovery_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.clone())
        .get_required_value("Payment Revenue Recovery Metadata")?
        .convert_back();
    match active_payment_attempt_id {
        Some(_) => {
            // update the connector payment transmission field to Unsuccessful and unset active attempt id
            revenue_recovery_metadata.set_payment_transmission_field_for_api_request(
                enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful,
            );

            let payment_update_req =
        api_payments::PaymentsUpdateIntentRequest::update_feature_metadata_and_active_attempt_with_api(
            payment_intent
                .feature_metadata
                .clone()
                .unwrap_or_default()
                .convert_back()
                .set_payment_revenue_recovery_metadata_using_api(
                    revenue_recovery_metadata.clone(),
                ),
            enums::UpdateActiveAttempt::Unset,
        );
            logger::info!(
                "Call made to payments update intent api , with the request body {:?}",
                payment_update_req
            );
            Box::pin(api::update_payment_intent_api(
                state,
                payment_intent.id.clone(),
                revenue_recovery_payment_data,
                payment_update_req,
            ))
            .await
            .change_context(errors::RecoveryError::PaymentCallFailed)?;

            Ok(Some(()))
        }
        None => Ok(None),
    }
}

pub async fn get_workflow_entries(
    state: &SessionState,
    payment_id: &GlobalPaymentId,
) -> RouterResult<(Option<ProcessTrackerStorage>, Option<ProcessTrackerStorage>)> {
    let db = &state.store;
    let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;

    // Get calculate workflow entry
    let calculate_task = CALCULATE_WORKFLOW;
    let calculate_process_tracker_id =
        format!("{runner}_{calculate_task}_{}", payment_id.get_string_repr());

    let calculate_workflow = db
        .as_scheduler()
        .find_process_by_id(&calculate_process_tracker_id)
        .await
        .ok()
        .flatten();

    // Get execute workflow entry
    let execute_task = EXECUTE_WORKFLOW;
    let execute_process_tracker_id =
        payment_id.get_execute_revenue_recovery_id(execute_task, runner);

    let execute_workflow = db
        .as_scheduler()
        .find_process_by_id(&execute_process_tracker_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed to fetch execute workflow entry for payment_id: {}",
            payment_id.get_string_repr()
        ))?;

    Ok((calculate_workflow, execute_workflow))
}

fn determine_recovery_status_from_workflows(
    calculate_business_status: Option<String>,
    calculate_process_tracker_status: Option<String>,
    execute_business_status: Option<String>,
    execute_process_tracker_status: Option<String>,
    default_fallback: impl FnOnce() -> RecoveryStatus,
) -> RecoveryStatus {
    match (
        calculate_business_status,
        calculate_process_tracker_status,
        execute_business_status,
        execute_process_tracker_status,
    ) {
        // Queued status conditions
        (Some(cal_biz_status), Some(cal_pt_status), _, _)
            if (cal_biz_status == business_status::PENDING
                && (cal_pt_status
                    == ProcessTrackerStatus::Processing.to_string().to_uppercase()
                    || cal_pt_status
                        == ProcessTrackerStatus::Pending.to_string().to_uppercase())) =>
        {
            RecoveryStatus::Queued
        }

        // Scheduled status conditions
        (Some(cal_biz_status), Some(cal_pt_status), Some(exe_biz_status), Some(exe_pt_status))
            if (cal_biz_status == business_status::CALCULATE_WORKFLOW_SCHEDULED
                && cal_pt_status == ProcessTrackerStatus::Finish.to_string().to_uppercase())
                || (exe_biz_status == business_status::PENDING
                    && exe_pt_status == ProcessTrackerStatus::New.to_string().to_uppercase())
                || (exe_biz_status == business_status::PENDING
                    && exe_pt_status
                        == ProcessTrackerStatus::Pending.to_string().to_uppercase())
                || (exe_biz_status == business_status::PENDING
                    && exe_pt_status
                        == ProcessTrackerStatus::ProcessStarted
                            .to_string()
                            .to_uppercase()) =>
        {
            RecoveryStatus::Scheduled
        }

        (_, _, Some(exe_biz_status), Some(exe_pt_status))
            if (exe_biz_status == business_status::PENDING
                && exe_pt_status
                    == ProcessTrackerStatus::Processing.to_string().to_uppercase()) =>
        {
            RecoveryStatus::Processing
        }

        // Terminated status conditions
        (Some(cal_biz_status), _, _, _)
            if cal_biz_status == business_status::CALCULATE_WORKFLOW_FINISH
                || cal_biz_status == business_status::RETRIES_EXCEEDED
                || cal_biz_status == business_status::FAILURE
                || cal_biz_status == business_status::GLOBAL_FAILURE =>
        {
            RecoveryStatus::Terminated
        }

        // Default fallback
        _ => default_fallback(),
    }
}

pub fn map_recovery_status(
    intent_status: IntentStatus,
    calculate_workflow: Option<&ProcessTrackerStorage>,
    execute_workflow: Option<&ProcessTrackerStorage>,
    attempt_count: i16,
    max_retry_threshold: i16,
) -> RecoveryStatus {
    let (calculate_business_status, calculate_process_tracker_status) = calculate_workflow
        .map(|calculate| {
            (
                Some(calculate.business_status.clone()),
                Some(calculate.status.to_string().to_uppercase()),
            )
        })
        .unwrap_or((None, None));

    let (execute_business_status, execute_process_tracker_status) = execute_workflow
        .map(|execute| {
            (
                Some(execute.business_status.clone()),
                Some(execute.status.to_string().to_uppercase()),
            )
        })
        .unwrap_or((None, None));

    match intent_status {
        // Only Failed payments are eligible for recovery
        IntentStatus::Failed => determine_recovery_status_from_workflows(
            calculate_business_status,
            calculate_process_tracker_status,
            execute_business_status,
            execute_process_tracker_status,
            || {
                if attempt_count > max_retry_threshold {
                    RecoveryStatus::NoPicked
                } else {
                    RecoveryStatus::Monitoring
                }
            },
        ),

        IntentStatus::PartiallyCaptured | IntentStatus::PartiallyCapturedAndCapturable => {
            determine_recovery_status_from_workflows(
                calculate_business_status,
                calculate_process_tracker_status,
                execute_business_status,
                execute_process_tracker_status,
                || RecoveryStatus::PartiallyRecovered,
            )
        }

        // For all other intent statuses, return the mapped recovery status
        IntentStatus::Succeeded => RecoveryStatus::Recovered,
        IntentStatus::Processing => RecoveryStatus::Processing,
        IntentStatus::Cancelled
        | IntentStatus::CancelledPostCapture
        | IntentStatus::Conflicted
        | IntentStatus::Expired => RecoveryStatus::Terminated,

        // For statuses that don't need recovery
        IntentStatus::RequiresCustomerAction
        | IntentStatus::RequiresMerchantAction
        | IntentStatus::RequiresPaymentMethod
        | IntentStatus::RequiresConfirmation
        | IntentStatus::RequiresCapture
        | IntentStatus::PartiallyAuthorizedAndRequiresCapture => RecoveryStatus::Pending,
    }
}

pub fn map_to_recovery_payment_item(
    payment_intent: PaymentIntent,
    payment_attempt: Option<PaymentAttempt>,
    calculate_workflow: Option<ProcessTrackerStorage>,
    execute_workflow: Option<ProcessTrackerStorage>,
    max_retry_threshold: i16,
) -> RecoveryPaymentsListResponseItem {
    // Map the recovery status
    let recovery_status = map_recovery_status(
        payment_intent.status,
        calculate_workflow.as_ref(),
        execute_workflow.as_ref(),
        payment_intent.attempt_count,
        max_retry_threshold,
    );

    RecoveryPaymentsListResponseItem {
        id: payment_intent.id,
        merchant_id: payment_intent.merchant_id,
        profile_id: payment_intent.profile_id,
        customer_id: payment_intent.customer_id,
        status: recovery_status,
        amount: api_models::payments::PaymentAmountDetailsResponse::foreign_from((
            &payment_intent.amount_details,
            payment_attempt.as_ref().map(|p| &p.amount_details),
        )),
        created: payment_intent.created_at,
        payment_method_type: payment_attempt
            .as_ref()
            .and_then(|p| p.payment_method_type.into()),
        payment_method_subtype: payment_attempt
            .as_ref()
            .and_then(|p| p.payment_method_subtype.into()),
        connector: payment_attempt.as_ref().and_then(|p| p.connector.clone()),
        merchant_connector_id: payment_attempt
            .as_ref()
            .and_then(|p| p.merchant_connector_id.clone()),
        customer: None,
        merchant_reference_id: payment_intent.merchant_reference_id,
        description: payment_intent
            .description
            .map(|val| val.get_string_repr().to_string()),
        attempt_count: payment_intent.attempt_count,
        error: payment_attempt
            .as_ref()
            .and_then(|p| p.error.as_ref())
            .map(api_models::payments::ErrorDetails::foreign_from),
        cancellation_reason: payment_attempt
            .as_ref()
            .and_then(|p| p.cancellation_reason.clone()),
        modified_at: payment_attempt.as_ref().map(|p| p.modified_at),
        last_attempt_at: payment_attempt.as_ref().map(|p| p.created_at),
    }
}
