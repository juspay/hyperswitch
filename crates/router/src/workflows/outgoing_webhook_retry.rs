use api_models::{
    enums::EventType,
    webhooks::{OutgoingWebhook, OutgoingWebhookContent},
};
use common_utils::ext_traits::{StringExt, ValueExt};
use error_stack::ResultExt;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    types::process_data,
    utils as scheduler_utils,
};

use crate::{
    core::webhooks as webhooks_core,
    db::StorageInterface,
    errors, logger,
    routes::AppState,
    types::{storage, transformers::ForeignFrom},
};

pub struct OutgoingWebhookRetryWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for OutgoingWebhookRetryWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data: webhooks_core::types::OutgoingWebhookTrackingData = process
            .tracking_data
            .clone()
            .parse_value("OutgoingWebhookTrackingData")?;

        let db = &*state.store;
        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;
        let merchant_account = db
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
            .await?;
        let business_profile = db
            .find_business_profile_by_profile_id(&tracking_data.business_profile_id)
            .await?;

        let event_id = format!(
            "{}_{}",
            tracking_data.primary_object_id, tracking_data.event_type
        );
        let event = db.find_event_by_event_id(&event_id).await?;

        let (content, event_type) = match tracking_data.event_class {
            diesel_models::enums::EventClass::Payments => {
                use api_models::payments::{
                    HeaderPayload, PaymentIdType, PaymentsResponse, PaymentsRetrieveRequest,
                };

                use crate::{
                    core::{
                        payment_methods::Oss,
                        payments::{payments_core, CallConnectorAction, PaymentStatus},
                    },
                    services::{ApplicationResponse, AuthFlow},
                    types::api::PSync,
                };

                let payment_id = tracking_data.primary_object_id.clone();
                let request = PaymentsRetrieveRequest {
                    resource_id: PaymentIdType::PaymentIntentId(payment_id),
                    merchant_id: Some(tracking_data.merchant_id.clone()),
                    force_sync: false,
                    ..Default::default()
                };

                let payments_response =
                    match payments_core::<PSync, PaymentsResponse, _, _, _, Oss>(
                        state.clone(),
                        merchant_account.clone(),
                        key_store,
                        PaymentStatus,
                        request,
                        AuthFlow::Client,
                        CallConnectorAction::Avoid,
                        None,
                        HeaderPayload::default(),
                    )
                    .await?
                    {
                        ApplicationResponse::Json(payments_response)
                        | ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
                            Ok(payments_response)
                        }
                        ApplicationResponse::StatusOk
                        | ApplicationResponse::TextPlain(_)
                        | ApplicationResponse::JsonForRedirection(_)
                        | ApplicationResponse::Form(_)
                        | ApplicationResponse::PaymentLinkForm(_)
                        | ApplicationResponse::FileData(_) => {
                            Err(errors::ProcessTrackerError::ResourceFetchingFailed {
                                resource_name: tracking_data.primary_object_id.clone(),
                            })
                        }
                    }?;
                let event_type = Option::<EventType>::foreign_from(payments_response.status);
                (
                    OutgoingWebhookContent::PaymentDetails(payments_response),
                    event_type,
                )
            }
            diesel_models::enums::EventClass::Refunds => todo!(),
            diesel_models::enums::EventClass::Disputes => todo!(),
            diesel_models::enums::EventClass::Mandates => todo!(),
        };

        match event_type {
            // Resource status is same as the event type of the current event
            Some(event_type) if event_type == tracking_data.event_type => {
                let outgoing_webhook = OutgoingWebhook {
                    merchant_id: tracking_data.merchant_id.clone(),
                    event_id: event_id.clone(),
                    event_type,
                    content: content.clone(),
                    timestamp: event.created_at,
                };

                webhooks_core::trigger_appropriate_webhook_and_raise_event(
                    state.clone(),
                    merchant_account,
                    business_profile,
                    outgoing_webhook,
                    webhooks_core::types::WebhookDeliveryAttempt::AutomaticRetry,
                    content,
                    event_id,
                    event_type,
                    process,
                )
                .await;
            }
            // Resource status has changed since the event was created, finish task
            _ => {
                logger::warn!(
                    %event_id,
                    "The current status of the resource `{}` (event type: {:?}) and the status of \
                    the resource when the event was created (event type: {}) differ, finishing task",
                    tracking_data.primary_object_id,
                    event_type,
                    tracking_data.event_type
                );
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process.clone(),
                        "RESOURCE_STATUS_MISMATCH".to_string(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

/// Get the schedule time for the specified retry count.
///
/// The schedule time can be configured in configs with this key: `pt_mapping_outgoing_webhooks`.
///
/// ```json
/// {
///   "default_mapping": {
///     "start_after": 60,
///     "frequency": [300],
///     "count": [5]
///   },
///   "custom_merchant_mapping": {
///     "merchant_id1": {
///       "start_after": 30,
///       "frequency": [300],
///       "count": [2]
///     }
///   }
/// }
/// ```
///
/// This configuration value represents:
/// - `default_mapping.start_after`: The first retry attempt should happen after 60 seconds by
///   default.
/// - `default_mapping.frequency` and `count`: The next 5 retries should have an interval of 300
///   seconds between them by default.
/// - `custom_merchant_mapping.merchant_id1`: Merchant-specific retry configuration for merchant
///   with merchant ID `merchant_id1`.
pub(crate) async fn get_webhook_delivery_retry_schedule_time(
    db: &dyn StorageInterface,
    merchant_id: &str,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let key = "pt_mapping_outgoing_webhooks";

    let result = db
        .find_config_by_key(key)
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("OutgoingWebhookRetryProcessTrackerMapping")
                .change_context(errors::StorageError::DeserializationFailed)
        });
    let mapping = result.map_or_else(
        |error| {
            if error.current_context().is_db_not_found() {
                logger::debug!("Outgoing webhooks retry config `{key}` not found, ignoring");
            } else {
                logger::error!(
                    ?error,
                    "Failed to read outgoing webhooks retry config `{key}`"
                );
            }
            process_data::OutgoingWebhookRetryProcessTrackerMapping::default()
        },
        |mapping| {
            logger::debug!(?mapping, "Using custom outgoing webhooks retry config");
            mapping
        },
    );

    let time_delta = scheduler_utils::get_outgoing_webhook_retry_schedule_time(
        mapping,
        merchant_id,
        retry_count,
    );

    scheduler_utils::get_time_from_delta(time_delta)
}

/// Schedule the webhook delivery task for retry
pub(crate) async fn retry_webhook_delivery_task(
    db: &dyn StorageInterface,
    merchant_id: &str,
    process: storage::ProcessTracker,
) -> errors::CustomResult<(), errors::StorageError> {
    let schedule_time =
        get_webhook_delivery_retry_schedule_time(db, merchant_id, process.retry_count + 1).await;

    match schedule_time {
        Some(schedule_time) => {
            db.as_scheduler()
                .retry_process(process, schedule_time)
                .await
        }
        None => {
            db.as_scheduler()
                .finish_process_with_business_status(process, "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}
