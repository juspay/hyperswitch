mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::ResultExt;

use crate::{
    configs::settings::Connectors,
    utils::{self, BytesExt},
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, services,
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse, Response,
    }
};


use transformers as {{project-name | downcase}};

#[derive(Debug, Clone)]
pub struct {{project-name | downcase | pascal_case}};

impl api::ConnectorCommon for {{project-name | downcase | pascal_case}} {
    fn id(&self) -> &'static str {
        "{{project-name | downcase}}"
    }

    fn common_get_content_type(&self) -> &'static str {
        todo!()
        // Ex: "application/x-www-form-urlencoded"
    }

    fn base_url(&self, connectors: Connectors) -> String {
        connectors.{{project-name}}.base_url
    }

    fn get_auth_header(&self,_auth_type:&types::ConnectorAuthType)-> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        todo!()
    }
}

impl api::Payment for {{project-name | downcase | pascal_case}} {}

impl api::PreVerify for {{project-name | downcase | pascal_case}} {}
impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{
}

impl api::PaymentVoid for {{project-name | downcase | pascal_case}} {}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{}

impl api::PaymentSync for {{project-name | downcase | pascal_case}} {}
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for {{project-name | downcase | pascal_case}}
{}


impl api::PaymentCapture for {{project-name | downcase | pascal_case}} {}
impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{
    // Not Implemented (R)
}

impl api::PaymentSession for {{project-name | downcase | pascal_case}} {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}}
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for {{project-name | downcase | pascal_case}} {}

type Authorize = dyn services::ConnectorIntegration<
    api::Authorize,
    types::PaymentsAuthorizeData,
    types::PaymentsResponseData,
>;


impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, _req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::PaymentsAuthorizeRouterData, connectors: Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let {{project-name | downcase}}_req =
            utils::Encode::<{{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsRequest>::convert_and_url_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some({{project-name | downcase}}_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let response: {{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsResponse = res.response.parse_struct("PaymentIntentResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!({{project-name | downcase}}payments_create_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        todo!()
    }
}

impl api::Refund for {{project-name | downcase | pascal_case}} {}
impl api::RefundExecute for {{project-name | downcase | pascal_case}} {}
impl api::RefundSync for {{project-name | downcase | pascal_case}} {}

impl
    services::ConnectorIntegration<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, _req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::RefundsRouterData<api::Execute>, connectors: Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::RefundsRouterData<api::Execute>) -> CustomResult<Option<String>,errors::ConnectorError> {
        let {{project-name | downcase}}_req = utils::Encode::<{{project-name| downcase}}::{{project-name | downcase | pascal_case}}RefundRequest>::convert_and_url_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some({{project-name | downcase}}_req))
    }

    fn build_request(&self, req: &types::RefundsRouterData<api::Execute>, connectors: Connectors,) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
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
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>,errors::ConnectorError> {
        logger::debug!(target: "router::connector::{{project-name | downcase}}", response=?res);
        let response: {{project-name| downcase}}::RefundResponse = res.response.parse_struct("{{project-name | downcase}} RefundResponse").change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        todo!()
    }
}

type RSync = dyn services::ConnectorIntegration<
    api::RSync,
    types::RefundsData,
    types::RefundsResponseData,
>;
impl
    services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,_connectors: Connectors,) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        res: Response,
    ) -> CustomResult<types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,errors::ConnectorError,> {
        logger::debug!(target: "router::connector::{{project-name | downcase}}", response=?res);
        let response: {{project-name | downcase}}::RefundResponse = res.response.parse_struct("{{project-name | downcase}} RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(&self, _res: Bytes) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for {{project-name | downcase | pascal_case}} {
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

impl services::ConnectorRedirectResponse for {{project-name | downcase | pascal_case}} {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
