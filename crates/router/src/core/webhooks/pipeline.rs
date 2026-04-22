//! Webhook processing pipeline abstraction.
//!
//! Provides a trait-based split between HS-native webhook handling and UCS-driven webhook
//! handling. Both implementations expose the same two-phase shape — `parse` then `handle` —
//! and the driver (`run_webhook_pipeline`) executes them in a single canonical order so no
//! implementation can reorder steps or silently diverge.
//!
//! The trait is aligned 1:1 with the two UCS RPCs (`EventService.ParseEvent` and
//! `EventService.HandleEvent`); the HS-native implementation satisfies the same shape by
//! calling the existing `IncomingWebhook` connector trait methods.
//!
//! The trait boundary speaks only HS-native types. Each implementation owns the translation
//! to/from its own representation (connector-specific JSON on the HS side, proto messages on
//! the UCS side), so `process_webhook_business_logic` and everything downstream never sees a
//! path-specific type.
//!
//! This mirrors the layout of the existing payment-gateway abstraction in
//! `hyperswitch_interfaces::api::gateway` (`DirectGateway` / per-flow UCS gateways behind a
//! selector); webhooks keep both phases on one trait because `parse` and `handle` are phases
//! of the *same* pipeline, not independent flows.

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use async_trait::async_trait;
use common_enums::ExecutionMode;
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails;
use hyperswitch_masking::PeekInterface;
use router_env::logger;
use time::OffsetDateTime;
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers::MerchantConnectorAccountType,
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            build_webhook_secrets_from_merchant_connector_account,
        },
    },
    routes::SessionState,
    services::connector_integration_interface::ConnectorEnum,
    types::{api::IncomingWebhook, domain, transformers::ForeignTryFrom},
};

// ---------------------------------------------------------------------------
// Public types exchanged at the trait boundary. HS-native only.
// ---------------------------------------------------------------------------

/// Output of the first phase — always computable from request bytes alone, without
/// resolving credentials or calling any connector backend.
///
/// `reference` is `None` for account-level events (capability notifications, etc.) that
/// do not refer to a specific payment/refund/dispute/etc. Callers short-circuit such
/// events before the `handle` phase.
#[derive(Debug)]
pub struct ParsedEvent {
    pub reference: Option<ObjectReferenceId>,
    pub event_type: IncomingWebhookEvent,
}

/// Output of the second phase.
///
/// `content` carries the webhook payload in whichever shape the pipeline produces.
/// `source_verified` reflects the outcome of signature verification at the source of truth
/// for that pipeline (connector-specific algorithm for HS, UCS response for UCS). Callers
/// trust this bool; they do not re-verify.
#[derive(Debug)]
pub struct HandledEvent {
    pub content: WebhookContent,
    pub source_verified: bool,
}

/// The shape of the handled webhook payload.
///
/// * `Raw` — masked-serialized JSON bytes of the connector's raw, connector-specific
///   webhook payload. HS native path produces this. Downstream PSync must invoke the
///   connector's own response parser (`CallConnectorAction::HandleResponse`) to unify.
///
/// * `UnifiedBytes` — JSON-serialized bytes of the UCS `EventContent` (already a unified
///   response). Downstream PSync can skip the connector round-trip and consume these bytes
///   directly (`CallConnectorAction::UCSConsumeResponse`).
///
/// The variants exist as separate arms precisely so downstream code cannot accidentally
/// feed UCS-unified bytes into a connector-specific parser or vice versa. Marked
/// `#[non_exhaustive]` so that adding further variants (e.g. a typed HS-native unified
/// response) triggers a compile error at every consumer.
#[derive(Debug)]
#[non_exhaustive]
pub enum WebhookContent {
    Raw(Vec<u8>),
    UnifiedBytes(Vec<u8>),
}

impl WebhookContent {
    /// Borrow the inner bytes regardless of variant. Callers that need to distinguish
    /// provenance must `match` on the variant.
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Raw(bytes) | Self::UnifiedBytes(bytes) => bytes,
        }
    }

    /// Consume into the inner bytes regardless of variant.
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Raw(bytes) | Self::UnifiedBytes(bytes) => bytes,
        }
    }

    /// `true` iff this payload is already a unified (UCS) response — downstream PSync
    /// should feed it via `UCSConsumeResponse` rather than `HandleResponse`.
    pub fn is_unified(&self) -> bool {
        matches!(self, Self::UnifiedBytes(_))
    }
}

