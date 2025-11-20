use std::collections::HashMap;

use api_models::{
    webhook_events::{OutgoingWebhookRequestContent, OutgoingWebhookResponseContent},
    webhooks,
};
use common_utils::{
    ext_traits::{Encode, StringExt},
    request::RequestContent,
    type_name,
    types::keymanager::Identifier,
};
use diesel_models::process_tracker::business_status;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use hyperswitch_interfaces::consts;
use masking::{ExposeInterface, Mask, PeekInterface, Secret};
use router_env::{
    instrument,
    tracing::{self, Instrument},
};

use super::{types, utils, MERCHANT_ID};
#[cfg(feature = "stripe")]
use crate::compatibility::stripe::webhooks as stripe_webhooks;
use crate::{
    core::{
        errors::{self, CustomResult},
        metrics,
    },
    db::StorageInterface,
    events::outgoing_webhook_logs::{
        OutgoingWebhookEvent, OutgoingWebhookEventContent, OutgoingWebhookEventMetric,
    },
    logger,
    routes::{app::SessionStateInfo, SessionState},
    services,
    types::{
        api,
        domain::{self},
        storage::{self, enums},
        transformers::ForeignFrom,
    },
    utils::{OptionExt, ValueExt},
    workflows::outgoing_webhook_retry,
};

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn create_event_and_trigger_outgoing_webhook(
    state: SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    event_type: enums::EventType,
    event_class: enums::EventClass,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    content: api::OutgoingWebhookContent,
    primary_object_created_at: Option<time::PrimitiveDateTime>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let delivery_attempt = enums::WebhookDeliveryAttempt::InitialAttempt;
    let idempotent_event_id =
        utils::get_idempotent_event_id(&primary_object_id, event_type, delivery_attempt)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to generate idempotent event ID")?;
    let webhook_url_result = get_webhook_url_from_business_profile(&business_profile);

    if !state.conf.webhooks.outgoing_enabled
        || webhook_url_result.is_err()
        || webhook_url_result.as_ref().is_ok_and(String::is_empty)
    {
        logger::debug!(
            business_profile_id=?business_profile.get_id(),
            %idempotent_event_id,
            "Outgoing webhooks are disabled in application configuration, or merchant webhook URL \
             could not be obtained; skipping outgoing webhooks for event"
        );
        return Ok(());
    }

    let event_id = utils::generate_event_id();
    let merchant_id = business_profile.merchant_id.clone();
    let now = common_utils::date_time::now();

    let outgoing_webhook = api::OutgoingWebhook {
        merchant_id: merchant_id.clone(),
        event_id: event_id.clone(),
        event_type,
        content: content.clone(),
        timestamp: now,
    };

    let request_content =
        get_outgoing_webhook_request(&platform, outgoing_webhook, &business_profile)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to construct outgoing webhook request content")?;

    let event_metadata = storage::EventMetadata::foreign_from(&content);
    let key_manager_state = &(&state).into();
    let new_event = domain::Event {
        event_id: event_id.clone(),
        event_type,
        event_class,
        is_webhook_notified: false,
        primary_object_id,
        primary_object_type,
        created_at: now,
        merchant_id: Some(business_profile.merchant_id.clone()),
        business_profile_id: Some(business_profile.get_id().to_owned()),
        primary_object_created_at,
        idempotent_event_id: Some(idempotent_event_id.clone()),
        initial_attempt_id: Some(event_id.clone()),
        request: Some(
            crypto_operation(
                key_manager_state,
                type_name!(domain::Event),
                CryptoOperation::Encrypt(
                    request_content
                        .encode_to_string_of_json()
                        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                        .attach_printable("Failed to encode outgoing webhook request content")
                        .map(Secret::new)?,
                ),
                Identifier::Merchant(platform.get_processor().get_key_store().merchant_id.clone()),
                platform
                    .get_processor()
                    .get_key_store()
                    .key
                    .get_inner()
                    .peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to encrypt outgoing webhook request content")?,
        ),
        response: None,
        delivery_attempt: Some(delivery_attempt),
        metadata: Some(event_metadata),
        is_overall_delivery_successful: Some(false),
    };

    let lock_value = utils::perform_redis_lock(
        &state,
        &idempotent_event_id,
        platform.get_processor().get_account().get_id().to_owned(),
    )
    .await?;

    if lock_value.is_none() {
        return Ok(());
    }

    if (state
        .store
        .find_event_by_merchant_id_idempotent_event_id(
            &merchant_id,
            &idempotent_event_id,
            platform.get_processor().get_key_store(),
        )
        .await)
        .is_ok()
    {
        logger::debug!(
            "Event with idempotent ID `{idempotent_event_id}` already exists in the database"
        );
        utils::free_redis_lock(
            &state,
            &idempotent_event_id,
            platform.get_processor().get_account().get_id().to_owned(),
            lock_value,
        )
        .await?;
        return Ok(());
    }

    let event_insert_result = state
        .store
        .insert_event(new_event, platform.get_processor().get_key_store())
        .await;

    let event = match event_insert_result {
        Ok(event) => Ok(event),
        Err(error) => {
            logger::error!(event_insertion_failure=?error);
            Err(error
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("Failed to insert event in events table"))
        }
    }?;

    utils::free_redis_lock(
        &state,
        &idempotent_event_id,
        platform.get_processor().get_account().get_id().to_owned(),
        lock_value,
    )
    .await?;

    let process_tracker = add_outgoing_webhook_retry_task_to_process_tracker(
        &*state.store,
        &business_profile,
        &event,
    )
    .await
    .inspect_err(|error| {
        logger::error!(
            ?error,
            "Failed to add outgoing webhook retry task to process tracker"
        );
    })
    .ok();

    let cloned_key_store = platform.get_processor().get_key_store().clone();
    // Using a tokio spawn here and not arbiter because not all caller of this function
    // may have an actix arbiter
    tokio::spawn(
        async move {
            Box::pin(trigger_webhook_and_raise_event(
                state,
                business_profile,
                &cloned_key_store,
                event,
                request_content,
                delivery_attempt,
                Some(content),
                process_tracker,
            ))
            .await;
        }
        .in_current_span(),
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn trigger_webhook_and_raise_event(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: &domain::MerchantKeyStore,
    event: domain::Event,
    request_content: OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    content: Option<api::OutgoingWebhookContent>,
    process_tracker: Option<storage::ProcessTracker>,
) {
    logger::debug!(
        event_id=%event.event_id,
        idempotent_event_id=?event.idempotent_event_id,
        initial_attempt_id=?event.initial_attempt_id,
        "Attempting to send webhook"
    );

    let merchant_id = business_profile.merchant_id.clone();
    let trigger_webhook_result = trigger_webhook_to_merchant(
        state.clone(),
        business_profile,
        merchant_key_store,
        event.clone(),
        request_content,
        delivery_attempt,
        process_tracker,
    )
    .await;

    let _ = raise_webhooks_analytics_event(
        state,
        trigger_webhook_result,
        content,
        merchant_id,
        event,
        merchant_key_store,
    )
    .await;
}

async fn trigger_webhook_to_merchant(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: &domain::MerchantKeyStore,
    event: domain::Event,
    request_content: OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    process_tracker: Option<storage::ProcessTracker>,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let webhook_url = match (
        get_webhook_url_from_business_profile(&business_profile),
        process_tracker.clone(),
    ) {
        (Ok(webhook_url), _) => Ok(webhook_url),
        (Err(error), Some(process_tracker)) => {
            if !error
                .current_context()
                .is_webhook_delivery_retryable_error()
            {
                logger::debug!("Failed to obtain merchant webhook URL, aborting retries");
                state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(process_tracker, business_status::FAILURE)
                    .await
                    .change_context(
                        errors::WebhooksFlowError::OutgoingWebhookProcessTrackerTaskUpdateFailed,
                    )?;
            }
            Err(error)
        }
        (Err(error), None) => Err(error),
    }?;

    let event_id = event.event_id;

    let headers = request_content
        .headers
        .into_iter()
        .map(|(name, value)| (name, value.into_masked()))
        .collect();
    let request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&webhook_url)
        .attach_default_headers()
        .headers(headers)
        .set_body(RequestContent::RawBytes(
            request_content.body.expose().into_bytes(),
        ))
        .build();

    let response = state
        .api_client
        .send_request(&state, request, None, false)
        .await;

    metrics::WEBHOOK_OUTGOING_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, business_profile.merchant_id.clone())),
    );
    logger::debug!(outgoing_webhook_response=?response);

    match delivery_attempt {
        enums::WebhookDeliveryAttempt::InitialAttempt => match response {
            Err(client_error) => {
                api_client_error_handler(
                    state.clone(),
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event_id,
                    client_error,
                    delivery_attempt,
                    ScheduleWebhookRetry::NoSchedule,
                )
                .await?
            }
            Ok(response) => {
                let status_code = response.status();
                let updated_event = update_event_in_storage(
                    state.clone(),
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event_id,
                    response,
                )
                .await?;

                if status_code.is_success() {
                    update_overall_delivery_status_in_storage(
                        state.clone(),
                        merchant_key_store.clone(),
                        &business_profile.merchant_id,
                        updated_event,
                    )
                    .await?;

                    success_response_handler(
                        state.clone(),
                        &business_profile.merchant_id,
                        process_tracker,
                        business_status::INITIAL_DELIVERY_ATTEMPT_SUCCESSFUL,
                    )
                    .await?;
                } else {
                    error_response_handler(
                        state.clone(),
                        &business_profile.merchant_id,
                        delivery_attempt,
                        status_code.as_u16(),
                        "Ignoring error when sending webhook to merchant",
                        ScheduleWebhookRetry::NoSchedule,
                    )
                    .await?;
                }
            }
        },
        enums::WebhookDeliveryAttempt::AutomaticRetry => {
            let process_tracker = process_tracker
                .get_required_value("process_tracker")
                .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)
                .attach_printable("`process_tracker` is unavailable in automatic retry flow")?;
            match response {
                Err(client_error) => {
                    api_client_error_handler(
                        state.clone(),
                        merchant_key_store.clone(),
                        &business_profile.merchant_id,
                        &event_id,
                        client_error,
                        delivery_attempt,
                        ScheduleWebhookRetry::WithProcessTracker(Box::new(process_tracker)),
                    )
                    .await?;
                }
                Ok(response) => {
                    let status_code = response.status();
                    let updated_event = update_event_in_storage(
                        state.clone(),
                        merchant_key_store.clone(),
                        &business_profile.merchant_id,
                        &event_id,
                        response,
                    )
                    .await?;

                    if status_code.is_success() {
                        update_overall_delivery_status_in_storage(
                            state.clone(),
                            merchant_key_store.clone(),
                            &business_profile.merchant_id,
                            updated_event,
                        )
                        .await?;

                        success_response_handler(
                            state.clone(),
                            &business_profile.merchant_id,
                            Some(process_tracker),
                            "COMPLETED_BY_PT",
                        )
                        .await?;
                    } else {
                        error_response_handler(
                            state.clone(),
                            &business_profile.merchant_id,
                            delivery_attempt,
                            status_code.as_u16(),
                            "An error occurred when sending webhook to merchant",
                            ScheduleWebhookRetry::WithProcessTracker(Box::new(process_tracker)),
                        )
                        .await?;
                    }
                }
            }
        }
        enums::WebhookDeliveryAttempt::ManualRetry => match response {
            Err(client_error) => {
                api_client_error_handler(
                    state.clone(),
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event_id,
                    client_error,
                    delivery_attempt,
                    ScheduleWebhookRetry::NoSchedule,
                )
                .await?
            }
            Ok(response) => {
                let status_code = response.status();
                let _updated_event = update_event_in_storage(
                    state.clone(),
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event_id,
                    response,
                )
                .await?;

                if status_code.is_success() {
                    increment_webhook_outgoing_received_count(&business_profile.merchant_id);
                } else {
                    error_response_handler(
                        state,
                        &business_profile.merchant_id,
                        delivery_attempt,
                        status_code.as_u16(),
                        "Ignoring error when sending webhook to merchant",
                        ScheduleWebhookRetry::NoSchedule,
                    )
                    .await?;
                }
            }
        },
    }

    Ok(())
}

