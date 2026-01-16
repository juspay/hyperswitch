use common_utils::request::{Method as RequestMethod, Request, RequestContent};
use hyperswitch_interfaces::api_client::{call_connector_api, ApiClientWrapper};
use router_env::logger;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use crate::services::clients::payment_methods::{
    error::PaymentMethodClientError, ops, ops::ClientOperation,
};

pub struct ModularPaymentMethodClient<'a> {
    state: &'a dyn ApiClientWrapper,
    base_url: &'a Url,
}

impl<'a> ModularPaymentMethodClient<'a> {
    pub(crate) fn new(state: &'a dyn ApiClientWrapper, base_url: &'a Url) -> Self {
        Self { state, base_url }
    }

    pub(crate) async fn execute_request<T: DeserializeOwned>(
        &self,
        method: RequestMethod,
        path: &str,
        body: Option<Value>,
        operation: &str,
    ) -> Result<T, PaymentMethodClientError> {
        let url = self.base_url.join(path).map_err(|e| {
            logger::error!(operation, error=?e, "pm_client URL join failed");
            PaymentMethodClientError::TransportError {
                operation: operation.to_string(),
                message: format!("Failed to construct URL: {e}"),
            }
        })?;

        let mut request = Request::new(method, url.as_str());
        request.add_default_headers();

        if let Some(body) = body {
            request.set_body(RequestContent::Json(Box::new(body)));
        }

        let response = call_connector_api(self.state, request, operation)
            .await
            .map_err(|e| {
                logger::error!(operation, error=?e, "pm_client request failed");
                PaymentMethodClientError::TransportError {
                    operation: operation.to_string(),
                    message: format!("Connector API error: {e}"),
                }
            })?;

        match response {
            Ok(success) => serde_json::from_slice(&success.response).map_err(|e| {
                logger::error!(operation, error=?e, "pm_client response decode failed");
                PaymentMethodClientError::SerdeError {
                    operation: operation.to_string(),
                    message: format!("Failed to parse response: {e}"),
                }
            }),
            Err(err_resp) => {
                logger::warn!(
                    operation,
                    status = err_resp.status_code,
                    "pm_client upstream error"
                );
                let body = String::from_utf8_lossy(&err_resp.response);
                Err(PaymentMethodClientError::UpstreamError {
                    operation: operation.to_string(),
                    status: err_resp.status_code,
                    body: body.chars().take(500).collect(),
                })
            }
        }
    }

    pub async fn create_payment_method(
        &self,
        payload: Value,
    ) -> Result<ops::create::response::CreateV1Response, PaymentMethodClientError> {
        let op = ops::create::CreatePaymentMethod::new(payload);
        op.run(self).await
    }

    pub async fn retrieve_payment_method(
        &self,
        payment_method_id: String,
    ) -> Result<ops::retrieve::response::RetrieveV1Response, PaymentMethodClientError> {
        let op = ops::retrieve::RetrievePaymentMethod::new(payment_method_id);
        op.run(self).await
    }

    pub async fn update_payment_method(
        &self,
        payment_method_id: String,
        payload: Value,
    ) -> Result<ops::update::response::UpdateV1Response, PaymentMethodClientError> {
        let op = ops::update::UpdatePaymentMethod::new(payment_method_id, payload);
        op.run(self).await
    }

    pub async fn delete_payment_method(
        &self,
        payment_method_id: String,
    ) -> Result<ops::delete::response::DeleteV1Response, PaymentMethodClientError> {
        let op = ops::delete::DeletePaymentMethod::new(payment_method_id);
        op.run(self).await
    }
}