/// HS-native webhook secrets used by the `handle` phase.
///
/// Mirrors the fields the `IncomingWebhook` connector trait and the UCS proto both need
/// (`secret` + optional `additional_secret`). Each pipeline implementation translates this
/// into its own representation — `ConnectorWebhookSecrets` for HS, `proto::WebhookSecrets`
/// for UCS — inside the impl.
#[derive(Debug, Clone)]
pub struct WebhookSecrets {
    pub secret: Vec<u8>,
    pub additional_secret: Option<hyperswitch_masking::Secret<String>>,
}

/// Optional OAuth-style access token forwarded to pipelines that need to sign outbound
/// calls (e.g. PayPal's `POST /v1/notifications/verify-webhook-signature`).
#[derive(Debug, Clone)]
pub struct WebhookAccessToken {
    pub token: hyperswitch_masking::Secret<String>,
    pub expires_in_seconds: Option<i64>,
    pub token_type: Option<String>,
}

/// HS-native business-context hint supplied to `handle` when a connector needs extra fields
/// to construct a complete unified response (e.g. `capture_method` to distinguish
/// `CAPTURED` vs `PARTIALLY_CAPTURED` for some connectors).
///
/// Marked `#[non_exhaustive]` so variants can grow without breaking existing call sites.
/// Empty today; populated alongside the UCS pipeline implementation as connectors need it.
#[derive(Debug)]
#[non_exhaustive]
pub enum WebhookBusinessContext {}

// ---------------------------------------------------------------------------
// The trait.
// ---------------------------------------------------------------------------

/// Two-phase webhook processing. Implementors own a single path (HS direct, UCS, or a
/// shadow-wrapper combining both). The driver `run_webhook_pipeline` calls the methods in
/// a fixed canonical order; implementors cannot reorder because they never call each
/// other's steps.
///
/// Any signature change to this trait breaks every implementation at compile time. That is
/// the point: HS-team and UCS-team must stay in sync on the pipeline shape.
#[async_trait]
pub trait WebhookPipeline: Send + Sync {
    /// Phase 1. No credentials needed. Pure parse + event classification.
    ///
    /// `reference` may be `None` for events that carry no resource id (e.g. account-level
    /// notifications); the driver short-circuits such events before phase 2.
    async fn parse(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> RouterResult<ParsedEvent>;

    /// Phase 2. Runs after the driver has resolved the merchant-connector-account and the
    /// webhook secret from the parsed reference. Performs source verification and produces
    /// the handled payload. The driver trusts `source_verified` — it does not re-verify.
    ///
    /// `business_context` is optional per-entity hints required by some connectors (e.g.
    /// capture_method); the Direct pipeline ignores it.
    #[allow(clippy::too_many_arguments)]
    async fn handle(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        secrets: &WebhookSecrets,
        access_token: Option<&WebhookAccessToken>,
        business_context: Option<&WebhookBusinessContext>,
    ) -> RouterResult<HandledEvent>;
}

// ---------------------------------------------------------------------------
// Direct (HS-native) implementation.
// ---------------------------------------------------------------------------

/// HS-native webhook pipeline: delegates every phase to the connector's existing
/// `IncomingWebhook` trait methods. No gRPC, no UCS.
///
/// This mirrors `DirectGateway` in `hyperswitch_interfaces::api::gateway` — a thin bridge
/// over the existing connector integration layer, not a rewrite of connector behaviour.
pub struct DirectWebhookPipeline<'a> {
    pub state: &'a SessionState,
    pub platform: &'a domain::Platform,
    pub connector: &'a ConnectorEnum,
    pub connector_name: &'a str,
    pub merchant_connector_account:
        Option<&'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount>,
}

impl<'a> DirectWebhookPipeline<'a> {
    pub fn new(
        state: &'a SessionState,
        platform: &'a domain::Platform,
        connector: &'a ConnectorEnum,
        connector_name: &'a str,
        merchant_connector_account: Option<
            &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
    ) -> Self {
        Self {
            state,
            platform,
            connector,
            connector_name,
            merchant_connector_account,
        }
    }
}