async fn raise_webhooks_analytics_event(
    state: SessionState,
    trigger_webhook_result: CustomResult<(), errors::WebhooksFlowError>,
    content: Option<api::OutgoingWebhookContent>,
    merchant_id: common_utils::id_type::MerchantId,
    event: domain::Event,
    merchant_key_store: &domain::MerchantKeyStore,
) {
    let event_id = event.event_id;

    let error = if let Err(error) = trigger_webhook_result {
        logger::error!(?error, "Failed to send webhook to merchant");

        serde_json::to_value(error.current_context())
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .inspect_err(|error| {
                logger::error!(?error, "Failed to serialize outgoing webhook error as JSON");
            })
            .ok()
    } else {
        None
    };

    let outgoing_webhook_event_content = content
        .as_ref()
        .and_then(api::OutgoingWebhookContent::get_outgoing_webhook_event_content)
        .or_else(|| get_outgoing_webhook_event_content_from_event_metadata(event.metadata));

    // Fetch updated_event from db
    let updated_event = state
        .store
        .find_event_by_merchant_id_event_id(&merchant_id, &event_id, merchant_key_store)
        .await
        .attach_printable_lazy(|| format!("event not found for id: {}", &event_id))
        .map_err(|error| {
            logger::error!(?error);
            error
        })
        .ok();

    // Get status_code from webhook response
    let status_code = updated_event.and_then(|updated_event| {
        let webhook_response: Option<OutgoingWebhookResponseContent> =
            updated_event.response.and_then(|res| {
                res.peek()
                    .parse_struct("OutgoingWebhookResponseContent")
                    .map_err(|error| {
                        logger::error!(?error, "Error deserializing webhook response");
                        error
                    })
                    .ok()
            });
        webhook_response.and_then(|res| res.status_code)
    });

    let webhook_event = OutgoingWebhookEvent::new(
        state.tenant.tenant_id.clone(),
        merchant_id,
        event_id,
        event.event_type,
        outgoing_webhook_event_content,
        error,
        event.initial_attempt_id,
        status_code,
        event.delivery_attempt,
    );
    state.event_handler().log_event(&webhook_event);
}

