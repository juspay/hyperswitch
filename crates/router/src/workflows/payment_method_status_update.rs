use common_utils::{date_time, ext_traits::ValueExt};
use diesel_models::enums as storage_enums;
// use router_env::logger;
use scheduler::workflows::ProcessTrackerWorkflow;

use crate::{
    consts, errors,
    logger::error,
    routes::{metrics, AppState},
    types::storage::{self, PaymentMethodStatusTrackingData},
};

pub struct PaymentMethodStatusUpdateWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for PaymentMethodStatusUpdateWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data: PaymentMethodStatusTrackingData = process
            .tracking_data
            .clone()
            .parse_value("PaymentMethodStatusTrackingData")?;

        let task_id = process.id.clone();
        let retry_count = process.retry_count;
        let pm_id = tracking_data.payment_method_id;
        let pm_status = tracking_data.status;
        let merchant_id = tracking_data.merchant_id;

        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                merchant_id.as_str(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(merchant_id.as_str(), &key_store)
            .await?;

        let payment_method = db
            .find_payment_method(pm_id.as_str(), merchant_account.storage_scheme)
            .await?;

        let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
            status: Some(pm_status),
        };

        let res = db
            .update_payment_method(payment_method, pm_update, merchant_account.storage_scheme)
            .await
            .map_err(errors::ProcessTrackerError::EStorageError);

        if let Ok(_pm) = res {
            db.as_scheduler()
                .finish_process_with_business_status(process, "COMPLETED_BY_PT".to_string())
                .await?;
        } else if retry_count + 1 == 5 {
            db.as_scheduler()
                .finish_process_with_business_status(process, "COMPLETED_BY_PT".to_string())
                .await?;
        } else {
            let updated_schedule_time = date_time::now()
                .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));
            let updated_process_tracker_data = storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: Some(retry_count + 1),
                schedule_time: Some(updated_schedule_time),
                tracking_data: None,
                business_status: None,
                status: Some(storage_enums::ProcessTrackerStatus::New),
                updated_at: Some(date_time::now()),
            };

            let task_ids = vec![task_id];
            db.process_tracker_update_process_status_by_ids(task_ids, updated_process_tracker_data)
                .await?;
        };

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a AppState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing workflow");
        Ok(())
    }
}
