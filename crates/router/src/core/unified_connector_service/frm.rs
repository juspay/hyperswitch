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

use std::str::FromStr;

use common_enums::connector_enums::Connector;
use common_utils::{errors::CustomResult, id_type, types::MinorUnit};
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{
    platform::Processor, router_request_types::ResponseId,
    router_response_types::fraud_check::FraudCheckResponseData, types::OrderDetailsWithAmount,
};
use hyperswitch_interfaces::unified_connector_service::{
    transformers, UnifiedConnectorServiceError,
};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use unified_connector_service_client::payments as payments_grpc;

use super::{build_unified_connector_service_auth_metadata, get_ucs_client};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers::MerchantConnectorAccountType,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

/// Decide whether this FRM connector's risk evaluation should be executed by
/// the connector-service.
///
/// Mirrors `should_call_unified_connector_service` for payments: the connector
/// must be listed in `ucs_frm_connectors`, and UCS must actually be available
/// (client constructed and `UCS_ENABLED` set). Nothing is hardcoded — a
/// connector is routed to UCS only because configuration says so.
///
/// Unlike payments there is no Direct or Shadow path: a UCS-backed FRM provider
/// has no in-process connector to fall back to or shadow against, so the result
/// is a plain "use UCS or don't".
pub async fn should_call_unified_connector_service_for_frm(
    state: &SessionState,
    connector_name: &str,
) -> bool {
    let Ok(connector) = Connector::from_str(connector_name) else {
        router_env::logger::debug!(
            connector = connector_name,
            "FRM connector name is not a known connector; not routing to UCS"
        );
        return false;
    };

    let Some(ucs_config) = state.conf.grpc_client.unified_connector_service.as_ref() else {
        router_env::logger::debug!("UCS config not present; FRM will not be routed to UCS");
        return false;
    };

    if !ucs_config.ucs_frm_connectors.contains(&connector) {
        router_env::logger::debug!(
            connector = ?connector,
            "FRM connector not in ucs_frm_connectors; not routing to UCS"
        );
        return false;
    }

    // The connector is configured for UCS, so there is no native path. If UCS is
    // unavailable the risk evaluation cannot run at all — surface that clearly
    // rather than letting it look like a connector error.
    match super::check_ucs_availability(state).await {
        common_enums::UcsAvailability::Enabled => true,
        common_enums::UcsAvailability::Disabled => {
            router_env::logger::error!(
                connector = ?connector,
                "UCS is unavailable but FRM connector has no in-process implementation; \
                 the risk evaluation will not run for this payment"
            );
            false
        }
    }
}

/// The facts a pre-risk-check needs, gathered from `FrmData` at the call site.
///
/// Named fields rather than positional arguments: `amount`/`currency` and the
/// several `Option<&…>` values are easy to transpose in a long parameter list,
/// and the compiler would not catch it.
pub struct FrmPreRiskCheckContext<'a> {
    pub amount: MinorUnit,
    pub currency: common_enums::Currency,
    pub customer_id: Option<&'a id_type::CustomerId>,
    pub browser_info:
        Option<&'a hyperswitch_domain_models::router_request_types::BrowserInformation>,
    pub address: &'a hyperswitch_domain_models::payment_address::PaymentAddress,
    pub order_details: Option<&'a Vec<OrderDetailsWithAmount>>,
    pub merchant_transaction_id: String,
    /// Merchant-supplied provider signals (device id, tenure, velocity, …),
    /// forwarded verbatim as `connector_feature_data`.
    pub frm_metadata: Option<&'a common_utils::pii::SecretSerdeValue>,
    /// OAuth token for providers whose risk API is bearer-authenticated (Kount).
    /// Providers using a static key (nSure) leave this unset.
    pub access_token: Option<&'a hyperswitch_domain_models::router_data::AccessToken>,
}