pub(crate) async fn add_outgoing_webhook_retry_task_to_process_tracker(
    db: &dyn StorageInterface,
    business_profile: &domain::Profile,
    event: &domain::Event,
) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
    let schedule_time = outgoing_webhook_retry::get_webhook_delivery_retry_schedule_time(
        db,
        &business_profile.merchant_id,
        0,
    )
    .await
    .ok_or(errors::StorageError::ValueNotFound(
        "Process tracker schedule time".into(), // Can raise a better error here
    ))
    .attach_printable("Failed to obtain initial process tracker schedule time")?;

    let tracking_data = types::OutgoingWebhookTrackingData {
        merchant_id: business_profile.merchant_id.clone(),
        business_profile_id: business_profile.get_id().to_owned(),
        event_type: event.event_type,
        event_class: event.event_class,
        primary_object_id: event.primary_object_id.clone(),
        primary_object_type: event.primary_object_type,
        initial_attempt_id: event.initial_attempt_id.clone(),
    };

    let runner = storage::ProcessTrackerRunner::OutgoingWebhookRetryWorkflow;
    let task = "OUTGOING_WEBHOOK_RETRY";
    let tag = ["OUTGOING_WEBHOOKS"];
    let process_tracker_id = scheduler::utils::get_process_tracker_id(
        runner,
        task,
        &event.event_id,
        &business_profile.merchant_id,
    );
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
    )
    .map_err(errors::StorageError::from)?;

    let attributes = router_env::metric_attributes!(("flow", "OutgoingWebhookRetry"));
    match db.insert_process(process_tracker_entry).await {
        Ok(process_tracker) => {
            crate::routes::metrics::TASKS_ADDED_COUNT.add(1, attributes);
            Ok(process_tracker)
        }
        Err(error) => {
            crate::routes::metrics::TASK_ADDITION_FAILURES_COUNT.add(1, attributes);
            Err(error)
        }
    }
}

