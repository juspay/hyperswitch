use std::collections::HashMap;

use api_models::{webhook_events, webhooks};
use common_utils::{ext_traits, request, type_name, types::keymanager};
use diesel_models::process_tracker::business_status;
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use hyperswitch_interfaces::consts;
use router_env::{
    instrument,
    tracing::{self, Instrument},
};

use super::{
    types,
    utils::{self, increment_webhook_outgoing_received_count},
    MERCHANT_ID,
};
use crate::{
    core::{
        errors::{self, CustomResult},
        metrics,
    },
    events::outgoing_webhook_logs,
    logger,
    routes::{app::SessionStateInfo, SessionState},
    services,
    types::{
        api, domain,
        storage::{self, enums},
        transformers::ForeignFrom,
    },
};

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn create_event_and_trigger_outgoing_webhook(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: &domain::MerchantKeyStore,
    event_type: enums::EventType,
    event_class: enums::EventClass,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    content: api::OutgoingWebhookContent,
    primary_object_created_at: time::PrimitiveDateTime,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let delivery_attempt = enums::WebhookDeliveryAttempt::InitialAttempt;
    let idempotent_event_id =
        utils::get_idempotent_event_id(&primary_object_id, event_type, delivery_attempt)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to generate idempotent event ID")?;
    let webhook_url_result = business_profile
        .get_webhook_url_from_profile()
        .change_context(errors::WebhooksFlowError::MerchantWebhookUrlNotConfigured);

    if utils::is_outgoing_webhook_disabled(
        &state,
        &webhook_url_result,
        &business_profile,
        &idempotent_event_id,
    ) {
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

    let request_content = get_outgoing_webhook_request(outgoing_webhook, &business_profile)
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
        primary_object_created_at: Some(primary_object_created_at),
        idempotent_event_id: Some(idempotent_event_id.clone()),
        initial_attempt_id: Some(event_id.clone()),
        request: Some(
            crypto_operation(
                key_manager_state,
                type_name!(domain::Event),
                CryptoOperation::Encrypt(
                    ext_traits::Encode::encode_to_string_of_json(&request_content)
                        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                        .attach_printable("Failed to encode outgoing webhook request content")
                        .map(masking::Secret::new)?,
                ),
                keymanager::Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                masking::PeekInterface::peek(merchant_key_store.key.get_inner()),
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

    let event_insert_result = state
        .store
        .insert_event(new_event, merchant_key_store)
        .await;

    let event = match event_insert_result {
        Ok(event) => Ok(event),
        Err(error) => {
            if error.current_context().is_db_unique_violation() {
                // If the event_id already exists in the database, it indicates that the event for the resource has already been sent, so we skip the flow
                logger::debug!("Event with idempotent ID `{idempotent_event_id}` already exists in the database");
                return Ok(());
            } else {
                logger::error!(event_insertion_failure=?error);
                Err(error
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("Failed to insert event in events table"))
            }
        }
    }?;

    let cloned_key_store = merchant_key_store.clone();
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
    request_content: webhook_events::OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    content: Option<api::OutgoingWebhookContent>,
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
    )
    .await;

    let _ =
        raise_webhooks_analytics_event(state, trigger_webhook_result, content, merchant_id, event)
            .await;
}

async fn trigger_webhook_to_merchant(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: &domain::MerchantKeyStore,
    event: domain::Event,
    request_content: webhook_events::OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
) -> CustomResult<
    (domain::Event, Option<Report<errors::WebhooksFlowError>>),
    errors::WebhooksFlowError,
> {
    let webhook_url = business_profile
        .get_webhook_url_from_profile()
        .change_context(errors::WebhooksFlowError::MerchantWebhookUrlNotConfigured)?;

    let response = build_and_send_request(&state, request_content, webhook_url).await;

    metrics::WEBHOOK_OUTGOING_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, business_profile.merchant_id.clone())),
    );
    logger::debug!(outgoing_webhook_response=?response);

    match response {
        Ok(response) => {
            delivery_attempt
                .handle_success_response(
                    state,
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event.event_id,
                    None,
                    response,
                )
                .await
        }
        Err(client_error) => {
            delivery_attempt
                .handle_error_response(
                    state,
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event.event_id,
                    client_error,
                )
                .await
        }
    }
}

