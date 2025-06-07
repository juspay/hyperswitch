use common_utils::errors::CustomResult;
use error_stack::report;
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};

use crate::constants::headers;

#[derive(Clone)]
pub struct CtpMastercard;

impl api::Payment for CtpMastercard {}
impl api::PaymentSession for CtpMastercard {}
impl api::ConnectorAccessToken for CtpMastercard {}
impl api::MandateSetup for CtpMastercard {}
impl api::PaymentAuthorize for CtpMastercard {}
impl api::PaymentSync for CtpMastercard {}
impl api::PaymentCapture for CtpMastercard {}
impl api::PaymentVoid for CtpMastercard {}
impl api::Refund for CtpMastercard {}
impl api::RefundExecute for CtpMastercard {}
impl api::RefundSync for CtpMastercard {}
impl api::PaymentToken for CtpMastercard {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for CtpMastercard
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for CtpMastercard
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for CtpMastercard {
    fn id(&self) -> &'static str {
        "ctp_mastercard"
    }

    fn base_url<'a>(&self, _connectors: &'a Connectors) -> &'a str {
        ""
    }
}

impl ConnectorValidation for CtpMastercard {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for CtpMastercard {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for CtpMastercard {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for CtpMastercard
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for CtpMastercard
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for CtpMastercard {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for CtpMastercard {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for CtpMastercard {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for CtpMastercard {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for CtpMastercard {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for CtpMastercard {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorSpecifications for CtpMastercard {}
