//! Pay.com (`paydotcom`).
//!
//! Pay.com is a **UCS-only** connector: it is listed in `ucs_only_connectors`, so every
//! payment is executed over `ExecutionPath::UnifiedConnectorService` and the real
//! integration lives in connector-service (`connectors/paydotcom.rs` there). This module
//! exists only because a few orchestration hooks hang off the *native* connector trait
//! object even for UCS-only connectors — specifically
//! [`ConnectorSpecifications::is_pre_authentication_flow_required`], which tells the
//! Authorize flow to run a PreAuthenticate leg first for gateway-driven 3DS.
//!
//! The same pattern is used by `moneris` and `tsys_transit`; none of the
//! `ConnectorIntegration` impls below are ever exercised on the UCS path.
//!
//! # Gateway 3DS
//!
//! Pay.com's challenge journey is three HTTP calls, split across three flow executions:
//!
//! | # | HS flow | UCS RPC | Pay.com call |
//! |---|---|---|---|
//! | 1 | `pre_authentication_step` | `PaymentMethodAuthenticationService.PreAuthenticate` | `POST /v1/charges\|/v1/holds` — mints the `chrg_`/`hld_` id |
//! | 2 | `Authorize` | `PaymentService.Authorize` | `POST /v1/sessions/authentication/linked` — mints the challenge URL |
//! | 3 | `CompleteAuthorize` | `PaymentService.Authorize` | `POST /v1/{charges\|holds}/{id}/confirm` |
//!
//! The resource id minted by leg 1 travels to leg 2 on `connector_feature_data`, and to
//! leg 3 on the attempt's `connector_metadata`.

use std::sync::LazyLock;

use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, PostCaptureVoidSync, PreAuthorizeVoid,
            Session, SetupMandate, UpdatePostConfirm, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCancelPostCaptureSyncData, PaymentsCaptureData,
        PaymentsPreAuthorizeCancelData, PaymentsSessionData, PaymentsSyncData,
        PaymentsUpdatePostConfirmData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
};
use hyperswitch_interfaces::{
    api, configs::Connectors, errors, events::connector_api_logs::ConnectorEvent, types::Response,
    webhooks,
};
use hyperswitch_masking::Secret;

use crate::utils::PaymentsAuthorizeRequestData;

#[derive(Clone)]
pub struct Paydotcom {}

impl Paydotcom {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

/// Pay.com authenticates with a single API key sent as `x-paycom-api-key`
/// (`test_…` on sandbox, `live_…` in production). Mirrors the connector-service
/// `PaydotcomConfig` message, which carries exactly one field.
pub struct PaydotcomAuthType {
    pub api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaydotcomAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl api::Payment for Paydotcom {}
impl api::PaymentSession for Paydotcom {}
impl api::ConnectorAccessToken for Paydotcom {}
impl api::MandateSetup for Paydotcom {}
impl api::PaymentAuthorize for Paydotcom {}
impl api::PaymentSync for Paydotcom {}
impl api::PaymentCapture for Paydotcom {}
impl api::PaymentVoid for Paydotcom {}
impl api::PaymentUpdate for Paydotcom {}
impl api::PaymentPreAuthorizeVoid for Paydotcom {}
impl api::PaymentPostCaptureVoidSync for Paydotcom {}
impl api::Refund for Paydotcom {}
impl api::RefundExecute for Paydotcom {}
impl api::RefundSync for Paydotcom {}
impl api::PaymentToken for Paydotcom {}

impl api::ConnectorCommon for Paydotcom {
    fn id(&self) -> &'static str {
        "paydotcom"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.paydotcom.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(Vec::new())
    }

