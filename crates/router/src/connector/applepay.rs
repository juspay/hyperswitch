mod transformers;

use std::fmt::Debug;

use common_utils::ext_traits::ValueExt;
use error_stack::{IntoReport, ResultExt};

use self::transformers as applepay;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers, services,
    types::{
        self,
        api::{self, ConnectorCommon},
    },
    utils::{self, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Applepay;

impl ConnectorCommon for Applepay {
    fn id(&self) -> &'static str {
        "applepay"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.applepay.base_url.as_ref()
    }
}

impl api::Payment for Applepay {}
impl api::PaymentAuthorize for Applepay {}
impl api::PaymentSync for Applepay {}
impl api::PaymentVoid for Applepay {}
impl api::PaymentCapture for Applepay {}
impl api::PreVerify for Applepay {}
impl api::PaymentSession for Applepay {}
impl api::ConnectorAccessToken for Applepay {}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Applepay
{
    // Not Implemented (R)
}

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
        _connectors: &settings::Connectors,
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
        connectors: &settings::Connectors,
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
        let connector_req = applepay::ApplepaySessionRequest::try_from(req)?;
        let req = utils::Encode::<applepay::ApplepaySessionRequest>::encode_to_string_of_json(
            &connector_req,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
            .headers(types::PaymentsSessionType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PaymentsSessionType::get_request_body(self, req)?)
            .add_certificate(types::PaymentsSessionType::get_certificate(self, req)?)
            .add_certificate_key(types::PaymentsSessionType::get_certificate_key(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: applepay::ApplepaySessionTokenResponse = res
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
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: applepay::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.status_code,
            message: response.status_message,
            reason: None,
        })
    }

    fn get_certificate(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let metadata = req
            .connector_meta_data
            .to_owned()
            .get_required_value("connector_meta_data")
            .change_context(errors::ConnectorError::NoConnectorMetaData)?;

        let metadata: transformers::ApplePayMetadata = metadata
            .parse_value("ApplePayMetaData")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(metadata.session_token_data.certificate))
    }

    fn get_certificate_key(
        &self,
        req: &types::PaymentsSessionRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let metadata = req
            .connector_meta_data
            .to_owned()
            .get_required_value("connector_meta_data")
            .change_context(errors::ConnectorError::NoConnectorMetaData)?;

        let metadata: transformers::ApplePayMetadata = metadata
            .parse_value("ApplePayMetaData")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Some(metadata.session_token_data.certificate_keys))
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Applepay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