async fn raise_webhooks_analytics_event(
    state: SessionState,
    trigger_webhook_result: CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >,
    content: Option<api::OutgoingWebhookContent>,
    merchant_id: common_utils::id_type::MerchantId,
    fallback_event: domain::Event,
) {
    let (updated_event, optional_error) = match trigger_webhook_result {
        Ok((updated_event, error)) => (updated_event, error),
        Err(error) => (fallback_event, Some(error)),
    };
    let error = optional_error.and_then(|error| {
        logger::error!(?error, "Failed to send webhook to merchant");

        serde_json::to_value(error.current_context())
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .inspect_err(|error| {
                logger::error!(?error, "Failed to serialize outgoing webhook error as JSON");
            })
            .ok()
    });

    let outgoing_webhook_event_content = content
        .as_ref()
        .and_then(
            outgoing_webhook_logs::OutgoingWebhookEventMetric::get_outgoing_webhook_event_content,
        )
        .or_else(|| {
            updated_event
                .metadata
                .map(outgoing_webhook_logs::OutgoingWebhookEventContent::foreign_from)
        });

    // Get status_code from webhook response
    let status_code = {
        let webhook_response: Option<webhook_events::OutgoingWebhookResponseContent> =
            updated_event.response.and_then(|res| {
                ext_traits::StringExt::parse_struct(
                    masking::PeekInterface::peek(res.get_inner()),
                    "OutgoingWebhookResponseContent",
                )
                .map_err(|error| {
                    logger::error!(?error, "Error deserializing webhook response");
                    error
                })
                .ok()
            });
        webhook_response.and_then(|res| res.status_code)
    };

    let webhook_event = outgoing_webhook_logs::OutgoingWebhookEvent::new(
        state.tenant.tenant_id.clone(),
        merchant_id,
        updated_event.event_id,
        updated_event.event_type,
        outgoing_webhook_event_content,
        error,
        updated_event.initial_attempt_id,
        status_code,
        updated_event.delivery_attempt,
    );
    state.event_handler().log_event(&webhook_event);
}

pub(crate) fn get_outgoing_webhook_request(
    outgoing_webhook: api::OutgoingWebhook,
    business_profile: &domain::Profile,
) -> CustomResult<webhook_events::OutgoingWebhookRequestContent, errors::WebhooksFlowError> {
    #[inline]
    fn get_outgoing_webhook_request_inner<WebhookType: types::OutgoingWebhookType>(
        outgoing_webhook: api::OutgoingWebhook,
        business_profile: &domain::Profile,
    ) -> CustomResult<webhook_events::OutgoingWebhookRequestContent, errors::WebhooksFlowError>
    {
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
                ext_traits::ValueExt::parse_value::<HashMap<String, String>>(
                    masking::ExposeInterface::expose(headers.into_inner()),
                    "HashMap<String,String>",
                )
                .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
                .attach_printable("Failed to deserialize outgoing webhook custom HTTP headers")
            })
            .transpose()?;
        if let Some(ref map) = custom_headers {
            headers.extend(
                map.iter()
                    .map(|(key, value)| (key.clone(), masking::Mask::into_masked(value.clone()))),
            );
        };
        let outgoing_webhooks_signature = transformed_outgoing_webhook
            .get_outgoing_webhooks_signature(payment_response_hash_key)?;

        if let Some(signature) = outgoing_webhooks_signature.signature {
            WebhookType::add_webhook_header(&mut headers, signature)
        }

        Ok(webhook_events::OutgoingWebhookRequestContent {
            body: outgoing_webhooks_signature.payload,
            headers: headers
                .into_iter()
                .map(|(name, value)| (name, masking::Secret::new(value.into_inner())))
                .collect(),
        })
    }

    get_outgoing_webhook_request_inner::<webhooks::OutgoingWebhook>(
        outgoing_webhook,
        business_profile,
    )
}

