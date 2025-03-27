use api_models::{
    enums as api_enums,
    payments::{
        AmountDetails, PaymentRevenueRecoveryMetadata, PaymentsUpdateIntentRequest,
        ProxyPaymentsRequest,
    },
};
use common_utils::{
    self,
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use diesel_models::{enums, process_tracker::business_status};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    payments::{payment_attempt, PaymentConfirmData, PaymentIntent, PaymentIntentData},
    ApiModelToDieselModelConvertor,
};
use time::PrimitiveDateTime;

use crate::{
    core::{
        errors::{self, RouterResult},
        passive_churn_recovery::{self as core_pcr},
        payments::{self, operations::Operation},
    },
    db::StorageInterface,
    logger,
    routes::SessionState,
    types::{
        api::payments as api_types,
        storage::{self, passive_churn_recovery as pcr_storage_types},
        transformers::ForeignInto,
    },
    workflows::passive_churn_recovery_workflow::get_schedule_time_to_retry_mit_payments,
};

type RecoveryResult<T> = error_stack::Result<T, errors::RecoveryError>;

/// The status of Passive Churn Payments
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum PcrAttemptStatus {
    Succeeded,
    Failed,
    Processing,
    InvalidStatus(String),
    //  Cancelled,
}

impl PcrAttemptStatus {
    pub(crate) async fn update_pt_status_based_on_attempt_status_for_execute_payment(
        &self,
        db: &dyn StorageInterface,
        execute_task_process: &storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        match &self {
            Self::Succeeded | Self::Failed | Self::Processing => {
                // finish the current execute task
                db.finish_process_with_business_status(
                    execute_task_process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                )
                .await?;
            }

            Self::InvalidStatus(action) => {
                logger::debug!(
                    "Invalid Attempt Status for the Recovery Payment : {}",
                    action
                );
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(business_status::EXECUTE_WORKFLOW_COMPLETE)),
                };
                // update the process tracker status as Review
                db.update_process(execute_task_process.clone(), pt_update)
                    .await?;
            }
        };
        Ok(())
    }

    pub(crate) async fn update_pt_status_based_on_attempt_status_for_payments_sync(
        &self,
        state: &SessionState,
        process_tracker: storage::ProcessTracker,
        pcr_data: &pcr_storage_types::PcrPaymentData,
        tracking_data: &pcr_storage_types::PcrWorkflowTrackingData,
        payment_attempt: payment_attempt::PaymentAttempt,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;

        match self {
            Self::Succeeded => {
                // finish psync task as the payment was a success
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process_tracker,
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await?;
                // TODO: send back the successful webhook
            }
            Self::Failed => {
                // finish psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process_tracker.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await?;

                // get a reschedule time
                let schedule_time = get_schedule_time_to_retry_mit_payments(
                    db,
                    &pcr_data.merchant_account.get_id().clone(),
                    process_tracker.retry_count + 1,
                )
                .await;

                // check if retry is possible
                if let Some(schedule_time) = schedule_time {
                    // schedule a retry
                    db.as_scheduler()
                        .retry_process(process_tracker.clone(), schedule_time)
                        .await?;
                } else {
                    // TODO: Record a failure back to the billing connector
                }

                // TODO: Update connecter called field and active attempt
            }
            Self::Processing => {
                // do a psync payment
                let action = Box::pin(Action::payment_sync_call(
                    state,
                    pcr_data,
                    &tracking_data.global_payment_id,
                    &process_tracker,
                    payment_attempt,
                ))
                .await?;

                //handle the response
                action
                    .psync_response_handler(db, &process_tracker, tracking_data)
                    .await?;
            }
            Self::InvalidStatus(status) => logger::debug!(
                "Invalid Attempt Status for the Recovery Payment : {}",
                status
            ),
        }
        Ok(())
    }
}
pub enum Decision {
    Execute,
    Psync(enums::AttemptStatus, id_type::GlobalAttemptId),
    InvalidDecision,
    ReviewForSuccessfulPayment,
    ReviewForFailedPayment,
}

