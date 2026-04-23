//! Gateway abstraction for incoming webhook processing.
//!
//! Mirrors the payment gateway pattern in `hyperswitch_interfaces::api::gateway`:
//! a single `execute` method satisfied by every implementation, with the
//! dispatcher selecting between Direct (HS connector trait) and UCS (gRPC)
//! paths per request. Each implementation is stateless and self-contained.

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use async_trait::async_trait;
use common_enums::{ExecutionMode, ExecutionPath};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_interfaces::webhooks::{
    IncomingWebhookRequestDetails, WebhookContext, WebhookResourceData,
};
use hyperswitch_masking::{ErasedMaskSerialize, Secret};
use router_env::{logger, tracing::Instrument};
use time::OffsetDateTime;
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments::helpers::MerchantConnectorAccountType,
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            build_webhook_secrets_from_merchant_connector_account,
        },
        webhooks::{incoming::get_payment_attempt_from_object_reference_id, utils as webhook_utils},
    },
    routes::SessionState,
    services::connector_integration_interface::ConnectorEnum,
    types::{api::IncomingWebhook, domain, transformers::ForeignTryFrom},
    utils as helper_utils,
};

// ---------------------------------------------------------------------------
// Public types.
// ---------------------------------------------------------------------------

/// Payload shape returned by a successful webhook processing run.
///
/// `Raw` carries masked JSON bytes of the connector's native webhook body;
/// downstream PSync invokes the connector's response parser on these bytes.
/// `UnifiedBytes` carries JSON bytes of a UCS-unified `EventContent`;
/// downstream PSync consumes the unified response directly without a
/// connector round-trip.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum WebhookContent {
    // connector-native bytes
    Direct(Vec<u8>),
    // UCS unified EventContent bytes
    UnifiedConnectorService(Vec<u8>),
}

impl WebhookContent {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Direct(b) | Self::UnifiedConnectorService(b) => b,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Direct(b) | Self::UnifiedConnectorService(b) => b,
        }
    }
}

/// Outcome of a single gateway run. Either the event was filtered out before
/// credentialed processing, or it was processed to completion.
pub enum WebhookGatewayOutcome {
    Skipped {
        reference: Option<ObjectReferenceId>,
        event_type: IncomingWebhookEvent,
    },
    Processed {
        reference: Option<ObjectReferenceId>,
        event_type: IncomingWebhookEvent,
        source_verified: bool,
        content: WebhookContent,
        // Masked view of the resource object for API-event logs. Separate
        // from `content` because downstream processing needs unmasked bytes.
        masked_log_payload: serde_json::Value,
        merchant_connector_account: domain::MerchantConnectorAccount,
    },
}

impl WebhookGatewayOutcome {
    pub fn event_type(&self) -> IncomingWebhookEvent {
        match self {
            Self::Skipped { event_type, .. } | Self::Processed { event_type, .. } => *event_type,
        }
    }

    pub fn reference(&self) -> Option<&ObjectReferenceId> {
        match self {
            Self::Skipped { reference, .. } | Self::Processed { reference, .. } => {
                reference.as_ref()
            }
        }
    }
}

/// Runtime dependencies supplied to every gateway run.
#[derive(Clone)]
pub struct WebhookGatewayContext<'a> {
    pub state: &'a SessionState,
    pub platform: &'a domain::Platform,
    pub connector: &'a ConnectorEnum,
    pub connector_name: &'a str,
    /// Pre-resolved merchant-connector-account if the webhook URL carries the
    /// merchant-connector id. `None` when the URL identifies only the
    /// connector; the implementation resolves the MCA from the parsed event
    /// reference in that case.
    pub merchant_connector_account: Option<&'a domain::MerchantConnectorAccount>,
}

// ---------------------------------------------------------------------------
// The trait.
// ---------------------------------------------------------------------------

/// Single-method gateway for incoming webhook processing. Implementations are
/// stateless: one instance per request, `execute` is called once.
#[async_trait]
pub trait IncomingWebhookGateway: Send + Sync {
    async fn execute(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        ctx: &WebhookGatewayContext<'_>,
    ) -> RouterResult<WebhookGatewayOutcome>;
}

/// Result of the "should downstream business logic run" filter.
pub enum FilterDecision {
    Skip,
    Proceed,
}

