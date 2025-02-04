use common_enums::{self, IntentStatus};
use common_utils::{self, ext_traits::OptionExt, id_type, types::keymanager::KeyManagerState};
use diesel_models::process_tracker::business_status;
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    business_profile,
    errors::api_error_response,
    merchant_account,
    merchant_key_store::MerchantKeyStore,
    payments::{payment_attempt::PaymentAttempt, PaymentConfirmData, PaymentIntent},
};

use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        passive_churn_recovery::{self as core_pcr},
    },
    db::StorageInterface,
    logger,
    routes::SessionState,
    types::{
        api::payments as api_types,
        storage::{self, passive_churn_recovery as pcr_storage_types},
        transformers::ForeignInto,
    },
    workflows::passive_churn_recovery_workflow::{
        get_schedule_time_to_retry_mit_payments, retry_pcr_payment_task,
    },
};

type RecoveryResult<T> = error_stack::Result<T, errors::RecoveryError>;

/// The status of Passive Churn Payments
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum PCRAttemptStatus {
    Succeeded,
    Failed,
    Processing,
    InvalidAction(String),
    //  Cancelled,
}

impl PCRAttemptStatus {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn perform_action_based_on_status(
        &self,
        db: &dyn StorageInterface,
        merchant_id: &id_type::MerchantId,
        process_tracker: storage::ProcessTracker,
        process: &storage::ProcessTracker,
        key_manager_state: &KeyManagerState,
        payment_intent: PaymentIntent,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> Result<(), errors::ProcessTrackerError> {
        match &self {
            Self::Succeeded => {
                // finish psync task as the payment was a success
                db.finish_process_with_business_status(
                    process_tracker,
                    business_status::PSYNC_WORKFLOW_COMPLETE,
                )
                .await?;
                // TODO: send back the successful webhook

                // finish the current execute task as the payment has been completed
                db.finish_process_with_business_status(
                    process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE,
                )
                .await?;
            }

            Self::Failed => {
                // finish psync task
                db.finish_process_with_business_status(
                    process_tracker.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE,
                )
                .await?;

                // get a reschedule time
                let schedule_time = get_schedule_time_to_retry_mit_payments(
                    db,
                    merchant_id,
                    process_tracker.retry_count + 1,
                )
                .await;

                // check if retry is possible
                if let Some(schedule_time) = schedule_time {
                    // schedule a retry
                    db.retry_process(process.clone(), schedule_time).await?;
                } else {
                    let _ = core_pcr::terminal_payment_failure_handling(
                        db,
                        key_manager_state,
                        payment_intent.clone(),
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await
                    .map_err(|error| logger::error!(?error, "Failed to update the payment intent"));
                }
            }

            Self::Processing => {
                // finish the current execute task
                db.finish_process_with_business_status(
                    process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                )
                .await?;
            }

            Self::InvalidAction(action) => {
                logger::debug!(
                    "Invalid Attempt Status for the Recovery Payment : {}",
                    action
                )
            }
        };
        Ok(())
    }
    pub(crate) async fn perform_action_based_on_status_for_psync_task(
        &self,
        state: &SessionState,
        process_tracker: storage::ProcessTracker,
        pcr_data: &pcr_storage_types::PCRPaymentData,
        key_manager_state: &KeyManagerState,
        tracking_data: &pcr_storage_types::PCRWorkflowTrackingData,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;

        match self {
            Self::Succeeded => {
                // finish psync task as the payment was a success
                db.finish_process_with_business_status(
                    process_tracker,
                    business_status::PSYNC_WORKFLOW_COMPLETE,
                )
                .await?;
                // TODO: send back the successful webhook
            }
            Self::Failed => {
                // finish psync task
                db.finish_process_with_business_status(
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
                    db.retry_process(process_tracker.clone(), schedule_time)
                        .await?;
                } else {
                    let payment_intent = db
                        .find_payment_intent_by_id(
                            key_manager_state,
                            &tracking_data.global_payment_id,
                            &pcr_data.key_store,
                            pcr_data.merchant_account.storage_scheme,
                        )
                        .await
                        .to_not_found_response(
                            api_error_response::ApiErrorResponse::PaymentNotFound,
                        )?;
                    let _ = core_pcr::terminal_payment_failure_handling(
                        db,
                        key_manager_state,
                        payment_intent,
                        &pcr_data.key_store,
                        pcr_data.merchant_account.storage_scheme,
                    )
                    .await
                    .map_err(|error| logger::error!(?error, "Failed to update the payment intent"));
                }

                // TODO: Update connecter called field and active attempt
            }
            Self::Processing => {
                let payment_intent = db
                    .find_payment_intent_by_id(
                        key_manager_state,
                        &tracking_data.global_payment_id,
                        &pcr_data.key_store,
                        pcr_data.merchant_account.storage_scheme,
                    )
                    .await
                    .to_not_found_response(api_error_response::ApiErrorResponse::PaymentNotFound)?;
                // do a psync payment
                let action = Box::pin(Action::execute_payment_for_psync(
                    state,
                    pcr_data,
                    tracking_data,
                    &process_tracker,
                ))
                .await?;

                //handle the resp
                action
                    .psync_payment_response_handler(
                        db,
                        &process_tracker,
                        tracking_data,
                        pcr_data,
                        key_manager_state,
                        &payment_intent,
                    )
                    .await?;
            }
            Self::InvalidAction(action) => logger::debug!(
                "Invalid Attempt Status for the Recovery Payment : {}",
                action
            ),
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub enum Decision {
    ExecuteTask,
    PsyncTask(PaymentAttempt),
    ReviewTask,
}

impl Decision {
    pub async fn get_decision_based_on_params(
        db: &dyn StorageInterface,
        intent_status: IntentStatus,
        called_connector: bool,
        active_attempt_id: Option<id_type::GlobalAttemptId>,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_account: &merchant_account::MerchantAccount,
    ) -> RecoveryResult<Self> {
        Ok(match (intent_status, called_connector, active_attempt_id) {
            (IntentStatus::Processing, false, None) => Self::ExecuteTask,
            (IntentStatus::Processing, true, Some(active_attempt_id)) => {
                let payment_attempt = db
                    .find_payment_attempt_by_id(
                        key_manager_state,
                        merchant_key_store,
                        &active_attempt_id,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::RecoveryError::TaskNotFound)?;
                Self::PsyncTask(payment_attempt)
            }
            _ => Self::ReviewTask,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SyncPayment,
    RetryPayment(storage::ProcessTracker),
    TerminalFailure,
    SuccessfulPayment,
    ReviewPayment,
    ManualReviewAction,
}
impl Action {
    pub async fn execute_payment(
        db: &dyn StorageInterface,
        merchant_id: &id_type::MerchantId,
        payment_intent: &PaymentIntent,
        process: &storage::ProcessTracker,
    ) -> RecoveryResult<Self> {
        // call the proxy api
        let response = call_proxy_api::<api_types::Authorize>(payment_intent);
        // handle proxy api's response
        match response {
            Ok(payment_data) => match payment_data.payment_attempt.status.foreign_into() {
                PCRAttemptStatus::Succeeded => Ok(Self::SuccessfulPayment),
                PCRAttemptStatus::Failed => {
                    Ok(retry_pcr_payment_task(db, merchant_id.clone(), process.clone()).await)
                }

                PCRAttemptStatus::Processing => Ok(Self::SyncPayment),
                PCRAttemptStatus::InvalidAction(action) => {
                    logger::info!(?action, "Invalid Payment Status For PCR Payment");
                    Ok(Self::ManualReviewAction)
                }
            },
            Err(_) =>
            // check for an active attempt being constructed or not
            {
                match payment_intent.active_attempt_id.clone() {
                    Some(_) => Ok(Self::SyncPayment),
                    None => Ok(Self::ReviewPayment),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute_payment_response_handler(
        &self,
        db: &dyn StorageInterface,
        merchant_account: &merchant_account::MerchantAccount,
        payment_intent: &PaymentIntent,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        process: &storage::ProcessTracker,
        profile: &business_profile::Profile,
    ) -> RecoveryResult<()> {
        match self {
            Self::SyncPayment => {
                core_pcr::insert_psync_pcr_workflow(
                    db,
                    merchant_account.get_id().to_owned(),
                    payment_intent.id.clone(),
                    profile.get_id().to_owned(),
                    payment_intent.active_attempt_id.clone(),
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to create a psync workflow in the process tracker")?;

                db.finish_process_with_business_status(
                    process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::RetryPayment(process_tracker) => {
                let mut pt = process_tracker.clone();
                pt.schedule_time = get_schedule_time_to_retry_mit_payments(
                    db,
                    merchant_account.get_id(),
                    pt.retry_count + 1,
                )
                .await;
                let pt_task_update = diesel_models::ProcessTrackerUpdate::StatusUpdate {
                    status: storage::enums::ProcessTrackerStatus::Pending,
                    business_status: Some(business_status::PENDING.to_owned()),
                };
                db.update_process(pt.clone(), pt_task_update)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                // TODO: update the connector called field and make the active attempt None

                Ok(())
            }
            Self::TerminalFailure => {
                core_pcr::terminal_payment_failure_handling(
                    db,
                    key_manager_state,
                    payment_intent.clone(),
                    merchant_key_store,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::RecoveryError::RecoveryFailed)
                .attach_printable("Failed to update the payment intent with terminal status")?;
                Ok(())
            }
            Self::SuccessfulPayment => Ok(()),
            Self::ReviewPayment => Ok(()),
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                Ok(())
            }
        }
    }

    pub async fn execute_payment_for_psync(
        state: &SessionState,
        pcr_data: &pcr_storage_types::PCRPaymentData,
        tracking_data: &pcr_storage_types::PCRWorkflowTrackingData,
        process: &storage::ProcessTracker,
    ) -> RecoveryResult<Self> {
        let response = core_pcr::perform_psync_call(state, tracking_data, pcr_data).await;
        let db = &*state.store;
        let active_attempt_id = tracking_data.payment_attempt_id.clone();
        match response {
            Ok(payment_data) => {
                if let Some(payment_attempt) = payment_data.payment_attempt {
                    match payment_attempt.status.foreign_into() {
                        PCRAttemptStatus::Succeeded => Ok(Self::SuccessfulPayment),
                        PCRAttemptStatus::Failed => Ok(retry_pcr_payment_task(
                            db,
                            tracking_data.merchant_id.clone(),
                            process.clone(),
                        )
                        .await),

                        PCRAttemptStatus::Processing => Ok(Self::SyncPayment),
                        PCRAttemptStatus::InvalidAction(action) => {
                            logger::info!(?action, "Invalid Payment Status For PCR PSync Payment");
                            Ok(Self::ManualReviewAction)
                        }
                    }
                } else {
                    Ok(Self::ReviewPayment)
                }
            }
            Err(_) =>
            // check for an active attempt being constructed or not
            {
                match active_attempt_id.clone() {
                    Some(_) => Ok(Self::SyncPayment),
                    None => Ok(Self::ReviewPayment),
                }
            }
        }
    }
    pub async fn psync_payment_response_handler(
        &self,
        db: &dyn StorageInterface,
        process: &storage::ProcessTracker,
        tracking_data: &pcr_storage_types::PCRWorkflowTrackingData,
        pcr_data: &pcr_storage_types::PCRPaymentData,
        key_manager_state: &KeyManagerState,
        payment_intent: &PaymentIntent,
    ) -> RecoveryResult<()> {
        match self {
            Self::SyncPayment => {
                // retry the Psync Taks
                let pt_task_update = diesel_models::ProcessTrackerUpdate::StatusUpdate {
                    status: storage::enums::ProcessTrackerStatus::Pending,
                    business_status: Some(business_status::PENDING.to_owned()),
                };
                db.update_process(process.clone(), pt_task_update)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::RetryPayment(process_tracker) => {
                // finish the psync task
                db.finish_process_with_business_status(
                    process_tracker.clone(),
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
                let pt = db
                    .find_process_by_id(&process_tracker_id)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to find a entry in process tracker")?;
                let pt_task_update = diesel_models::ProcessTrackerUpdate::StatusUpdate {
                    status: storage::enums::ProcessTrackerStatus::Pending,
                    business_status: Some(business_status::PENDING.to_owned()),
                };

                db.update_process(
                    pt.get_required_value("ProcessTracker")
                        .change_context(errors::RecoveryError::ProcessTrackerFailure)
                        .attach_printable("Failed to find a entry in process tracker")?,
                    pt_task_update,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::TerminalFailure => {
                core_pcr::terminal_payment_failure_handling(
                    db,
                    key_manager_state,
                    payment_intent.clone(),
                    &pcr_data.key_store,
                    pcr_data.merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::RecoveryError::RecoveryFailed)
                .attach_printable("Failed to update the payment intent with terminal status")?;
                Ok(())
            }
            Self::SuccessfulPayment => todo!(),
            Self::ReviewPayment => todo!(),
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                Ok(())
            }
        }
    }
}

// This function would be converted to proxy_payments_core
fn call_proxy_api<F>(payment_intent: &PaymentIntent) -> RouterResult<PaymentConfirmData<F>>
where
    F: Send + Clone + Sync,
{
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
