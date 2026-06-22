use std::{collections::HashMap, str::FromStr};

use api_models::{
    webhook_events::{OutgoingWebhookRequestContent, OutgoingWebhookResponseContent},
    webhooks,
};
use common_enums::SurchargeEventMapper;
use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, StringExt},
    request::RequestContent,
    type_name,
    types::keymanager::Identifier,
};
use diesel_models::process_tracker::business_status;
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use hyperswitch_interfaces::{consts, webhooks::WebhookResourceData};
use hyperswitch_masking::{ExposeInterface, Mask, PeekInterface, Secret};
use router_env::{
    instrument,
    tracing::{self, Instrument},
};

use super::{types, utils, MERCHANT_CONNECTOR_ACCOUNT_ID, MERCHANT_ID};
#[cfg(feature = "stripe")]
use crate::compatibility::stripe::webhooks as stripe_webhooks;
use crate::{
    core::{
        errors::{self, utils::StorageErrorExt},
        metrics,
        payments::helpers::MerchantConnectorAccountType,
        webhooks::types::WebhookDeliveryResponse,
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

pub(crate) async fn get_webhook_events(
    state: &SessionState,
    platform: domain::Platform,
    primary_event_type: enums::EventType,
    primary_content: &api::OutgoingWebhookContent,
    webhook_resource_data: Option<WebhookResourceData>,
    provider_profile: &domain::Profile,
    webhook_recipient: &utils::WebhookRecipientContext,
) -> CustomResult<Vec<utils::WebhookPayload>, errors::ApiErrorResponse> {
    let mut webhook_events = Vec::new();

    let event_data = utils::WebhookPayload {
        event_type: primary_event_type,
        event_content: primary_content.clone(),
        recipient_data: utils::WebhookRecipientData::Merchant {
            merchant_id: webhook_recipient.merchant_account.get_id().clone(),
        },
    };
    webhook_events.push(event_data);

    #[cfg(feature = "v1")]
    if let Some(surcharge_connector_id) = provider_profile
        .surcharge_connector_details
        .as_ref()
        .and_then(|details| details.surcharge_connector_id.clone())
    {
        match get_surcharge_webhook_event(
            state,
            platform,
            primary_event_type,
            webhook_resource_data,
            surcharge_connector_id,
        )
        .await
        {
            Ok(Some(event_data)) => webhook_events.push(event_data),
            Ok(None) => {
                logger::debug!(
                    "No surcharge webhook event generated for primary event type {}",
                    primary_event_type
                );
            }
            Err(error) => {
                logger::error!(
                    ?error,
                    "Failed to fetch surcharge connector or build surcharge webhook event"
                );
            }
        }
    }

    Ok(webhook_events)
}

/// Fetches surcharge connector and builds surcharge webhook event if applicable.
#[cfg(feature = "v1")]
async fn get_surcharge_webhook_event(
    state: &SessionState,
    platform: domain::Platform,
    primary_event_type: enums::EventType,
    webhook_resource_data: Option<WebhookResourceData>,
    merchant_surcharge_connector_id: common_utils::id_type::MerchantConnectorAccountId,
) -> CustomResult<Option<utils::WebhookPayload>, errors::ApiErrorResponse> {
    let merchant_surcharge_connector = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            platform.get_provider().get_account().get_id(),
            &merchant_surcharge_connector_id,
            platform.get_provider().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_surcharge_connector_id
                .get_string_repr()
                .to_string(),
        })?;

    let connector_name =
        api::enums::SurchargeConnectors::from_str(&merchant_surcharge_connector.connector_name)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })?;

    let surcharge_event = match primary_event_type.to_surcharge_event() {
        Some(event) => event,
        None => return Ok(None),
    };

    if !connector_name.should_notify_connector(primary_event_type) {
        return Ok(None);
    }

    let resource = match webhook_resource_data {
        Some(r) => r,
        None => return Ok(None),
    };

    let payment_attempt = resource.get_payment_attempt();

    utils::WebhookPayload::build_surcharge_payload(
        surcharge_event,
        payment_attempt,
        &merchant_surcharge_connector,
    )
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn create_event_and_trigger_outgoing_webhook(
    state: SessionState,
    platform: domain::Platform,
    primary_event_type: enums::EventType,
    event_class: enums::EventClass,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    primary_content: api::OutgoingWebhookContent,
    primary_object_created_at: Option<time::PrimitiveDateTime>,
    webhook_recipient: utils::WebhookRecipientContext,
    webhook_resource_data: Option<WebhookResourceData>,
    provider_profile: domain::Profile,
) -> CustomResult<(), errors::ApiErrorResponse> {
    if !state.conf.webhooks.outgoing_enabled {
        logger::debug!(
            business_profile_id=?webhook_recipient.profile.get_id(),
            "Outgoing webhooks are disabled in application configuration"
        );
        return Ok(());
    };

    let events_to_trigger = get_webhook_events(
        &state,
        platform.clone(),
        primary_event_type,
        &primary_content,
        webhook_resource_data,
        &provider_profile,
        &webhook_recipient,
    )
    .await?;

    let provider_merchant_id = platform.get_provider().get_account().get_id().clone();
    let processor_merchant_id = platform.get_processor().get_account().get_id().clone();
    let now = common_utils::date_time::now();

    for event_data in events_to_trigger {
        let event_type = event_data.event_type;
        let _ = insert_event_and_spawn_webhook_delivery(
            state.clone(),
            &platform,
            event_data,
            &webhook_recipient,
            provider_merchant_id.clone(),
            processor_merchant_id.clone(),
            primary_object_id.clone(),
            primary_object_type,
            primary_object_created_at,
            now,
            event_class,
        )
        .await
        .inspect_err(|error| {
            logger::error!(
                ?error,
                "Failed to insert event and spawn webhook delivery for event type {}",
                event_type
            );
        });
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_event_and_spawn_webhook_delivery(
    state: SessionState,
    platform: &domain::Platform,
    event_data: utils::WebhookPayload,
    webhook_recipient: &utils::WebhookRecipientContext,
    provider_merchant_id: common_utils::id_type::MerchantId,
    processor_merchant_id: common_utils::id_type::MerchantId,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    primary_object_created_at: Option<time::PrimitiveDateTime>,
    now: time::PrimitiveDateTime,
    event_class: enums::EventClass,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let delivery_attempt = enums::WebhookDeliveryAttempt::InitialAttempt;
    let idempotent_event_id =
        utils::get_idempotent_event_id(&primary_object_id, event_data.event_type, delivery_attempt)
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to generate idempotent event ID")?;

    if let utils::WebhookRecipientData::Merchant { .. } = event_data.recipient_data {
        let webhook_url_result = get_webhook_url_from_business_profile(&webhook_recipient.profile);
        if webhook_url_result.is_err() || webhook_url_result.as_ref().is_ok_and(String::is_empty) {
            logger::debug!(
                business_profile_id=?webhook_recipient.profile.get_id(),
                %idempotent_event_id,
                "merchant webhook URL \
                 could not be obtained; skipping outgoing webhooks for event"
            );
        }
    };

    let event_id = utils::generate_event_id();
    let event_type = event_data.event_type;
    let content = event_data.event_content;

    let outgoing_webhook = api::OutgoingWebhook {
        merchant_id: provider_merchant_id.clone(),
        event_id: event_id.clone(),
        event_type,
        content: content.clone(),
        timestamp: now,
        processor_merchant_id: Some(processor_merchant_id.clone()),
    };

    let request_content = get_outgoing_webhook_request(
        &webhook_recipient.merchant_account,
        outgoing_webhook,
        &webhook_recipient.profile,
    )
    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
    .attach_printable("Failed to construct outgoing webhook request content")?;
    let recipient = event_data.recipient_data.get_event_recipient();
    let event_metadata = storage::EventMetadata::foreign_from(&content);
    let key_manager_state = &(&state).into();
    let new_event = domain::Event {
        event_id: event_id.clone(),
        event_type,
        event_class,
        is_webhook_notified: false,
        primary_object_id: primary_object_id.clone(),
        primary_object_type,
        created_at: now,
        merchant_id: Some(provider_merchant_id.clone()),
        business_profile_id: Some(webhook_recipient.profile.get_id().to_owned()),
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
                Identifier::Merchant(webhook_recipient.key_store.merchant_id.clone()),
                webhook_recipient.key_store.key.get_inner().peek(),
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
        processor_merchant_id: Some(processor_merchant_id.clone()),
        initiator_merchant_id: Some(webhook_recipient.key_store.merchant_id.clone()),
        recipient: Some(recipient),
    };

    let lock_value = utils::perform_redis_lock(
        &state,
        &idempotent_event_id,
        webhook_recipient.key_store.merchant_id.clone(),
    )
    .await?;

    if lock_value.is_none() {
        return Ok(());
    }

    if (state
        .store
        .find_event_by_initiator_merchant_id_idempotent_event_id(
            &webhook_recipient.key_store.merchant_id,
            &idempotent_event_id,
            &webhook_recipient.key_store,
        )
        .await)
        .is_ok()
    {
        logger::debug!(
            "Event with idempotent ID `{idempotent_event_id}` already exists in the database"
        );
        let _ = utils::free_redis_lock(
            &state,
            &idempotent_event_id,
            webhook_recipient.key_store.merchant_id.clone(),
            lock_value,
        )
        .await;
        return Ok(());
    }

    let event_insert_result = state
        .store
        .insert_event(new_event, &webhook_recipient.key_store)
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

    let _ = utils::free_redis_lock(
        &state,
        &idempotent_event_id,
        webhook_recipient.key_store.merchant_id.clone(),
        lock_value,
    )
    .await;

    let process_tracker = add_outgoing_webhook_retry_task_to_process_tracker(
        &*state.store,
        state.superposition_service.as_ref(),
        platform,
        webhook_recipient,
        &event,
        state.conf.application_source,
        event_data.recipient_data.clone(),
    )
    .await
    .inspect_err(|error| {
        logger::error!(
            ?error,
            "Failed to add outgoing webhook retry task to process tracker"
        );
    })
    .ok();

    let cloned_state = state.clone();
    let cloned_key_store = webhook_recipient.key_store.clone();
    let cloned_provider_merchant_id = provider_merchant_id.clone();
    let cloned_processor_merchant_id = processor_merchant_id.clone();
    let cloned_profile = webhook_recipient.profile.clone();
    // Using a tokio spawn here and not arbiter because not all caller of this function
    // may have an actix arbiter
    tokio::spawn(
        async move {
            Box::pin(trigger_webhook_and_raise_event(
                cloned_state,
                cloned_profile,
                &cloned_key_store,
                cloned_provider_merchant_id,
                cloned_processor_merchant_id,
                event,
                request_content,
                delivery_attempt,
                Some(content),
                process_tracker,
                event_data.recipient_data,
            ))
            .await;
        }
        .in_current_span(),
    );

    Ok(())
}

/// Trait for dispatching outgoing webhook delivery.
///
/// Two concrete implementations exist:
/// - [`MerchantWebhook`]: delivers webhooks to merchant-configured URLs
/// - [`ConnectorWebhook`]: reserved for delivering connector-facing webhooks
#[async_trait::async_trait]
trait WebhookTrigger: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn trigger_and_raise(
        &self,
        state: SessionState,
        business_profile: domain::Profile,
        merchant_key_store: domain::MerchantKeyStore,
        provider_merchant_id: common_utils::id_type::MerchantId,
        processor_merchant_id: common_utils::id_type::MerchantId,
        event: domain::Event,
        request_content: OutgoingWebhookRequestContent,
        delivery_attempt: enums::WebhookDeliveryAttempt,
        content: Option<api::OutgoingWebhookContent>,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    );
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn trigger_webhook_and_raise_event(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: &domain::MerchantKeyStore,
    provider_merchant_id: common_utils::id_type::MerchantId,
    processor_merchant_id: common_utils::id_type::MerchantId,
    event: domain::Event,
    request_content: OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    content: Option<api::OutgoingWebhookContent>,
    process_tracker: Option<storage::ProcessTracker>,
    recipient_data: utils::WebhookRecipientData,
) {
    let trigger: Box<dyn WebhookTrigger> = match event.recipient {
        Some(enums::EventRecipient::Connector) => Box::new(types::ConnectorWebhook),
        _ => Box::new(types::MerchantWebhook),
    };

    trigger
        .trigger_and_raise(
            state,
            business_profile,
            merchant_key_store.clone(),
            provider_merchant_id,
            processor_merchant_id,
            event,
            request_content,
            delivery_attempt,
            content,
            process_tracker,
            recipient_data,
        )
        .await;
}

#[async_trait::async_trait]
impl WebhookTrigger for types::MerchantWebhook {
    async fn trigger_and_raise(
        &self,
        state: SessionState,
        business_profile: domain::Profile,
        merchant_key_store: domain::MerchantKeyStore,
        provider_merchant_id: common_utils::id_type::MerchantId,
        processor_merchant_id: common_utils::id_type::MerchantId,
        event: domain::Event,
        request_content: OutgoingWebhookRequestContent,
        delivery_attempt: enums::WebhookDeliveryAttempt,
        content: Option<api::OutgoingWebhookContent>,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    ) {
        logger::debug!(
            event_id=%event.event_id,
            idempotent_event_id=?event.idempotent_event_id,
            initial_attempt_id=?event.initial_attempt_id,
            "Attempting to send webhook"
        );

        let trigger_webhook_result = Box::pin(trigger_webhook_to_merchant(
            state.clone(),
            business_profile,
            &merchant_key_store,
            event.clone(),
            request_content,
            delivery_attempt,
            process_tracker,
            recipient_data,
        ))
        .await;

        let _ = raise_webhooks_analytics_event(
            state,
            trigger_webhook_result,
            content,
            provider_merchant_id,
            processor_merchant_id,
            event,
        )
        .await;
    }
}

#[async_trait::async_trait]
impl WebhookTrigger for types::ConnectorWebhook {
    async fn trigger_and_raise(
        &self,
        state: SessionState,
        business_profile: domain::Profile,
        merchant_key_store: domain::MerchantKeyStore,
        provider_merchant_id: common_utils::id_type::MerchantId,
        processor_merchant_id: common_utils::id_type::MerchantId,
        event: domain::Event,
        request_content: OutgoingWebhookRequestContent,
        delivery_attempt: enums::WebhookDeliveryAttempt,
        content: Option<api::OutgoingWebhookContent>,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    ) {
        logger::debug!(
            event_id=%event.event_id,
            "Attempting to notify connector via UCS"
        );

        let trigger_webhook_result = Box::pin(trigger_webhook_to_connector(
            state.clone(),
            business_profile,
            merchant_key_store.clone(),
            event.clone(),
            delivery_attempt,
            request_content,
            process_tracker,
            recipient_data,
        ))
        .await;

        let _ = raise_webhooks_analytics_event(
            state,
            trigger_webhook_result,
            content,
            provider_merchant_id,
            processor_merchant_id,
            event,
        )
        .await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn trigger_webhook_to_connector(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: domain::MerchantKeyStore,
    event: domain::Event,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    request_content: OutgoingWebhookRequestContent,
    process_tracker: Option<storage::ProcessTracker>,
    recipient_data: utils::WebhookRecipientData,
) -> CustomResult<
    (domain::Event, Option<Report<errors::WebhooksFlowError>>),
    errors::WebhooksFlowError,
> {
    let provider_merchant_id = business_profile.merchant_id.clone();

    let merchant_connector_id = match &recipient_data {
        utils::WebhookRecipientData::Connector {
            merchant_connector_id,
            ..
        } => merchant_connector_id,
        _ => {
            logger::error!("Missing merchant_connector_id for connector webhook");
            return Err(errors::WebhooksFlowError::MerchantConfigNotFound.into());
        }
    };

    let mca_result = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &provider_merchant_id,
            merchant_connector_id,
            &merchant_key_store,
        )
        .await;

    let mca = match mca_result {
        Ok(mca) => mca,
        Err(err) => {
            logger::error!(?err, "Failed to find merchant connector account");
            return Err(errors::WebhooksFlowError::MerchantConfigNotFound.into());
        }
    };

    let connector_name = mca.connector_name.clone();

    let (notify_request, connector_auth_metadata, notify_event_type) =
        crate::core::unified_connector_service::build_notify_connector_request(
            &event,
            request_content,
            &provider_merchant_id,
            MerchantConnectorAccountType::DbVal(Box::new(mca)),
            connector_name,
        )
        .change_context(errors::WebhooksFlowError::WebhookRequestConstructionFailed)?;

    let response = crate::core::unified_connector_service::call_unified_connector_service_for_notify_connector(
            &state,
            &event,
            connector_auth_metadata,
            notify_request,
            notify_event_type,
            &provider_merchant_id,
            business_profile.get_id(),
        )
        .await;

    metrics::WEBHOOK_OUTGOING_COUNT.add(
        1,
        router_env::metric_attributes!((
            MERCHANT_CONNECTOR_ACCOUNT_ID,
            merchant_connector_id.clone()
        )),
    );
    logger::debug!(outgoing_webhook_response=?response);

    match response {
        Ok(ref resp) if resp.is_success() => {
            update_payment_attempt_from_webhook_response(
                &state,
                &event,
                &provider_merchant_id,
                &merchant_key_store,
            )
            .await;
        }
        _ => {}
    };

    match response {
        Ok(response) => {
            delivery_attempt
                .handle_success_response(
                    state,
                    merchant_key_store.clone(),
                    &provider_merchant_id,
                    &event.event_id,
                    process_tracker,
                    response,
                    recipient_data,
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
                    process_tracker,
                    client_error,
                    recipient_data,
                )
                .await
        }
    }
}

async fn update_payment_attempt_from_webhook_response(
    state: &SessionState,
    event: &domain::Event,
    processor_merchant_id: &common_utils::id_type::MerchantId,
    merchant_key_store: &domain::MerchantKeyStore,
) {
    if event.event_type == enums::EventType::SurchargePaymentSucceeded {
        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(processor_merchant_id, merchant_key_store)
            .await;

        let storage_scheme = match &merchant_account {
            Ok(ma) => ma.storage_scheme,
            Err(err) => {
                logger::warn!(?err, "Using default PostgresOnly storage scheme");
                enums::MerchantStorageScheme::PostgresOnly
            }
        };

        let attempt_id = match &event.metadata {
            Some(diesel_models::EventMetadata::Surcharge {
                payment_id: _,
                attempt_id,
            }) => attempt_id,
            _ => {
                logger::debug!(
                "Could not find attempt_id in event metadata for event_id: {}, skipping payment attempt update",
                event.event_id
            );
                return;
            }
        };

        let payment_attempt_result = state
            .store
            .find_payment_attempt_by_attempt_id_processor_merchant_id(
                attempt_id,
                processor_merchant_id,
                storage_scheme,
                merchant_key_store,
            )
            .await;
        let payment_attempt = match payment_attempt_result {
            Ok(pa) => pa,
            Err(err) => {
                logger::warn!(
                    ?err,
                    attempt_id = %attempt_id,
                    "Could not find payment attempt for sale_notified update"
                );
                return;
            }
        };
        let existing_details = match payment_attempt.external_surcharge_details.clone() {
            Some(details) => details,
            None => {
                logger::debug!(
                    attempt_id = %attempt_id,
                    "No external_surcharge_details found; nothing to update"
                );
                return;
            }
        };
        let updated_details = common_types::payments::ExternalSurchargeDetails {
            external_surcharge_id: existing_details.external_surcharge_id,
            external_surcharge_amount: existing_details.external_surcharge_amount,
            sale_notified: true,
        };
        let update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ExternalSurchargeUpdate {
        external_surcharge_details: updated_details,
        updated_by: "OutgoingWebhookFlow".to_string(),
    };
        let _ = state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_attempt,
                update,
                storage_scheme,
                merchant_key_store,
            )
            .await
            .inspect_err(|err| {
                logger::error!(
                    ?err,
                    attempt_id = %attempt_id,
                    "Failed to update sale_notified on payment attempt"
                );
            });
    }
    logger::info!("Successfully updated payment attempt");
}

#[allow(clippy::too_many_arguments)]
async fn trigger_webhook_to_merchant(
    state: SessionState,
    business_profile: domain::Profile,
    merchant_key_store: &domain::MerchantKeyStore,
    event: domain::Event,
    request_content: OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    process_tracker: Option<storage::ProcessTracker>,
    recipient_data: utils::WebhookRecipientData,
) -> CustomResult<
    (domain::Event, Option<Report<errors::WebhooksFlowError>>),
    errors::WebhooksFlowError,
> {
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

    match response {
        Ok(response) => {
            delivery_attempt
                .handle_success_response(
                    state,
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event_id.clone(),
                    process_tracker,
                    response,
                    recipient_data,
                )
                .await
        }
        Err(client_error) => {
            delivery_attempt
                .handle_error_response(
                    state,
                    merchant_key_store.clone(),
                    &business_profile.merchant_id,
                    &event_id.clone(),
                    process_tracker,
                    client_error,
                    recipient_data,
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
    provider_merchant_id: common_utils::id_type::MerchantId,
    processor_merchant_id: common_utils::id_type::MerchantId,
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
        .and_then(OutgoingWebhookEventMetric::get_outgoing_webhook_event_content)
        .or_else(|| get_outgoing_webhook_event_content_from_event_metadata(updated_event.metadata));

    // Get status_code from webhook response
    let status_code = {
        let webhook_response: Option<OutgoingWebhookResponseContent> =
            updated_event.response.and_then(|res| {
                StringExt::parse_struct(
                    PeekInterface::peek(res.get_inner()),
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

    let webhook_event = OutgoingWebhookEvent::new(
        state.tenant.tenant_id.clone(),
        provider_merchant_id,
        Some(processor_merchant_id),
        updated_event.initiator_merchant_id,
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

pub(crate) async fn add_outgoing_webhook_retry_task_to_process_tracker(
    db: &dyn StorageInterface,
    superposition_client: &external_services::superposition::SuperpositionClient,
    platform: &domain::Platform,
    webhook_recipient: &utils::WebhookRecipientContext,
    event: &domain::Event,
    application_source: common_enums::ApplicationSource,
    webhook_recipient_data: utils::WebhookRecipientData,
) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
    let processor_merchant_id = platform.get_processor().get_account().get_id().clone();
    let provider_merchant_id = platform.get_provider().get_account().get_id().clone();
    let dimensions = crate::core::configs::dimension_state::Dimensions::new()
        .with_processor_merchant_id(processor_merchant_id.clone().into());
    let schedule_time = outgoing_webhook_retry::get_webhook_delivery_retry_schedule_time(
        db,
        superposition_client,
        &dimensions,
        0,
    )
    .await
    .ok_or(errors::StorageError::ValueNotFound(
        "Process tracker schedule time".into(),
    ))
    .attach_printable("Failed to obtain initial process tracker schedule time")?;

    let tracking_data = types::OutgoingWebhookTrackingData {
        merchant_id: provider_merchant_id,
        business_profile_id: webhook_recipient.profile.get_id().to_owned(),
        processor_merchant_id: Some(processor_merchant_id),
        initiator_merchant_id: Some(webhook_recipient.key_store.merchant_id.clone()),
        event_type: event.event_type,
        event_class: event.event_class,
        primary_object_id: event.primary_object_id.clone(),
        primary_object_type: event.primary_object_type,
        initial_attempt_id: event.initial_attempt_id.clone(),
        recipient_data: webhook_recipient_data,
    };

    let runner = storage::ProcessTrackerRunner::OutgoingWebhookRetryWorkflow;
    let task = "OUTGOING_WEBHOOK_RETRY";
    let tag = ["OUTGOING_WEBHOOKS"];
    let process_tracker_id = scheduler::utils::get_process_tracker_id(
        runner,
        task,
        &event.event_id,
        &webhook_recipient.profile.merchant_id,
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
        application_source,
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
    webhook_recipient_account: &domain::MerchantAccount,
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

    match webhook_recipient_account.get_compatible_connector() {
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
        .update_event_by_event_id(event_id, event_update, &merchant_key_store)
        .await
        .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
}

#[allow(clippy::too_many_arguments)]
async fn api_client_error_handler(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    webhook_recipient_merchant_id: &common_utils::id_type::MerchantId,
    event_id: &str,
    client_error: Report<errors::ApiClientError>,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    schedule_webhook_retry: ScheduleWebhookRetry,
    recipient_data: utils::WebhookRecipientData,
) -> CustomResult<
    (domain::Event, Option<Report<errors::WebhooksFlowError>>),
    errors::WebhooksFlowError,
> {
    // Not including detailed error message in response information since it contains too
    // much of diagnostic information to be exposed to the merchant.
    let updated_event = update_event_if_client_error(
        state.clone(),
        merchant_key_store,
        event_id,
        "Unable to send request to merchant/connector server".to_string(),
    )
    .await?;

    let error = client_error.change_context(errors::WebhooksFlowError::WebhookCallFailed);
    logger::error!(
        ?error,
        ?delivery_attempt,
        "An error occurred when sending webhook to merchant/connector"
    );

    if let ScheduleWebhookRetry::WithProcessTracker(process_tracker) = schedule_webhook_retry {
        // Schedule a retry attempt for webhook delivery using the webhook recipient's
        // merchant_id for retry schedule lookup, consistent with initial scheduling.
        outgoing_webhook_retry::retry_webhook_delivery_task(
            &*state.store,
            webhook_recipient_merchant_id,
            state.superposition_service.as_ref(),
            *process_tracker,
            recipient_data,
        )
        .await
        .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)?;
    }

    Ok((updated_event, Some(error)))
}

async fn update_webhook_response_in_storage<R: WebhookDeliveryResponse>(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    event_id: &str,
    response: R,
    status_code: u16,
    is_webhook_notified: bool,
) -> CustomResult<domain::Event, errors::WebhooksFlowError> {
    let key_manager_state = &(&state).into();
    let response_headers = response.get_response_headers();
    let error_message = response.get_error_message();
    let response_body = response.get_response_body().await;
    let response_to_store = OutgoingWebhookResponseContent {
        body: Some(response_body),
        headers: Some(response_headers),
        status_code: Some(status_code),
        error_message,
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
        .update_event_by_event_id(event_id, event_update, &merchant_key_store)
        .await
        .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
}

async fn update_overall_delivery_status_in_storage(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
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
            .update_event_by_event_id(
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

pub(crate) async fn success_response_handler(
    state: SessionState,
    recipient_data: &utils::WebhookRecipientData,
    process_tracker: Option<storage::ProcessTracker>,
    business_status: &'static str,
) -> CustomResult<(), errors::WebhooksFlowError> {
    utils::increment_webhook_outgoing_received_count(recipient_data);

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
        diesel_models::EventMetadata::Surcharge {
            payment_id,
            attempt_id,
        } => OutgoingWebhookEventContent::Surcharge {
            payment_id,
            attempt_id,
        },
    })
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
            webhooks::OutgoingWebhookContent::SurchargeDetails(surcharge) => Self::Surcharge {
                payment_id: surcharge.payment_id.clone(),
                attempt_id: surcharge.attempt_id.clone(),
            },
        }
    }
}

async fn handle_successful_delivery(
    state: SessionState,
    merchant_key_store: domain::MerchantKeyStore,
    updated_event: &domain::Event,
    process_tracker: Option<storage::ProcessTracker>,
    business_status: &'static str,
    recipient_data: utils::WebhookRecipientData,
) -> CustomResult<(), errors::WebhooksFlowError> {
    update_overall_delivery_status_in_storage(
        state.clone(),
        merchant_key_store.clone(),
        updated_event,
    )
    .await?;

    success_response_handler(
        state.clone(),
        &recipient_data,
        process_tracker,
        business_status,
    )
    .await
}

#[cfg(feature = "v1")]
trait OutgoingWebhookResponseHandlerV1 {
    #[allow(clippy::too_many_arguments)]
    async fn handle_success_response<R: WebhookDeliveryResponse>(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        response: R,
        recipient_data: utils::WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >;

    #[allow(clippy::too_many_arguments)]
    async fn handle_error_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        client_error: Report<errors::ApiClientError>,
        recipient_data: utils::WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >;
}

#[cfg(feature = "v1")]
impl OutgoingWebhookResponseHandlerV1 for enums::WebhookDeliveryAttempt {
    async fn handle_success_response<R: WebhookDeliveryResponse>(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        response: R,
        recipient_data: utils::WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    > {
        let status_code = response.status();
        let is_webhook_notified = response.is_success();

        let updated_event = update_webhook_response_in_storage(
            state.clone(),
            merchant_key_store.clone(),
            event_id,
            response,
            status_code,
            is_webhook_notified,
        )
        .await?;

        let webhook_action_handler = get_action_handler(*self);
        let result = if is_webhook_notified {
            webhook_action_handler
                .notified_action(
                    state,
                    merchant_key_store,
                    &updated_event,
                    process_tracker,
                    recipient_data,
                )
                .await
        } else {
            webhook_action_handler
                .not_notified_action(
                    state,
                    merchant_id,
                    status_code,
                    recipient_data,
                    process_tracker,
                )
                .await
        };

        Ok((updated_event, result))
    }
    async fn handle_error_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        client_error: Report<errors::ApiClientError>,
        recipient_data: utils::WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    > {
        let schedule_webhook_retry = match self {
            Self::InitialAttempt | Self::ManualRetry => ScheduleWebhookRetry::NoSchedule,
            Self::AutomaticRetry => ScheduleWebhookRetry::WithProcessTracker(Box::new(
                process_tracker
                    .get_required_value("process_tracker")
                    .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)
                    .attach_printable("`process_tracker` is unavailable in automatic retry flow")?,
            )),
        };

        api_client_error_handler(
            state,
            merchant_key_store,
            merchant_id,
            event_id,
            client_error,
            *self,
            schedule_webhook_retry,
            recipient_data,
        )
        .await
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
trait WebhookNotificationHandlerV1: Send + Sync {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>>;

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
        recipient_data: utils::WebhookRecipientData,
        process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>>;
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl WebhookNotificationHandlerV1 for types::InitialAttempt {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_successful_delivery(
            state,
            merchant_key_store,
            updated_event,
            process_tracker,
            business_status::INITIAL_DELIVERY_ATTEMPT_SUCCESSFUL,
            recipient_data,
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
        recipient_data: utils::WebhookRecipientData,
        _process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_failed_delivery(
            state.clone(),
            merchant_id,
            status_code,
            recipient_data,
            ScheduleWebhookRetry::NoSchedule,
            "Ignoring error when sending webhook to merchant",
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl WebhookNotificationHandlerV1 for types::AutomaticRetry {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_successful_delivery(
            state,
            merchant_key_store,
            updated_event,
            process_tracker,
            business_status::COMPLETED_BY_PT,
            recipient_data,
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
        recipient_data: utils::WebhookRecipientData,
        process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        let process_tracker = match process_tracker
            .get_required_value("process_tracker")
            .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)
            .attach_printable("`process_tracker` is unavailable in automatic retry flow")
        {
            Ok(process_tracker) => process_tracker,
            Err(err) => return Some(err),
        };

        handle_failed_delivery(
            state.clone(),
            merchant_id,
            status_code,
            recipient_data,
            ScheduleWebhookRetry::WithProcessTracker(Box::new(process_tracker)),
            "Ignoring error when sending webhook to merchant",
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl WebhookNotificationHandlerV1 for types::ManualRetry {
    async fn notified_action(
        &self,
        _state: SessionState,
        _merchant_key_store: domain::MerchantKeyStore,
        _updated_event: &domain::Event,
        _process_tracker: Option<storage::ProcessTracker>,
        recipient_data: utils::WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        utils::increment_webhook_outgoing_received_count(&recipient_data);
        None
    }

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
        recipient_data: utils::WebhookRecipientData,
        _process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        handle_failed_delivery(
            state.clone(),
            merchant_id,
            status_code,
            recipient_data,
            ScheduleWebhookRetry::NoSchedule,
            "Ignoring error when sending webhook to merchant",
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

async fn handle_failed_delivery(
    state: SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    status_code: u16,
    recipient_data: utils::WebhookRecipientData,
    schedule_webhook_retry: ScheduleWebhookRetry,
    log_message: &'static str,
) -> CustomResult<(), errors::WebhooksFlowError> {
    utils::increment_webhook_outgoing_not_received_count(&recipient_data);

    let error = report!(errors::WebhooksFlowError::NotReceivedByReceipt);
    logger::warn!(?error, status_code, %log_message);

    if let ScheduleWebhookRetry::WithProcessTracker(process_tracker) = schedule_webhook_retry {
        outgoing_webhook_retry::retry_webhook_delivery_task(
            &*state.store,
            merchant_id,
            state.superposition_service.as_ref(),
            *process_tracker,
            recipient_data,
        )
        .await
        .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)?;
    }

    Err(error)
}

fn get_action_handler(
    attempt: enums::WebhookDeliveryAttempt,
) -> Box<dyn WebhookNotificationHandlerV1> {
    match attempt {
        enums::WebhookDeliveryAttempt::InitialAttempt => Box::new(types::InitialAttempt),
        enums::WebhookDeliveryAttempt::AutomaticRetry => Box::new(types::AutomaticRetry),
        enums::WebhookDeliveryAttempt::ManualRetry => Box::new(types::ManualRetry),
    }
}
