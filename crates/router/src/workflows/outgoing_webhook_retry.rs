use api_models::{
    enums::EventType,
    webhook_events::OutgoingWebhookRequestContent,
    webhooks::{OutgoingWebhook, OutgoingWebhookContent},
};
use common_utils::ext_traits::{StringExt, ValueExt};
use error_stack::ResultExt;
use masking::PeekInterface;
use router_env::tracing::{self, instrument};
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    types::process_data,
    utils as scheduler_utils,
};

use crate::{
    core::webhooks::{self as webhooks_core, types::OutgoingWebhookTrackingData},
    db::StorageInterface,
    errors, logger,
    routes::{app::ReqState, AppState},
    types::{domain, storage},
};

pub struct OutgoingWebhookRetryWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for OutgoingWebhookRetryWorkflow {
    #[instrument(skip_all)]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let delivery_attempt = storage::enums::WebhookDeliveryAttempt::AutomaticRetry;
        let tracking_data: OutgoingWebhookTrackingData = process
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
        let business_profile = db
            .find_business_profile_by_profile_id(&tracking_data.business_profile_id)
            .await?;

        let event_id = webhooks_core::utils::generate_event_id();
        let idempotent_event_id = webhooks_core::utils::get_idempotent_event_id(
            &tracking_data.primary_object_id,
            tracking_data.event_type,
            delivery_attempt,
        );

        let initial_event = match &tracking_data.initial_attempt_id {
            Some(initial_attempt_id) => {
                db.find_event_by_merchant_id_event_id(
                    &business_profile.merchant_id,
                    initial_attempt_id,
                    &key_store,
                )
                .await?
            }
            // Tracking data inserted by old version of application, fetch event using old event ID
            // format
            None => {
                let old_event_id = format!(
                    "{}_{}",
                    tracking_data.primary_object_id, tracking_data.event_type
                );
                db.find_event_by_merchant_id_event_id(
                    &business_profile.merchant_id,
                    &old_event_id,
                    &key_store,
                )
                .await?
            }
        };

        let now = common_utils::date_time::now();
        let new_event = domain::Event {
            event_id,
            event_type: initial_event.event_type,
            event_class: initial_event.event_class,
            is_webhook_notified: false,
            primary_object_id: initial_event.primary_object_id,
            primary_object_type: initial_event.primary_object_type,
            created_at: now,
            merchant_id: Some(business_profile.merchant_id.clone()),
            business_profile_id: Some(business_profile.profile_id.clone()),
            primary_object_created_at: initial_event.primary_object_created_at,
            idempotent_event_id: Some(idempotent_event_id),
            initial_attempt_id: Some(initial_event.event_id.clone()),
            request: initial_event.request,
            response: None,
            delivery_attempt: Some(delivery_attempt),
        };

        let event = db
            .insert_event(new_event, &key_store)
            .await
            .map_err(|error| {
                logger::error!(?error, "Failed to insert event in events table");
                error
            })?;