    fn build_error_response(
        &self,
        _res: Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Paydotcom".to_string()).into())
    }
}

impl api::ConnectorValidation for Paydotcom {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Paydotcom {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Paydotcom {}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Paydotcom
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Paydotcom
{
}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Paydotcom {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Paydotcom {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Paydotcom {}
impl
    api::ConnectorIntegration<
        UpdatePostConfirm,
        PaymentsUpdatePostConfirmData,
        PaymentsResponseData,
    > for Paydotcom
{
}
impl
    api::ConnectorIntegration<
        PreAuthorizeVoid,
        PaymentsPreAuthorizeCancelData,
        PaymentsResponseData,
    > for Paydotcom
{
}
impl
    api::ConnectorIntegration<
        PostCaptureVoidSync,
        PaymentsCancelPostCaptureSyncData,
        PaymentsResponseData,
    > for Paydotcom
{
}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Paydotcom {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Paydotcom {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Paydotcom
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Paydotcom {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }
}

static PAYDOTCOM_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        // Pay.com holds are captured with `/v1/holds/{id}/capture`; charges settle immediately.
        let default_capture_methods = vec![
            common_enums::CaptureMethod::Automatic,
            common_enums::CaptureMethod::Manual,
            common_enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::UnionPay,
        ];

        let card_details = |networks: Vec<common_enums::CardNetwork>| PaymentMethodDetails {
            // One-time card payments only: no mandates / MIT on this integration.
            mandates: common_enums::FeatureStatus::NotSupported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: networks,
                    },
                ),
            ),
        };

        let mut paydotcom_supported_payment_methods = SupportedPaymentMethods::new();

        paydotcom_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Credit,
            card_details(supported_card_network.clone()),
        );

        paydotcom_supported_payment_methods.add(
            common_enums::PaymentMethod::Card,
            common_enums::PaymentMethodType::Debit,
            card_details(supported_card_network.clone()),
        );

        paydotcom_supported_payment_methods
    });

static PAYDOTCOM_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Pay.com",
    description: "Pay.com is a payment gateway offering card acquiring over its REST API v1.",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Beta,
};

static PAYDOTCOM_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl api::ConnectorSpecifications for Paydotcom {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PAYDOTCOM_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PAYDOTCOM_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&PAYDOTCOM_SUPPORTED_WEBHOOK_FLOWS)
    }

    /// Gateway-driven 3DS runs as PreAuthenticate -> Authenticate -> CompleteAuthorize.
    /// Leg 1 mints the `chrg_`/`hld_` id the challenge will authenticate. Non-3DS and
    /// external-MPI 3DS are single-call Authorize and must not take this path.
    fn is_pre_authentication_flow_required(&self, current_flow: api::CurrentFlowInfo) -> bool {
        match current_flow {
            api::CurrentFlowInfo::Authorize {
                auth_type,
                request_data,
            } => {
                auth_type.is_three_ds()
                    && request_data.is_card()
                    // External-MPI 3DS already carries the merchant's own eci/cavv and
                    // settles in a single Authorize call — no PreAuthenticate leg.
                    && request_data.authentication_data.is_none()
            }
            api::CurrentFlowInfo::CompleteAuthorize { .. }
            | api::CurrentFlowInfo::SetupMandate { .. }
            | api::CurrentFlowInfo::Psync { .. }
            | api::CurrentFlowInfo::UpdatePostConfirm { .. }
            | api::CurrentFlowInfo::ConnectorWebhookRegister { .. } => false,
        }
    }

    /// Leg 2 of the same journey: connector-service turns the id leg 1 minted into a
    /// linked authentication session and returns the challenge URL. The gate is identical
    /// to `is_pre_authentication_flow_required` because the two legs are inseparable —
    /// every PreAuthenticate this connector runs leaves a resource that only the
    /// Authenticate leg can hand a shopper.
    ///
    /// The `chrg_`/`hld_` id reaches this leg on `authentication_data`, which
    /// `authentication_step` already carries over from the PreAuthenticate response.
    fn is_authentication_flow_required(&self, current_flow: api::CurrentFlowInfo) -> bool {
        match current_flow {
            api::CurrentFlowInfo::Authorize {
                auth_type,
                request_data,
            } => {
                auth_type.is_three_ds()
                    && request_data.is_card()
                    && request_data.authentication_data.is_none()
            }
            api::CurrentFlowInfo::CompleteAuthorize { .. }
            | api::CurrentFlowInfo::SetupMandate { .. }
            | api::CurrentFlowInfo::Psync { .. }
            | api::CurrentFlowInfo::UpdatePostConfirm { .. }
            | api::CurrentFlowInfo::ConnectorWebhookRegister { .. } => false,
        }
    }
}