async fn build_and_send_request(
    state: &SessionState,
    request_content: webhook_events::OutgoingWebhookRequestContent,
    webhook_url: String,
) -> Result<reqwest::Response, Report<common_enums::ApiClientError>> {
    let headers = request_content
        .headers
        .into_iter()
        .map(|(name, value)| (name, masking::Mask::into_masked(value)))
        .collect();
    let request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&webhook_url)
        .attach_default_headers()
        .headers(headers)
        .set_body(request::RequestContent::RawBytes(
            masking::ExposeInterface::expose(request_content.body).into_bytes(),
        ))
        .build();

    state
        .api_client
        .send_request(state, request, None, false)
        .await
}

async fn api_client_error_handler(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    merchant_id: &common_utils::id_type::MerchantId,
    event_id: &str,
    client_error: Report<errors::ApiClientError>,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    _schedule_webhook_retry: types::ScheduleWebhookRetry,
) -> CustomResult<
    (domain::Event, Option<Report<errors::WebhooksFlowError>>),
    errors::WebhooksFlowError,
> {
    // Not including detailed error message in response information since it contains too
    // much of diagnostic information to be exposed to the merchant.
    let is_webhook_notified = false;
    let response_to_store = webhook_events::OutgoingWebhookResponseContent {
        body: None,
        headers: None,
        status_code: None,
        error_message: Some("Unable to send request to merchant server".to_string()),
    };
    let updated_event = update_event_in_storage(
        state,
        is_webhook_notified,
        response_to_store,
        merchant_key_store,
        merchant_id,
        event_id,
    )
    .await?;

    let error = client_error.change_context(errors::WebhooksFlowError::CallToMerchantFailed);
    logger::error!(
        ?error,
        ?delivery_attempt,
        "An error occurred when sending webhook to merchant"
    );

    //TODO: add outgoing webhook retries support
    // if let ScheduleWebhookRetry::WithProcessTracker(process_tracker) = schedule_webhook_retry {
    //     // Schedule a retry attempt for webhook delivery
    //     outgoing_webhook_retry::retry_webhook_delivery_task(
    //         &*state.store,
    //         merchant_id,
    //         *process_tracker,
    //     )
    //     .await
    //     .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)?;
    // }

    Ok((updated_event, Some(error)))
}

