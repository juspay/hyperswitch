use common_enums::{self, AttemptStatus, IntentStatus};
use common_utils::{self, ext_traits::OptionExt, id_type};
use diesel_models::{enums, process_tracker::business_status};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    business_profile, merchant_account,
    payments::{PaymentConfirmData, PaymentIntent},
};
use time::PrimitiveDateTime;

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
}
#[derive(Debug, Clone)]
pub enum Decision {
    Execute,
    Psync(AttemptStatus, id_type::GlobalAttemptId),
    InvalidDecision,
}

impl Decision {
    pub async fn get_decision_based_on_params(
        state: &SessionState,
        intent_status: IntentStatus,
        called_connector: bool,
        active_attempt_id: Option<id_type::GlobalAttemptId>,
        pcr_data: &storage::passive_churn_recovery::PcrPaymentData,
        payment_id: &id_type::GlobalPaymentId,
    ) -> RecoveryResult<Self> {
        Ok(match (intent_status, called_connector, active_attempt_id) {
            (IntentStatus::Failed, false, None) => Self::Execute,
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
                Self::Psync(payment_attempt.status, payment_attempt.get_id().clone())
            }
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
        db: &dyn StorageInterface,
        merchant_account: &merchant_account::MerchantAccount,
        payment_intent: &PaymentIntent,
        execute_task_process: &storage::ProcessTracker,
        profile: &business_profile::Profile,
    ) -> Result<(), errors::ProcessTrackerError> {
        match self {
            Self::SyncPayment(attempt_id) => {
                core_pcr::insert_psync_pcr_task(
                    db,
                    merchant_account.get_id().to_owned(),
                    payment_intent.id.clone(),
                    profile.get_id().to_owned(),
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
                let mut pt = execute_task_process.clone();
                // update the schedule time
                pt.schedule_time = Some(*schedule_time);

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
        mandate_data: None,
    };
    Ok(response)
}
