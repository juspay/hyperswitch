#![allow(dead_code)]
mod transformers;
use std::fmt::Debug;

use bytes::Bytes;
use error_stack::{IntoReport, ResultExt};
use transformers as klarna;

use crate::{
    configs::settings::Connectors,
    core::errors::{self, CustomResult},
    headers,
    services::{self, logger},
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Klarna;

impl api::ConnectorCommon for Klarna {
    fn id(&self) -> &'static str {
        "klarna"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.klarna.base_url
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: klarna::KlarnaAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.basic_token)])
    }
}

impl api::Payment for Klarna {}

impl api::PaymentAuthorize for Klarna {}
impl api::PaymentSync for Klarna {}
impl api::PaymentVoid for Klarna {}
impl api::PaymentCapture for Klarna {}
impl api::PaymentSession for Klarna {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Klarna
{
    fn get_headers(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsAuthorizeType::get_content_type(self).to_string(),
            ),
            (headers::X_ROUTER.to_string(), "test".to_string()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSessionRouterData,
        connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "payments/v1/sessions"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        // encode only for for urlencoded things.
        let klarna_req = utils::Encode::<klarna::KlarnaSessionRequest>::convert_and_encode(req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(klarna_payment_logs=?klarna_req);
        Ok(Some(klarna_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSessionType::get_headers(self, req)?)
                .header(headers::X_ROUTER, "test")
                .body(types::PaymentsSessionType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaSessionResponse = res
            .response
            .parse_struct("KlarnaPaymentsResponse")
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
        let response: klarna::KlarnaErrorResponse = res
            .parse_struct("KlarnaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            code: response.error_code,
            message: response.error_messages.join(" & "),
            reason: None,
        })
    }
}

impl api::PreVerify for Klarna {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Klarna
{
    // TODO: Critical Implement
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Klarna
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Klarna
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Klarna
{
    //Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Klarna
{
}

impl api::Refund for Klarna {}
impl api::RefundExecute for Klarna {}
impl api::RefundSync for Klarna {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Klarna
{
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Klarna
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Klarna {
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

impl services::ConnectorRedirectResponse for Klarna {}
