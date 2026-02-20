use common_enums::enums;
use common_utils::{
    consts::DEFAULT_LOCALE,
    ext_traits::{OptionExt, StringExt, ValueExt},
};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use hyperswitch_domain_models::payouts::{
    payout_attempt::PayoutAttemptUpdate, payouts::PayoutsUpdate,
};
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors, utils as scheduler_utils,
};

use crate::{
    consts::REQUEST_TIMEOUT_ERROR_MESSAGE_FROM_PAYOUT_SYNC,
    core::{payouts, webhooks},
    db::StorageInterface,
    errors as core_errors,
    routes::SessionState,
    types::{api, domain, storage},
};

pub struct PayoutSyncWorkFlow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PayoutSyncWorkFlow {
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

        payouts::create_payout_retrieve(state, &platform, &connector_data, &mut payout_data)
            .await?;

        match payout_data.payout_attempt.status.is_non_terminal_status() {
            false => {
                state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
                    .await?;

                let event_type: Option<enums::EventType> = payout_data.payout_attempt.status.into();

                let business_profile = db
                    .find_business_profile_by_profile_id(&key_store, &payout_data.profile_id)
                    .await
                    .map_err(errors::ProcessTrackerError::EStorageError)?;

                // Trigger webhook to the merchant if it is a terminal status
                // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
                if let Some(outgoing_event_type) = event_type {
                    let payout_response =
                        payouts::response_handler(state, &platform, &payout_data).await?;

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
            }
            true => {
                let connector = payout_data
                    .payout_attempt
                    .connector
                    .clone()
                    .ok_or(errors::ProcessTrackerError::MissingRequiredField)?;

                let is_last_retry =
                    retry_sync_task(db, connector, merchant_id.clone(), process).await?;

                // If the payment status is still processing and there is no connector transaction_id
                // then change the payment status to failed if all retries exceeded
                if is_last_retry
                    && payout_data.payout_attempt.status == enums::PayoutStatus::Pending
                    && payout_data
                        .payout_attempt
                        .connector_payout_id
                        .as_ref()
                        .is_none()
                {
                    let payouts_update = PayoutsUpdate::StatusUpdate {
                        status: enums::PayoutStatus::Failed,
                    };

                    payout_data.payouts = db
                        .update_payout(
                            &payout_data.payouts,
                            payouts_update,
                            &payout_data.payout_attempt,
                            platform.get_processor().get_account().storage_scheme,
                        )
                        .await
                        .map_err(errors::ProcessTrackerError::EStorageError)?;

                    let payout_attempt_update = PayoutAttemptUpdate::StatusUpdate {
                        connector_payout_id: payout_data.payout_attempt.connector_payout_id.clone(),
                        status: enums::PayoutStatus::Failed,
                        error_message: Some(
                            REQUEST_TIMEOUT_ERROR_MESSAGE_FROM_PAYOUT_SYNC.to_string(),
                        ),
                        error_code: None,
                        is_eligible: payout_data.payout_attempt.is_eligible,
                        unified_code: None,
                        unified_message: None,
                        payout_connector_metadata: payout_data
                            .payout_attempt
                            .payout_connector_metadata
                            .clone(),
                    };

                    payout_data.payout_attempt = db
                        .update_payout_attempt(
                            &payout_data.payout_attempt,
                            payout_attempt_update,
                            &payout_data.payouts,
                            platform.get_processor().get_account().storage_scheme,
                        )
                        .await
                        .map_err(errors::ProcessTrackerError::EStorageError)?;

                    let event_type: Option<enums::EventType> =
                        payout_data.payout_attempt.status.into();

                    let business_profile = db
                        .find_business_profile_by_profile_id(&key_store, &payout_data.profile_id)
                        .await
                        .map_err(errors::ProcessTrackerError::EStorageError)?;

                    // Trigger the outgoing webhook to notify the merchant about failed payment
                    // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
                    if let Some(outgoing_event_type) = event_type {
                        let payout_response =
                            payouts::response_handler(state, &platform, &payout_data).await?;

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
                }
            }
        };

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
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: common_utils::errors::CustomResult<
        process_data::ConnectorPTMapping,
        core_errors::StorageError,
    > = db
        .find_config_by_key(&format!("payout_tracker_mapping_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("ConnectorPTMapping")
                .change_context(core_errors::StorageError::DeserializationFailed)
        });
    let mapping = match mapping {
        Ok(x) => x,
        Err(error) => {
            router_env::logger::info!(?error, "Redis Mapping Error");
            process_data::ConnectorPTMapping::default()
        }
    };

    let time_delta = scheduler_utils::get_schedule_time(mapping, merchant_id, retry_count);

    Ok(scheduler_utils::get_time_from_delta(time_delta))
}

/// Schedule the task for retry
///
/// Returns bool which indicates whether this was the last retry or not
pub async fn retry_sync_task(
    db: &dyn StorageInterface,
    connector: String,
    merchant_id: common_utils::id_type::MerchantId,
    pt: storage::ProcessTracker,
) -> Result<bool, errors::ProcessTrackerError> {
    let schedule_time =
        get_payout_sync_process_schedule_time(db, &connector, &merchant_id, pt.retry_count + 1)
            .await?;

    match schedule_time {
        Some(s_time) => {
            db.as_scheduler().retry_process(pt, s_time).await?;
            Ok(false)
        }
        None => {
            db.as_scheduler()
                .finish_process_with_business_status(pt, business_status::RETRIES_EXCEEDED)
                .await?;
            Ok(true)
        }
    }
}
