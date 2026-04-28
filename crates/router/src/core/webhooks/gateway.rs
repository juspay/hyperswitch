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
        errors::{self, utils::ConnectorErrorExt, RouterResult},
        metrics,
        payments::helpers::MerchantConnectorAccountType,
        unified_connector_service::{
            self, build_unified_connector_service_auth_metadata,
            build_webhook_secrets_from_merchant_connector_account, transformers::HandleEventInputs,
        },
        webhooks::{
            incoming::get_payment_attempt_from_object_reference_id, utils as webhook_utils,
            MERCHANT_ID,
        },
    },
    routes::SessionState,
    services::{self, connector_integration_interface::ConnectorEnum},
    types::{api::IncomingWebhook, domain, transformers::ForeignTryFrom},
    utils as helper_utils,
};

// ---------------------------------------------------------------------------
// Public types.
// ---------------------------------------------------------------------------

/// Payload shape returned by a successful webhook processing run.
///
/// `Direct` carries JSON bytes of the connector's native webhook resource
/// object; downstream PSync invokes the connector's response parser on these
/// bytes. `UnifiedConnectorService` carries JSON bytes of a UCS-unified
/// `EventContent`; downstream PSync consumes the unified response directly
/// without a connector round-trip.
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

/// Outcome of a single inbound-webhook processing run. Returned by the Direct
/// and UCS gateway implementations and by the UAS path (which is not a
/// gateway). Either the event was filtered out before credentialed
/// processing, or processing ran to completion.
///
/// Each producer builds its own `ack_response` — the HTTP body sent back to
/// the calling connector. Outer code treats it as opaque.
pub enum WebhookOutcome {
    Skipped {
        /// Best-effort reference. May be absent when the filter rejects the
        /// event before (or without) a reference fetch — kept for observability
        /// only, since business logic doesn't run.
        reference: Option<ObjectReferenceId>,
        event_type: IncomingWebhookEvent,
        ack_response: services::ApplicationResponse<serde_json::Value>,
    },
    Processed {
        /// Mandatory for a processed webhook: every downstream business-logic
        /// branch (payment sync, refund, payout, mandate, dispute, relay) keys
        /// off this to look up the target resource. Gateway impls must error
        /// out rather than return `Processed` without one.
        reference: ObjectReferenceId,
        event_type: IncomingWebhookEvent,
        source_verified: bool,
        content: WebhookContent,
        // Masked view of the resource object for API-event logs. Separate
        // from `content` because downstream processing needs unmasked bytes.
        masked_log_payload: common_utils::pii::SecretSerdeValue,
        merchant_connector_account: Box<domain::MerchantConnectorAccount>,
        ack_response: services::ApplicationResponse<serde_json::Value>,
    },
}

impl WebhookOutcome {
    pub fn event_type(&self) -> IncomingWebhookEvent {
        match self {
            Self::Skipped { event_type, .. } | Self::Processed { event_type, .. } => *event_type,
        }
    }

    pub fn reference(&self) -> Option<&ObjectReferenceId> {
        match self {
            Self::Skipped { reference, .. } => reference.as_ref(),
            Self::Processed { reference, .. } => Some(reference),
        }
    }
}

/// Runtime dependencies supplied to every gateway run. Owned (not borrowed)
/// so the dispatcher can hand the same context off to the synchronous primary
/// run and a `tokio::spawn`-ed shadow run without per-field cloning. Mirrors
/// `RouterGatewayContext` in the payments path. All inner fields are
/// `Arc`-wrapped or small, so `clone()` is cheap.
#[derive(Clone)]
pub struct WebhookGatewayContext {
    pub state: SessionState,
    pub platform: domain::Platform,
    pub connector: ConnectorEnum,
    pub connector_name: String,
    /// Pre-resolved merchant-connector-account if the webhook URL carries the
    /// merchant-connector id. `None` when the URL identifies only the
    /// connector; the implementation resolves the MCA from the parsed event
    /// reference in that case.
    pub merchant_connector_account: Option<domain::MerchantConnectorAccount>,
    /// Selected execution path for this webhook invocation. The dispatcher
    /// matches on this rather than carrying it as a separate argument.
    pub execution_path: ExecutionPath,
    /// Derived from `execution_path`: `UnifiedConnectorService → Primary`,
    /// `ShadowUnifiedConnectorService → Shadow`, `Direct → NotApplicable`.
    /// Carried alongside `execution_path` rather than recomputed at every UCS
    /// gateway entry point.
    pub execution_mode: ExecutionMode,
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
        ctx: &WebhookGatewayContext,
    ) -> RouterResult<WebhookOutcome>;
}

