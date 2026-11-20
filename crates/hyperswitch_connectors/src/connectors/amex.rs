//! Local connector registration for Amex. Payment and refund calls run through UCS.

use std::sync::LazyLock;

use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse},
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
    api, configs::Connectors, errors, events::connector_api_logs::ConnectorEvent, types::Response,
    webhooks,
};

#[derive(Clone)]
pub struct Amex;

impl Amex {
    pub fn new() -> &'static Self {
        &Self
    }
}

impl api::Payment for Amex {}
impl api::PaymentSession for Amex {}
impl api::ConnectorAccessToken for Amex {}
impl api::MandateSetup for Amex {}
impl api::PaymentAuthorize for Amex {}
impl api::PaymentSync for Amex {}
impl api::PaymentCapture for Amex {}
impl api::PaymentVoid for Amex {}
impl api::Refund for Amex {}
impl api::RefundExecute for Amex {}
impl api::RefundSync for Amex {}
impl api::PaymentToken for Amex {}

impl api::ConnectorCommon for Amex {
    fn id(&self) -> &'static str {
        "amex"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.amex.base_url.as_ref()
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
        Err(errors::ConnectorError::NotImplemented("Amex uses UCS".to_string()).into())
    }
}

impl api::ConnectorValidation for Amex {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Amex {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Amex {}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Amex
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Amex {}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Amex {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Amex {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Amex {}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Amex {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Amex {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Amex
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Amex {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented.into())
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented.into())
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err(errors::ConnectorError::WebhooksNotImplemented.into())
    }
}

static AMEX_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static AMEX_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Amex",
    description: "American Express gateway via UCS",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Alpha,
};

static AMEX_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl api::ConnectorSpecifications for Amex {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&AMEX_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*AMEX_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&AMEX_SUPPORTED_WEBHOOK_FLOWS)
    }
}