async fn update_event_in_storage(
    state: SessionState,
    is_webhook_notified: bool,
    outgoing_webhook_response: webhook_events::OutgoingWebhookResponseContent,
    merchant_key_store: domain::MerchantKeyStore,
    merchant_id: &common_utils::id_type::MerchantId,
    event_id: &str,
) -> CustomResult<domain::Event, errors::WebhooksFlowError> {
    let key_manager_state = &(&state).into();
    let event_update = domain::EventUpdate::UpdateResponse {
        is_webhook_notified,
        response: Some(
            crypto_operation(
                key_manager_state,
                type_name!(domain::Event),
                CryptoOperation::Encrypt(
                    ext_traits::Encode::encode_to_string_of_json(&outgoing_webhook_response)
                        .change_context(
                            errors::WebhooksFlowError::OutgoingWebhookResponseEncodingFailed,
                        )
                        .map(masking::Secret::new)?,
                ),
                keymanager::Identifier::Merchant(merchant_key_store.merchant_id.clone()),
                masking::PeekInterface::peek(merchant_key_store.key.get_inner()),
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
    updated_event: &domain::Event,
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

async fn handle_successful_delivery(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    updated_event: &domain::Event,
    merchant_id: &common_utils::id_type::MerchantId,
    process_tracker: Option<storage::ProcessTracker>,
    business_status: &'static str,
) -> CustomResult<(), errors::WebhooksFlowError> {
    update_overall_delivery_status_in_storage(
        state.clone(),
        merchant_key_store.clone(),
        merchant_id,
        updated_event,
    )
    .await?;

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

async fn handle_failed_delivery(
    _state: SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    status_code: u16,
    log_message: &'static str,
    _schedule_webhook_retry: types::ScheduleWebhookRetry,
) -> CustomResult<(), errors::WebhooksFlowError> {
    utils::increment_webhook_outgoing_not_received_count(merchant_id);

    let error = report!(errors::WebhooksFlowError::NotReceivedByMerchant);
    logger::warn!(?error, ?delivery_attempt, status_code, %log_message);

    //TODO: add outgoing webhook retries support
    // if let ScheduleWebhookRetry::WithProcessTracker(process_tracker) = schedule_webhook_retry {
    //     // Schedule a retry attempt for webhook delivery
    //     outgoing_webhook_retry::retry_webhook_delivery_task(
    //         &*state.store,
    //         merchant_id,
    //         *process_tracker,
    //     )
    //     .await
    //     .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)?;
    // }

    Err(error)
}

impl ForeignFrom<&api::OutgoingWebhookContent> for storage::EventMetadata {
    fn foreign_from(content: &api::OutgoingWebhookContent) -> Self {
        match content {
            webhooks::OutgoingWebhookContent::PaymentDetails(payments_response) => Self::Payment {
                payment_id: payments_response.id.clone(),
            },
            webhooks::OutgoingWebhookContent::RefundDetails(refund_response) => Self::Refund {
                payment_id: refund_response.payment_id.clone(),
                refund_id: refund_response.id.clone(),
            },
            webhooks::OutgoingWebhookContent::DisputeDetails(dispute_response) => {
                //TODO: add support for dispute outgoing webhook
                todo!()
            }
            webhooks::OutgoingWebhookContent::MandateDetails(mandate_response) => Self::Mandate {
                payment_method_id: mandate_response.payment_method_id.clone(),
                mandate_id: mandate_response.mandate_id.clone(),
            },
            #[cfg(feature = "payouts")]
            webhooks::OutgoingWebhookContent::PayoutDetails(payout_response) => Self::Payout {
                payout_id: payout_response.payout_id.clone(),
            },
        }
    }
}

impl ForeignFrom<storage::EventMetadata> for outgoing_webhook_logs::OutgoingWebhookEventContent {
    fn foreign_from(event_metadata: storage::EventMetadata) -> Self {
        match event_metadata {
            diesel_models::EventMetadata::Payment { payment_id } => Self::Payment {
                payment_id,
                content: serde_json::Value::Null,
            },
            diesel_models::EventMetadata::Payout { payout_id } => Self::Payout {
                payout_id,
                content: serde_json::Value::Null,
            },
            diesel_models::EventMetadata::Refund {
                payment_id,
                refund_id,
            } => Self::Refund {
                payment_id,
                refund_id,
                content: serde_json::Value::Null,
            },
            diesel_models::EventMetadata::Dispute {
                payment_id,
                attempt_id,
                dispute_id,
            } => Self::Dispute {
                payment_id,
                attempt_id,
                dispute_id,
                content: serde_json::Value::Null,
            },
            diesel_models::EventMetadata::Mandate {
                payment_method_id,
                mandate_id,
            } => Self::Mandate {
                payment_method_id,
                mandate_id,
                content: serde_json::Value::Null,
            },
            diesel_models::EventMetadata::Subscription {
                subscription_id,
                invoice_id,
                payment_id,
            } => Self::Subscription {
                subscription_id,
                invoice_id,
                payment_id,
                content: serde_json::Value::Null,
            },
        }
    }
}

trait OutgoingWebhookResponseHandler {
    async fn handle_error_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        client_error: Report<errors::ApiClientError>,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >;

    async fn handle_success_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        response: reqwest::Response,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >;
}

impl OutgoingWebhookResponseHandler for enums::WebhookDeliveryAttempt {
    async fn handle_error_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        client_error: Report<errors::ApiClientError>,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    > {
        let schedule_webhook_retry = match self {
            Self::InitialAttempt | Self::ManualRetry => types::ScheduleWebhookRetry::NoSchedule,
            Self::AutomaticRetry => {
                // ScheduleWebhookRetry::WithProcessTracker(Box::new(process_tracker))
                todo!()
            }
        };

        api_client_error_handler(
            state,
            merchant_key_store,
            merchant_id,
            event_id,
            client_error,
            *self,
            schedule_webhook_retry,
        )
        .await
    }

    async fn handle_success_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        response: reqwest::Response,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    > {
        let status_code = response.status();
        let is_webhook_notified = status_code.is_success();
        let response_struct = types::WebhookResponse { response };
        let outgoing_webhook_response = response_struct
            .get_outgoing_webhook_response_content()
            .await;
        let updated_event = update_event_in_storage(
            state.clone(),
            is_webhook_notified,
            outgoing_webhook_response,
            merchant_key_store.clone(),
            merchant_id,
            event_id,
        )
        .await?;

        let webhook_action_handler = get_action_handler(*self);
        let result = if is_webhook_notified {
            webhook_action_handler
                .notified_action(
                    state,
                    merchant_key_store,
                    &updated_event,
                    merchant_id,
                    process_tracker,
                )
                .await
        } else {
            webhook_action_handler
                .not_notified_action(state, merchant_id, status_code.as_u16())
                .await
        };

        Ok((updated_event, result))
    }
}

#[async_trait::async_trait]
trait WebhookNotificationHandler: Send + Sync {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        merchant_id: &common_utils::id_type::MerchantId,
        process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>>;

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
    ) -> Option<Report<errors::WebhooksFlowError>>;
}

struct InitialAttempt;
struct AutomaticRetry;
struct ManualRetry;

#[async_trait::async_trait]
impl WebhookNotificationHandler for InitialAttempt {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        merchant_id: &common_utils::id_type::MerchantId,
        process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_successful_delivery(
            state,
            merchant_key_store,
            updated_event,
            merchant_id,
            process_tracker,
            business_status::INITIAL_DELIVERY_ATTEMPT_SUCCESSFUL,
        )
        .await
        .err()
        .map(|error: Report<errors::WebhooksFlowError>| report!(error))
    }

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_failed_delivery(
            state.clone(),
            merchant_id,
            enums::WebhookDeliveryAttempt::InitialAttempt,
            status_code,
            "Ignoring error when sending webhook to merchant",
            types::ScheduleWebhookRetry::NoSchedule,
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

#[async_trait::async_trait]
impl WebhookNotificationHandler for AutomaticRetry {
    async fn notified_action(
        &self,
        _state: SessionState,
        _merchant_key_store: domain::MerchantKeyStore,
        _updated_event: &domain::Event,
        _merchant_id: &common_utils::id_type::MerchantId,
        _process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        todo!()
    }

    async fn not_notified_action(
        &self,
        _state: SessionState,
        _merchant_id: &common_utils::id_type::MerchantId,
        _status_code: u16,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        todo!()
    }
}

#[async_trait::async_trait]
impl WebhookNotificationHandler for ManualRetry {
    async fn notified_action(
        &self,
        _state: SessionState,
        _merchant_key_store: domain::MerchantKeyStore,
        _updated_event: &domain::Event,
        merchant_id: &common_utils::id_type::MerchantId,
        _process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        increment_webhook_outgoing_received_count(merchant_id);
        None
    }

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_failed_delivery(
            state.clone(),
            merchant_id,
            enums::WebhookDeliveryAttempt::ManualRetry,
            status_code,
            "Ignoring error when sending webhook to merchant",
            types::ScheduleWebhookRetry::NoSchedule,
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

fn get_action_handler(
    attempt: enums::WebhookDeliveryAttempt,
) -> Box<dyn WebhookNotificationHandler> {
    match attempt {
        enums::WebhookDeliveryAttempt::InitialAttempt => Box::new(InitialAttempt),
        enums::WebhookDeliveryAttempt::AutomaticRetry => Box::new(AutomaticRetry),
        enums::WebhookDeliveryAttempt::ManualRetry => Box::new(ManualRetry),
    }
}
