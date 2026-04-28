//! Gateway abstraction for incoming webhook processing.

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

/// JSON bytes of the webhook resource. `Direct` is the connector-native shape
/// (parsed by the connector); `UnifiedConnectorService` is a UCS `EventContent`
/// (consumed directly by downstream PSync).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum WebhookContent {
    Direct(Vec<u8>),
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

/// Outcome of a single inbound-webhook processing run.
pub enum WebhookOutcome {
    Skipped {
        reference: Option<ObjectReferenceId>,
        event_type: IncomingWebhookEvent,
        ack_response: services::ApplicationResponse<serde_json::Value>,
    },
    Processed {
        reference: ObjectReferenceId,
        event_type: IncomingWebhookEvent,
        source_verified: bool,
        content: WebhookContent,
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

#[derive(Clone)]
pub struct WebhookGatewayContext {
    pub state: SessionState,
    pub platform: domain::Platform,
    pub connector: ConnectorEnum,
    pub connector_name: String,
    /// `None` when the webhook URL identifies the connector by name only;
    /// resolved post-decode from the parsed object reference in that case.
    pub merchant_connector_account: Option<domain::MerchantConnectorAccount>,
    pub execution_path: ExecutionPath,
    pub execution_mode: ExecutionMode,
}

#[async_trait]
pub trait IncomingWebhookGateway: Send + Sync {
    async fn execute(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        ctx: &WebhookGatewayContext,
    ) -> RouterResult<WebhookOutcome>;
}

pub enum FilterDecision {
    Skip,
    Proceed,
}

impl FilterDecision {
    pub async fn evaluate(event_type: IncomingWebhookEvent, ctx: &WebhookGatewayContext) -> Self {
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

pub struct DirectIncomingWebhookGateway;

#[async_trait]
impl IncomingWebhookGateway for DirectIncomingWebhookGateway {
    async fn execute(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        ctx: &WebhookGatewayContext,
    ) -> RouterResult<WebhookOutcome> {
        let decoded_body = ctx
            .connector
            .decode_webhook_body(
                request,
                ctx.platform.get_processor().get_account().get_id(),
                ctx.merchant_connector_account
                    .as_ref()
                    .and_then(|mca| mca.connector_webhook_details.clone()),
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

        let reference = ctx
            .connector
            .get_webhook_object_reference_id(&decoded_request)
            .ok();

        let mca = resolve_mca(ctx, reference.as_ref()).await?;

        let webhook_context = build_webhook_context(&ctx.state, &ctx.platform, reference.as_ref())
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
                let masked_log_payload =
                    Secret::new(resource_object.masked_serialize().unwrap_or_else(|error| {
                        logger::warn!(
                            ?error,
                            "Failed to mask-serialize webhook resource object for logging"
                        );
                        serde_json::Value::Null
                    }));

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

/// Two-phase UCS flow: `ParseEvent` (pre-credential) then `HandleEvent`.
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
        let merchant_event_id = build_merchant_event_id(ctx);

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
            FilterDecision::Skip => WebhookOutcome::Skipped {
                reference,
                event_type,
                ack_response: services::ApplicationResponse::StatusOk,
            },
            FilterDecision::Proceed => {
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
                let handle_headers = build_ucs_headers_builder(ctx, Some(&mca), ctx.execution_mode);
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

                let event_content = handle_response.event_content.ok_or_else(|| {
                    error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                        .attach_printable(
                            "UCS HandleEvent returned no event_content for a non-filtered event",
                        )
                })?;
                let bytes = serde_json::to_vec(&event_content)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encode unified event content")?;
                let masked_log_payload =
                    Secret::new(event_content.masked_serialize().unwrap_or_else(|error| {
                        logger::warn!(
                            ?error,
                            "Failed to mask-serialize unified event content for logging"
                        );
                        serde_json::Value::Null
                    }));

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
            let direct_result = DirectIncomingWebhookGateway.execute(request, ctx).await;
            let primary_snapshot = WebhookShadowSnapshot::from_result(&direct_result);
            spawn_shadow_ucs_run(ctx, request, primary_snapshot);
            direct_result
        }
    }
}

fn spawn_shadow_ucs_run(
    ctx: &WebhookGatewayContext,
    request: &IncomingWebhookRequestDetails<'_>,
    primary_snapshot: WebhookShadowSnapshot,
) {
    let mut inner_ctx = ctx.clone();
    inner_ctx.execution_path = ExecutionPath::UnifiedConnectorService;
    inner_ctx.execution_mode = ExecutionMode::Shadow;

    let request_owned = OwnedRequestDetails::from(request);

    tokio::spawn(
        async move {
            let request_ref = request_owned.borrow();
            let shadow_result = UcsIncomingWebhookGateway
                .execute(&request_ref, &inner_ctx)
                .await;
            if let Err(error) = shadow_result.as_ref() {
                logger::warn!(?error, "UCS shadow webhook run failed");
            }
            let shadow_snapshot = WebhookShadowSnapshot::from_result(&shadow_result);
            report_shadow_diff(
                &inner_ctx.state,
                &inner_ctx.connector_name,
                &primary_snapshot,
                &shadow_snapshot,
            )
            .await;
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
    event_type: Option<IncomingWebhookEvent>,
    source_verified: Option<bool>,
    content_kind: Option<&'static str>,
    reference: Option<ObjectReferenceId>,
    error: Option<String>,
}

impl WebhookShadowSnapshot {
    fn from_result(result: &RouterResult<WebhookOutcome>) -> Self {
        match result {
            Ok(outcome) => Self::from(outcome),
            Err(error) => Self {
                variant: "error",
                event_type: None,
                source_verified: None,
                content_kind: None,
                reference: None,
                error: Some(format!("{error:?}")),
            },
        }
    }
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
                event_type: Some(*event_type),
                source_verified: None,
                content_kind: None,
                reference: reference.clone(),
                error: None,
            },
            WebhookOutcome::Processed {
                event_type,
                source_verified,
                content,
                reference,
                ..
            } => Self {
                variant: "processed",
                event_type: Some(*event_type),
                source_verified: Some(*source_verified),
                content_kind: Some(match content {
                    WebhookContent::Direct(_) => "direct",
                    WebhookContent::UnifiedConnectorService(_) => "unified_connector_service",
                }),
                reference: Some(reference.clone()),
                error: None,
            },
        }
    }
}

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

/// Source verification for the Direct and UAS paths. UCS returns its own
/// `source_verified` flag from `HandleEvent`.
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

/// `mca = None` → routing-only metadata for `ParseEvent` (pre-credential).
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