impl FilterDecision {
    pub async fn evaluate(
        event_type: IncomingWebhookEvent,
        ctx: &WebhookGatewayContext<'_>,
    ) -> Self {
        let supported = !matches!(event_type, IncomingWebhookEvent::EventNotSupported);
        let enabled = !webhook_utils::is_webhook_event_disabled(
            &*ctx.state.store,
            ctx.connector_name,
            ctx.platform.get_processor().get_account().get_id(),
            &event_type,
        )
        .await;
        let flow: api_models::webhooks::WebhookFlow = event_type.into();
        let return_response = matches!(flow, api_models::webhooks::WebhookFlow::ReturnResponse);

        if supported && enabled && !return_response {
            Self::Proceed
        } else {
            Self::Skip
        }
    }
}

// ---------------------------------------------------------------------------
// Direct (HS connector trait) implementation.
// ---------------------------------------------------------------------------

/// Processes the webhook through the connector's `IncomingWebhook` trait
/// methods. No gRPC traffic.
pub struct DirectIncomingWebhookGateway;

#[async_trait]
impl IncomingWebhookGateway for DirectIncomingWebhookGateway {
    async fn execute(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        ctx: &WebhookGatewayContext<'_>,
    ) -> RouterResult<WebhookGatewayOutcome> {
        let reference = ctx.connector.get_webhook_object_reference_id(request).ok();

        let mca = resolve_mca(ctx, reference.as_ref()).await?;

        // Decode before extracting event/resource — connectors may transform
        // the raw body (e.g. signed or encoded payloads).
        let decoded_body = ctx
            .connector
            .decode_webhook_body(
                request,
                ctx.platform.get_processor().get_account().get_id(),
                mca.connector_webhook_details.clone(),
                ctx.connector_name,
            )
            .await
            .switch()
            .attach_printable("Failed to decode incoming webhook body")?;

        let decoded_request = IncomingWebhookRequestDetails {
            method: request.method.clone(),
            uri: request.uri.clone(),
            headers: request.headers,
            query_params: request.query_params.clone(),
            body: &decoded_body,
        };

        let webhook_context = build_webhook_context(ctx.state, ctx.platform, reference.as_ref())
            .await
            .ok()
            .flatten();

        let event_type = ctx
            .connector
            .get_webhook_event_type(&decoded_request, webhook_context.as_ref())
            .switch()
            .attach_printable("Failed to classify webhook event type")?;

        let outcome = match FilterDecision::evaluate(event_type, ctx).await {
            FilterDecision::Skip => WebhookGatewayOutcome::Skipped {
                reference,
                event_type,
            },
            FilterDecision::Proceed => {
                let source_verified = verify_direct_source(ctx, &decoded_request, &mca).await?;
                let resource_object = ctx
                    .connector
                    .get_webhook_resource_object(&decoded_request)
                    .switch()
                    .attach_printable("Failed to extract webhook resource object")?;
                let bytes = serde_json::to_vec(&resource_object)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encode webhook resource object")?;
                // Log-only view; failure here must not fail the webhook.
                let masked_log_payload = resource_object
                    .masked_serialize()
                    .unwrap_or_else(|error| {
                        logger::warn!(?error, "Failed to mask-serialize webhook resource object for logging");
                        serde_json::Value::Null
                    });

                WebhookGatewayOutcome::Processed {
                    reference,
                    event_type,
                    source_verified,
                    content: WebhookContent::Direct(bytes),
                    masked_log_payload,
                    merchant_connector_account: mca,
                }
            }
        };

        Ok(outcome)
    }
}

// ---------------------------------------------------------------------------
// UCS (gRPC two-phase) implementation.
// ---------------------------------------------------------------------------

/// Processes the webhook through the Unified Connector Service. The first
/// RPC (`ParseEvent`) extracts the reference and event type without
/// credentials. When the event passes the filter, the second RPC
/// (`HandleEvent`) verifies the source and returns a unified response body.
pub struct UcsIncomingWebhookGateway {
    pub execution_mode: ExecutionMode,
}

impl UcsIncomingWebhookGateway {
    pub fn new(execution_mode: ExecutionMode) -> Self {
        Self { execution_mode }
    }
}

