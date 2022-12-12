mod transformers;

use std::fmt::Debug;

use base64;
use bytes::Bytes;
use error_stack::ResultExt;
use ring::{digest, hmac};
use transformers as cybersource;

use crate::{
    configs::settings::Connectors,
    consts,
    core::errors::{self, CustomResult},
    headers, logger, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Cybersource;

impl Cybersource {
    pub fn generate_digest(payload: &[u8]) -> String {
        let digest = digest::digest(&digest::SHA256, payload);
        base64::encode(digest)
    }
}

impl api::ConnectorCommon for Cybersource {
    fn id(&self) -> &'static str {
        "cybersource"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.cybersource.base_url
    }
}

impl api::Payment for Cybersource {}
impl api::PaymentAuthorize for Cybersource {}
impl api::PaymentSync for Cybersource {}
impl api::PaymentVoid for Cybersource {}
impl api::PaymentCapture for Cybersource {}
impl api::PreVerify for Cybersource {}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Cybersource
{
    // TODO: Critical implement
}

impl api::PaymentSession for Cybersource {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Cybersource
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Cybersource
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Cybersource
{
    // fn get_headers(
    //     &self,
    //     _req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    // ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
    // }

    // fn get_request_body(
    //     &self,
    //     _req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    // ) -> CustomResult<Option<String>, errors::ConnectorError> {
    // }

    // fn get_url(
    //     &self,
    //     _req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    //     _connectors: Connectors,
    // ) -> CustomResult<String, errors::ConnectorError> {
    // }

    // fn build_request(
    //     &self,
    //     _req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    //     _connectors: Connectors,
    // ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
    // }

    // fn handle_response(
    //     &self,
    //     _data: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    //     _res: Response,
    // ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
    // }

    // fn get_error_response(
    //     &self,
    //     _res: Bytes,
    // ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    // }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let cybersource_req =
            utils::Encode::<cybersource::CybersourcePaymentsRequest>::convert_and_url_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        logger::debug!(cybersource_req);

        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let cybersource_req =
            utils::Encode::<cybersource::CybersourcePaymentsRequest>::convert_and_url_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(cybersource_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(cybersourcepayments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        todo!()
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Cybersource
{
}

impl api::Refund for Cybersource {}
impl api::RefundExecute for Cybersource {}
impl api::RefundSync for Cybersource {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let cybersource_req =
            utils::Encode::<cybersource::CybersourceRefundRequest>::convert_and_url_encode(req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(cybersource_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(self, req)?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }
    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        _res: Response,
    ) -> CustomResult<
        types::RouterData<api::Execute, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    >
    where
        api::Execute: Clone,
        types::RefundsData: Clone,
        types::RefundsResponseData: Clone,
    {
        Ok(data.clone())
    }

    fn get_error_response(
        &self,
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        todo!()
    }
}

#[allow(dead_code)]
impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Cybersource {
    fn get_webhook_object_reference_id(
        &self,
        _body: &[u8],
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_webhook_event_type(
        &self,
        _body: &[u8],
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        todo!()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        todo!()
    }
}

impl services::ConnectorRedirectResponse for Cybersource {}
