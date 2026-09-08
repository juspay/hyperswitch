pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{consts::BASE64_ENGINE, errors::CustomResult, request::Request};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSessionRouterData, PaymentsSyncRouterData, RefreshTokenRouterData,
        RefundExecuteRouterData, RefundSyncRouterData, SetupMandateRouterData,
        TokenizationRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};
use hyperswitch_masking::{Mask, PeekInterface};
use transformers as saferpay;

use crate::{constants::headers, utils::PaymentsAuthorizeRequestData};

/// Saferpay (SIX Payment Services).
///
/// Saferpay is a UCS-only connector: every payment flow is executed by the
/// Unified Connector Service. This struct only exists so that Saferpay can be
/// registered as a `Connector` on the Hyperswitch side (merchant connector
/// account creation, routing, feature matrix). All direct flow implementations
/// intentionally return `FlowNotSupported`.
#[derive(Clone)]
pub struct Saferpay {}

impl Saferpay {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Saferpay {}
impl api::PaymentSession for Saferpay {}
impl api::ConnectorAccessToken for Saferpay {}
impl api::MandateSetup for Saferpay {}
impl api::PaymentAuthorize for Saferpay {}
impl api::PaymentSync for Saferpay {}
impl api::PaymentCapture for Saferpay {}
impl api::PaymentVoid for Saferpay {}
impl api::Refund for Saferpay {}
impl api::RefundExecute for Saferpay {}
impl api::RefundSync for Saferpay {}
impl api::PaymentToken for Saferpay {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Saferpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut auth_header);
        Ok(header)
    }
}

impl ConnectorCommon for Saferpay {
    fn id(&self) -> &'static str {
        "saferpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.saferpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = saferpay::SaferpayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let raw_basic_token = format!("{}:{}", auth.api_username.peek(), auth.api_password.peek());
        let basic_token = format!("Basic {}", BASE64_ENGINE.encode(raw_basic_token));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            basic_token.into_masked(),
        )])
    }
}

impl ConnectorValidation for Saferpay {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Session".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Saferpay
{
    fn build_request(
        &self,
        _req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentMethodToken".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Saferpay {
    fn build_request(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "AccessTokenAuth".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Saferpay
{
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "SetupMandate".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Authorize".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PSync".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Execute".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Saferpay {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "RSync".to_string(),
            connector: "Saferpay".to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Saferpay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static SAFERPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        // Manual only. Saferpay has no sale mode — no capture field on its authorize
        // calls, no combined authorize+capture endpoint, and no terminal-level capture
        // setting — so an authorization is always settled by an explicit Capture.
        let supported_capture_methods = vec![enums::CaptureMethod::Manual];
        let supported_card_network = vec![
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::UnionPay,
            common_enums::CardNetwork::Visa,
        ];

        let mut saferpay_supported_payment_methods = SupportedPaymentMethods::new();

        for payment_method_type in [
            enums::PaymentMethodType::Credit,
            enums::PaymentMethodType::Debit,
        ] {
            saferpay_supported_payment_methods.add(
                enums::PaymentMethod::Card,
                payment_method_type,
                PaymentMethodDetails {
                    mandates: enums::FeatureStatus::NotSupported,
                    refunds: enums::FeatureStatus::Supported,
                    supported_capture_methods: supported_capture_methods.clone(),
                    specific_features: Some(
                        api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                            api_models::feature_matrix::CardSpecificFeatures {
                                three_ds: common_enums::FeatureStatus::Supported,
                                no_three_ds: common_enums::FeatureStatus::Supported,
                                supported_card_networks: supported_card_network.clone(),
                            },
                        ),
                    ),
                },
            );
        }

        saferpay_supported_payment_methods
    });

static SAFERPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Saferpay",
    description:
        "Saferpay is the payment gateway of SIX Payment Services (Worldline), widely used by merchants in Switzerland, Austria, Germany and Luxembourg",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static SAFERPAY_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Saferpay {
    /// Saferpay runs 3-D Secure itself: `Transaction/Initialize` returns a redirect to
    /// its own hosted DCC + 3DS pages. That opening call is the PreAuthenticate leg, so
    /// it must fire on Authorize for any 3DS card attempt. Without this the connector
    /// would fall through to `AuthorizeDirect`, which never authenticates.
    fn is_pre_authentication_flow_required(&self, current_flow: api::CurrentFlowInfo) -> bool {
        match current_flow {
            api::CurrentFlowInfo::Authorize {
                request_data,
                auth_type,
            } => auth_type == common_enums::AuthenticationType::ThreeDs && request_data.is_card(),
            api::CurrentFlowInfo::CompleteAuthorize { .. }
            | api::CurrentFlowInfo::SetupMandate { .. }
            | api::CurrentFlowInfo::Psync { .. }
            | api::CurrentFlowInfo::UpdatePostConfirm { .. }
            | api::CurrentFlowInfo::ConnectorWebhookRegister { .. } => false,
        }
    }

    /// Both authenticate legs stay off. They exist to hand `AuthenticationData` to a
    /// following Authorize; Saferpay's second call *is* the authorization, so it runs as
    /// the complete-authorize Authorize instead. Leaving `is_post_authentication_flow_required`
    /// at its `false` default is what lets the return go straight there.
    fn is_authentication_flow_required(&self, _current_flow: api::CurrentFlowInfo) -> bool {
        false
    }

    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&SAFERPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*SAFERPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&SAFERPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
