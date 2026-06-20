use api_models::{webhook_events, webhooks};
use common_utils::{crypto::SignMessage, ext_traits::Encode};
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData;
use hyperswitch_masking::Secret;
use serde::Serialize;

use crate::{
    core::{errors, webhooks::utils::WebhookRecipientData},
    headers, logger,
    routes::SessionState,
    services::request::Maskable,
    types::{
        api,
        domain::{self},
        storage::{self, enums},
    },
};

#[derive(Debug)]
pub enum ScheduleWebhookRetry {
    WithProcessTracker(Box<storage::ProcessTracker>),
    NoSchedule,
}

pub struct OutgoingWebhookPayloadWithSignature {
    pub payload: Secret<String>,
    pub signature: Option<String>,
}

pub trait OutgoingWebhookType:
    Serialize + From<webhooks::OutgoingWebhook> + Sync + Send + std::fmt::Debug + 'static
{
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<impl AsRef<[u8]>>,
    ) -> errors::CustomResult<OutgoingWebhookPayloadWithSignature, errors::WebhooksFlowError>;

    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String);
}

impl OutgoingWebhookType for webhooks::OutgoingWebhook {
    fn get_outgoing_webhooks_signature(
        &self,
        payment_response_hash_key: Option<impl AsRef<[u8]>>,
    ) -> errors::CustomResult<OutgoingWebhookPayloadWithSignature, errors::WebhooksFlowError> {
        let webhook_signature_payload = self
            .encode_to_string_of_json()
            .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
            .attach_printable("failed encoding outgoing webhook payload")?;

        let signature = payment_response_hash_key
            .map(|key| {
                common_utils::crypto::HmacSha512::sign_message(
                    &common_utils::crypto::HmacSha512,
                    key.as_ref(),
                    webhook_signature_payload.as_bytes(),
                )
            })
            .transpose()
            .change_context(errors::WebhooksFlowError::OutgoingWebhookSigningFailed)
            .attach_printable("Failed to sign the message")?
            .map(hex::encode);

        Ok(OutgoingWebhookPayloadWithSignature {
            payload: webhook_signature_payload.into(),
            signature,
        })
    }

    fn add_webhook_header(header: &mut Vec<(String, Maskable<String>)>, signature: String) {
        header.push((headers::X_WEBHOOK_SIGNATURE.to_string(), signature.into()))
    }
}

/// Tracking data serialized into the process tracker for outgoing webhook retries.
///
/// This data is persisted so that the retry workflow can reconstruct the correct
/// context (merchant account, keystore, business profile) for webhook redelivery.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct OutgoingWebhookTrackingData {
    /// The provider (business owner) merchant id. Always populated.
    pub(crate) merchant_id: common_utils::id_type::MerchantId,
    /// The business profile of the webhook recipient (initiator's profile).
    pub(crate) business_profile_id: common_utils::id_type::ProfileId,
    /// The merchant_id of the merchant whose connector credentials were used for payment processing.
    /// In standard setups this equals `merchant_id`.
    pub(crate) processor_merchant_id: Option<common_utils::id_type::MerchantId>,
    /// The merchant_id of the webhook recipient (the merchant that initiated the
    /// operation). Used during retries to look up the correct keystore directly.
    /// `None` for tracking data created by older deployments (falls back to `merchant_id`).
    #[serde(default)]
    pub(crate) initiator_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub(crate) event_type: enums::EventType,
    pub(crate) event_class: enums::EventClass,
    pub(crate) primary_object_id: String,
    pub(crate) primary_object_type: enums::EventObjectType,
    pub(crate) initial_attempt_id: Option<String>,
    pub(crate) recipient_data: WebhookRecipientData,
}

pub struct WebhookResponse {
    pub response: reqwest::Response,
}

impl WebhookResponse {
    pub async fn get_outgoing_webhook_response_content(
        self,
    ) -> webhook_events::OutgoingWebhookResponseContent {
        let status_code = self.response.status();
        let response_headers = self
            .response
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
        let response_body = self
            .response
            .text()
            .await
            .map(Secret::from)
            .unwrap_or_else(|error| {
                logger::warn!("Response contains non-UTF-8 characters: {error:?}");
                Secret::from(String::from("Non-UTF-8 response body"))
            });
        webhook_events::OutgoingWebhookResponseContent {
            body: Some(response_body),
            headers: Some(response_headers),
            status_code: Some(status_code.as_u16()),
            error_message: None,
        }
    }
}