fn get_webhook_url_from_business_profile(
    business_profile: &domain::Profile,
) -> CustomResult<String, errors::WebhooksFlowError> {
    let webhook_details = business_profile
        .webhook_details
        .clone()
        .get_required_value("webhook_details")
        .change_context(errors::WebhooksFlowError::MerchantWebhookDetailsNotFound)?;

    webhook_details
        .webhook_url
        .get_required_value("webhook_url")
        .change_context(errors::WebhooksFlowError::MerchantWebhookUrlNotConfigured)
        .map(ExposeInterface::expose)
}

pub(crate) fn get_outgoing_webhook_request(
    platform: &domain::Platform,
    outgoing_webhook: api::OutgoingWebhook,
    business_profile: &domain::Profile,
) -> CustomResult<OutgoingWebhookRequestContent, errors::WebhooksFlowError> {
    #[inline]
    fn get_outgoing_webhook_request_inner<WebhookType: types::OutgoingWebhookType>(
        outgoing_webhook: api::OutgoingWebhook,
        business_profile: &domain::Profile,
    ) -> CustomResult<OutgoingWebhookRequestContent, errors::WebhooksFlowError> {
        let mut headers = vec![
            (
                reqwest::header::CONTENT_TYPE.to_string(),
                mime::APPLICATION_JSON.essence_str().into(),
            ),
            (
                reqwest::header::USER_AGENT.to_string(),
                consts::USER_AGENT.to_string().into(),
            ),
        ];

        let transformed_outgoing_webhook = WebhookType::from(outgoing_webhook);
        let payment_response_hash_key = business_profile.payment_response_hash_key.clone();
        let custom_headers = business_profile
            .outgoing_webhook_custom_http_headers
            .clone()
            .map(|headers| {
                headers
                    .into_inner()
                    .expose()
                    .parse_value::<HashMap<String, String>>("HashMap<String,String>")
                    .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
                    .attach_printable("Failed to deserialize outgoing webhook custom HTTP headers")
            })
            .transpose()?;
        if let Some(ref map) = custom_headers {
            headers.extend(
                map.iter()
                    .map(|(key, value)| (key.clone(), value.clone().into_masked())),
            );
        };
        let outgoing_webhooks_signature = transformed_outgoing_webhook
            .get_outgoing_webhooks_signature(payment_response_hash_key)?;

        if let Some(signature) = outgoing_webhooks_signature.signature {
            WebhookType::add_webhook_header(&mut headers, signature)
        }

        Ok(OutgoingWebhookRequestContent {
            body: outgoing_webhooks_signature.payload,
            headers: headers
                .into_iter()
                .map(|(name, value)| (name, Secret::new(value.into_inner())))
                .collect(),
        })
    }

    match platform
        .get_processor()
        .get_account()
        .get_compatible_connector()
    {
        #[cfg(feature = "stripe")]
        Some(api_models::enums::Connector::Stripe) => get_outgoing_webhook_request_inner::<
            stripe_webhooks::StripeOutgoingWebhook,
        >(outgoing_webhook, business_profile),
        _ => get_outgoing_webhook_request_inner::<webhooks::OutgoingWebhook>(
            outgoing_webhook,
            business_profile,
        ),
    }
}

