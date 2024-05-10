use pm_auth::{
    consts,
    core::errors::ConnectorError,
    types::{self as pm_auth_types, api::BoxedConnectorIntegration, PaymentAuthRouterData},
};

use crate::{
    core::errors::{self},
    logger,
    routes::AppState,
    services::{self},
};

pub async fn execute_connector_processing_step<'b, 'a, T, Req, Resp>(
    state: &'b AppState,
    connector_integration: BoxedConnectorIntegration<'a, T, Req, Resp>,
    req: &'b PaymentAuthRouterData<T, Req, Resp>,
    connector: &pm_auth_types::PaymentMethodAuthConnectors,
) -> errors::CustomResult<PaymentAuthRouterData<T, Req, Resp>, ConnectorError>
where
    T: Clone + 'static,
    Req: Clone + 'static,
    Resp: Clone + 'static,
{
    let mut router_data = req.clone();

    let connector_request = connector_integration.build_request(req, connector)?;

    match connector_request {
        Some(request) => {
            logger::debug!(connector_request=?request);
            let response = services::api::call_connector_api(
                state,
                request,
                "execute_connector_processing_step",
            )
            .await;
            logger::debug!(connector_response=?response);
            match response {
                Ok(body) => {
                    let response = match body {
                        Ok(body) => {
                            let body = pm_auth_types::Response {
                                headers: body.headers,
                                response: body.response,
                                status_code: body.status_code,
                            };
                            let connector_http_status_code = Some(body.status_code);
                            let mut data =
                                connector_integration.handle_response(&router_data, body)?;
                            data.connector_http_status_code = connector_http_status_code;

                            data
                        }
                        Err(body) => {
                            let body = pm_auth_types::Response {
                                headers: body.headers,
                                response: body.response,
                                status_code: body.status_code,
                            };
                            router_data.connector_http_status_code = Some(body.status_code);

                            let error = match body.status_code {
                                500..=511 => connector_integration.get_5xx_error_response(body)?,
                                _ => connector_integration.get_error_response(body)?,
                            };

                            router_data.response = Err(error);

                            router_data
                        }
                    };
                    Ok(response)
                }
                Err(error) => {
                    if error.current_context().is_upstream_timeout() {
                        let error_response = pm_auth_types::ErrorResponse {
                            code: consts::REQUEST_TIMEOUT_ERROR_CODE.to_string(),
                            message: consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string(),
                            reason: Some(consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string()),
                            status_code: 504,
                        };
                        router_data.response = Err(error_response);
                        router_data.connector_http_status_code = Some(504);
                        Ok(router_data)
                    } else {
                        Err(error.change_context(ConnectorError::ProcessingStepFailed(None)))
                    }
                }
            }
        }
        None => Ok(router_data),
    }
}
