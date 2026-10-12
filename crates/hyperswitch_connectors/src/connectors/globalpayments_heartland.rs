pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{errors::CustomResult, request::Request};
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
use transformers as globalpayments_heartland;

use crate::constants::headers;

/// Global Payments Heartland (Portico gateway).
///
/// Distinct from the existing `globalpay` connector, which is Global Payments'
/// GP-API / Realex platform. This one is the Heartland Portico SOAP gateway
/// (`Hps.Exchange.PosGateway`).
///
/// Heartland is a UCS-only connector: every payment flow is executed by the
/// Unified Connector Service. This struct only exists so that Heartland can be
/// registered as a `Connector` on the Hyperswitch side (merchant connector
/// account creation, routing, feature matrix). All direct flow implementations
/// intentionally return `FlowNotSupported`.
#[derive(Clone)]
pub struct GlobalpaymentsHeartland {}

impl GlobalpaymentsHeartland {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for GlobalpaymentsHeartland {}
impl api::PaymentSession for GlobalpaymentsHeartland {}
impl api::ConnectorAccessToken for GlobalpaymentsHeartland {}
impl api::MandateSetup for GlobalpaymentsHeartland {}
impl api::PaymentAuthorize for GlobalpaymentsHeartland {}
impl api::PaymentSync for GlobalpaymentsHeartland {}
impl api::PaymentCapture for GlobalpaymentsHeartland {}
impl api::PaymentVoid for GlobalpaymentsHeartland {}
impl api::Refund for GlobalpaymentsHeartland {}
impl api::RefundExecute for GlobalpaymentsHeartland {}
impl api::RefundSync for GlobalpaymentsHeartland {}
impl api::PaymentToken for GlobalpaymentsHeartland {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response>
    for GlobalpaymentsHeartland
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

impl ConnectorCommon for GlobalpaymentsHeartland {
    fn id(&self) -> &'static str {
        "globalpayments_heartland"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "text/xml; charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.globalpayments_heartland.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        // Validate the shape of the merchant connector account auth, then send no
        // header at all: Portico carries its SecretAPIKey inside the SOAP body at
        // Ver1.0/Header/SecretAPIKey, not in an HTTP header.
        let _auth = globalpayments_heartland::GlobalpaymentsHeartlandAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![])
    }
}

impl ConnectorValidation for GlobalpaymentsHeartland {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Session".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentMethodToken".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "AccessTokenAuth".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "SetupMandate".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Authorize".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PSync".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>
    for GlobalpaymentsHeartland
{
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for GlobalpaymentsHeartland {
    fn build_request(
        &self,
        _req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Execute".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for GlobalpaymentsHeartland {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "RSync".to_string(),
            connector: "GlobalpaymentsHeartland".to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for GlobalpaymentsHeartland {
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

static GLOBALPAYMENTS_HEARTLAND_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        // Portico supports both modes natively: CreditSale is a single-message sale,
        // and CreditAuth settles later via CreditAddToBatch. Both verified against the
        // cert gateway.
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
        ];
        let supported_card_network = vec![
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::Visa,
        ];

        let mut globalpayments_heartland_supported_payment_methods = SupportedPaymentMethods::new();

        for payment_method_type in [
            enums::PaymentMethodType::Credit,
            enums::PaymentMethodType::Debit,
        ] {
            globalpayments_heartland_supported_payment_methods.add(
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

        globalpayments_heartland_supported_payment_methods
    });

static GLOBALPAYMENTS_HEARTLAND_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Global Payments Heartland",
    description:
        "Global Payments' Heartland Portico gateway (Hps.Exchange.PosGateway), a SOAP card-acquiring platform used by merchants across the United States",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static GLOBALPAYMENTS_HEARTLAND_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for GlobalpaymentsHeartland {
    // No `is_pre_authentication_flow_required` / `is_authentication_flow_required`
    // overrides: Portico does not host a 3-D Secure challenge and returns no ACS URL.
    // 3DS here is pure pass-through — the CAVV, ECI and directory-server transaction id
    // ride along on the authorization inside a <Secure3D> block. The `false` defaults
    // are therefore correct.

    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&GLOBALPAYMENTS_HEARTLAND_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*GLOBALPAYMENTS_HEARTLAND_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&GLOBALPAYMENTS_HEARTLAND_SUPPORTED_WEBHOOK_FLOWS)
    }
}