        match &event.request {
            Some(request) => {
                let request_content: OutgoingWebhookRequestContent = request
                    .get_inner()
                    .peek()
                    .parse_struct("OutgoingWebhookRequestContent")?;

                webhooks_core::trigger_webhook_and_raise_event(
                    state.clone(),
                    business_profile,
                    &key_store,
                    event,
                    request_content,
                    storage::enums::WebhookDeliveryAttempt::AutomaticRetry,
                    None,
                    Some(process),
                )
                .await;
            }

            // Event inserted by old version of application, fetch current information about
            // resource
            None => {
                let merchant_account = db
                    .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
                    .await?;

                // TODO: Add request state for the PT flows as well
                let (content, event_type) = get_outgoing_webhook_content_and_event_type(
                    state.clone(),
                    state.get_req_state(),
                    merchant_account.clone(),
                    key_store.clone(),
                    &tracking_data,
                )
                .await?;

                match event_type {
                    // Resource status is same as the event type of the current event
                    Some(event_type) if event_type == tracking_data.event_type => {
                        let outgoing_webhook = OutgoingWebhook {
                            merchant_id: tracking_data.merchant_id.clone(),
                            event_id: event.event_id.clone(),
                            event_type,
                            content: content.clone(),
                            timestamp: event.created_at,
                        };

                        let request_content = webhooks_core::get_outgoing_webhook_request(
                            &merchant_account,
                            outgoing_webhook,
                            business_profile.payment_response_hash_key.as_deref(),
                        )
                        .map_err(|error| {
                            logger::error!(
                                ?error,
                                "Failed to obtain outgoing webhook request content"
                            );
                            errors::ProcessTrackerError::EApiErrorResponse
                        })?;

                        webhooks_core::trigger_webhook_and_raise_event(
                            state.clone(),
                            business_profile,
                            &key_store,
                            event,
                            request_content,
                            storage::enums::WebhookDeliveryAttempt::AutomaticRetry,
                            Some(content),
                            Some(process),
                        )
                        .await;
                    }
                    // Resource status has changed since the event was created, finish task
                    _ => {
                        logger::warn!(
                            %event.event_id,
                            "The current status of the resource `{}` (event type: {:?}) and the status of \
                            the resource when the event was created (event type: {:?}) differ, finishing task",
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
            }
        };

        Ok(())
    }

    #[instrument(skip_all)]
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
#[instrument(skip_all)]
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
#[instrument(skip_all)]
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

#[instrument(skip_all)]
async fn get_outgoing_webhook_content_and_event_type(
    state: AppState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    tracking_data: &OutgoingWebhookTrackingData,
) -> Result<(OutgoingWebhookContent, Option<EventType>), errors::ProcessTrackerError> {
    use api_models::{
        mandates::MandateId,
        payments::{HeaderPayload, PaymentIdType, PaymentsResponse, PaymentsRetrieveRequest},
        refunds::{RefundResponse, RefundsRetrieveRequest},
    };

    use crate::{
        core::{
            disputes::retrieve_dispute,
            mandate::get_mandate,
            payment_methods::Oss,
            payments::{payments_core, CallConnectorAction, PaymentStatus},
            refunds::refund_retrieve_core,
        },
        services::{ApplicationResponse, AuthFlow},
        types::{
            api::{DisputeId, PSync},
            transformers::ForeignFrom,
        },
    };

    match tracking_data.event_class {
        diesel_models::enums::EventClass::Payments => {
            let payment_id = tracking_data.primary_object_id.clone();
            let request = PaymentsRetrieveRequest {
                resource_id: PaymentIdType::PaymentIntentId(payment_id),
                merchant_id: Some(tracking_data.merchant_id.clone()),
                force_sync: false,
                ..Default::default()
            };

            let payments_response =
                match Box::pin(payments_core::<PSync, PaymentsResponse, _, _, _, Oss>(
                    state,
                    req_state,
                    merchant_account,
                    key_store,
                    PaymentStatus,
                    request,
                    AuthFlow::Client,
                    CallConnectorAction::Avoid,
                    None,
                    HeaderPayload::default(),
                ))
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
            logger::debug!(current_resource_status=%payments_response.status);

            Ok((
                OutgoingWebhookContent::PaymentDetails(payments_response),
                event_type,
            ))
        }

        diesel_models::enums::EventClass::Refunds => {
            let refund_id = tracking_data.primary_object_id.clone();
            let request = RefundsRetrieveRequest {
                refund_id,
                force_sync: Some(false),
                merchant_connector_details: None,
            };

            let refund = Box::pin(refund_retrieve_core(
                state,
                merchant_account,
                key_store,
                request,
            ))
            .await?;
            let event_type = Option::<EventType>::foreign_from(refund.refund_status);
            logger::debug!(current_resource_status=%refund.refund_status);
            let refund_response = RefundResponse::foreign_from(refund);

            Ok((
                OutgoingWebhookContent::RefundDetails(refund_response),
                event_type,
            ))
        }

        diesel_models::enums::EventClass::Disputes => {
            let dispute_id = tracking_data.primary_object_id.clone();
            let request = DisputeId { dispute_id };

            let dispute_response =
                match retrieve_dispute(state, merchant_account, request).await? {
                    ApplicationResponse::Json(dispute_response)
                    | ApplicationResponse::JsonWithHeaders((dispute_response, _)) => {
                        Ok(dispute_response)
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
                }
                .map(Box::new)?;
            let event_type = Some(EventType::foreign_from(dispute_response.dispute_status));
            logger::debug!(current_resource_status=%dispute_response.dispute_status);

            Ok((
                OutgoingWebhookContent::DisputeDetails(dispute_response),
                event_type,
            ))
        }

        diesel_models::enums::EventClass::Mandates => {
            let mandate_id = tracking_data.primary_object_id.clone();
            let request = MandateId { mandate_id };

            let mandate_response =
                match get_mandate(state, merchant_account, key_store, request).await? {
                    ApplicationResponse::Json(mandate_response)
                    | ApplicationResponse::JsonWithHeaders((mandate_response, _)) => {
                        Ok(mandate_response)
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
                }
                .map(Box::new)?;
            let event_type = Option::<EventType>::foreign_from(mandate_response.status);
            logger::debug!(current_resource_status=%mandate_response.status);

            Ok((
                OutgoingWebhookContent::MandateDetails(mandate_response),
                event_type,
            ))
        }
    }
}
