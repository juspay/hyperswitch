pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType},
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
        ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};
use transformers as rapidconnect;

#[derive(Clone)]
pub struct Rapidconnect;

impl Rapidconnect {
    pub fn new() -> &'static Self {
        &Self
    }
}

impl api::Payment for Rapidconnect {}
impl api::PaymentSession for Rapidconnect {}
impl api::ConnectorAccessToken for Rapidconnect {}
impl api::MandateSetup for Rapidconnect {}
impl api::PaymentAuthorize for Rapidconnect {}
impl api::PaymentSync for Rapidconnect {}
impl api::PaymentCapture for Rapidconnect {}
impl api::PaymentVoid for Rapidconnect {}
impl api::Refund for Rapidconnect {}
impl api::RefundExecute for Rapidconnect {}
impl api::RefundSync for Rapidconnect {}
impl api::PaymentToken for Rapidconnect {}

impl ConnectorCommon for Rapidconnect {
    fn id(&self) -> &'static str {
        "rapidconnect"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.rapidconnect.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        // Datawire carries the credentials in the request envelope, not in an HTTP header,
        // so only validate the auth type here
        rapidconnect::RapidconnectAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![])
    }
}

impl ConnectorValidation for Rapidconnect {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Rapidconnect
{
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Rapidconnect {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Rapidconnect {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Rapidconnect
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Rapidconnect {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Rapidconnect {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Rapidconnect {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Rapidconnect {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Rapidconnect {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Rapidconnect {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Rapidconnect {
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

static RAPIDCONNECT_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static RAPIDCONNECT_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Rapidconnect",
    description: "Rapidconnect connector",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static RAPIDCONNECT_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Rapidconnect {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&RAPIDCONNECT_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*RAPIDCONNECT_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&RAPIDCONNECT_SUPPORTED_WEBHOOK_FLOWS)
    }
}
