pub mod auth_service;

use std::fmt::Debug;

use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use masking::Maskable;

use crate::{
    core::errors::ConnectorError,
    types::{
        self as auth_types,
        api::auth_service::{AuthService, PaymentInitiation},
    },
};

#[async_trait::async_trait]
pub trait ConnectorIntegration<T, Req, Resp>: ConnectorIntegrationAny<T, Req, Resp> + Sync {
    fn get_headers(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

    fn get_url(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(String::new())
    }

    fn get_request_body(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
    ) -> CustomResult<RequestContent, ConnectorError> {
        Ok(RequestContent::Json(Box::new(serde_json::json!(r#"{}"#))))
    }

    fn build_request(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &super::PaymentAuthRouterData<T, Req, Resp>,
        _res: auth_types::Response,
    ) -> CustomResult<super::PaymentAuthRouterData<T, Req, Resp>, ConnectorError>
    where
        T: Clone,
        Req: Clone,
        Resp: Clone,
    {
        Ok(data.clone())
    }

    fn get_error_response(
        &self,
        _res: auth_types::Response,
    ) -> CustomResult<auth_types::ErrorResponse, ConnectorError> {
        Ok(auth_types::ErrorResponse::get_not_implemented())
    }

    fn get_5xx_error_response(
        &self,
        res: auth_types::Response,
    ) -> CustomResult<auth_types::ErrorResponse, ConnectorError> {
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
        Ok(auth_types::ErrorResponse {
            code: res.status_code.to_string(),
            message: error_message.to_string(),
            reason: String::from_utf8(res.response.to_vec()).ok(),
            status_code: res.status_code,
        })
    }
}

pub trait ConnectorCommonExt<Flow, Req, Resp>:
    ConnectorCommon + ConnectorIntegration<Flow, Req, Resp>
{
    fn build_headers(
        &self,
        _req: &auth_types::PaymentAuthRouterData<Flow, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(Vec::new())
    }
}

pub type BoxedConnectorIntegration<'a, T, Req, Resp> =
    Box<&'a (dyn ConnectorIntegration<T, Req, Resp> + Send + Sync)>;

pub trait ConnectorIntegrationAny<T, Req, Resp>: Send + Sync + 'static {
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp>;
}

impl<S, T, Req, Resp> ConnectorIntegrationAny<T, Req, Resp> for S
where
    S: ConnectorIntegration<T, Req, Resp>,
{
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp> {
        Box::new(self)
    }
}

pub trait AuthServiceConnector: AuthService + Send + Debug + PaymentInitiation {}

impl<T: Send + Debug + AuthService + PaymentInitiation> AuthServiceConnector for T {}

pub type BoxedPaymentAuthConnector = Box<&'static (dyn AuthServiceConnector + Sync)>;

#[derive(Clone, Debug)]
pub struct PaymentAuthConnectorData {
    pub connector: BoxedPaymentAuthConnector,
    pub connector_name: super::PaymentMethodAuthConnectors,
}

pub trait ConnectorCommon {
    fn id(&self) -> &'static str;

    fn get_auth_header(
        &self,
        _auth_type: &auth_types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(Vec::new())
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a auth_types::PaymentMethodAuthConnectors) -> &'a str;

    fn build_error_response(
        &self,
        res: auth_types::Response,
    ) -> CustomResult<auth_types::ErrorResponse, ConnectorError> {
        Ok(auth_types::ErrorResponse {
            status_code: res.status_code,
            code: crate::consts::NO_ERROR_CODE.to_string(),
            message: crate::consts::NO_ERROR_MESSAGE.to_string(),
            reason: None,
        })
    }
}
