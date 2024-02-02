use common_utils::ext_traits::ValueExt;
use diesel_models::enums::{self as storage_enums};

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

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for ApiKeyExpiryWorkflow {
        /// Executes the workflow for API key expiry, including sending email notifications to merchants about expiring API keys and handling retry logic for sending reminders. 
    /// 
    /// # Arguments
    /// * `state` - The application state containing the store and email client
    /// * `process` - The process tracker for the workflow
    /// 
    /// # Returns
    /// * `Result<(), errors::ProcessTrackerError>` - Result indicating success or an error of type `errors::ProcessTrackerError`
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

        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                tracking_data.merchant_id.as_str(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(tracking_data.merchant_id.as_str(), &key_store)
            .await?;

        let email_id = merchant_account
            .merchant_details
            .parse_value::<api::MerchantDetails>("MerchantDetails")?
            .primary_email;

        let task_id = process.id.clone();

        let retry_count = process.retry_count;

        let expires_in = tracking_data
            .expiry_reminder_days
            .get(
                usize::try_from(retry_count)
                    .map_err(|_| errors::ProcessTrackerError::TypeConversionError)?,
            )
            .ok_or(errors::ProcessTrackerError::EApiErrorResponse(
                errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "index",
                }
                .into(),
            ))?;

        state
            .email_client
            .clone()
            .send_email(
                email_id.ok_or_else(|| errors::ProcessTrackerError::MissingRequiredField)?,
                "API Key Expiry Notice".to_string(),
                format!("Dear Merchant,\n
It has come to our attention that your API key will expire in {expires_in} days. To ensure uninterrupted access to our platform and continued smooth operation of your services, we kindly request that you take the necessary actions as soon as possible.\n\n
Thanks,\n
Team Hyperswitch"),
            )
            .await
            .map_err(|_| errors::ProcessTrackerError::FlowExecutionError {
                flow: "ApiKeyExpiryWorkflow",
            })?;

        // If all the mails have been sent, then retry_count would be equal to length of the expiry_reminder_days vector
        if retry_count
            == i32::try_from(tracking_data.expiry_reminder_days.len() - 1)
                .map_err(|_| errors::ProcessTrackerError::TypeConversionError)?
        {
            process
                .finish_with_status(db, format!("COMPLETED_BY_PT_{task_id}"))
                .await?
        }
        // If tasks are remaining that has to be scheduled
        else {
            let expiry_reminder_day = tracking_data
                .expiry_reminder_days
                .get(
                    usize::try_from(retry_count + 1)
                        .map_err(|_| errors::ProcessTrackerError::TypeConversionError)?,
                )
                .ok_or(errors::ProcessTrackerError::EApiErrorResponse(
                    errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "index",
                    }
                    .into(),
                ))?;

            let updated_schedule_time = tracking_data.api_key_expiry.map(|api_key_expiry| {
                api_key_expiry.saturating_sub(time::Duration::days(i64::from(*expiry_reminder_day)))
            });
            let updated_process_tracker_data = storage::ProcessTrackerUpdate::Update {
                name: None,
                retry_count: Some(retry_count + 1),
                schedule_time: updated_schedule_time,
                tracking_data: None,
                business_status: None,
                status: Some(storage_enums::ProcessTrackerStatus::New),
                updated_at: Some(common_utils::date_time::now()),
            };
            let task_ids = vec![task_id];
            db.process_tracker_update_process_status_by_ids(task_ids, updated_process_tracker_data)
                .await?;
            // Remaining tasks are re-scheduled, so will be resetting the added count
            metrics::TASKS_RESET_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[metrics::request::add_attributes("flow", "ApiKeyExpiry")],
            );
        }

        Ok(())
    }

        /// Asynchronously handles any errors that occur during the execution of a workflow. 
    ///
    /// # Arguments
    ///
    /// * `state` - The reference to the application state
    /// * `process` - The storage process tracker
    /// * `error` - The process tracker error that occurred
    ///
    /// # Returns
    ///
    /// A custom result that either contains an empty value or a process tracker error
    ///
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