impl ForeignTryFrom<FrmPreRiskCheckContext<'_>> for payments_grpc::FrmServicePreRiskCheckRequest {
    type Error = error_stack::Report<UnifiedConnectorServiceError>;

    fn foreign_try_from(ctx: FrmPreRiskCheckContext<'_>) -> Result<Self, Self::Error> {
        let grpc_currency = payments_grpc::Currency::foreign_try_from(ctx.currency)?;

        let amount = payments_grpc::Money {
            minor_amount: ctx.amount.get_amount_as_i64(),
            currency: grpc_currency.into(),
        };

        // `customer_id` is the stable merchant-side key risk providers use to
        // build cross-transaction history for the buyer.
        let customer_info = ctx.customer_id.map(|id| payments_grpc::Customer {
            id: Some(id.get_string_repr().to_owned()),
            ..Default::default()
        });

        let browser_info = ctx
            .browser_info
            .map(|info| payments_grpc::BrowserInformation {
                user_agent: info.user_agent.clone(),
                ip_address: info.ip_address.map(|ip| ip.to_string()),
                language: info.language.clone(),
                accept_header: info.accept_header.clone(),
                ..Default::default()
            });

        let order_details = ctx
            .order_details
            .map(|details| details.iter().map(build_order_detail).collect())
            .unwrap_or_default();

        Ok(Self {
            amount: Some(amount),
            customer_info,
            browser_info,
            merchant_transaction_id: Some(ctx.merchant_transaction_id),
            order_details,
            address: build_payment_address(ctx.address),
            connector_feature_data: ctx
                .frm_metadata
                .map(|metadata| Secret::new(metadata.clone().expose().to_string())),
            // Bearer-authenticated providers read the token from
            // `state.access_token`; prism threads it onto FrmFlowData.
            state: ctx.access_token.map(|token| payments_grpc::ConnectorState {
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

fn build_order_detail(detail: &OrderDetailsWithAmount) -> payments_grpc::OrderDetailsWithAmount {
    payments_grpc::OrderDetailsWithAmount {
        product_name: detail.product_name.clone(),
        quantity: u32::from(detail.quantity),
        amount: detail.amount.get_amount_as_i64(),
        requires_shipping: detail.requires_shipping,
        product_id: detail.product_id.clone(),
        category: detail.category.clone(),
        sub_category: detail.sub_category.clone(),
        brand: detail.brand.clone(),
        // `sku` / `product_link` exist on the UCS proto but not on
        // Hyperswitch's domain type, so they are left unset here.
        ..Default::default()
    }
}

fn build_payment_address(
    address: &hyperswitch_domain_models::payment_address::PaymentAddress,
) -> Option<payments_grpc::PaymentAddress> {
    let billing_address = address.get_payment_billing().map(build_address);
    let shipping_address = address.get_shipping().map(build_address);
    (billing_address.is_some() || shipping_address.is_some()).then_some(
        payments_grpc::PaymentAddress {
            billing_address,
            shipping_address,
        },
    )
}

fn build_address(address: &hyperswitch_domain_models::address::Address) -> payments_grpc::Address {
    let details = address.address.as_ref();
    let secret = |value: Option<&Secret<String>>| value.cloned();
    payments_grpc::Address {
        first_name: secret(details.and_then(|d| d.first_name.as_ref())),
        last_name: secret(details.and_then(|d| d.last_name.as_ref())),
        line1: secret(details.and_then(|d| d.line1.as_ref())),
        line2: secret(details.and_then(|d| d.line2.as_ref())),
        line3: secret(details.and_then(|d| d.line3.as_ref())),
        city: details.and_then(|d| d.city.clone()).map(Secret::new),
        state: secret(details.and_then(|d| d.state.as_ref())),
        zip_code: secret(details.and_then(|d| d.zip.as_ref())),
        country_alpha2_code: details
            .and_then(|d| d.country)
            .and_then(|country| payments_grpc::CountryAlpha2::from_str_name(&country.to_string()))
            .map(|code| code.into()),
        email: address
            .email
            .as_ref()
            .map(|email| Secret::new(email.peek().to_owned())),
        phone_number: secret(
            address
                .phone
                .as_ref()
                .and_then(|phone| phone.number.as_ref()),
        ),
        phone_country_code: address
            .phone
            .as_ref()
            .and_then(|phone| phone.country_code.clone()),
    }
}

/// Convert the UCS verdict back into Hyperswitch's FRM response shape.
///
/// Deliberately conservative: anything that is not an explicit approval leaves
/// the transaction short of `Legit`, so an unrecognised or missing verdict never
/// silently approves a payment.
pub fn handle_unified_connector_service_response_for_frm_pre_risk_check(
    response: payments_grpc::FrmServicePreRiskCheckResponse,
) -> CustomResult<FraudCheckResponseData, UnifiedConnectorServiceError> {
    use payments_grpc::FrmDecision;

    // Same status-code handling every payments UCS handler performs: a
    // non-success code from the connector-service means the provider never
    // produced a verdict, so it must not be read as an implicit approval.
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let decision = response
        .frm_decision
        .and_then(|decision| FrmDecision::try_from(decision).ok());

    let status = match (status_code, decision) {
        (200..=299, Some(FrmDecision::Approve)) => diesel_models::enums::FraudCheckStatus::Legit,
        (200..=299, Some(FrmDecision::Reject)) => diesel_models::enums::FraudCheckStatus::Fraud,
        (200..=299, Some(FrmDecision::Review)) => {
            diesel_models::enums::FraudCheckStatus::ManualReview
        }
        // `Error`/`Unspecified`/absent verdict, or any non-2xx status: the
        // provider gave us nothing usable. Hold for review rather than approve.
        (200..=299, _) => diesel_models::enums::FraudCheckStatus::ManualReview,
        (code, _) => {
            router_env::logger::warn!(
                status_code = code,
                "FRM pre risk check returned a non-success status; treating as manual review"
            );
            diesel_models::enums::FraudCheckStatus::ManualReview
        }
    };

    Ok(FraudCheckResponseData::TransactionResponse {
        resource_id: response
            .frm_transaction_id
            .clone()
            .map(ResponseId::ConnectorTransactionId)
            .unwrap_or(ResponseId::NoResponseId),
        status,
        connector_metadata: None,
        reason: response.reason.map(serde_json::Value::String),
        score: response.risk_score,
    })
}

/// Execute the pre-authorization risk check against the connector-service.
///
/// Mirrors `call_unified_connector_service_for_surcharge_calculate`; the
/// connector name travels as `x-frm-connector` rather than `x-connector`.
#[cfg(feature = "v1")]
pub async fn call_unified_connector_service_for_frm_pre_risk_check(
    state: &SessionState,
    processor: &Processor,
    merchant_connector_account: MerchantConnectorAccountType,
    connector_name: String,
    profile_id: &id_type::ProfileId,
    context: FrmPreRiskCheckContext<'_>,
) -> RouterResult<FraudCheckResponseData> {
    let ucs_client = get_ucs_client(state)?;

    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        processor.get_account().get_id(),
        connector_name,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for the FRM pre risk check")?;

    let request = payments_grpc::FrmServicePreRiskCheckRequest::foreign_try_from(context)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build the FRM pre risk check request")?;

    let lineage_ids = LineageIds::new(processor.get_account().get_id().clone(), profile_id.clone());

    let grpc_headers = state
        .get_grpc_headers_ucs(common_enums::ExecutionMode::Primary)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
        .build();

    let response = ucs_client
        .frm_pre_risk_check(request, connector_auth_metadata, grpc_headers)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("UCS frm_pre_risk_check gRPC call failed")?;

    handle_unified_connector_service_response_for_frm_pre_risk_check(response.into_inner())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse the UCS FRM pre risk check response")
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
    profile_id: &id_type::ProfileId,
) -> RouterResult<Option<hyperswitch_domain_models::router_data::AccessToken>> {
    let merchant_id = processor.get_account().get_id();
    let access_token_key = common_utils::access_token::get_default_access_token_key(
        merchant_id,
        connector_name.to_string(),
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
        .lineage_ids(LineageIds::new(merchant_id.clone(), profile_id.clone()))
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
                None,
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