/// Unified interface for webhook delivery responses from different sources.
///
/// Implemented for [`reqwest::Response`] (merchant webhook path) and
/// [`NotifyConnectorResponseData`] (UCS connector notification path).
#[async_trait::async_trait]
pub(crate) trait WebhookDeliveryResponse: Send {
    fn status(&self) -> u16;
    fn is_success(&self) -> bool;
    fn get_response_headers(&self) -> Vec<(String, Secret<String>)>;
    fn get_error_message(&self) -> Option<String>;
    async fn get_response_body(self) -> Secret<String>;
}

#[async_trait::async_trait]
impl WebhookDeliveryResponse for reqwest::Response {
    fn status(&self) -> u16 {
        self.status().as_u16()
    }

    fn is_success(&self) -> bool {
        self.status().is_success()
    }

    fn get_response_headers(&self) -> Vec<(String, Secret<String>)> {
        self.headers()
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
            .collect::<Vec<_>>()
    }

    async fn get_response_body(self) -> Secret<String> {
        self.text().await.map(Secret::from).unwrap_or_else(|error| {
            logger::warn!("Response contains non-UTF-8 characters: {error:?}");
            Secret::from(String::from("Non-UTF-8 response body"))
        })
    }

    fn get_error_message(&self) -> Option<String> {
        None
    }
}

#[async_trait::async_trait]
impl WebhookDeliveryResponse for NotifyConnectorResponseData {
    fn status(&self) -> u16 {
        self.status_code
    }

    fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    fn get_response_headers(&self) -> Vec<(String, Secret<String>)> {
        vec![]
    }

    fn get_error_message(&self) -> Option<String> {
        self.error_message.clone()
    }

    async fn get_response_body(self) -> Secret<String> {
        Secret::from(serde_json::to_string(&self).unwrap_or_else(|error| {
            logger::warn!("Failed to serialize response: {error:?}");
            String::from("Failed to serialize response")
        }))
    }
}

/// Trait for dispatching outgoing webhook delivery.
///
/// Two concrete implementations exist:
/// - [`MerchantWebhook`]: delivers webhooks to merchant-configured URLs
/// - [`ConnectorWebhook`]: reserved for delivering connector-facing webhooks
#[async_trait::async_trait]
pub(crate) trait WebhookTrigger: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn trigger_and_raise(
        &self,
        state: SessionState,
        business_profile: domain::Profile,
        merchant_key_store: domain::MerchantKeyStore,
        provider_merchant_id: common_utils::id_type::MerchantId,
        processor_merchant_id: common_utils::id_type::MerchantId,
        event: domain::Event,
        request_content: webhook_events::OutgoingWebhookRequestContent,
        delivery_attempt: enums::WebhookDeliveryAttempt,
        content: Option<api::OutgoingWebhookContent>,
        process_tracker: Option<storage::ProcessTracker>,
    );
}

pub(crate) struct MerchantWebhook;
pub(crate) struct ConnectorWebhook;

#[async_trait::async_trait]
pub(crate) trait OutgoingWebhookResponseHandler {
    async fn handle_success_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        response_content: webhook_events::OutgoingWebhookResponseContent,
        is_webhook_notified: bool,
        recipient_data: WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >;

    async fn handle_error_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        client_error: Report<errors::ApiClientError>,
        recipient_data: WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    >;
}

#[async_trait::async_trait]
pub(crate) trait WebhookNotificationHandler: Send + Sync {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>>;

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
        recipient_data: WebhookRecipientData,
        process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>>;
}

pub(crate) struct InitialAttempt;
pub(crate) struct AutomaticRetry;
pub(crate) struct ManualRetry;

