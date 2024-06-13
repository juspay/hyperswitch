use common_utils::{
    errors::CustomResult,
    request::{Method, Request, RequestContent},
};
use hyperswitch_domain_models::{router_data::ErrorResponse, router_data_new::RouterDataNew};
use masking::Maskable;
use serde_json::json;

use crate::{
    errors, events::connector_api_logs::ConnectorEvent, routes::metrics,
    services::CaptureSyncMethod, settings::Connectors, types,
};

pub type BoxedConnectorIntegrationNew<'a, Flow, ResourceCommonData, Req, Resp> =
    Box<&'a (dyn ConnectorIntegrationNew<Flow, ResourceCommonData, Req, Resp> + Send + Sync)>;

pub trait ConnectorIntegrationAnyNew<Flow, ResourceCommonData, Req, Resp>:
    Send + Sync + 'static
{
    fn get_connector_integration_new(
        &self,
    ) -> BoxedConnectorIntegrationNew<'_, Flow, ResourceCommonData, Req, Resp>;
}

impl<S, Flow, ResourceCommonData, Req, Resp>
    ConnectorIntegrationAnyNew<Flow, ResourceCommonData, Req, Resp> for S
where
    S: ConnectorIntegrationNew<Flow, ResourceCommonData, Req, Resp> + Send + Sync,
{
    fn get_connector_integration_new(
        &self,
    ) -> BoxedConnectorIntegrationNew<'_, Flow, ResourceCommonData, Req, Resp> {
        Box::new(self)
    }
}

pub trait ConnectorIntegrationNew<Flow, ResourceCommonData, Req, Resp>:
    ConnectorIntegrationAnyNew<Flow, ResourceCommonData, Req, Resp> + Sync + types::api::ConnectorCommon
{
    fn get_headers(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

    /// primarily used when creating signature based on request method of payment flow
    fn get_http_method(&self) -> Method {
        Method::Post
    }

    fn get_url(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(String::new())
    }

    fn get_request_body(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::Json(Box::new(json!(r#"{}"#))))
    }

    fn get_request_form_data(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
    ) -> CustomResult<Option<reqwest::multipart::Form>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request_new(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        metrics::UNIMPLEMENTED_FLOW.add(
            &metrics::CONTEXT,
            1,
            &[metrics::request::add_attributes("connector", self.id())],
        );
        Ok(None)
    }

    fn handle_response_new(
        &self,
        data: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        _res: types::Response,
    ) -> CustomResult<RouterDataNew<Flow, ResourceCommonData, Req, Resp>, errors::ConnectorError>
    where
        Flow: Clone,
        ResourceCommonData: Clone,
        Req: Clone,
        Resp: Clone,
    {
        event_builder.map(|e| e.set_error(json!({"error": "Not Implemented"})));
        Ok(data.clone())
    }

    fn get_error_response_new(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        event_builder.map(|event| event.set_error(json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
        Ok(ErrorResponse::get_not_implemented())
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        event_builder.map(|event| event.set_error(json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
        let error_message = match res.status_code {
            500 => "internal_server_error",
            501 => "not_implemented",
            502 => "bad_gateway",
            503 => "service_unavailable",
            504 => "gateway_timeout",
            505 => "http_version_not_supported",
            506 => "variant_also_negotiates",
            507 => "insufficient_storage",
            508 => "loop_detected",
            510 => "not_extended",
            511 => "network_authentication_required",
            _ => "unknown_error",
        };
        Ok(ErrorResponse {
            code: res.status_code.to_string(),
            message: error_message.to_string(),
            reason: String::from_utf8(res.response.to_vec()).ok(),
            status_code: res.status_code,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }

    // whenever capture sync is implemented at the connector side, this method should be overridden
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<CaptureSyncMethod, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("multiple capture sync".into()).into())
    }

    fn get_certificate(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn get_certificate_key(
        &self,
        _req: &RouterDataNew<Flow, ResourceCommonData, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }
}
