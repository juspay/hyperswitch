use common_utils::ext_traits::ValueExt;
use storage_models::enums::{self as storage_enums};

use super::{ApiKeyExpiryWorkflow, ProcessTrackerWorkflow};
use crate::{
    errors,
    logger::error,
    routes::AppState,
    types::{
        api,
        storage::{self, ProcessTrackerExt},
    },
    utils::OptionExt,
};

#[allow(clippy::unwrap_used)]
#[async_trait::async_trait]
impl ProcessTrackerWorkflow for ApiKeyExpiryWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data: storage::ApiKeyExpiryWorkflow = process
            .tracking_data
            .clone()
            .parse_value("ApiKeyExpiryWorkflow")?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(tracking_data.merchant_id.as_str())
            .await?;

        let email_id = merchant_account
            .merchant_details
            .parse_value::<api::MerchantDetails>("MerchantDetails")?
            .primary_email;

        let task_id = process.id.clone();

        let retry_count = process.retry_count;

        // if retry_count is greater than zero, it means there is already a scheduled task.
        if retry_count > 0 {
            let expires_in =
                tracking_data.expiry_reminder_days[usize::try_from(retry_count - 1).unwrap()];
            state
                .email_client
                .send_email(
                    email_id.ok_or_else(|| errors::ProcessTrackerError::MissingRequiredField)?,
                    "API Key Expiry Notice".to_string(),
                    format!("Dear Merchant,\n
It has come to our attention that your API key is due in {expires_in} days. To ensure uninterrupted access to our platform and continued smooth operation of your services, we kindly request that you take the necessary actions as soon as possible.\n\n
Thanks,\n
Team Hyperswitch"),
                )
                .await
                .map_err(|_| errors::ProcessTrackerError::FlowExecutionError {
                    flow: "ApiKeyExpiryWorkflow",
                })?;
        }

        // If all the mails have been sent, then retry_count would be equal to length of the expiry_reminder_days vector
        if retry_count == i32::try_from(tracking_data.expiry_reminder_days.len()).unwrap() {
            process
                .finish_with_status(db, format!("COMPLETED_BY_PT_{task_id}"))
                .await?
        }
        // If still task has to be scheduled
        else {
            let expiry_reminder_day =
                tracking_data.expiry_reminder_days[usize::try_from(retry_count).unwrap()];
            let updated_schedule_time =
                tracking_data
                    .api_key_expiry
                    .unwrap()
                    .saturating_sub(time::Duration::days(
                        i64::try_from(expiry_reminder_day).unwrap(),
                    ));
            let updated_process_tracker_data = storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: Some(retry_count + 1),
                schedule_time: Some(updated_schedule_time),
                tracking_data: None,
                business_status: None,
                status: Some(storage_enums::ProcessTrackerStatus::New),
                updated_at: Some(common_utils::date_time::now()),
            };
            let task_ids = vec![task_id.clone()];
            db.process_tracker_update_process_status_by_ids(task_ids, updated_process_tracker_data)
                .await?;
        }

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