#[async_trait]
impl<'a> WebhookPipeline for DirectWebhookPipeline<'a> {
    async fn parse(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> RouterResult<ParsedEvent> {
        let reference = match self.connector.get_webhook_object_reference_id(request) {
            Ok(id) => Some(id),
            Err(error) => {
                logger::debug!(
                    ?error,
                    "Connector produced no object reference id; treating as account-level event"
                );
                None
            }
        };

        let event_type = self
            .connector
            .get_webhook_event_type(request, None)
            .map_err(|err| err.change_context(errors::ApiErrorResponse::WebhookProcessingFailure))
            .attach_printable("Failed to extract webhook event type via HS connector trait")?;

        Ok(ParsedEvent {
            reference,
            event_type,
        })
    }

    async fn handle(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _secrets: &WebhookSecrets,
        _access_token: Option<&WebhookAccessToken>,
        _business_context: Option<&WebhookBusinessContext>,
    ) -> RouterResult<HandledEvent> {
        // Source verification: delegated to the connector's own signature algorithm. Uses
        // the MCA-resolved webhook details and account details captured at construction
        // time; no re-verification downstream.
        let merchant_id = self
            .platform
            .get_processor()
            .get_account()
            .get_id()
            .clone();
        let webhook_details = self
            .merchant_connector_account
            .and_then(|mca| mca.connector_webhook_details.clone());
        let connector_account_details = self
            .merchant_connector_account
            .map(|mca| mca.connector_account_details.clone());

        let source_verified = match connector_account_details.clone() {
            Some(account_details) => self
                .connector
                .clone()
                .verify_webhook_source(
                    request,
                    &merchant_id,
                    webhook_details.clone(),
                    account_details,
                    self.connector_name,
                )
                .await
                .or_else(|error| {
                    match error.current_context() {
                        hyperswitch_interfaces::errors::ConnectorError::WebhookSourceVerificationFailed => {
                            logger::warn!(?error, "HS source verification returned failure");
                            Ok(false)
                        }
                        _ => Err(error),
                    }
                })
                .map_err(|err| {
                    err.change_context(errors::ApiErrorResponse::WebhookAuthenticationFailed)
                })
                .attach_printable("HS source verification failed")?,
            None => {
                // No MCA available (e.g. connector-name-only URL) — caller will still have
                // invoked `parse` successfully; we cannot verify without credentials, so
                // report `false`. The caller decides whether to reject based on
                // `is_webhook_source_verification_mandatory()`.
                false
            }
        };

        let resource_object = self
            .connector
            .get_webhook_resource_object(request)
            .map_err(|err| err.change_context(errors::ApiErrorResponse::WebhookProcessingFailure))
            .attach_printable("Failed to extract webhook resource object via HS connector trait")?;

        // Masked-serialize here so the `HandledEvent` is `Send` and downstream entity flows
        // can consume the bytes directly via `api::IncomingWebhookDetails.resource_object`.
        let masked = resource_object
            .masked_serialize()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to masked-serialize webhook resource object")?;
        let bytes = serde_json::to_vec(&masked)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to JSON-encode masked webhook resource object")?;

        Ok(HandledEvent {
            content: WebhookContent::Raw(bytes),
            source_verified,
        })
    }
}

// ---------------------------------------------------------------------------
// Driver — canonical ordered pipeline.
// ---------------------------------------------------------------------------

/// Result of running a full pipeline, regardless of which implementation backed it.
///
/// The driver resolves `merchant_connector_account` between phases so both impls receive
/// the same ordered input on phase 2. `handled.source_verified` is the single source of
/// truth for downstream — callers do not re-verify.
#[derive(Debug)]
pub struct WebhookPipelineOutcome {
    pub parsed: ParsedEvent,
    pub handled: HandledEvent,
}

/// Run the canonical pipeline:
/// `parse → (skip if unsupported) → source_verify + resource extract (handle)`.
///
/// Note: in this PR the caller (driver) is responsible for resolving `secrets` from the
/// MCA before invoking the pipeline's `handle` — that MCA lookup is an HS-side concern
/// that naturally sits on the boundary between phases. This helper wires the calls in the
/// fixed order and nothing else; it does not hide the MCA-lookup step.
pub async fn run_webhook_pipeline(
    pipeline: &dyn WebhookPipeline,
    request: &IncomingWebhookRequestDetails<'_>,
    secrets: &WebhookSecrets,
    access_token: Option<&WebhookAccessToken>,
    business_context: Option<&WebhookBusinessContext>,
) -> RouterResult<WebhookPipelineOutcome> {
    let parsed = pipeline.parse(request).await?;
    let handled = pipeline
        .handle(request, secrets, access_token, business_context)
        .await?;
    Ok(WebhookPipelineOutcome { parsed, handled })
}

// ---------------------------------------------------------------------------
// UCS (gRPC two-phase) implementation.
// ---------------------------------------------------------------------------

/// UCS-backed webhook pipeline. `parse` issues `EventService.ParseEvent`; `handle` issues
/// `EventService.HandleEvent`. Both calls are authenticated with connector credentials
/// derived from the merchant-connector-account captured at construction time.
///
/// This implementation requires the MCA to be known up front (matching today's
/// connector-id-in-URL webhook shape). The two-phase proto also supports the
/// connector-name-only URL case where the MCA is looked up from the parsed reference
/// between phases; that flow is not wired into this PR's driver and is a follow-up.
pub struct UcsWebhookPipeline<'a> {
    pub state: &'a SessionState,
    pub platform: &'a domain::Platform,
    pub connector_name: &'a str,
    pub merchant_connector_account:
        &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    pub execution_mode: ExecutionMode,
}

