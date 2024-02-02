pub mod auth_service;

use std::fmt::Debug;

use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use masking::Maskable;

use crate::{
    core::errors::ConnectorError,
    types::{self as auth_types, api::auth_service::AuthService},
};

#[async_trait::async_trait]
pub trait ConnectorIntegration<T, Req, Resp>: ConnectorIntegrationAny<T, Req, Resp> + Sync {
        /// Retrieves the headers for the payment authentication router data and payment method authentication connectors.
    fn get_headers(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(vec![])
    }

        /// Retrieves the content type of the data as a reference to a static string.
    fn get_content_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

        /// Retrieves the URL for the payment authentication method.
    fn get_url(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(String::new())
    }

        /// Retrieves the request body content for the payment authorization router data. 
    /// 
    /// # Arguments
    /// 
    /// * `req` - A reference to the payment authorization router data.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the request content as `RequestContent` or a `ConnectorError` if an error occurs.
    /// 
    fn get_request_body(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
    ) -> CustomResult<RequestContent, ConnectorError> {
        Ok(RequestContent::Json(Box::new(serde_json::json!(r#"{}"#))))
    }

        /// Builds a request for payment authorization using the provided router data and payment method auth connectors.
    fn build_request(
        &self,
        _req: &super::PaymentAuthRouterData<T, Req, Resp>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(None)
    }

        /// Handles the response from the payment authentication router. 
    /// It takes the data, the response, and returns a custom result containing the cloned data or a connector error.
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

        /// Returns a custom result containing an error response or a connector error.
    fn get_error_response(
        &self,
        _res: auth_types::Response,
    ) -> CustomResult<auth_types::ErrorResponse, ConnectorError> {
        Ok(auth_types::ErrorResponse::get_not_implemented())
    }

        /// Returns a 5xx error response based on the given Response object. It matches the status code of the response and constructs an ErrorResponse object with the appropriate error message. If the status code is not recognized, it sets the error message to "unknown_error".
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
        /// Builds headers for payment authentication request.
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
        /// Returns a boxed connector integration for the current instance, allowing it to be used as a trait object. 
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp> {
        Box::new(self)
    }
}

pub trait AuthServiceConnector: AuthService + Send + Debug {}

impl<T: Send + Debug + AuthService> AuthServiceConnector for T {}

pub type BoxedPaymentAuthConnector = Box<&'static (dyn AuthServiceConnector + Sync)>;

#[derive(Clone, Debug)]
pub struct PaymentAuthConnectorData {
    pub connector: BoxedPaymentAuthConnector,
    pub connector_name: super::PaymentMethodAuthConnectors,
}

pub trait ConnectorCommon {
    fn id(&self) -> &'static str;

        /// Retrieves the authentication header for the specified authentication type.
    ///
    /// # Arguments
    ///
    /// * `auth_type` - The type of authentication to retrieve the header for.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of tuples, where each tuple consists of a `String` and a `Maskable<String>`. The `String` represents the key of the header, while the `Maskable<String>` represents the value of the header with masking capabilities.
    ///
    /// # Errors
    ///
    /// Returns a `ConnectorError` if an error occurs while retrieving the authentication header.
    ///
    fn get_auth_header(
        &self,
        _auth_type: &auth_types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(Vec::new())
    }

        /// Returns the common content type "application/json".
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a auth_types::PaymentMethodAuthConnectors) -> &'a str;

        /// Builds an error response based on the given `Response` object.
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