#[derive(Debug)]
enum ScheduleWebhookRetry {
    WithProcessTracker(Box<storage::ProcessTracker>),
    NoSchedule,
}

async fn update_event_if_client_error(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    merchant_id: &common_utils::id_type::MerchantId,
    event_id: &str,
    error_message: String,
) -> CustomResult<domain::Event, errors::WebhooksFlowError> {
    let is_webhook_notified = false;
    let key_manager_state = &(&state).into();
    let response_to_store = OutgoingWebhookResponseContent {
        body: None,
        headers: None,
        status_code: None,
        error_message: Some(error_message),
    };

    let event_update = domain::EventUpdate::UpdateResponse {
        is_webhook_notified,
        response: Some(
            crypto_operation(
                key_manager_state,
                type_name!(domain::Event),
                CryptoOperation::Encrypt(
                    response_to_store
                        .encode_to_string_of_json()
                        .change_context(
                            errors::WebhooksFlowError::OutgoingWebhookResponseEncodingFailed,
                        )
                        .map(Secret::new)?,
                ),
                Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                merchant_key_store.key.get_inner().peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
            .attach_printable("Failed to encrypt outgoing webhook response content")?,
        ),
    };

    state
        .store
        .update_event_by_merchant_id_event_id(
            merchant_id,
            event_id,
            event_update,
            &merchant_key_store,
        )
        .await
        .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
}

async fn api_client_error_handler(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    merchant_id: &common_utils::id_type::MerchantId,
    event_id: &str,
    client_error: error_stack::Report<errors::ApiClientError>,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    schedule_webhook_retry: ScheduleWebhookRetry,
) -> CustomResult<(), errors::WebhooksFlowError> {
    // Not including detailed error message in response information since it contains too
    // much of diagnostic information to be exposed to the merchant.
    update_event_if_client_error(
        state.clone(),
        merchant_key_store,
        merchant_id,
        event_id,
        "Unable to send request to merchant server".to_string(),
    )
    .await?;

    let error = client_error.change_context(errors::WebhooksFlowError::CallToMerchantFailed);
    logger::error!(
        ?error,
        ?delivery_attempt,
        "An error occurred when sending webhook to merchant"
    );

    if let ScheduleWebhookRetry::WithProcessTracker(process_tracker) = schedule_webhook_retry {
        // Schedule a retry attempt for webhook delivery
        outgoing_webhook_retry::retry_webhook_delivery_task(
            &*state.store,
            merchant_id,
            *process_tracker,
        )
        .await
        .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)?;
    }

    Err(error)
}