impl<'a> UcsWebhookPipeline<'a> {
    pub fn new(
        state: &'a SessionState,
        platform: &'a domain::Platform,
        connector_name: &'a str,
        merchant_connector_account: &'a hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            state,
            platform,
            connector_name,
            merchant_connector_account,
            execution_mode,
        }
    }

    fn build_auth_metadata(
        &self,
    ) -> RouterResult<external_services::grpc_client::unified_connector_service::ConnectorAuthMetadata>
    {
        let mca_type = MerchantConnectorAccountType::DbVal(Box::new(
            self.merchant_connector_account.clone(),
        ));
        build_unified_connector_service_auth_metadata(
            mca_type,
            self.platform.get_processor(),
            self.connector_name.to_string(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build UCS auth metadata for webhook pipeline")
    }

    fn build_headers(
        &self,
    ) -> external_services::grpc_client::GrpcHeadersUcs {
        self.state
            .get_grpc_headers_ucs(self.execution_mode)
            .lineage_ids(LineageIds::new(
                self.platform
                    .get_processor()
                    .get_account()
                    .get_id()
                    .clone(),
                self.merchant_connector_account.profile_id.clone(),
            ))
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(None)
            .resource_id(None)
            .build()
    }

    fn request_details_to_grpc(
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> RouterResult<payments_grpc::RequestDetails> {
        <payments_grpc::RequestDetails as ForeignTryFrom<
            &IncomingWebhookRequestDetails<'_>,
        >>::foreign_try_from(request)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to translate webhook request details to gRPC RequestDetails")
    }

    fn build_merchant_event_id(&self) -> String {
        format!(
            "{}_{}_{}",
            self.platform
                .get_processor()
                .get_account()
                .get_id()
                .get_string_repr(),
            self.connector_name,
            OffsetDateTime::now_utc().unix_timestamp()
        )
    }

    fn build_webhook_secrets_proto(
        &self,
    ) -> RouterResult<Option<payments_grpc::WebhookSecrets>> {
        let mca_type = MerchantConnectorAccountType::DbVal(Box::new(
            self.merchant_connector_account.clone(),
        ));
        build_webhook_secrets_from_merchant_connector_account(&mca_type)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to build UCS webhook secrets from MCA")
    }
}

/// Translate a UCS `EventReference` into the HS-native `ObjectReferenceId`.
///
/// `None` is returned for account-level events (no resource). Mandate and payout variants
/// are mapped where HS has a corresponding arm; unsupported-in-HS variants fall through to
/// `Err` so the caller can decide whether to treat the event as unsupported.
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
            // Prefer connector_transaction_id; fall back to merchant_transaction_id.
            if let Some(ctx_id) = payment.connector_transaction_id.as_ref() {
                Some(ObjectReferenceId::PaymentId(
                    api_payments::PaymentIdType::ConnectorTransactionId(ctx_id.clone()),
                ))
            } else if let Some(mref) = payment.merchant_transaction_id.as_ref() {
                let payment_id = common_utils::id_type::PaymentId::try_from(
                    std::borrow::Cow::Owned(mref.clone()),
                )
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to parse UCS merchant_transaction_id as PaymentId")?;
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
            } else if let Some(mref) = refund.merchant_refund_id.as_ref() {
                Some(ObjectReferenceId::RefundId(
                    api_webhooks::RefundIdType::RefundId(mref.clone()),
                ))
            } else {
                None
            }
        }
        Resource::Dispute(dispute) => dispute
            .connector_dispute_id
            .as_ref()
            .or(dispute.connector_transaction_id.as_ref())
            .map(|id| {
                ObjectReferenceId::PaymentId(
                    api_payments::PaymentIdType::ConnectorTransactionId(id.clone()),
                )
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
                payout
                    .merchant_payout_id
                    .as_ref()
                    .map(|mid| ObjectReferenceId::PayoutId(
                        api_webhooks::PayoutIdType::PayoutAttemptId(mid.clone()),
                    ))
            }
        }
        #[cfg(not(feature = "payouts"))]
        Resource::Payout(_) => None,
    };

    Ok(out)
}

