pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::errors::CustomResult;
use error_stack::report;
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
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};

#[derive(Clone)]
pub struct Betterpayment {}

impl Betterpayment {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Betterpayment {}
impl api::PaymentSession for Betterpayment {}
impl api::ConnectorAccessToken for Betterpayment {}
impl api::MandateSetup for Betterpayment {}
impl api::PaymentAuthorize for Betterpayment {}
impl api::PaymentSync for Betterpayment {}
impl api::PaymentCapture for Betterpayment {}
impl api::PaymentVoid for Betterpayment {}
impl api::Refund for Betterpayment {}
impl api::RefundExecute for Betterpayment {}
impl api::RefundSync for Betterpayment {}
impl api::PaymentToken for Betterpayment {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Betterpayment
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Betterpayment
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(vec![])
    }
}

impl ConnectorCommon for Betterpayment {
    fn id(&self) -> &'static str {
        "betterpayment"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.betterpayment.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(vec![])
    }
}

impl ConnectorValidation for Betterpayment {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Betterpayment {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Betterpayment
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Betterpayment
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Betterpayment {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Betterpayment {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Betterpayment {
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

static BETTERPAYMENT_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let mut supported = SupportedPaymentMethods::new();
        supported.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::Wero,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: vec![enums::CaptureMethod::Automatic],
                specific_features: None,
            },
        );
        supported
    });

static BETTERPAYMENT_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Betterpayment",
    description: "Better Payment Germany GmbH (Deutsche Bank Group) — Wero digital wallet payments",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static BETTERPAYMENT_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Betterpayment {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&BETTERPAYMENT_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*BETTERPAYMENT_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&BETTERPAYMENT_SUPPORTED_WEBHOOK_FLOWS)
    }
}