async fn update_event_in_storage(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    merchant_id: &common_utils::id_type::MerchantId,
    event_id: &str,
    response: reqwest::Response,
) -> CustomResult<domain::Event, errors::WebhooksFlowError> {
    let status_code = response.status();
    let is_webhook_notified = status_code.is_success();
    let key_manager_state = &(&state).into();
    let response_headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                value
                    .to_str()
                    .map(|s| Secret::from(String::from(s)))
                    .unwrap_or_else(|error| {
                        logger::warn!(
                            "Response header {} contains non-UTF-8 characters: {error:?}",
                            name.as_str()
                        );
                        Secret::from(String::from("Non-UTF-8 header value"))
                    }),
            )
        })
        .collect::<Vec<_>>();
    let response_body = response
        .text()
        .await
        .map(Secret::from)
        .unwrap_or_else(|error| {
            logger::warn!("Response contains non-UTF-8 characters: {error:?}");
            Secret::from(String::from("Non-UTF-8 response body"))
        });
    let response_to_store = OutgoingWebhookResponseContent {
        body: Some(response_body),
        headers: Some(response_headers),
        status_code: Some(status_code.as_u16()),
        error_message: None,
    };

    let event_update = domain::EventUpdate::UpdateResponse {
        is_webhook_notified,
        response: Some(
            crypto_operation(
                key_manager_state,
                type_name!(domain::Event),
                CryptoOperation::Encrypt(
                    response_to_store
                        .encode_to_string_of_json()
                        .change_context(
                            errors::WebhooksFlowError::OutgoingWebhookResponseEncodingFailed,
                        )
                        .map(Secret::new)?,
                ),
                Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                merchant_key_store.key.get_inner().peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
            .attach_printable("Failed to encrypt outgoing webhook response content")?,
        ),
    };
    state
        .store
        .update_event_by_merchant_id_event_id(
            merchant_id,
            event_id,
            event_update,
            &merchant_key_store,
        )
        .await
        .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
}

async fn update_overall_delivery_status_in_storage(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    merchant_id: &common_utils::id_type::MerchantId,
    updated_event: domain::Event,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let update_overall_delivery_status = domain::EventUpdate::OverallDeliveryStatusUpdate {
        is_overall_delivery_successful: true,
    };

    let initial_attempt_id = updated_event.initial_attempt_id.as_ref();
    let delivery_attempt = updated_event.delivery_attempt;

    if let Some((
        initial_attempt_id,
        enums::WebhookDeliveryAttempt::InitialAttempt
        | enums::WebhookDeliveryAttempt::AutomaticRetry,
    )) = initial_attempt_id.zip(delivery_attempt)
    {
        state
            .store
            .update_event_by_merchant_id_event_id(
                merchant_id,
                initial_attempt_id.as_str(),
                update_overall_delivery_status,
                &merchant_key_store,
            )
            .await
            .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
            .attach_printable("Failed to update initial delivery attempt")?;
    }

    Ok(())
}

fn increment_webhook_outgoing_received_count(merchant_id: &common_utils::id_type::MerchantId) {
    metrics::WEBHOOK_OUTGOING_RECEIVED_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, merchant_id.clone())),
    )
}

async fn success_response_handler(
    state: SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    process_tracker: Option<storage::ProcessTracker>,
    business_status: &'static str,
) -> CustomResult<(), errors::WebhooksFlowError> {
    increment_webhook_outgoing_received_count(merchant_id);

    match process_tracker {
        Some(process_tracker) => state
            .store
            .as_scheduler()
            .finish_process_with_business_status(process_tracker, business_status)
            .await
            .change_context(
                errors::WebhooksFlowError::OutgoingWebhookProcessTrackerTaskUpdateFailed,
            ),
        None => Ok(()),
    }
}