/// Result of the "should downstream business logic run" filter.
pub enum FilterDecision {
    Skip,
    Proceed,
}

impl FilterDecision {
    pub async fn evaluate(
        event_type: IncomingWebhookEvent,
        ctx: &WebhookGatewayContext,
    ) -> Self {
        let supported = !matches!(event_type, IncomingWebhookEvent::EventNotSupported);
        let enabled = !webhook_utils::is_webhook_event_disabled(
            &*ctx.state.store,
            &ctx.connector_name,
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
        ctx: &WebhookGatewayContext,
    ) -> RouterResult<WebhookOutcome> {
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
                &ctx.connector_name,
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

        // Some connectors (signed-body envelopes, encoded payloads) only expose
        // the object reference after the body is decoded. The pre-decode fetch
        // above is best-effort; retry against the decoded body so connectors
        // get a populated `WebhookContext` and `event_type` classification.
        let reference = reference.or_else(|| {
            ctx.connector
                .get_webhook_object_reference_id(&decoded_request)
                .ok()
        });

        let webhook_context =
            build_webhook_context(&ctx.state, &ctx.platform, reference.as_ref())
                .await
                .ok()
                .flatten();

        let event_type = ctx
            .connector
            .get_webhook_event_type(&decoded_request, webhook_context.as_ref())
            .allow_webhook_event_type_not_found(
                ctx.state
                    .conf
                    .webhooks
                    .ignore_error
                    .event_type
                    .unwrap_or(true),
            )
            .switch()
            .attach_printable("Could not find event type in incoming webhook body")?
            .unwrap_or_else(|| {
                metrics::WEBHOOK_EVENT_TYPE_IDENTIFICATION_FAILURE_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        (
                            MERCHANT_ID,
                            ctx.platform.get_processor().get_account().get_id().clone()
                        ),
                        ("connector", ctx.connector_name.to_string())
                    ),
                );
                IncomingWebhookEvent::EventNotSupported
            });

        let ack_response = ctx
            .connector
            .get_webhook_api_response(
                &decoded_request,
                None,
                Some(mca.connector_account_details.clone()),
            )
            .switch()
            .attach_printable("Failed to build webhook ack via connector")?;

