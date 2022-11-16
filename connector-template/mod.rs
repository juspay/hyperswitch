mod transformers;

use std::fmt::Debug;

use bytes::Bytes;
use error_stack::ResultExt;

use crate::{
    configs::settings::ConnectorParams,
    utils::{self, BytesExt},
    core::errors::{self, CustomResult},
    logger, services,
    types::{
        self,
        api,
        ErrorResponse, Response,
    }
};


use transformers as {{project-name | downcase}};

#[derive(Debug, Clone)]
pub struct {{project-name | downcase | pascal_case}} {
    pub base_url: String,
}

impl {{project-name | downcase | pascal_case}} {
    pub fn make(params: &ConnectorParams) -> Self {
        Self {
            base_url: params.base_url.to_owned(),
        }
    }
}

impl api::ConnectorCommon for {{project-name | downcase | pascal_case}} {
    fn id(&self) -> &'static str {
        "{{project-name | downcase}}"
    }

    fn common_get_content_type(&self) -> &'static str {
        todo!()
        // Ex: "application/x-www-form-urlencoded"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn get_auth_header(&self,_auth_type:&types::ConnectorAuthType)-> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        todo!()
    }
}

impl api::Payment for {{project-name | downcase | pascal_case}} {}

impl api::PaymentAuthorize for {{project-name | downcase | pascal_case}} {}

type Authorize = dyn services::ConnectorIntegration<
    api::Authorize,
    types::PaymentsRequestData,
    types::PaymentsResponseData,
>;


impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsRequestData,
        types::PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, _req: &types::PaymentsRouterData) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::PaymentsRouterData) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::PaymentsRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let {{project-name | downcase}}_req =
            utils::Encode::<{{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsRequest>::convert_and_url_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some({{project-name | downcase}}_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsRouterData,errors::ConnectorError> {
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

type Execute = dyn services::ConnectorIntegration<
    api::Execute,
    types::RefundsRequestData,
    types::RefundsResponseData,
>;

impl
    services::ConnectorIntegration<
        api::Execute,
        types::RefundsRequestData,
        types::RefundsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, _req: &types::RefundsRouterData) -> CustomResult<Vec<(String,String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::RefundsRouterData) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    }

    fn get_request_body(&self, req: &types::RefundsRouterData) -> CustomResult<Option<String>,errors::ConnectorError> {
        let {{project-name | downcase}}_req = utils::Encode::<{{project-name| downcase}}::{{project-name | downcase | pascal_case}}RefundRequest>::convert_and_url_encode(req).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some({{project-name | downcase}}_req))
    }

    fn build_request(&self, req: &types::RefundsRouterData) -> CustomResult<Option<services::Request>,errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&Execute::get_url(self, req)?)
            .headers(Execute::get_headers(self, req)?)
            .body(Execute::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self, 
        data: &types::RefundsRouterData,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData,errors::ConnectorError> {
        logger::debug!(target: "router::connector::{{project-name | downcase}}", response=?res);
        let response: {{project-name| downcase}}::{{project-name | downcase| pascal_case}}RefundResponse = res.response.parse_struct("{{project-name | downcase}} RefundResponse").change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
    api::Sync,
    types::RefundsRequestData,
    types::RefundsResponseData,
>;
impl
    services::ConnectorIntegration<api::Sync, types::RefundsRequestData, types::RefundsResponseData> for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, _req: &types::RefundsRouterData) -> CustomResult<Vec<(String, String)>,errors::ConnectorError> {
        todo!()
    }

    fn get_content_type(&self) -> &'static str {
        todo!()
    }

    fn get_url(&self, _req: &types::RefundsRouterData) -> CustomResult<String,errors::ConnectorError> {
        todo!()
    } 

    fn handle_response(
        &self,
        data: &types::RefundsRouterData,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData,errors::ConnectorError> {
        logger::debug!(target: "router::connector::{{project-name | downcase}}", response=?res);
        let response: {{project-name | downcase}}::{{project-name | downcase | pascal_case}}RefundResponse = res.response.parse_struct("{{project-name | downcase}} RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?; 
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
    ) -> CustomResult<String, errors::ConnectorError> {
        todo!()
    }

    fn get_webhook_resource_object(
        &self,
        _body: &[u8],
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        todo!()
    }
}
