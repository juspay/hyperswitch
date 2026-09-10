//! Bridge from Hyperswitch's FRM core to the Unified Connector Service.
//!
//! Native FRM providers (Signifyd, Riskified, CyberSource Decision Manager) run
//! in-process. UCS-backed providers instead have their risk evaluation executed
//! by the connector-service, which owns the provider-specific transformation.
//!
//! ```text
//! FrmData                        ──▶ FrmServicePreRiskCheckRequest
//! FrmServicePreRiskCheckResponse ──▶ FraudCheckResponseData
//! ```
//!
//! The merchant's `frm_metadata` is forwarded verbatim as
//! `connector_feature_data`. That is how provider-specific signals (device
//! fingerprint, account tenure, velocity counters) reach the connector without
//! Hyperswitch needing to model them — the same escape hatch Signifyd uses for
//! its device `session_id`.

use common_utils::{id_type, types::MinorUnit};
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{
    platform::Processor, router_data::RouterData, router_flow_types::fraud_check as frm_api,
    router_request_types::fraud_check::FraudCheckCheckoutData,
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::unified_connector_service::UnifiedConnectorServiceError;
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use unified_connector_service_client::payments as payments_grpc;

use super::{
    build_unified_connector_service_auth_metadata, build_unified_connector_service_payment_method,
    get_ucs_client,
};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers::MerchantConnectorAccountType,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

/// Build the pre-risk-check request from the FRM `Checkout` router data.
///
/// Everything the risk provider needs is already on the router data: the
/// request carries the instrument and buyer details, and the top-level fields
/// carry address, token, access token and `frm_metadata`.
impl ForeignTryFrom<&RouterData<frm_api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>>
    for payments_grpc::FrmServicePreRiskCheckRequest
{
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(
        router_data: &RouterData<frm_api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>,
    ) -> Result<Self, Self::Error> {
        let request = &router_data.request;

        let currency = request.currency.ok_or_else(|| {
            error_stack::report!(UnifiedConnectorServiceError::MissingRequiredField {
                field_name: "currency".into(),
            })
        })?;
        let grpc_currency = payments_grpc::Currency::foreign_try_from(currency)?;

        let amount = payments_grpc::Money {
            minor_amount: request.amount.get_amount_as_i64(),
            currency: grpc_currency.into(),
        };

        // `customer_id` is the stable merchant-side key risk providers use to
        // build cross-transaction history for the buyer; the contact details
        // alongside it are what they match on when the id is new.
        let has_customer = request.customer_id.is_some()
            || request.customer_name.is_some()
            || request.email.is_some()
            || request.phone.is_some();
        let customer_info = has_customer.then(|| payments_grpc::Customer {
            id: request
                .customer_id
                .as_ref()
                .map(|id| id.get_string_repr().to_owned()),
            // Sent as the full name; providers that want the parts split it
            // themselves, since Hyperswitch does not store them apart.
            name: request
                .customer_name
                .as_ref()
                .map(|name| name.peek().to_owned()),
            email: request
                .email
                .as_ref()
                .map(|email| Secret::new(email.peek().to_owned())),
            phone_number: request
                .phone
                .as_ref()
                .map(|phone| Secret::new(phone.peek().to_owned())),
            phone_country_code: request.phone_country_code.clone(),
            ..Default::default()
        });

        // Reuses the same builder the payments UCS path uses, so the instrument
        // is encoded identically for a risk check and for the authorization that
        // follows it.
        //
        // A payment method Hyperswitch cannot encode degrades to `None` rather
        // than failing: the FRM pre-check propagates its error with `?` in
        // `pre_payment_frm_core`, so returning `Err` here would fail the payment
        // outright over a risk-signal encoding problem.
        let payment_method =
            request
                .payment_method_data_full
                .as_ref()
                .and_then(|payment_method_data| {
                    build_unified_connector_service_payment_method(
                        payment_method_data.clone(),
                        router_data.payment_method_type,
                        router_data.payment_method_token.as_ref(),
                        None,
                    )
                    .inspect_err(|error| {
                        router_env::logger::warn!(
                            ?error,
                            "Failed to encode the payment method for the FRM pre risk check; \
                         the provider will score this transaction without instrument details"
                        )
                    })
                    .ok()
                });

        // Merchant identity for risk scoring. The MCC lives on the business
        // profile, which this path does not load, so it is left unset rather
        // than issuing an extra fetch for a field no current provider reads.
        let merchant_details = Some(payments_grpc::MerchantDetails {
            merchant_id: Some(router_data.merchant_id.get_string_repr().to_owned()),
            merchant_category_code: None,
        });

        let browser_info =
            request
                .browser_info
                .as_ref()
                .map(|info| payments_grpc::BrowserInformation {
                    user_agent: info.user_agent.clone(),
                    ip_address: info.ip_address.map(|ip| ip.to_string()),
                    language: info.language.clone(),
                    accept_header: info.accept_header.clone(),
                    ..Default::default()
                });

        let order_details =
            super::transformers::build_ucs_order_details(request.order_details.as_deref());

        Ok(Self {
            amount: Some(amount),
            customer_info,
            payment_method,
            browser_info,
            merchant_transaction_id: Some(router_data.attempt_id.clone()),
            order_details,
            address: Some(payments_grpc::PaymentAddress::foreign_try_from(
                router_data.address.clone(),
            )?),
            merchant_details,
            connector_feature_data: router_data
                .frm_metadata
                .as_ref()
                .map(|metadata| Secret::new(metadata.clone().expose().to_string())),
            // Bearer-authenticated providers read the token from
            // `state.access_token`; prism threads it onto FrmFlowData.
            state: router_data
                .access_token
                .as_ref()
                .map(|token| payments_grpc::ConnectorState {
                    access_token: Some(payments_grpc::AccessToken {
                        token: Some(token.token.clone()),
                        expires_in_seconds: Some(token.expires),
                        token_type: None,
                    }),
                    connector_customer_id: None,
                }),
            ..Default::default()
        })
    }
}

/// Fetch (and cache) the OAuth token a bearer-authenticated FRM provider needs.
///
/// Kount's risk API is bearer-authenticated: prism reads the token from
/// `state.access_token` and cannot mint one itself on the plain FRM service.
/// Reads Redis first, falling back to UCS `CreateServerAuthenticationToken`.
/// Providers using a static key (nSure) never reach this — `should_do_access_token`
/// is connector-side, so we gate on whether UCS returns a token at all.
#[cfg(feature = "v1")]
pub async fn get_frm_access_token(
    state: &SessionState,
    processor: &Processor,
    connector_name: &str,
    merchant_connector_account: &MerchantConnectorAccountType,
    lineage_ids: LineageIds,
) -> RouterResult<Option<hyperswitch_domain_models::router_data::AccessToken>> {
    let merchant_id = processor.get_account().get_id();

    // Scope the cached token to the merchant connector account, the way the
    // payments path does (`access_token::get_cached_access_token_for_ucs`).
    // Keying on the connector name alone would make two accounts for the same
    // FRM provider under one merchant share a token.
    let merchant_connector_id = match merchant_connector_account {
        MerchantConnectorAccountType::DbVal(mca) => Some(mca.get_id()),
        MerchantConnectorAccountType::CacheVal(_) => None,
    };

    let access_token_key = common_utils::access_token::get_default_access_token_key(
        merchant_id,
        merchant_connector_id
            .as_ref()
            .map(|id| id.get_string_repr().to_string())
            .unwrap_or_else(|| connector_name.to_string()),
    );

    if let Ok(Some(token)) = state.store.get_access_token(access_token_key.clone()).await {
        router_env::logger::debug!(connector = connector_name, "FRM access token cache hit");
        return Ok(Some(token));
    }

    let ucs_client = get_ucs_client(state)?;
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account.clone(),
        merchant_id,
        connector_name.to_string(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for the FRM access token")?;

    let grpc_headers = state
        .get_grpc_headers_ucs(common_enums::ExecutionMode::Primary)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .build();

    let response = ucs_client
        .create_access_token(
            payments_grpc::MerchantAuthenticationServiceCreateServerAuthenticationTokenRequest::default(),
            connector_auth_metadata,
            grpc_headers,
            common_enums::ConnectorType::PaymentVas,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("UCS create_access_token gRPC call failed for FRM")?;

    let (token_result, _status) =
        super::handle_unified_connector_service_response_for_create_access_token(
            response.into_inner(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse the UCS FRM access token response")?;

    match token_result {
        Ok(token) => {
            // Best-effort cache; a write failure only costs an extra token call.
            let _ = super::set_access_token_for_ucs(
                state,
                processor,
                connector_name,
                token.clone(),
                merchant_connector_id.as_ref(),
                None,
            )
            .await;
            Ok(Some(token))
        }
        Err(err) => {
            router_env::logger::error!(
                connector = connector_name,
                error = ?err,
                "UCS returned an error for the FRM access token"
            );
            Ok(None)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle notifications
//
// prism exposes four FRM notification events on a single `NotifyConnector` RPC:
//
//   FRM_PAYMENT_SUCCEEDED · FRM_PAYMENT_FAILURE
//   FRM_REFUND_PROCESSED  · FRM_CHARGEBACK_RECEIVED
//
// They differ only in the event type and which `notification_type` variant is
// populated; auth, headers, token and response handling are identical. The
// generic sender below owns that shared work so each event is a thin caller.
// Chargeback is wired here; the payment/refund events follow the same shape.
// ─────────────────────────────────────────────────────────────────────────────

/// Which lifecycle event to report, plus the detail payload it carries.
pub enum FrmNotification {
    /// A chargeback/dispute was opened against a previously-scored payment.
    ChargebackReceived {
        connector_dispute_id: Option<String>,
        merchant_dispute_id: Option<String>,
        chargeback_reason: Option<String>,
    },
}

impl FrmNotification {
    fn event_type(&self) -> payments_grpc::NotifyEventType {
        match self {
            Self::ChargebackReceived { .. } => {
                payments_grpc::NotifyEventType::FrmChargebackReceived
            }
        }
    }

    fn notification_type(self) -> payments_grpc::frm_notification_content::NotificationType {
        match self {
            Self::ChargebackReceived {
                connector_dispute_id,
                merchant_dispute_id,
                chargeback_reason,
            } => payments_grpc::frm_notification_content::NotificationType::Chargeback(
                payments_grpc::FrmChargebackDetails {
                    connector_dispute_id,
                    merchant_dispute_id,
                    chargeback_reason,
                },
            ),
        }
    }
}

/// Facts shared by every FRM lifecycle notification.
pub struct FrmNotificationContext<'a> {
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub connector_transaction_id: Option<String>,
    /// Correlation id returned by the original risk check. Without it the
    /// provider cannot tie the notification to the transaction it scored.
    pub frm_transaction_id: Option<String>,
    pub profile_id: &'a id_type::ProfileId,
}

/// Send an FRM lifecycle notification to the connector-service.
///
/// Generic over the event: the caller supplies the [`FrmNotification`] variant
/// and this handles auth metadata, `x-frm-connector` routing, the access token
/// for bearer-authenticated providers, and the gRPC call.
#[cfg(feature = "v1")]
pub async fn call_unified_connector_service_for_frm_notification(
    state: &SessionState,
    processor: &Processor,
    merchant_connector_account: MerchantConnectorAccountType,
    connector_name: String,
    context: FrmNotificationContext<'_>,
    notification: FrmNotification,
) -> RouterResult<()> {
    let ucs_client = get_ucs_client(state)?;

    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        processor.get_account().get_id(),
        connector_name.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for the FRM notification")?;

    let grpc_currency = payments_grpc::Currency::foreign_try_from(context.currency)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert currency for the FRM notification")?;

    let event_type = notification.event_type();

    let content = payments_grpc::NotifyConnectorContent {
        content: Some(
            payments_grpc::notify_connector_content::Content::FrmNotification(
                payments_grpc::FrmNotificationContent {
                    connector_transaction_id: context.connector_transaction_id.clone(),
                    amount: Some(payments_grpc::Money {
                        minor_amount: context.amount.get_amount_as_i64(),
                        currency: grpc_currency.into(),
                    }),
                    frm_transaction_id: context.frm_transaction_id.clone(),
                    notification_type: Some(notification.notification_type()),
                    ..Default::default()
                },
            ),
        ),
    };

    let request = payments_grpc::NotifyConnectorRequest {
        event_id: format!("frm-{}", common_utils::generate_id_with_default_len("evt")),
        event_type: event_type.into(),
        content: Some(content),
        timestamp: common_utils::date_time::now_unix_timestamp(),
        ..Default::default()
    };

    let grpc_headers = state
        .get_grpc_headers_ucs(common_enums::ExecutionMode::Primary)
        .lineage_ids(LineageIds::new(
            processor.get_account().get_id().clone(),
            context.profile_id.clone(),
        ))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .build();

    ucs_client
        .notify_connector(request, connector_auth_metadata, grpc_headers, event_type)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("UCS notify_connector gRPC call failed for the FRM notification")?;

    Ok(())
}