#[async_trait]
impl IncomingWebhookGateway for UcsIncomingWebhookGateway {
    async fn execute(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        ctx: &WebhookGatewayContext<'_>,
    ) -> RouterResult<WebhookGatewayOutcome> {
        let client = ctx
            .state
            .grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("UCS client is not configured")
            })?;

        let request_proto = request_details_to_grpc(request)?;

        // ParseEvent is pre-credential: the UCS server only needs the connector
        // name to dispatch to the right plugin, and the plugin parses webhook
        // bytes locally without calling the upstream connector. Credentials are
        // required only for HandleEvent.
        let parse_response = client
            .incoming_webhook_parse_event(
                payments_grpc::EventServiceParseRequest {
                    request_details: Some(request_proto.clone()),
                },
                build_ucs_auth_metadata(ctx, None)?,
                build_ucs_headers(ctx, None, self.execution_mode),
            )
            .await
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("UCS ParseEvent call failed")?
            .into_inner();

        let reference = match parse_response.reference.as_ref() {
            Some(r) => event_reference_to_object_ref(r)?,
            None => None,
        };
        let event_type = parse_response
            .event_type
            .map(IncomingWebhookEvent::from_ucs_event_type)
            .unwrap_or(IncomingWebhookEvent::EventNotSupported);

        let outcome = match FilterDecision::evaluate(event_type, ctx).await {
            FilterDecision::Skip => WebhookGatewayOutcome::Skipped {
                reference,
                event_type,
            },
            FilterDecision::Proceed => {
                let mca = resolve_mca(ctx, reference.as_ref()).await?;
                let webhook_secrets = build_webhook_secrets_from_merchant_connector_account(
                    &MerchantConnectorAccountType::DbVal(Box::new(mca.clone())),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to resolve webhook secrets from merchant connector account")?;

                let handle_response = client
                    .incoming_webhook_handle_event(
                        payments_grpc::EventServiceHandleRequest {
                            merchant_event_id: Some(build_merchant_event_id(ctx)),
                            request_details: Some(request_proto),
                            webhook_secrets,
                            access_token: None,
                            event_context: None,
                        },
                        build_ucs_auth_metadata(ctx, Some(&mca))?,
                        build_ucs_headers(ctx, Some(&mca), self.execution_mode),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("UCS HandleEvent call failed")?
                    .into_inner();

                let (bytes, masked_log_payload) = match handle_response.event_content.as_ref() {
                    Some(content) => {
                        let bytes = serde_json::to_vec(content)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to encode unified event content")?;
                        // Log-only view; failure here must not fail the webhook.
                        let masked = content.masked_serialize().unwrap_or_else(|error| {
                            logger::warn!(?error, "Failed to mask-serialize unified event content for logging");
                            serde_json::Value::Null
                        });
                        (bytes, masked)
                    }
                    None => (Vec::new(), serde_json::Value::Null),
                };

                WebhookGatewayOutcome::Processed {
                    reference,
                    event_type,
                    source_verified: handle_response.source_verified,
                    content: WebhookContent::UnifiedConnectorService(bytes),
                    masked_log_payload,
                    merchant_connector_account: mca,
                }
            }
        };

        Ok(outcome)
    }
}

// ---------------------------------------------------------------------------
// Dispatcher.
// ---------------------------------------------------------------------------

/// Entry point. Selects the gateway implementation for the given execution
/// path and runs it. In shadow mode the primary (Direct) run is returned to
/// the caller synchronously while a UCS run is fired in the background and
/// its result diffed against the primary.
pub async fn execute_incoming_webhook_gateway(
    ctx: &WebhookGatewayContext<'_>,
    request: &IncomingWebhookRequestDetails<'_>,
    execution_path: ExecutionPath,
) -> RouterResult<WebhookGatewayOutcome> {
    match execution_path {
        ExecutionPath::Direct => DirectIncomingWebhookGateway.execute(request, ctx).await,
        ExecutionPath::UnifiedConnectorService => {
            UcsIncomingWebhookGateway::new(ExecutionMode::Primary)
                .execute(request, ctx)
                .await
        }
        ExecutionPath::ShadowUnifiedConnectorService => {
            let direct_outcome = DirectIncomingWebhookGateway.execute(request, ctx).await?;
            spawn_shadow_ucs_run(ctx, request, &direct_outcome);
            Ok(direct_outcome)
        }
    }
}

