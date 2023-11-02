pub mod transformers;

use std::fmt::Debug;

use api_models::enums::AuthenticationType;
use common_utils::ext_traits::ValueExt;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use transformers as powertranz;

use super::utils::{
    self as connector_utils, PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
};
use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Powertranz;

impl api::Payment for Powertranz {}
impl api::PaymentSession for Powertranz {}
impl api::ConnectorAccessToken for Powertranz {}
impl api::MandateSetup for Powertranz {}
impl api::PaymentAuthorize for Powertranz {}
impl api::PaymentsCompleteAuthorize for Powertranz {}
impl api::PaymentSync for Powertranz {}
impl api::PaymentCapture for Powertranz {}
impl api::PaymentVoid for Powertranz {}
impl api::Refund for Powertranz {}
impl api::RefundExecute for Powertranz {}
impl api::RefundSync for Powertranz {}
impl api::PaymentToken for Powertranz {}

const POWER_TRANZ_ID: &str = "PowerTranz-PowerTranzId";
const POWER_TRANZ_PASSWORD: &str = "PowerTranz-PowerTranzPassword";

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Powertranz
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Powertranz
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut auth_header = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut auth_header);
        Ok(header)
    }
}

impl ConnectorCommon for Powertranz {
    fn id(&self) -> &'static str {
        "powertranz"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.powertranz.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = powertranz::PowertranzAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                POWER_TRANZ_ID.to_string(),
                auth.power_tranz_id.expose().into_masked(),
            ),
            (
                POWER_TRANZ_PASSWORD.to_string(),
                auth.power_tranz_password.expose().into_masked(),
            ),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        // For error scenerios connector respond with 200 http status code and error response object in response
        // For http status code other than 200 they send empty response back
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: consts::NO_ERROR_CODE.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: None,
        })
    }
}

impl ConnectorValidation for Powertranz {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Powertranz
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Powertranz
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Powertranz
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Powertranz
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let mut endpoint = match req.request.is_auto_capture()? {
            true => "sale",
            false => "auth",
        }
        .to_string();
        // 3ds payments uses different endpoints
        if req.auth_type == AuthenticationType::ThreeDs {
            endpoint.insert_str(0, "spi/")
        };
        Ok(format!("{}{endpoint}", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = powertranz::PowertranzPaymentsRequest::try_from(req)?;
        let powertranz_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<powertranz::PowertranzPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(powertranz_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: powertranz::PowertranzBaseResponse = res
            .response
            .parse_struct("Powertranz PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Powertranz
{
    fn get_headers(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json-patch+json"
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}spi/payment", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let redirect_payload: powertranz::RedirectResponsePayload = req
            .request
            .get_redirect_response_payload()?
            .parse_value("PowerTranz RedirectResponsePayload")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let spi_token = format!(r#""{}""#, redirect_payload.spi_token);
        let powertranz_req =
            types::RequestBody::log_and_get_request_body(&spi_token, |spi_token| {
                Ok(spi_token.to_string())
            })
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(powertranz_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCompleteAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsCompleteAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCompleteAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: powertranz::PowertranzBaseResponse = res
            .response
            .parse_struct("Powertranz PaymentsCompleteAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Powertranz
{
    // default implementation of build_request method will be executed
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Powertranz
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}capture", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = powertranz::PowertranzBaseRequest::try_from(&req.request)?;
        let powertranz_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<powertranz::PowertranzBaseRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(powertranz_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: powertranz::PowertranzBaseResponse = res
            .response
            .parse_struct("Powertranz PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Powertranz
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}void", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = powertranz::PowertranzBaseRequest::try_from(&req.request)?;
        let powertranz_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<powertranz::PowertranzBaseRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(powertranz_req))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: powertranz::PowertranzBaseResponse = res
            .response
            .parse_struct("powertranz CancelResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Powertranz
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}refund", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = powertranz::PowertranzBaseRequest::try_from(req)?;
        let powertranz_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<powertranz::PowertranzBaseRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(powertranz_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: powertranz::PowertranzBaseResponse = res
            .response
            .parse_struct("powertranz RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Powertranz
{
    // default implementation of build_request method will be executed
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Powertranz {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
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