#[async_trait::async_trait]
impl WebhookNotificationHandler for InitialAttempt {
    async fn notified_action(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        updated_event: &domain::Event,
        process_tracker: Option<storage::ProcessTracker>,
        recipient_data: WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        update_overall_delivery_status_in_storage(state.clone(), merchant_key_store, updated_event)
            .await?;

        increment_webhook_outgoing_received_count(&recipient_data);

        utils::success_response_handler(
            state.clone(),
            &recipient_data,
            Some(process_tracker),
            business_status::COMPLETED_BY_PT,
        )
        .await?;
    }

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
        recipient_data: WebhookRecipientData,
        _process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        super::utils::handle_failed_delivery(
            state,
            merchant_id,
            status_code,
            recipient_data,
            ScheduleWebhookRetry::NoSchedule,
            "Ignoring error when sending webhook to merchant/connector",
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
        _process_tracker: Option<storage::ProcessTracker>,
        _recipient_data: WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        None
    }

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
        recipient_data: WebhookRecipientData,
        process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        let pt = process_tracker
            .expect("process_tracker required for automatic retry not_notified_action");
        super::utils::handle_failed_delivery(
            state,
            merchant_id,
            status_code,
            recipient_data,
            ScheduleWebhookRetry::WithProcessTracker(Box::new(pt)),
            "An error occurred when sending webhook to merchant/connector",
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

#[async_trait::async_trait]
impl WebhookNotificationHandler for ManualRetry {
    async fn notified_action(
        &self,
        _state: SessionState,
        _merchant_key_store: domain::MerchantKeyStore,
        _updated_event: &domain::Event,
        _process_tracker: Option<storage::ProcessTracker>,
        recipient_data: WebhookRecipientData,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        super::utils::increment_webhook_outgoing_received_count(&recipient_data);
        None
    }

    async fn not_notified_action(
        &self,
        state: SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        status_code: u16,
        recipient_data: WebhookRecipientData,
        _process_tracker: Option<storage::ProcessTracker>,
    ) -> Option<Report<errors::WebhooksFlowError>> {
        super::utils::handle_failed_delivery(
            state,
            merchant_id,
            status_code,
            recipient_data,
            ScheduleWebhookRetry::NoSchedule,
            "Ignoring error when sending webhook to merchant/connector",
        )
        .await
        .err()
        .map(|error| report!(error))
    }
}

pub(crate) fn get_action_handler(
    attempt: enums::WebhookDeliveryAttempt,
) -> Box<dyn WebhookNotificationHandler> {
    match attempt {
        enums::WebhookDeliveryAttempt::InitialAttempt => Box::new(InitialAttempt),
        enums::WebhookDeliveryAttempt::AutomaticRetry => Box::new(AutomaticRetry),
        enums::WebhookDeliveryAttempt::ManualRetry => Box::new(ManualRetry),
    }
}

#[async_trait::async_trait]
impl OutgoingWebhookResponseHandler for enums::WebhookDeliveryAttempt {
    async fn handle_success_response(
        &self,
        state: SessionState,
        merchant_key_store: domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        event_id: &str,
        process_tracker: Option<storage::ProcessTracker>,
        response_content: webhook_events::OutgoingWebhookResponseContent,
        is_webhook_notified: bool,
        recipient_data: WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    > {
        let updated_event = super::utils::update_event_in_storage(
            state.clone(),
            is_webhook_notified,
            response_content,
            merchant_key_store,
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
                    process_tracker,
                    recipient_data,
                )
                .await
        } else {
            webhook_action_handler
                .not_notified_action(
                    state,
                    merchant_id,
                    response_content.status_code.unwrap_or(0),
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
        recipient_data: WebhookRecipientData,
    ) -> CustomResult<
        (domain::Event, Option<Report<errors::WebhooksFlowError>>),
        errors::WebhooksFlowError,
    > {
        let schedule_webhook_retry = match self {
            Self::InitialAttempt | Self::ManualRetry => ScheduleWebhookRetry::NoSchedule,
            Self::AutomaticRetry => process_tracker
                .map(|pt| ScheduleWebhookRetry::WithProcessTracker(Box::new(pt)))
                .unwrap_or(ScheduleWebhookRetry::NoSchedule),
        };

        super::utils::api_client_error_handler(
            state,
            merchant_key_store,
            merchant_id,
            event_id,
            client_error,
            schedule_webhook_retry,
            recipient_data,
        )
        .await
    }
}
