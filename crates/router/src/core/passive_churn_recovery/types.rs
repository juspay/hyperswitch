use common_enums::{self, IntentStatus};
use common_utils::{self, ext_traits::OptionExt, id_type};
use diesel_models::{enums, process_tracker::business_status};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    business_profile, merchant_account,
    payments::{payment_attempt::PaymentAttempt, PaymentConfirmData, PaymentIntent},
};

use crate::{
    core::{
        errors::{self, RouterResult},
        passive_churn_recovery::{self as core_pcr},
    },
    db::StorageInterface,
    logger,
    routes::SessionState,
    types::{api::payments as api_types, storage, transformers::ForeignInto},
    workflows::passive_churn_recovery_workflow::get_schedule_time_to_retry_mit_payments,
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
    pub(crate) async fn update_pt_status_based_on_attempt_status(
        &self,
        db: &dyn StorageInterface,
        merchant_id: &id_type::MerchantId,
        pt_psync_process: storage::ProcessTracker,
        execute_task_process: &storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        match &self {
            Self::Succeeded => {
                // finish psync task as the payment was a success
                db.finish_process_with_business_status(
                    pt_psync_process,
                    business_status::PSYNC_WORKFLOW_COMPLETE,
                )
                .await?;
                // TODO: send back the successful webhook

                // finish the current execute task as the payment has been completed
                db.finish_process_with_business_status(
                    execute_task_process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE,
                )
                .await?;
            }

            Self::Failed => {
                // finish psync task
                db.finish_process_with_business_status(
                    pt_psync_process.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE,
                )
                .await?;

                // get a reschedule time
                let schedule_time = get_schedule_time_to_retry_mit_payments(
                    db,
                    merchant_id,
                    execute_task_process.retry_count + 1,
                )
                .await;

                // check if retry is possible
                if let Some(schedule_time) = schedule_time {
                    // schedule a retry
                    db.retry_process(execute_task_process.clone(), schedule_time)
                        .await?;
                } else {
                    // TODO: Record a failure back to the billing connector
                }
            }

            Self::Processing => {
                // finish the current execute task
                db.finish_process_with_business_status(
                    execute_task_process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                )
                .await?;
            }

            Self::InvalidAction(action) => {
                logger::debug!(
                    "Invalid Attempt Status for the Recovery Payment : {}",
                    action
                );
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(
                        business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                    )),
                };
                // update the process tracker status as Review
                db.update_process(execute_task_process.clone(), pt_update)
                    .await?;
            }
        };
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub enum Decision {
    ExecuteTask,
    PsyncTask(PaymentAttempt),
    InvalidTask,
}

impl Decision {
    pub async fn get_decision_based_on_params(
        state: &SessionState,
        intent_status: IntentStatus,
        called_connector: bool,
        active_attempt_id: Option<id_type::GlobalAttemptId>,
        pcr_data: &storage::passive_churn_recovery::PCRPaymentData,
        payment_id: &id_type::GlobalPaymentId,
    ) -> RecoveryResult<Self> {
        Ok(match (intent_status, called_connector, active_attempt_id) {
            (IntentStatus::Failed, false, None) => Self::ExecuteTask,
            (IntentStatus::Processing, true, Some(_)) => {
                let psync_data = core_pcr::call_psync_api(state, payment_id, pcr_data)
                    .await
                    .change_context(errors::RecoveryError::PaymentCallFailed)
                    .attach_printable("Error while executing the Psync call")?;
                let payment_attempt = psync_data
                    .payment_attempt
                    .get_required_value("Payment Attempt")
                    .change_context(errors::RecoveryError::ValueNotFound)
                    .attach_printable("Error while executing the Psync call")?;
                Self::PsyncTask(payment_attempt)
            }
            _ => Self::InvalidTask,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SyncPayment,
    RetryPayment,
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
                    Self::decide_retry_failure_action(db, merchant_id, process.clone()).await
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

    pub async fn execute_payment_task_response_handler(
        &self,
        db: &dyn StorageInterface,
        merchant_account: &merchant_account::MerchantAccount,
        payment_intent: &PaymentIntent,
        execute_task_process: &storage::ProcessTracker,
        profile: &business_profile::Profile,
    ) -> Result<(), errors::ProcessTrackerError> {
        match self {
            Self::SyncPayment => {
                core_pcr::insert_psync_pcr_task(
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

            Self::RetryPayment => {
                let mut pt = execute_task_process.clone();
                // update the schedule time
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
                db.as_scheduler()
                    .update_process(pt.clone(), pt_task_update)
                    .await?;
                // TODO: update the connector called field and make the active attempt None

                Ok(())
            }
            Self::TerminalFailure => {
                // TODO: Record a failure transaction back to Billing Connector
                Ok(())
            }
            Self::SuccessfulPayment => Ok(()),
            Self::ReviewPayment => Ok(()),
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(
                        business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                    )),
                };
                // update the process tracker status as Review
                db.as_scheduler()
                    .update_process(execute_task_process.clone(), pt_update)
                    .await?;
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
            Some(_) => Ok(Self::RetryPayment),

            None => Ok(Self::TerminalFailure),
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
