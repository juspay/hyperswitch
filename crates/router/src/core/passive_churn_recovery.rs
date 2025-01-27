use api_models::payments::{
    PaymentsCreateIntentRequest, PaymentsRetrieveRequest, PaymentsUpdateIntentRequest,
};
use common_enums::{AttemptStatus, IntentStatus, PCRAttemptStatus};
use common_utils::{self, errors::CustomResult, id_type, types::keymanager::KeyManagerState};
use error_stack::{self, report, ResultExt};
use hyperswitch_domain_models::{
    business_profile,
    errors::api_error_response,
    merchant_account,
    merchant_key_store::MerchantKeyStore,
    payments::{payment_attempt::PaymentAttempt, PaymentConfirmData, PaymentIntent},
};
use scheduler::errors;

use super::errors::RecoveryError;
use crate::{
    core::errors::RouterResult,
    db::StorageInterface,
    logger,
    routes::{metrics, SessionState},
    types::{
        api::payments as api_types,
        storage::{self, passive_churn_recovery as pcr},
    },
    workflows::passive_churn_recovery_workflow::{
        get_schedule_time_to_retry_mit_payments, retry_pcr_payment_task,
    },
};

type RecoveryResult<T> = error_stack::Result<T, RecoveryError>;
pub async fn decide_execute_pcr_workflow(
    state: &SessionState,
    process: &storage::ProcessTracker,
    payment_intent: &PaymentIntent,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &MerchantKeyStore,
    merchant_account: &merchant_account::MerchantAccount,
    _profile: &business_profile::Profile,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let decision_task = Decision::get_decision_based_on_params(
        db,
        payment_intent.status,
        true,
        payment_intent.active_attempt_id.clone(),
        key_manager_state,
        merchant_key_store,
        merchant_account,
    )
    .await?;

    match decision_task {
        Decision::ExecuteTask => {
            let action =
                Action::execute_payment(db, merchant_account.get_id(), payment_intent, process)
                    .await?;
            action.execute_payment_response_handler();
        }
        Decision::PsyncTask(attempt_status) => {
            // find if a psync task is already present
            let process_tracker_entry = db.find_process_by_id(&process.id).await?;
            // validate if its a psync task
            match process_tracker_entry {
                Some(mut process_tracker) if process.name == Some("PSYNC_WORKFLOW".to_string()) => {
                    match attempt_status.into() {
                        PCRAttemptStatus::Succeeded => {
                            // finish psync task as the payment was a success
                            db.finish_process_with_business_status(
                                process_tracker,
                                "COMPLETED_PSYNC_TASK",
                            )
                            .await?;
                            // TODO: send back the successful webhook

                            // finish the current execute task as the payment has been completed
                            db.finish_process_with_business_status(
                                process.clone(),
                                "COMPLETED_EXECUTE_TASK",
                            )
                            .await?;
                        }

                        PCRAttemptStatus::Failed => {
                            // reschedule a payment
                            process_tracker.schedule_time =
                                get_schedule_time_to_retry_mit_payments(
                                    db,
                                    merchant_account.get_id(),
                                    process_tracker.retry_count.clone(),
                                )
                                .await;
                            // finish psync task
                            db.finish_process_with_business_status(
                                process_tracker.clone(),
                                "COMPLETED_PSYNC_TASK",
                            )
                            .await?;

                            let action = Action::execute_payment(
                                db,
                                merchant_account.get_id(),
                                payment_intent,
                                &process_tracker,
                            )
                            .await?;
                            action.execute_payment_response_handler();
                        }

                        PCRAttemptStatus::Processing => {
                            // finish the current execute task
                            db.finish_process_with_business_status(
                                process.clone(),
                                "COMPLETED_EXECUTE_TASK",
                            )
                            .await?;
                        }

                        PCRAttemptStatus::InvalidAction(action) => {
                            logger::debug!(
                                "Inavlid Attempt Status for the Recovery Payment : {}",
                                action
                            )
                        }
                    }
                }
                Some(pt) => logger::debug!("Inavlid Process Tracker name : {}", pt.id),
                None => {
                    let req = PaymentsRetrieveRequest {
                        force_sync: false,
                        param: None,
                    };
                    insert_psync_pcr_workflow(
                        &state,
                        merchant_account.get_id().clone(),
                        payment_intent.get_id().clone(),
                        req,
                        storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                    )
                    .await?;
                }
            };
        }
        Decision::ReviewTask => {
            db.finish_process_with_business_status(process.clone(), "COMPLETED_EXECUTE_TASK")
                .await?;
            logger::warn!("Abnormal State Identified")
        }
    }
    Ok(())
}

async fn insert_psync_pcr_workflow(
    state: &SessionState,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::GlobalPaymentId,
    request: PaymentsRetrieveRequest,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
    let db = &*state.store;
    let task = "PSYNC_WORKFLOW";
    let process_tracker_id = format!("{runner}_{task}_{}", payment_id.get_string_repr());
    let schedule_time = common_utils::date_time::now();
    let psync_workflow_tracking_data = pcr::PCRPsyncWorkflowTrackingData {
        global_payment_id: payment_id,
        merchant_id,
        request,
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

#[derive(Debug, Clone)]
pub enum Decision {
    ExecuteTask,
    PsyncTask(AttemptStatus),
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
                    .change_context(RecoveryError::TaskNotFound)?;
                Self::PsyncTask(payment_attempt.status)
            }
            _ => Self::ReviewTask,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SyncPayment,
    RetryPayment(storage::ProcessTracker),
    TerminalPaymentStatus,
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
            Ok(payment_data) => match payment_data.payment_attempt.status.into() {
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

    pub fn execute_payment_response_handler(&self) -> RecoveryResult<()> {
        match self {
            Self::SyncPayment => Ok(()),
            Self::RetryPayment(_) => Ok(()),
            Self::TerminalPaymentStatus => Ok(()),
            Self::SuccessfulPayment => Ok(()),
            Self::ReviewPayment => Ok(()),
            Self::ManualReviewAction => Ok(()),
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