        let outcome = match FilterDecision::evaluate(event_type, ctx).await {
            FilterDecision::Skip => WebhookOutcome::Skipped {
                reference,
                event_type,
                ack_response,
            },
            FilterDecision::Proceed => {
                // Reference is required for business logic. We've already
                // retried against the decoded body above, so a `None` here
                // means the connector genuinely couldn't extract one.
                let reference = reference.ok_or_else(|| {
                    error_stack::report!(errors::ApiErrorResponse::WebhookResourceNotFound)
                        .attach_printable(
                            "Could not find object reference id in incoming webhook body",
                        )
                })?;
                let source_verified =
                    verify_webhook_source_via_connector(ctx, &decoded_request, &mca).await?;
                let resource_object = ctx
                    .connector
                    .get_webhook_resource_object(&decoded_request)
                    .switch()
                    .attach_printable("Failed to extract webhook resource object")?;
                let bytes = serde_json::to_vec(&resource_object)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encode webhook resource object")?;
                // Log-only view; failure here must not fail the webhook.
                let masked_log_payload = Secret::new(
                    resource_object.masked_serialize().unwrap_or_else(|error| {
                        logger::warn!(
                            ?error,
                            "Failed to mask-serialize webhook resource object for logging"
                        );
                        serde_json::Value::Null
                    }),
                );

                WebhookOutcome::Processed {
                    reference,
                    event_type,
                    source_verified,
                    content: WebhookContent::Direct(bytes),
                    masked_log_payload,
                    merchant_connector_account: Box::new(mca),
                    ack_response,
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
///
/// Stateless — `execution_mode` lives on `WebhookGatewayContext` so the
/// dispatcher can switch primary/shadow without instantiating a new gateway.
pub struct UcsIncomingWebhookGateway;

#[async_trait]
impl IncomingWebhookGateway for UcsIncomingWebhookGateway {
    async fn execute(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        ctx: &WebhookGatewayContext,
    ) -> RouterResult<WebhookOutcome> {
        let client = ctx
            .state
            .grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("UCS client is not configured")
            })?
            .clone();

        let connector_name = ctx.connector_name.clone();
        let merchant_id = ctx.platform.get_processor().get_account().get_id().clone();
        // Stable id for this webhook invocation: shared by the ParseEvent and
        // HandleEvent connector-log entries and used verbatim as HandleEvent's
        // `merchant_event_id`.
        let merchant_event_id = build_merchant_event_id(ctx);

        // ParseEvent is pre-credential: the UCS server only needs the connector
        // name to dispatch to the right plugin, and the plugin parses webhook
        // bytes locally without calling the upstream connector. Credentials are
        // required only for HandleEvent.
        let parse_request = payments_grpc::EventServiceParseRequest::foreign_try_from(request)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to build UCS EventServiceParseRequest")?;
        let parse_auth = build_ucs_auth_metadata(ctx, None)?;
        let parse_headers = build_ucs_headers_builder(ctx, None, ctx.execution_mode);
        let parse_client = client.clone();
        let parse_response = unified_connector_service::ucs_webhook_logging_wrapper(
            &ctx.state,
            connector_name.clone(),
            "EventServiceParseEvent",
            merchant_id.clone(),
            merchant_event_id.clone(),
            parse_request,
            parse_headers,
            ctx.execution_mode,
            |request, headers| async move {
                parse_client
                    .incoming_webhook_parse_event(request, parse_auth, headers)
                    .await
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("UCS ParseEvent call failed")
                    .map(|response| response.into_inner())
            },
        )
        .await?;

        let reference = match parse_response.reference.as_ref() {
            Some(r) => event_reference_to_object_ref(r)?,
            None => None,
        };
        let event_type = parse_response
            .event_type
            .map(IncomingWebhookEvent::from_ucs_event_type)
            .unwrap_or(IncomingWebhookEvent::EventNotSupported);

        let outcome = match FilterDecision::evaluate(event_type, ctx).await {
            FilterDecision::Skip => {
                // HandleEvent isn't invoked for filtered events, so UCS has
                // no ack suggestion. Default to `StatusOk` — same as the HS
                // trait's default for an uncustomized `get_webhook_api_response`.
                WebhookOutcome::Skipped {
                    reference,
                    event_type,
                    ack_response: services::ApplicationResponse::StatusOk,
                }
            }
            FilterDecision::Proceed => {
                // A ParseEvent result that proceeds past the filter must carry
                // a reference — downstream business logic needs it to look up
                // the target resource. No reference + Proceed is a UCS server
                // inconsistency, not a normal outcome.
                let reference = reference.ok_or_else(|| {
                    error_stack::report!(errors::ApiErrorResponse::WebhookResourceNotFound)
                        .attach_printable(
                            "UCS ParseEvent produced no object reference for a non-filtered event",
                        )
                })?;
                let mca = resolve_mca(ctx, Some(&reference)).await?;
                let webhook_secrets = build_webhook_secrets_from_merchant_connector_account(
                    &MerchantConnectorAccountType::DbVal(Box::new(mca.clone())),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to resolve webhook secrets from merchant connector account",
                )?;

                let event_context = build_event_context(ctx, Some(&reference)).await;

                let handle_request =
                    payments_grpc::EventServiceHandleRequest::foreign_try_from(HandleEventInputs {
                        request_details: request,
                        webhook_secrets,
                        event_context,
                        merchant_event_id: merchant_event_id.clone(),
                    })
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to build UCS EventServiceHandleRequest")?;
                let handle_auth = build_ucs_auth_metadata(ctx, Some(&mca))?;
                let handle_headers =
                    build_ucs_headers_builder(ctx, Some(&mca), ctx.execution_mode);
                let handle_client = client.clone();
                let handle_response = unified_connector_service::ucs_webhook_logging_wrapper(
                    &ctx.state,
                    connector_name.clone(),
                    "EventServiceHandleEvent",
                    merchant_id.clone(),
                    merchant_event_id.clone(),
                    handle_request,
                    handle_headers,
                    ctx.execution_mode,
                    |request, headers| async move {
                        handle_client
                            .incoming_webhook_handle_event(request, handle_auth, headers)
                            .await
                            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                            .attach_printable("UCS HandleEvent call failed")
                            .map(|response| response.into_inner())
                    },
                )
                .await?;

                // `event_content` is required by the proto for a non-filtered
                // event — the downstream PSync gateway will try to deserialize
                // these bytes as `EventContent`. Erroring here surfaces the UCS
                // inconsistency at the right layer rather than letting empty
                // bytes blow up deep in the payment pipeline.
                let event_content = handle_response.event_content.ok_or_else(|| {
                    error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                        .attach_printable(
                            "UCS HandleEvent returned no event_content for a non-filtered event",
                        )
                })?;
                let bytes = serde_json::to_vec(&event_content)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encode unified event content")?;
                // Log-only view; failure here must not fail the webhook.
                let masked_log_payload = Secret::new(
                    event_content.masked_serialize().unwrap_or_else(|error| {
                        logger::warn!(
                            ?error,
                            "Failed to mask-serialize unified event content for logging"
                        );
                        serde_json::Value::Null
                    }),
                );

                // Use UCS's suggested ack when present; otherwise default to
                // `StatusOk` (200 empty body). Mirrors the HS connector-trait
                // default for `get_webhook_api_response` — "no customization →
                // 200 OK." No cross-path fallback to HS code.
                let ack_response = handle_response
                    .event_ack_response
                    .map(ucs_ack_to_application_response)
                    .unwrap_or(services::ApplicationResponse::StatusOk);

                WebhookOutcome::Processed {
                    reference,
                    event_type,
                    source_verified: handle_response.source_verified,
                    content: WebhookContent::UnifiedConnectorService(bytes),
                    masked_log_payload,
                    merchant_connector_account: Box::new(mca),
                    ack_response,
                }
            }
        };

        Ok(outcome)
    }
}

// ---------------------------------------------------------------------------
// Dispatcher.
// ---------------------------------------------------------------------------

/// Entry point. Selects the gateway implementation from `ctx.execution_path`
/// and runs it. In shadow mode the primary (Direct) run is returned to the
/// caller synchronously while a UCS run is fired in the background and its
/// result diffed against the primary.
pub async fn execute_incoming_webhook_gateway(
    ctx: &WebhookGatewayContext,
    request: &IncomingWebhookRequestDetails<'_>,
) -> RouterResult<WebhookOutcome> {
    match ctx.execution_path {
        ExecutionPath::Direct => DirectIncomingWebhookGateway.execute(request, ctx).await,
        ExecutionPath::UnifiedConnectorService => {
            UcsIncomingWebhookGateway.execute(request, ctx).await
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
    ctx: &WebhookGatewayContext,
    request: &IncomingWebhookRequestDetails<'_>,
    primary: &WebhookOutcome,
) {
    // Owned context — clone is cheap (Arc-wrapped or small fields). Force the
    // shadow run into UCS+Shadow regardless of how the parent was selected.
    let mut inner_ctx = ctx.clone();
    inner_ctx.execution_path = ExecutionPath::UnifiedConnectorService;
    inner_ctx.execution_mode = ExecutionMode::Shadow;

    let request_owned = OwnedRequestDetails::from(request);
    let primary_snapshot = WebhookShadowSnapshot::from(primary);

    tokio::spawn(
        async move {
            let request_ref = request_owned.borrow();
            match UcsIncomingWebhookGateway
                .execute(&request_ref, &inner_ctx)
                .await
            {
                Ok(shadow_outcome) => {
                    let shadow_snapshot = WebhookShadowSnapshot::from(&shadow_outcome);
                    report_shadow_diff(
                        &inner_ctx.state,
                        &inner_ctx.connector_name,
                        &primary_snapshot,
                        &shadow_snapshot,
                    )
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

#[derive(Debug, serde::Serialize)]
struct WebhookShadowSnapshot {
    variant: &'static str,
    event_type: IncomingWebhookEvent,
    source_verified: Option<bool>,
    content_kind: Option<&'static str>,
    reference: Option<ObjectReferenceId>,
}

impl From<&WebhookOutcome> for WebhookShadowSnapshot {
    fn from(outcome: &WebhookOutcome) -> Self {
        match outcome {
            WebhookOutcome::Skipped {
                event_type,
                reference,
                ..
            } => Self {
                variant: "skipped",
                event_type: *event_type,
                source_verified: None,
                content_kind: None,
                reference: reference.clone(),
            },
            WebhookOutcome::Processed {
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
                reference: Some(reference.clone()),
            },
        }
    }
}

/// Logs a shadow diff summary and, when a comparison service is configured,
/// POSTs both outcomes to it for offline validation.
async fn report_shadow_diff(
    state: &SessionState,
    connector_name: &str,
    primary: &WebhookShadowSnapshot,
    shadow: &WebhookShadowSnapshot,
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
    ctx: &WebhookGatewayContext,
    reference: Option<&ObjectReferenceId>,
) -> RouterResult<domain::MerchantConnectorAccount> {
    match (ctx.merchant_connector_account.as_ref(), reference) {
        (Some(mca), _) => Ok(mca.clone()),
        (None, Some(reference)) => Box::pin(helper_utils::get_mca_from_object_reference_id(
            &ctx.state,
            reference.clone(),
            &ctx.platform,
            &ctx.connector_name,
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
        Some(reference @ ObjectReferenceId::PaymentId(_)) => {
            // Swallow fetch failures: a missing payment_attempt is not a
            // webhook-level failure — `get_webhook_event_type` simply runs
            // without enriched context. Surface the underlying DB error via
            // `warn!` so silent misses are still observable.
            let payment_attempt = get_payment_attempt_from_object_reference_id(
                state,
                reference.clone(),
                platform.get_processor(),
            )
            .await
            .inspect_err(|error| {
                logger::warn!(
                    ?error,
                    "Failed to fetch payment_attempt for webhook context"
                );
            })
            .ok();
            Ok(payment_attempt.map(|payment_attempt| {
                let data = WebhookResourceData::Payment { payment_attempt };
                WebhookContext::from(&data)
            }))
        }
        _ => Ok(None),
    }
}

/// Runs the non-UCS source verification chain: either an outbound HTTP callback
/// to the connector's verification endpoint (for connectors listed in
/// `webhook_source_verification_call`) or an in-process `verify_webhook_source`
/// against the connector integration. Used by both the Direct gateway path and
/// the UAS path — UCS returns its own `source_verified` flag from `HandleEvent`.
pub(super) async fn verify_webhook_source_via_connector(
    ctx: &WebhookGatewayContext,
    request: &IncomingWebhookRequestDetails<'_>,
    mca: &domain::MerchantConnectorAccount,
) -> RouterResult<bool> {
    use std::str::FromStr;

    let connector_enum = api_models::enums::Connector::from_str(&ctx.connector_name)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| {
            format!("unable to parse connector name {:?}", ctx.connector_name)
        })?;

    let requires_external_source_verification = ctx
        .state
        .conf
        .webhook_source_verification_call
        .connectors_with_webhook_source_verification_call
        .contains(&connector_enum);

    if requires_external_source_verification {
        webhook_utils::verify_webhook_source_verification_call(
            ctx.connector.clone(),
            &ctx.state,
            &ctx.platform,
            mca.clone(),
            &ctx.connector_name,
            request,
        )
        .await
        .or_else(|error| match error.current_context() {
            errors::ConnectorError::WebhookSourceVerificationFailed => {
                logger::error!(?error, "Source Verification Failed");
                Ok(false)
            }
            _ => Err(error),
        })
        .switch()
        .attach_printable("There was an issue in incoming webhook source verification")
    } else {
        ctx.connector
            .clone()
            .verify_webhook_source(
                request,
                ctx.platform.get_processor().get_account().get_id(),
                mca.connector_webhook_details.clone(),
                mca.connector_account_details.clone(),
                &ctx.connector_name,
            )
            .await
            .or_else(|error| match error.current_context() {
                hyperswitch_interfaces::errors::ConnectorError::WebhookSourceVerificationFailed => {
                    logger::error!(?error, "Source Verification Failed");
                    Ok(false)
                }
                _ => Err(error),
            })
            .switch()
            .attach_printable("There was an issue in incoming webhook source verification")
    }
}

/// Builds the gRPC auth envelope. With an MCA, credentials come from connector
/// account details. Without one (ParseEvent runs pre-MCA), only the routing
/// fields (`connector_name`, `auth_type`, `merchant_id`) are set; credential
/// fields stay `None` and the UCS client header builder skips them.
fn build_ucs_auth_metadata(
    ctx: &WebhookGatewayContext,
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

/// Builds the UCS gRPC headers builder. `profile_id` comes from the MCA when
/// available; a placeholder stands in when the MCA is not yet resolved. The
/// caller (`ucs_webhook_logging_wrapper`) finalises the builder via `.build()`.
fn build_ucs_headers_builder(
    ctx: &WebhookGatewayContext,
    mca: Option<&domain::MerchantConnectorAccount>,
    mode: ExecutionMode,
) -> external_services::grpc_client::GrpcHeadersUcsBuilderFinal {
    let merchant_id = ctx.platform.get_processor().get_account().get_id().clone();
    let profile_id = mca
        .map(|m| m.profile_id.clone())
        .unwrap_or_else(|| consts::PROFILE_ID_UNAVAILABLE.clone());
    ctx.state
        .get_grpc_headers_ucs(mode)
        .lineage_ids(LineageIds::new(merchant_id, profile_id))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
}

fn build_merchant_event_id(ctx: &WebhookGatewayContext) -> String {
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
    use api_models::{payments as api_payments, webhooks as api_webhooks};
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
            } else {
                payment.merchant_transaction_id.as_ref().map(|mref| {
                    ObjectReferenceId::PaymentId(api_payments::PaymentIdType::PaymentAttemptId(
                        mref.clone(),
                    ))
                })
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
            ObjectReferenceId::MandateId(api_webhooks::MandateIdType::ConnectorMandateId(
                id.clone(),
            ))
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

async fn build_event_context(
    ctx: &WebhookGatewayContext,
    reference: Option<&ObjectReferenceId>,
) -> Option<payments_grpc::EventContext> {
    let payment_attempt = match reference {
        Some(reference @ ObjectReferenceId::PaymentId(_)) => {
            get_payment_attempt_from_object_reference_id(
                &ctx.state,
                reference.clone(),
                ctx.platform.get_processor(),
            )
            .await
            .ok()?
        }
        _ => return None,
    };

    let capture_method = payment_attempt
        .capture_method
        .and_then(|cm| payments_grpc::CaptureMethod::foreign_try_from(cm).ok())
        .map(i32::from);

    Some(payments_grpc::EventContext {
        event_context: Some(payments_grpc::event_context::EventContext::Payment(
            payments_grpc::PaymentEventContext { capture_method },
        )),
    })
}

/// Converts UCS's suggested ack into the `ApplicationResponse` shape the
/// outer webhook handler already uses. Best-effort: JSON body maps to
/// `Json` (or `JsonWithHeaders` when headers are present), non-JSON UTF-8
/// maps to `TextPlain`, empty body to `StatusOk`, binary to `FileData`.
/// Status code is not first-class in `ApplicationResponse`; only the 200
/// paths are represented faithfully.
fn ucs_ack_to_application_response(
    ack: payments_grpc::EventAckResponse,
) -> services::ApplicationResponse<serde_json::Value> {
    let payments_grpc::EventAckResponse {
        status_code: _,
        headers,
        body,
    } = ack;

    if body.is_empty() {
        return services::ApplicationResponse::StatusOk;
    }

    let masked_headers: Vec<(String, hyperswitch_masking::Maskable<String>)> = headers
        .into_iter()
        .map(|(k, v)| (k, hyperswitch_masking::Maskable::new_normal(v)))
        .collect();

    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&body) {
        return if masked_headers.is_empty() {
            services::ApplicationResponse::Json(value)
        } else {
            services::ApplicationResponse::JsonWithHeaders((value, masked_headers))
        };
    }

    match String::from_utf8(body.clone()) {
        Ok(text) => services::ApplicationResponse::TextPlain(text),
        Err(_) => services::ApplicationResponse::FileData((body, mime::APPLICATION_OCTET_STREAM)),
    }
}
