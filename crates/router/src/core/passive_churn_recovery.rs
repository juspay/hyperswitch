pub mod transformers;
pub mod types;
use crate::{
    core::{
        errors::{self as error, RouterResult},
        passive_churn_recovery::types as pcr_types,
    },
    db::StorageInterface,
    logger,
    routes::{metrics, SessionState},
    types::{
        storage::{self, passive_churn_recovery as pcr},
        transformers::ForeignInto,
    },
};
use api_models::payments::PaymentsRetrieveRequest;
use common_enums::{self, IntentStatus};
use common_utils::{self, id_type, types::keymanager::KeyManagerState};
use diesel_models::process_tracker::business_status;
use error_stack::{self, report, ResultExt};
use hyperswitch_domain_models::{
    business_profile, errors::api_error_response, merchant_account,
    merchant_key_store::MerchantKeyStore, payments::PaymentIntent,
};
use scheduler::errors;
use storage_impl::errors::StorageError;

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
    let decision_task = pcr_types::Decision::get_decision_based_on_params(
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
        pcr_types::Decision::ExecuteTask => {
            let action = pcr_types::Action::execute_payment(
                db,
                merchant_account.get_id(),
                payment_intent,
                process,
            )
            .await?;
            action
                .execute_payment_response_handler(
                    db,
                    merchant_account,
                    payment_intent,
                    key_manager_state,
                    merchant_key_store,
                    process,
                )
                .await?;
        }

        pcr_types::Decision::PsyncTask(payment_attempt) => {
            // find if a psync task is already present
            let process_tracker_entry = db.find_process_by_id(&process.id).await?;

            // validate if its a psync task
            match process_tracker_entry {
                Some(process_tracker) if process.name == Some("PSYNC_WORKFLOW".to_string()) => {
                    let pcr_status: pcr_types::PCRAttemptStatus =
                        payment_attempt.status.foreign_into();

                    pcr_status
                        .perform_action_based_on_status(
                            db,
                            merchant_account.get_id(),
                            process_tracker,
                            process,
                            key_manager_state,
                            payment_intent.clone(),
                            merchant_key_store,
                            merchant_account.storage_scheme,
                        )
                        .await?;
                }

                Some(pt) => logger::debug!("Invalid Process Tracker name : {}", pt.id),

                None => {
                    let req = PaymentsRetrieveRequest {
                        force_sync: false,
                        param: None,
                    };
                    insert_psync_pcr_workflow(
                        db,
                        merchant_account.get_id().clone(),
                        payment_intent.get_id().clone(),
                        req,
                        storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                    )
                    .await?;
                }
            };
        }
        pcr_types::Decision::ReviewTask => {
            db.finish_process_with_business_status(
                process.clone(),
                business_status::EXECUTE_WORKFLOW_COMPLETE,
            )
            .await?;
            logger::warn!("Abnormal State Identified")
        }
    }
    Ok(())
}

async fn insert_psync_pcr_workflow(
    db: &dyn StorageInterface,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::GlobalPaymentId,
    request: PaymentsRetrieveRequest,
    runner: storage::ProcessTrackerRunner,
) -> RouterResult<storage::ProcessTracker> {
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

async fn terminal_payment_failure_handling(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    payment_intent: PaymentIntent,
    merchant_key_store: &MerchantKeyStore,
    storage_scheme: common_enums::MerchantStorageScheme,
) -> error::CustomResult<(), errors::ProcessTrackerError> {
    let payment_intent_update = storage::PaymentIntentUpdate::ConfirmIntent {
        status: IntentStatus::Failed,
        updated_by: storage_scheme.to_string(),
        active_attempt_id: None,
    };
    // mark the intent as failure
    db.update_payment_intent(
        key_manager_state,
        payment_intent,
        payment_intent_update,
        merchant_key_store,
        storage_scheme,
    )
    .await
    .change_context(errors::ProcessTrackerError::EStorageError(report!(
        StorageError::DatabaseConnectionError
    )))?;
    Ok(())
}
