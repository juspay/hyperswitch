mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};

use self::transformers as applepay;
use crate::{
    configs::settings::Connectors,
    core::errors::{self, CustomResult},
    headers, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Applepay;

impl api::ConnectorCommon for Applepay {
    fn id(&self) -> &'static str {
        "applepay"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.applepay.base_url
    }
}

impl api::Payment for Applepay {}
impl api::PaymentAuthorize for Applepay {}
impl api::PaymentSync for Applepay {}
impl api::PaymentVoid for Applepay {}
impl api::PaymentCapture for Applepay {}
impl api::PreVerify for Applepay {}
impl api::PaymentSession for Applepay {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Applepay
{
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Applepay
{
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Applepay
{
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Applepay
{
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Applepay
{
}

#[async_trait::async_trait]
impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Applepay
{
    fn get_headers(
        &self,
        _req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSessionType::get_content_type(self).to_string(),
        )];
        Ok(header)
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSessionRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "paymentservices/paymentSession"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req = utils::Encode::<applepay::ApplepaySessionRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            // TODO: [ORCA-346] Requestbuilder needs &str migrate get_url to send &str instead of owned string
            .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
            .headers(types::PaymentsSessionType::get_headers(self, req)?)
            .body(types::PaymentsSessionType::get_request_body(self, req)?)
            .add_certificate(types::PaymentsSessionType::get_certificate(self, req)?)
            .add_certificate_key(types::PaymentsSessionType::get_certificate_key(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: applepay::ApplepaySessionResponse = res
            .response
            .parse_struct("ApplepaySessionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: applepay::ErrorResponse = res
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response.status_code,
            message: response.status_message,
            reason: None,
        })
    }

    fn get_certificate(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        req.request
            .certificate
            .to_owned()
            .get_required_value("certificate")
            .change_context(errors::ConnectorError::FailedToObtainCertificate)
    }

    fn get_certificate_key(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        req.request
            .certificate_keys
            .to_owned()
            .get_required_value("certificate_keys")
            .change_context(errors::ConnectorError::FailedToObtainCertificateKey)
    }
}

impl api::Refund for Applepay {}
impl api::RefundExecute for Applepay {}
impl api::RefundSync for Applepay {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Applepay
{
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Applepay
{
}

impl services::ConnectorRedirectResponse for Applepay {}

#[async_trait::async_trait]
impl api::IncomingWebhook for Applepay {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