impl Decision {
    pub async fn get_decision_based_on_params(
        state: &SessionState,
        intent_status: enums::IntentStatus,
        called_connector: enums::PaymentConnectorTransmission,
        active_attempt_id: Option<id_type::GlobalAttemptId>,
        pcr_data: &storage::passive_churn_recovery::PcrPaymentData,
        payment_id: &id_type::GlobalPaymentId,
    ) -> RecoveryResult<Self> {
        Ok(match (intent_status, called_connector, active_attempt_id) {
            (
                enums::IntentStatus::Failed,
                enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful,
                None,
            ) => Self::Execute,
            (
                enums::IntentStatus::Processing,
                enums::PaymentConnectorTransmission::ConnectorCallSucceeded,
                Some(_),
            ) => {
                let psync_data = core_pcr::call_psync_api(state, payment_id, pcr_data)
                    .await
                    .change_context(errors::RecoveryError::PaymentCallFailed)
                    .attach_printable("Error while executing the Psync call")?;
                let payment_attempt = psync_data
                    .payment_attempt
                    .get_required_value("Payment Attempt")
                    .change_context(errors::RecoveryError::ValueNotFound)
                    .attach_printable("Error while executing the Psync call")?;
                Self::Psync(payment_attempt.status, payment_attempt.get_id().clone())
            }
            (
                enums::IntentStatus::Failed,
                enums::PaymentConnectorTransmission::ConnectorCallSucceeded,
                Some(_),
            ) => Self::ReviewForFailedPayment,
            (
                enums::IntentStatus::Succeeded,
                enums::PaymentConnectorTransmission::ConnectorCallSucceeded,
                Some(_),
            ) => Self::ReviewForSuccessfulPayment,
            _ => Self::InvalidDecision,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SyncPayment(id_type::GlobalAttemptId),
    RetryPayment(PrimitiveDateTime),
    TerminalFailure,
    SuccessfulPayment,
    ReviewPayment,
    ManualReviewAction,
}
impl Action {
    pub async fn execute_payment(
        state: &SessionState,
        merchant_id: &id_type::MerchantId,
        payment_intent: &PaymentIntent,
        process: &storage::ProcessTracker,
        pcr_data: &storage::passive_churn_recovery::PcrPaymentData,
        revenue_recovery_metadata: &PaymentRevenueRecoveryMetadata,
    ) -> RecoveryResult<Self> {
        let db = &*state.store;
        let response =
            call_proxy_api(state, payment_intent, pcr_data, revenue_recovery_metadata).await;
        // handle proxy api's response
        match response {
            Ok(payment_data) => match payment_data.payment_attempt.status.foreign_into() {
                PcrAttemptStatus::Succeeded => Ok(Self::SuccessfulPayment),
                PcrAttemptStatus::Failed => {
                    Self::decide_retry_failure_action(db, merchant_id, process.clone()).await
                }

                PcrAttemptStatus::Processing => {
                    Ok(Self::SyncPayment(payment_data.payment_attempt.id))
                }
                PcrAttemptStatus::InvalidStatus(action) => {
                    logger::info!(?action, "Invalid Payment Status For PCR Payment");
                    Ok(Self::ManualReviewAction)
                }
            },
            Err(err) =>
            // check for an active attempt being constructed or not
            {
                logger::error!(execute_payment_res=?err);
                match payment_intent.active_attempt_id.clone() {
                    Some(attempt_id) => Ok(Self::SyncPayment(attempt_id)),
                    None => Ok(Self::ReviewPayment),
                }
            }
        }
    }

    pub async fn execute_payment_task_response_handler(
        &self,
        state: &SessionState,
        payment_intent: &PaymentIntent,
        execute_task_process: &storage::ProcessTracker,
        pcr_data: &storage::passive_churn_recovery::PcrPaymentData,
        revenue_recovery_metadata: &mut PaymentRevenueRecoveryMetadata,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        match self {
            Self::SyncPayment(attempt_id) => {
                core_pcr::insert_psync_pcr_task(
                    db,
                    pcr_data.merchant_account.get_id().to_owned(),
                    payment_intent.id.clone(),
                    pcr_data.profile.get_id().to_owned(),
                    attempt_id.clone(),
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to create a psync workflow in the process tracker")?;

                db.as_scheduler()
                    .finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::RetryPayment(schedule_time) => {
                db.as_scheduler()
                    .retry_process(execute_task_process.clone(), *schedule_time)
                    .await?;

                // update the connector payment transmission field to Unsuccessful and unset active attempt id
                revenue_recovery_metadata.set_payment_transmission_field_for_api_request(
                    enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful,
                );

                let payment_update_req = PaymentsUpdateIntentRequest::update_feature_metadata_and_active_attempt_with_api(
        payment_intent.feature_metadata.clone().unwrap_or_default().convert_back().set_payment_revenue_recovery_metadata_using_api(
                revenue_recovery_metadata.clone()
            ),
            api_enums::UpdateActiveAttempt::Unset,
        );
                logger::info!(
                    "Call made to payments update intent api , with the request body {:?}",
                    payment_update_req
                );
                update_payment_intent_api(
                    state,
                    payment_intent.id.clone(),
                    pcr_data,
                    payment_update_req,
                )
                .await
                .change_context(errors::RecoveryError::PaymentCallFailed)?;
                Ok(())
            }
            Self::TerminalFailure => {
                // TODO: Record a failure transaction back to Billing Connector
                db.as_scheduler()
                    .finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::SuccessfulPayment => {
                // TODO: Record a successful transaction back to Billing Connector
                db.as_scheduler()
                    .finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::ReviewPayment => {
                let tracking_data_for_review = execute_task_process
                    .tracking_data
                    .clone()
                    .parse_value::<pcr_storage_types::PcrWorkflowTrackingData>(
                    "PCRWorkflowTrackingData",
                )?;
                core_pcr::insert_review_task(
                    db,
                    &tracking_data_for_review,
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                )
                .await?;
                db.finish_process_with_business_status(
                    execute_task_process.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE_FOR_REVIEW,
                )
                .await?;
                Ok(())
            }
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(business_status::EXECUTE_WORKFLOW_COMPLETE)),
                };
                // update the process tracker status as Review
                db.as_scheduler()
                    .update_process(execute_task_process.clone(), pt_update)
                    .await?;
                Ok(())
            }
        }
    }

    pub async fn payment_sync_call(
        state: &SessionState,
        pcr_data: &pcr_storage_types::PcrPaymentData,
        global_payment_id: &id_type::GlobalPaymentId,
        process: &storage::ProcessTracker,
        payment_attempt: payment_attempt::PaymentAttempt,
    ) -> RecoveryResult<Self> {
        let response = core_pcr::call_psync_api(state, global_payment_id, pcr_data).await;
        let db = &*state.store;
        let active_attempt_id = payment_attempt.get_id().clone();
        match response {
            Ok(_payment_data) => match payment_attempt.status.foreign_into() {
                PcrAttemptStatus::Succeeded => Ok(Self::SuccessfulPayment),
                PcrAttemptStatus::Failed => {
                    Self::decide_retry_failure_action(
                        db,
                        &pcr_data.merchant_account.get_id(),
                        process.clone(),
                    )
                    .await
                }

                PcrAttemptStatus::Processing => Ok(Self::SyncPayment(active_attempt_id)),
                PcrAttemptStatus::InvalidStatus(action) => {
                    logger::info!(?action, "Invalid Payment Status For PCR PSync Payment");
                    Ok(Self::ManualReviewAction)
                }
            },
            Err(err) =>
            // if there is an error while psync we create a new Review Task
            {
                logger::error!(sync_payment_response=?err);
                Ok(Self::ReviewPayment)
            }
        }
    }
    pub async fn psync_response_handler(
        &self,
        db: &dyn StorageInterface,
        psync_task_process: &storage::ProcessTracker,
        tracking_data: &pcr_storage_types::PcrWorkflowTrackingData,
    ) -> RecoveryResult<()> {
        match self {
            Self::SyncPayment(_active_attempt_id) => {
                // retry the Psync Taks
                let pt_task_update = diesel_models::ProcessTrackerUpdate::StatusUpdate {
                    status: storage::enums::ProcessTrackerStatus::Pending,
                    business_status: Some(business_status::PENDING.to_owned()),
                };
                // ? get a schedule time for psync and retry the process
                db.as_scheduler()
                    .update_process(psync_task_process.clone(), pt_task_update)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::RetryPayment(schedule_time) => {
                // finish the psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        psync_task_process.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;

                // TODO: Update connecter called field and active attempt

                // retry the execute task
                let task = "EXECUTE_WORKFLOW";
                let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
                let process_tracker_id = format!(
                    "{runner}_{task}_{}",
                    tracking_data.global_payment_id.get_string_repr()
                );
                let execute_task_process = db
                    .as_scheduler()
                    .find_process_by_id(&process_tracker_id)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)?
                    .ok_or(errors::RecoveryError::ProcessTrackerFailure)?;

                db.as_scheduler()
                    .retry_process(execute_task_process, *schedule_time)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::TerminalFailure => {
                // TODO: Record a failure transaction back to Billing Connector
                // finish the current psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        psync_task_process.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::SuccessfulPayment => {
                // TODO: Record a successful transaction back to Billing Connector
                // finish the current psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        psync_task_process.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::ReviewPayment => {
                core_pcr::insert_review_task(
                    db,
                    tracking_data.clone(),
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to insert the task to process tracker")?;

                db.finish_process_with_business_status(
                    psync_task_process.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE_FOR_REVIEW,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(business_status::PSYNC_WORKFLOW_COMPLETE)),
                };
                // update the process tracker status as Review
                db.as_scheduler()
                    .update_process(psync_task_process.clone(), pt_update)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;

                Ok(())
            }
        }
    }

    pub(crate) async fn decide_retry_failure_action(
        db: &dyn StorageInterface,
        merchant_id: &id_type::MerchantId,
        pt: storage::ProcessTracker,
    ) -> RecoveryResult<Self> {
        let schedule_time =
            get_schedule_time_to_retry_mit_payments(db, merchant_id, pt.retry_count + 1).await;
        match schedule_time {
            Some(schedule_time) => Ok(Self::RetryPayment(schedule_time)),

            None => Ok(Self::TerminalFailure),
        }
    }
}

async fn call_proxy_api(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    pcr_data: &storage::passive_churn_recovery::PcrPaymentData,
    revenue_recovery: &PaymentRevenueRecoveryMetadata,
) -> RouterResult<PaymentConfirmData<api_types::Authorize>> {
    let operation = payments::operations::proxy_payments_intent::PaymentProxyIntent;
    let req = ProxyPaymentsRequest {
        return_url: None,
        amount: AmountDetails::new(payment_intent.amount_details.clone().into()),
        recurring_details: revenue_recovery.get_payment_token_for_api_request(),
        shipping: None,
        browser_info: None,
        connector: revenue_recovery.connector.to_string(),
        merchant_connector_id: revenue_recovery.get_merchant_connector_id_for_api_request(),
    };
    logger::info!(
        "Call made to payments proxy api , with the request body {:?}",
        req
    );

    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            payment_intent.get_id(),
            &req,
            &pcr_data.merchant_account,
            &pcr_data.profile,
            &pcr_data.key_store,
            &hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        )
        .await?;

    let (payment_data, _req, _, _) = Box::pin(payments::proxy_for_payments_operation_core::<
        api_types::Authorize,
        _,
        _,
        _,
        PaymentConfirmData<api_types::Authorize>,
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

pub async fn update_payment_intent_api(
    state: &SessionState,
    global_payment_id: id_type::GlobalPaymentId,
    pcr_data: &storage::passive_churn_recovery::PcrPaymentData,
    update_req: PaymentsUpdateIntentRequest,
) -> RouterResult<PaymentIntentData<api_types::PaymentUpdateIntent>> {
    // TODO : Use api handler instead of calling payments_intent_operation_core
    let operation = payments::operations::PaymentUpdateIntent;
    let (payment_data, _req, _) = payments::payments_intent_operation_core::<
        api_types::PaymentUpdateIntent,
        _,
        _,
        PaymentIntentData<api_types::PaymentUpdateIntent>,
    >(
        state,
        state.get_req_state(),
        pcr_data.merchant_account.clone(),
        pcr_data.profile.clone(),
        pcr_data.key_store.clone(),
        operation,
        update_req,
        global_payment_id,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
        None,
    )
    .await?;
    Ok(payment_data)
}