async fn error_response_handler(
    state: SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    status_code: u16,
    log_message: &'static str,
    schedule_webhook_retry: ScheduleWebhookRetry,
) -> CustomResult<(), errors::WebhooksFlowError> {
    metrics::WEBHOOK_OUTGOING_NOT_RECEIVED_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, merchant_id.clone())),
    );

    let error = report!(errors::WebhooksFlowError::NotReceivedByMerchant);
    logger::warn!(?error, ?delivery_attempt, status_code, %log_message);

    if let ScheduleWebhookRetry::WithProcessTracker(process_tracker) = schedule_webhook_retry {
        // Schedule a retry attempt for webhook delivery
        outgoing_webhook_retry::retry_webhook_delivery_task(
            &*state.store,
            merchant_id,
            *process_tracker,
        )
        .await
        .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)?;
    }

    Err(error)
}

impl ForeignFrom<&api::OutgoingWebhookContent> for storage::EventMetadata {
    fn foreign_from(content: &api::OutgoingWebhookContent) -> Self {
        match content {
            webhooks::OutgoingWebhookContent::PaymentDetails(payments_response) => Self::Payment {
                payment_id: payments_response.payment_id.clone(),
            },
            webhooks::OutgoingWebhookContent::RefundDetails(refund_response) => Self::Refund {
                payment_id: refund_response.payment_id.clone(),
                refund_id: refund_response.refund_id.clone(),
            },
            webhooks::OutgoingWebhookContent::DisputeDetails(dispute_response) => Self::Dispute {
                payment_id: dispute_response.payment_id.clone(),
                attempt_id: dispute_response.attempt_id.clone(),
                dispute_id: dispute_response.dispute_id.clone(),
            },
            webhooks::OutgoingWebhookContent::MandateDetails(mandate_response) => Self::Mandate {
                payment_method_id: mandate_response.payment_method_id.clone(),
                mandate_id: mandate_response.mandate_id.clone(),
            },
            #[cfg(feature = "payouts")]
            webhooks::OutgoingWebhookContent::PayoutDetails(payout_response) => Self::Payout {
                payout_id: payout_response.payout_id.clone(),
            },
            webhooks::OutgoingWebhookContent::SubscriptionDetails(subscription) => {
                Self::Subscription {
                    subscription_id: subscription.id.clone(),
                    invoice_id: subscription.get_optional_invoice_id(),
                    payment_id: subscription.get_optional_payment_id(),
                }
            }
        }
    }
}

fn get_outgoing_webhook_event_content_from_event_metadata(
    event_metadata: Option<storage::EventMetadata>,
) -> Option<OutgoingWebhookEventContent> {
    event_metadata.map(|metadata| match metadata {
        diesel_models::EventMetadata::Payment { payment_id } => {
            OutgoingWebhookEventContent::Payment {
                payment_id,
                content: serde_json::Value::Null,
            }
        }
        diesel_models::EventMetadata::Payout { payout_id } => OutgoingWebhookEventContent::Payout {
            payout_id,
            content: serde_json::Value::Null,
        },
        diesel_models::EventMetadata::Refund {
            payment_id,
            refund_id,
        } => OutgoingWebhookEventContent::Refund {
            payment_id,
            refund_id,
            content: serde_json::Value::Null,
        },
        diesel_models::EventMetadata::Dispute {
            payment_id,
            attempt_id,
            dispute_id,
        } => OutgoingWebhookEventContent::Dispute {
            payment_id,
            attempt_id,
            dispute_id,
            content: serde_json::Value::Null,
        },
        diesel_models::EventMetadata::Mandate {
            payment_method_id,
            mandate_id,
        } => OutgoingWebhookEventContent::Mandate {
            payment_method_id,
            mandate_id,
            content: serde_json::Value::Null,
        },
        diesel_models::EventMetadata::Subscription {
            subscription_id,
            invoice_id,
            payment_id,
        } => OutgoingWebhookEventContent::Subscription {
            subscription_id,
            invoice_id,
            payment_id,
            content: serde_json::Value::Null,
        },
    })
}