// ---------------------------------------------------------------------------
// Shadow-mode orchestration.
// ---------------------------------------------------------------------------

fn spawn_shadow_ucs_run(
    ctx: &WebhookGatewayContext<'_>,
    request: &IncomingWebhookRequestDetails<'_>,
    primary: &WebhookGatewayOutcome,
) {
    let state = ctx.state.clone();
    let platform = ctx.platform.clone();
    let connector = ctx.connector.clone();
    let connector_name = ctx.connector_name.to_owned();
    let mca_owned = ctx.merchant_connector_account.cloned();
    let request_owned = OwnedRequestDetails::from(request);
    let primary_snapshot = OutcomeSnapshot::from(primary);

    tokio::spawn(
        async move {
            let request_ref = request_owned.borrow();
            let inner_ctx = WebhookGatewayContext {
                state: &state,
                platform: &platform,
                connector: &connector,
                connector_name: connector_name.as_str(),
                merchant_connector_account: mca_owned.as_ref(),
            };
            match UcsIncomingWebhookGateway::new(ExecutionMode::Shadow)
                .execute(&request_ref, &inner_ctx)
                .await
            {
                Ok(shadow_outcome) => {
                    let shadow_snapshot = OutcomeSnapshot::from(&shadow_outcome);
                    report_shadow_diff(&state, &connector_name, &primary_snapshot, &shadow_snapshot)
                        .await;
                }
                Err(error) => logger::warn!(?error, "UCS shadow webhook run failed"),
            }
        }
        .in_current_span(),
    );
}

struct OwnedRequestDetails {
    method: http::Method,
    uri: http::Uri,
    headers: actix_web::http::header::HeaderMap,
    body: Vec<u8>,
    query_params: String,
}

impl From<&IncomingWebhookRequestDetails<'_>> for OwnedRequestDetails {
    fn from(req: &IncomingWebhookRequestDetails<'_>) -> Self {
        Self {
            method: req.method.clone(),
            uri: req.uri.clone(),
            headers: req.headers.clone(),
            body: req.body.to_vec(),
            query_params: req.query_params.clone(),
        }
    }
}

impl OwnedRequestDetails {
    fn borrow(&self) -> IncomingWebhookRequestDetails<'_> {
        IncomingWebhookRequestDetails {
            method: self.method.clone(),
            uri: self.uri.clone(),
            headers: &self.headers,
            body: self.body.as_slice(),
            query_params: self.query_params.clone(),
        }
    }
}

#[derive(serde::Serialize)]
struct OutcomeSnapshot {
    variant: &'static str,
    event_type: IncomingWebhookEvent,
    source_verified: Option<bool>,
    content_kind: Option<&'static str>,
    reference: Option<ObjectReferenceId>,
}

impl From<&WebhookGatewayOutcome> for OutcomeSnapshot {
    fn from(outcome: &WebhookGatewayOutcome) -> Self {
        match outcome {
            WebhookGatewayOutcome::Skipped {
                event_type,
                reference,
            } => Self {
                variant: "skipped",
                event_type: *event_type,
                source_verified: None,
                content_kind: None,
                reference: reference.clone(),
            },
            WebhookGatewayOutcome::Processed {
                event_type,
                source_verified,
                content,
                reference,
                ..
            } => Self {
                variant: "processed",
                event_type: *event_type,
                source_verified: Some(*source_verified),
                content_kind: Some(match content {
                    WebhookContent::Direct(_) => "direct",
                    WebhookContent::UnifiedConnectorService(_) => "unified_connector_service",
                }),
                reference: reference.clone(),
            },
        }
    }
}

/// Logs a shadow diff summary and, when a comparison service is configured,
/// POSTs both outcomes to it for offline validation.
async fn report_shadow_diff(
    state: &SessionState,
    connector_name: &str,
    primary: &OutcomeSnapshot,
    shadow: &OutcomeSnapshot,
) {
    logger::info!(
        primary_event_type = ?primary.event_type,
        shadow_event_type = ?shadow.event_type,
        event_type_match = primary.event_type == shadow.event_type,
        primary_source_verified = ?primary.source_verified,
        shadow_source_verified = ?shadow.source_verified,
        "Webhook shadow diff",
    );

    use hyperswitch_interfaces::{
        api_client::ApiClientWrapper, helpers::GetComparisonServiceConfig,
    };
    if let Some(config) = state.get_comparison_service_config() {
        hyperswitch_interfaces::helpers::serialize_webhook_outcome_and_send_to_comparison_service(
            state,
            primary,
            shadow,
            config,
            connector_name.to_string(),
            state.get_request_id_str(),
        )
        .await;
    }
}

