use common_enums::enums;
use common_utils::{
    consts::DEFAULT_LOCALE,
    ext_traits::{OptionExt, ValueExt},
};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors, utils as scheduler_utils,
};

use crate::{
    core::{
        configs::{self, dimension_state::DimensionsWithMerchantIdAndConnector},
        payouts, webhooks,
    },
    errors as core_errors,
    routes::SessionState,
    types::{api, domain, storage},
};

pub struct PayoutSyncWorkFlow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PayoutSyncWorkFlow {
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        // Gather context
        let db = &*state.store;
        let tracking_data: api::PayoutRetrieveRequest = process
            .tracking_data
            .clone()
            .parse_value("PayoutRetrieveRequest")?;

        let merchant_id = tracking_data
            .merchant_id
            .clone()
            .get_required_value("merchant_id")?;
        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await?;

        let request = api::payouts::PayoutRequest::PayoutRetrieveRequest(tracking_data);

        let platform = domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account,
            key_store.clone(),
            None,
        );

        let mut payout_data = Box::pin(payouts::make_payout_data(
            state,
            &platform,
            None,
            &request,
            DEFAULT_LOCALE,
        ))
        .await?;

        let connector_name = payout_data
            .payout_attempt
            .connector
            .clone()
            .ok_or(errors::ProcessTrackerError::MissingRequiredField)?;

        let connector_data = api::ConnectorData::get_payout_connector_by_name(
            &state.conf.connectors,
            connector_name.as_str(),
            api::GetToken::Connector,
            payout_data.payout_attempt.merchant_connector_id.clone(),
        )
        .change_context(core_errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get the connector data")?;

        Box::pin(payouts::create_payout_retrieve(
            state,
            &platform,
            &connector_data,
            &mut payout_data,
        ))
        .await?;

        let dimensions = configs::dimension_state::Dimensions::new()
            .with_merchant_id(merchant_id.clone())
            .with_connector(connector_data.connector_name);

        if payout_data.payout_attempt.status.is_terminal_status() {
            Self::process_terminal_task(
                state,
                &mut payout_data,
                &key_store,
                &platform,
                process.clone(),
            )
            .await?;
        } else {
            Self::retry_payout_sync_task(
                state,
                payout_data.payouts.payout_id,
                process,
                &dimensions,
            )
            .await?;
        }

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> core_errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

impl PayoutSyncWorkFlow {
    pub async fn add_payout_sync_task_to_process_tracker(
        state: &SessionState,
        payout_data: &payouts::PayoutData,
        application_source: common_enums::ApplicationSource,
        dimensions: &DimensionsWithMerchantIdAndConnector,
    ) -> common_utils::errors::CustomResult<(), core_errors::ApiErrorResponse> {
        let db = &*state.store;
        let scheduled_time = Self::get_payout_sync_process_schedule_time(
            state,
            payout_data.payouts.payout_id.clone(),
            0,
            dimensions,
        )
        .await
        .change_context(core_errors::ApiErrorResponse::InternalServerError)?;
        match scheduled_time {
            Some(schedule_time) => {
                let runner = storage::ProcessTrackerRunner::PayoutSyncWorkFlow;
                let task = "PAYOUTS_SYNC";
                let tag = ["PAYOUTS", "SYNC"];
                let process_tracker_id = scheduler_utils::get_process_tracker_id(
                    runner,
                    task,
                    &payout_data.payout_attempt.payout_attempt_id,
                    &payout_data.payout_attempt.merchant_id,
                );
                let tracking_data = api::PayoutRetrieveRequest {
                    payout_id: payout_data.payouts.payout_id.to_owned(),
                    force_sync: Some(true),
                    merchant_id: Some(payout_data.payouts.merchant_id.to_owned()),
                };
                let process_tracker_entry = storage::ProcessTrackerNew::new(
                    process_tracker_id,
                    task,
                    runner,
                    tag,
                    tracking_data,
                    None,
                    schedule_time,
                    common_types::consts::API_VERSION,
                    application_source,
                )
                .change_context(core_errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting process schedule time")?;

                db.insert_process(process_tracker_entry)
                    .await
                    .change_context(core_errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to insert the process tracker entry")?;
                Ok(())
            }
            None => Ok(()),
        }
    }
    /// Get the next schedule time
    ///
    /// The schedule time can be configured in configs by this key `payout_tracker_mapping_trustpay`
    /// ```json
    /// {
    ///     "default_mapping": {
    ///         "start_after": 60,
    ///         "frequencies": [(300, 5), (1800, 2)],
    ///     },
    ///     "max_retries_count": 5
    /// }
    /// ```
    ///
    /// This config represents
    ///
    /// `start_after`: The first psync should happen after 60 seconds
    ///
    /// `frequencies`: Do 5 retries with an interval of 300 seconds between them.
    ///     After than do 2 retries with an interval of 1800 seconds.
    pub async fn get_payout_sync_process_schedule_time(
        state: &SessionState,
        payout_id: common_utils::id_type::PayoutId,
        retry_count: i32,
        dimensions: &DimensionsWithMerchantIdAndConnector,
    ) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
        let value = dimensions
            .get_payout_tracker_mapping(
                state.store.as_ref(),
                state.superposition_service.as_deref(),
                Some(&payout_id),
            )
            .await;

        let time_delta = Self::get_schedule_time(value, retry_count);

        Ok(scheduler_utils::get_time_from_delta(time_delta))
    }

    /// get the schedule time for next process
    pub fn get_schedule_time(mapping: process_data::RetryMapping, retry_count: i32) -> Option<i32> {
        // For first try, get the `start_after` time
        if retry_count == 0 {
            Some(mapping.start_after)
        } else {
            scheduler_utils::get_delay(retry_count, &mapping.frequencies)
        }
    }

    /// process the task if the status is terminal
    #[cfg(feature = "v1")]
    pub async fn process_terminal_task(
        state: &SessionState,
        payout_data: &mut payouts::PayoutData,
        key_store: &MerchantKeyStore,
        platform: &domain::Platform,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
            .await?;

        let event_type: Option<enums::EventType> = payout_data.payout_attempt.status.into();

        let business_profile = db
            .find_business_profile_by_profile_id(key_store, &payout_data.profile_id)
            .await
            .map_err(errors::ProcessTrackerError::EStorageError)?;

        // Trigger webhook to the merchant if it is a terminal status
        // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
        if let Some(outgoing_event_type) = event_type {
            let payout_response = payouts::response_handler(state, platform, payout_data).await?;

            Box::pin(webhooks::create_event_and_trigger_outgoing_webhook(
                state.clone(),
                platform.get_processor().clone(),
                business_profile,
                outgoing_event_type,
                enums::EventClass::Payouts,
                payout_data.payouts.payout_id.get_string_repr().to_string(),
                enums::EventObjectType::PayoutDetails,
                api::OutgoingWebhookContent::PayoutDetails(Box::new(payout_response)),
                Some(payout_data.payout_attempt.created_at),
            ))
            .await?;
        }

        Ok(())
    }

    /// Schedule the task for retry
    pub async fn retry_payout_sync_task(
        state: &SessionState,
        payout_id: common_utils::id_type::PayoutId,
        pt: storage::ProcessTracker,
        dimensions: &DimensionsWithMerchantIdAndConnector,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let schedule_time: Option<time::PrimitiveDateTime> =
            Self::get_payout_sync_process_schedule_time(
                state,
                payout_id,
                pt.retry_count + 1,
                dimensions,
            )
            .await?;

        match schedule_time {
            Some(s_time) => {
                db.as_scheduler().retry_process(pt, s_time).await?;
            }
            None => {
                db.as_scheduler()
                    .finish_process_with_business_status(pt, business_status::RETRIES_EXCEEDED)
                    .await?;
            }
        }

        Ok(())
    }
}
