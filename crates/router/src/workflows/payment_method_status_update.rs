use common_utils::ext_traits::ValueExt;
use diesel_models::enums as storage_enums;
// use router_env::logger;
use scheduler::workflows::ProcessTrackerWorkflow;

use crate::{
    errors,
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

        // let task_id = process.id.clone();
        // let retry_count = process.retry_count;
        let pm_id = tracking_data.payment_method_id;
        // let pm_status = tracking_data.status;
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
            status: Some(storage_enums::PaymentMethodStatus::Inactive),
        };

        db.update_payment_method(payment_method, pm_update, merchant_account.storage_scheme)
            .await
            .map_err(|err| errors::ProcessTrackerError::EStorageError(err))?;

        db.as_scheduler()
            .finish_process_with_business_status(process, "COMPLETED_BY_PT".to_string())
            .await?;

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