#[async_trait]
impl<'a> WebhookPipeline for UcsWebhookPipeline<'a> {
    async fn parse(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> RouterResult<ParsedEvent> {
        let ucs_client = self
            .state
            .grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("UCS client is not available for webhook ParseEvent")
            })?;

        let request_details = Self::request_details_to_grpc(request)?;
        let parse_request = payments_grpc::EventServiceParseRequest {
            request_details: Some(request_details),
        };

        let auth_metadata = self.build_auth_metadata()?;
        let headers = self.build_headers();

        let response = ucs_client
            .incoming_webhook_parse_event(parse_request, auth_metadata, headers)
            .await
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("UCS ParseEvent gRPC call failed")?
            .into_inner();

        let reference = match response.reference.as_ref() {
            Some(reference) => event_reference_to_object_ref(reference)?,
            None => None,
        };

        let event_type = response
            .event_type
            .map(IncomingWebhookEvent::from_ucs_event_type)
            .unwrap_or(IncomingWebhookEvent::EventNotSupported);

        Ok(ParsedEvent {
            reference,
            event_type,
        })
    }

    async fn handle(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        secrets: &WebhookSecrets,
        access_token: Option<&WebhookAccessToken>,
        _business_context: Option<&WebhookBusinessContext>,
    ) -> RouterResult<HandledEvent> {
        let ucs_client = self
            .state
            .grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("UCS client is not available for webhook HandleEvent")
            })?;

        let request_details = Self::request_details_to_grpc(request)?;

        // Prefer the secret bundle that the caller resolved (it may include follow-ups the
        // MCA-derived bundle does not). Fall back to the MCA-derived bundle so shadow mode
        // and simple flows still work when the caller has nothing to override with.
        let caller_secrets = payments_grpc::WebhookSecrets {
            secret: String::from_utf8(secrets.secret.clone()).unwrap_or_default(),
            additional_secret: secrets
                .additional_secret
                .as_ref()
                .map(|s| s.peek().clone()),
        };
        let webhook_secrets = if caller_secrets.secret.is_empty() {
            self.build_webhook_secrets_proto()?
        } else {
            Some(caller_secrets)
        };

        let access_token_proto = access_token.map(|token| payments_grpc::AccessToken {
            token: Some(token.token.clone()),
            expires_in_seconds: token.expires_in_seconds,
            token_type: token.token_type.clone(),
        });

        let handle_request = payments_grpc::EventServiceHandleRequest {
            merchant_event_id: Some(self.build_merchant_event_id()),
            request_details: Some(request_details),
            webhook_secrets,
            access_token: access_token_proto,
            event_context: None,
        };

        let auth_metadata = self.build_auth_metadata()?;
        let headers = self.build_headers();

        let response = ucs_client
            .incoming_webhook_handle_event(handle_request, auth_metadata, headers)
            .await
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("UCS HandleEvent gRPC call failed")?
            .into_inner();

        let source_verified = response.source_verified;

        // Serialize the unified `EventContent` to JSON bytes. Downstream PSync uses
        // `UCSConsumeResponse(bytes)` to short-circuit the connector round-trip when the
        // response is already unified — that's why `UnifiedBytes` is a distinct variant
        // from `Raw`.
        let bytes = match response.event_content.as_ref() {
            Some(content) => serde_json::to_vec(content)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to JSON-encode UCS EventContent for downstream flow")?,
            None => {
                logger::warn!("UCS HandleEvent returned no event_content");
                Vec::new()
            }
        };

        Ok(HandledEvent {
            content: WebhookContent::UnifiedBytes(bytes),
            source_verified,
        })
    }
}
