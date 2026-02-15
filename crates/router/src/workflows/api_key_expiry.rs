use common_utils::{errors::ValidationError, ext_traits::ValueExt, types::user::ThemeLineage};
use diesel_models::{
    enums as storage_enums, process_tracker::business_status, ApiKeyExpiryTrackingData,
};
use router_env::logger;
use scheduler::{workflows::ProcessTrackerWorkflow, SchedulerSessionState};

use crate::{
    consts, errors,
    logger::error,
    routes::{metrics, SessionState},
    services::email::types::ApiKeyExpiryReminder,
    types::{api, domain::UserEmail, storage},
    utils::{
        user::{self as user_utils, theme as theme_utils},
        OptionExt,
    },
};

pub struct ApiKeyExpiryWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for ApiKeyExpiryWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data: ApiKeyExpiryTrackingData = process
            .tracking_data
            .clone()
            .parse_value("ApiKeyExpiryTrackingData")?;
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
            .await?;

        let email_id = merchant_account
            .merchant_details
            .clone()
            .parse_value::<api::MerchantDetails>("MerchantDetails")?
            .primary_email
            .ok_or(errors::ProcessTrackerError::EValidationError(
                ValidationError::MissingRequiredField {
                    field_name: "email".to_string(),
                }
                .into(),
            ))?;

        let task_id = process.id.clone();

        let retry_count = process.retry_count;

        let api_key_name = tracking_data.api_key_name.clone();

        let prefix = tracking_data.prefix.clone();

        let expires_in = tracking_data
            .expiry_reminder_days
            .get(
                usize::try_from(retry_count)
                    .map_err(|_| errors::ProcessTrackerError::TypeConversionError)?,
            )
            .ok_or(errors::ProcessTrackerError::EApiErrorResponse)?;

        let theme = theme_utils::get_most_specific_theme_using_lineage(
            state,
            ThemeLineage::Merchant {
                tenant_id: state.tenant.tenant_id.clone(),
                org_id: merchant_account.get_org_id().clone(),
                merchant_id: merchant_account.get_id().clone(),
            },
        )
        .await
        .map_err(|err| {
            logger::error!(?err, "Failed to get theme");
            errors::ProcessTrackerError::EApiErrorResponse
        })?;

        let email_contents = ApiKeyExpiryReminder {
            recipient_email: UserEmail::from_pii_email(email_id).map_err(|error| {
                logger::error!(
                    ?error,
                    "Failed to convert recipient's email to UserEmail from pii::Email"
                );
                errors::ProcessTrackerError::EApiErrorResponse
            })?,
            subject: consts::EMAIL_SUBJECT_API_KEY_EXPIRY,
            expires_in: *expires_in,
            api_key_name,
            prefix,
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };

        state
            .email_client
            .clone()
            .compose_and_send_email(
                user_utils::get_base_url(state),
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await
            .map_err(errors::ProcessTrackerError::EEmailError)?;

        // If all the mails have been sent, then retry_count would be equal to length of the expiry_reminder_days vector
        if retry_count
            == i32::try_from(tracking_data.expiry_reminder_days.len() - 1)
                .map_err(|_| errors::ProcessTrackerError::TypeConversionError)?
        {
            state
                .get_db()
                .as_scheduler()
                .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
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
                .ok_or(errors::ProcessTrackerError::EApiErrorResponse)?;

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
            metrics::TASKS_RESET_COUNT
                .add(1, router_env::metric_attributes!(("flow", "ApiKeyExpiry")));
        }

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a SessionState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing workflow");
        Ok(())
    }
}