// ---------------------------------------------------------------------------
// Internal helpers.
// ---------------------------------------------------------------------------

async fn resolve_mca(
    ctx: &WebhookGatewayContext<'_>,
    reference: Option<&ObjectReferenceId>,
) -> RouterResult<domain::MerchantConnectorAccount> {
    match (ctx.merchant_connector_account, reference) {
        (Some(mca), _) => Ok(mca.clone()),
        (None, Some(reference)) => Box::pin(helper_utils::get_mca_from_object_reference_id(
            ctx.state,
            reference.clone(),
            ctx.platform,
            ctx.connector_name,
        ))
        .await,
        (None, None) => Err(error_stack::report!(
            errors::ApiErrorResponse::WebhookResourceNotFound
        )
        .attach_printable(
            "Webhook URL did not include a merchant-connector id and the event carries no resource reference",
        )),
    }
}

async fn build_webhook_context(
    state: &SessionState,
    platform: &domain::Platform,
    reference: Option<&ObjectReferenceId>,
) -> RouterResult<Option<WebhookContext>> {
    match reference {
        Some(reference @ ObjectReferenceId::PaymentId(_)) => Ok(
            get_payment_attempt_from_object_reference_id(
                state,
                reference.clone(),
                platform.get_processor(),
            )
            .await
            .ok()
            .map(|payment_attempt| {
                let data = WebhookResourceData::Payment { payment_attempt };
                WebhookContext::from(&data)
            }),
        ),
        _ => Ok(None),
    }
}

async fn verify_direct_source(
    ctx: &WebhookGatewayContext<'_>,
    request: &IncomingWebhookRequestDetails<'_>,
    mca: &domain::MerchantConnectorAccount,
) -> RouterResult<bool> {
    ctx.connector
        .clone()
        .verify_webhook_source(
            request,
            ctx.platform.get_processor().get_account().get_id(),
            mca.connector_webhook_details.clone(),
            mca.connector_account_details.clone(),
            ctx.connector_name,
        )
        .await
        .or_else(|error| match error.current_context() {
            hyperswitch_interfaces::errors::ConnectorError::WebhookSourceVerificationFailed => {
                logger::warn!(?error, "Webhook source verification returned failure");
                Ok(false)
            }
            _ => Err(error),
        })
        .switch()
        .attach_printable("Webhook source verification errored")
}

fn request_details_to_grpc(
    request: &IncomingWebhookRequestDetails<'_>,
) -> RouterResult<payments_grpc::RequestDetails> {
    <payments_grpc::RequestDetails as ForeignTryFrom<&IncomingWebhookRequestDetails<'_>>>::foreign_try_from(
        request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to translate webhook request details to gRPC format")
}

