mod transformers;

use std::fmt::Debug;
use common_utils::errors::ReportSwitchExt;
use error_stack::{ResultExt, IntoReport};

use crate::{
    configs::settings,
    utils::{self, BytesExt},
    core::{
        errors::{self, CustomResult},
    },
    headers, services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    }
};


use transformers as {{project-name | downcase}};

#[derive(Debug, Clone)]
pub struct {{project-name | downcase | pascal_case}};

impl api::Payment for {{project-name | downcase | pascal_case}} {}
impl api::PaymentSession for {{project-name | downcase | pascal_case}} {}
impl api::ConnectorAccessToken for {{project-name | downcase | pascal_case}} {}
impl api::PreVerify for {{project-name | downcase | pascal_case}} {}
impl api::PaymentAuthorize for {{project-name | downcase | pascal_case}} {}
impl api::PaymentSync for {{project-name | downcase | pascal_case}} {}
impl api::PaymentCapture for {{project-name | downcase | pascal_case}} {}
impl api::PaymentVoid for {{project-name | downcase | pascal_case}} {}
impl api::Refund for {{project-name | downcase | pascal_case}} {}
impl api::RefundExecute for {{project-name | downcase | pascal_case}} {}
impl api::RefundSync for {{project-name | downcase | pascal_case}} {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for {{project-name | downcase | pascal_case}} 
where
    Self: ConnectorIntegration<Flow, Request, Response>,{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self).to_string(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for {{project-name | downcase | pascal_case}} {
    fn id(&self) -> &'static str {
        "{{project-name | downcase}}"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.{{project-name}}.base_url.as_ref()
    }

    fn get_auth_header(&self, auth_type:&types::ConnectorAuthType)-> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        let auth =  {{project-name | downcase}}::{{project-name | downcase | pascal_case}}AuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: {{project-name | downcase}}::{{project-name | downcase | pascal_case}}ErrorResponse = res
            .response
            .parse_struct("{{project-name | downcase | pascal_case}}ErrorResponse")
            .switch()?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
        })
    }
}

impl
    ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for {{project-name | downcase | pascal_case}}
{
}

impl
    ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{
}

impl
    ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, req: &types::PaymentsAuthorizeRouterData, connectors: &settings::Connectors,) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &types::PaymentsAuthorizeRouterData, _connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }
    
    fn get_request_body(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let req_obj = {{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsRequest::try_from(req)?;
        let {{project-name | downcase}}_req =
            utils::Encode::<{{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsRequest>::encode_to_string_of_json(
                &req_obj,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some({{project-name | downcase}}_req))
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
    ) -> CustomResult<types::PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let response: {{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsResponse = res.response.parse_struct("{{project-name | downcase | pascal_case}} PaymentsAuthorizeResponse").switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for {{project-name | downcase | pascal_case}}
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: {{project-name | downcase}}:: {{project-name | downcase | pascal_case}}PaymentsResponse = res
            .response
            .parse_struct("{{project-name | downcase}} PaymentsSyncResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
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
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
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
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: {{project-name | downcase }}::{{project-name | downcase | pascal_case}}PaymentsResponse = res
            .response
            .parse_struct("{{project-name | downcase | pascal_case}} PaymentsCaptureResponse")
            .switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
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
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{}

impl
    ConnectorIntegration<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &types::RefundsRouterData<api::Execute>, _connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Option<String>,errors::ConnectorError> {
        let req_obj = {{project-name | downcase}}::{{project-name | downcase | pascal_case}}RefundRequest::try_from(req)?;
        let {{project-name | downcase}}_req =
            utils::Encode::<{{project-name | downcase}}::{{project-name | downcase | pascal_case}}RefundRequest>::encode_to_string_of_json(
                &req_obj,
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some({{project-name | downcase}}_req))
    }

    fn build_request(&self, req: &types::RefundsRouterData<api::Execute>, connectors: &settings::Connectors,) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .headers(types::RefundExecuteType::get_headers(self, req, connectors)?)
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>,errors::ConnectorError> {
        let response: {{project-name| downcase}}::RefundResponse = res.response.parse_struct("{{project-name | downcase}} RefundResponse").switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, req: &types::RefundSyncRouterData,connectors: &settings::Connectors,) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &types::RefundSyncRouterData,_connectors: &settings::Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData,errors::ConnectorError,> {
        let response: {{project-name | downcase}}::RefundResponse = res.response.parse_struct("{{project-name | downcase}} RefundSyncResponse").switch()?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for {{project-name | downcase | pascal_case}} {
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