/// Builds the gRPC auth envelope. With an MCA, credentials come from connector
/// account details. Without one (ParseEvent runs pre-MCA), only the routing
/// fields (`connector_name`, `auth_type`, `merchant_id`) are set; credential
/// fields stay `None` and the UCS client header builder skips them.
fn build_ucs_auth_metadata(
    ctx: &WebhookGatewayContext<'_>,
    mca: Option<&domain::MerchantConnectorAccount>,
) -> RouterResult<external_services::grpc_client::unified_connector_service::ConnectorAuthMetadata>
{
    match mca {
        Some(mca) => build_unified_connector_service_auth_metadata(
            MerchantConnectorAccountType::DbVal(Box::new(mca.clone())),
            ctx.platform.get_processor(),
            ctx.connector_name.to_string(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build UCS auth metadata"),
        None => {
            let merchant_id = ctx
                .platform
                .get_processor()
                .get_account()
                .get_id()
                .get_string_repr()
                .to_string();
            Ok(
                external_services::grpc_client::unified_connector_service::ConnectorAuthMetadata {
                    connector_name: ctx.connector_name.to_string(),
                    auth_type: "NoKey".to_string(),
                    api_key: None,
                    key1: None,
                    key2: None,
                    api_secret: None,
                    auth_key_map: None,
                    merchant_id: Secret::new(merchant_id),
                    connector_config: None,
                },
            )
        }
    }
}

/// Builds gRPC headers. `profile_id` comes from the MCA when available; a
/// placeholder stands in when the MCA is not yet resolved.
fn build_ucs_headers(
    ctx: &WebhookGatewayContext<'_>,
    mca: Option<&domain::MerchantConnectorAccount>,
    mode: ExecutionMode,
) -> external_services::grpc_client::GrpcHeadersUcs {
    let merchant_id = ctx
        .platform
        .get_processor()
        .get_account()
        .get_id()
        .clone();
    let profile_id = mca
        .map(|m| m.profile_id.clone())
        .unwrap_or_else(|| consts::PROFILE_ID_UNAVAILABLE.clone());
    ctx.state
        .get_grpc_headers_ucs(mode)
        .lineage_ids(LineageIds::new(merchant_id, profile_id))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .build()
}

fn build_merchant_event_id(ctx: &WebhookGatewayContext<'_>) -> String {
    format!(
        "{}_{}_{}",
        ctx.platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
        ctx.connector_name,
        OffsetDateTime::now_utc().unix_timestamp()
    )
}

fn event_reference_to_object_ref(
    reference: &payments_grpc::EventReference,
) -> RouterResult<Option<ObjectReferenceId>> {
    use api_models::payments as api_payments;
    use api_models::webhooks as api_webhooks;
    use payments_grpc::event_reference::Resource;

    let Some(resource) = reference.resource.as_ref() else {
        return Ok(None);
    };

    let out = match resource {
        Resource::Payment(payment) => {
            if let Some(ctx_id) = payment.connector_transaction_id.as_ref() {
                Some(ObjectReferenceId::PaymentId(
                    api_payments::PaymentIdType::ConnectorTransactionId(ctx_id.clone()),
                ))
            } else if let Some(mref) = payment.merchant_transaction_id.as_ref() {
                let payment_id = common_utils::id_type::PaymentId::try_from(
                    std::borrow::Cow::Owned(mref.clone()),
                )
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Invalid merchant_transaction_id in UCS payment reference")?;
                Some(ObjectReferenceId::PaymentId(
                    api_payments::PaymentIdType::PaymentIntentId(payment_id),
                ))
            } else {
                None
            }
        }
        Resource::Refund(refund) => {
            if let Some(cr_id) = refund.connector_refund_id.as_ref() {
                Some(ObjectReferenceId::RefundId(
                    api_webhooks::RefundIdType::ConnectorRefundId(cr_id.clone()),
                ))
            } else {
                refund.merchant_refund_id.as_ref().map(|mid| {
                    ObjectReferenceId::RefundId(api_webhooks::RefundIdType::RefundId(mid.clone()))
                })
            }
        }
        Resource::Dispute(dispute) => dispute
            .connector_dispute_id
            .as_ref()
            .or(dispute.connector_transaction_id.as_ref())
            .map(|id| {
                ObjectReferenceId::PaymentId(api_payments::PaymentIdType::ConnectorTransactionId(
                    id.clone(),
                ))
            }),
        Resource::Mandate(mandate) => mandate.connector_mandate_id.as_ref().map(|id| {
            ObjectReferenceId::MandateId(api_webhooks::MandateIdType::ConnectorMandateId(id.clone()))
        }),
        #[cfg(feature = "payouts")]
        Resource::Payout(payout) => {
            if let Some(cid) = payout.connector_payout_id.as_ref() {
                Some(ObjectReferenceId::PayoutId(
                    api_webhooks::PayoutIdType::ConnectorPayoutId(cid.clone()),
                ))
            } else {
                payout.merchant_payout_id.as_ref().map(|mid| {
                    ObjectReferenceId::PayoutId(api_webhooks::PayoutIdType::PayoutAttemptId(
                        mid.clone(),
                    ))
                })
            }
        }
        #[cfg(not(feature = "payouts"))]
        Resource::Payout(_) => None,
    };

    Ok(out)
}
