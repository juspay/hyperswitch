use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use common_enums::ApiClientError;
use common_utils::{
    consts::{X_CONNECTOR_NAME, X_FLOW_NAME, X_REQUEST_ID},
    errors::CustomResult,
    request::{Request, RequestContent},
};
use error_stack::{report, ResultExt};
use http::Method;
use hyperswitch_domain_models::{
    errors::api_error_response,
    router_data::{ErrorResponse, RouterData},
};
use masking::Maskable;
use reqwest::multipart::Form;
use router_env::{instrument, logger, tracing, RequestId};
use serde_json::json;

use crate::{
    configs,
    connector_integration_interface::{
        BoxedConnectorIntegrationInterface, ConnectorEnum, RouterDataConversion,
    },
    consts,
    errors::ConnectorError,
    events,
    events::connector_api_logs::ConnectorEvent,
    metrics, types,
    types::Proxy,
};

/// A trait representing a converter for connector names to their corresponding enum variants.
pub trait ConnectorConverter: Send + Sync {
    /// Get the connector enum variant by its name
    fn get_connector_enum_by_name(
        &self,
        connector: &str,
    ) -> CustomResult<ConnectorEnum, api_error_response::ApiErrorResponse>;
}
/// A trait representing a builder for HTTP requests.
pub trait RequestBuilder: Send + Sync {
    /// Build a JSON request
    fn json(&mut self, body: serde_json::Value);
    /// Build a URL encoded form request
    fn url_encoded_form(&mut self, body: serde_json::Value);
    /// Set the timeout duration for the request
    fn timeout(&mut self, timeout: Duration);
    /// Build a multipart request
    fn multipart(&mut self, form: Form);
    /// Add a header to the request
    fn header(&mut self, key: String, value: Maskable<String>) -> CustomResult<(), ApiClientError>;
    /// Send the request and return a future that resolves to the response
    fn send(
        self,
    ) -> CustomResult<
        Box<dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static>,
        ApiClientError,
    >;
}

/// A trait representing an API client capable of making HTTP requests.
#[async_trait::async_trait]
pub trait ApiClient: dyn_clone::DynClone
where
    Self: Send + Sync,
{
    /// Create a new request with the specified HTTP method and URL
    fn request(
        &self,
        method: Method,
        url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    /// Create a new request with the specified HTTP method, URL, and client certificate
    fn request_with_certificate(
        &self,
        method: Method,
        url: String,
        certificate: Option<masking::Secret<String>>,
        certificate_key: Option<masking::Secret<String>>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    /// Send a request and return the response
    async fn send_request(
        &self,
        state: &dyn ApiClientWrapper,
        request: Request,
        option_timeout_secs: Option<u64>,
        forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError>;

    /// Add a request ID to the client for tracking purposes
    fn add_request_id(&mut self, request_id: RequestId);

    /// Get the current request ID, if any
    fn get_request_id(&self) -> Option<RequestId>;

    /// Get the current request ID as a string, if any
    fn get_request_id_str(&self) -> Option<String>;

    /// Add a flow name to the client for tracking purposes
    fn add_flow_name(&mut self, flow_name: String);
}

dyn_clone::clone_trait_object!(ApiClient);

/// A wrapper trait to get the ApiClient and Proxy from the state
pub trait ApiClientWrapper: Send + Sync {
    /// Get the ApiClient instance
    fn get_api_client(&self) -> &dyn ApiClient;
    /// Get the Proxy configuration
    fn get_proxy(&self) -> Proxy;
    /// Get the request ID as String if any
    fn get_request_id_str(&self) -> Option<String>;
    /// Get the request ID as &RequestId if any
    fn get_request_id(&self) -> Option<RequestId>;
    /// Get the tenant information
    fn get_tenant(&self) -> configs::Tenant;
    /// Get connectors configuration
    fn get_connectors(&self) -> configs::Connectors;
    /// Get the event handler
    fn event_handler(&self) -> &dyn events::EventHandlerInterface;
}

/// Handle the flow by interacting with connector module
/// `connector_request` is applicable only in case if the `CallConnectorAction` is `Trigger`
/// In other cases, It will be created if required, even if it is not passed
#[instrument(skip_all, fields(connector_name, payment_method))]
pub async fn execute_connector_processing_step<
    'b,
    'a,
    T,
    ResourceCommonData: Clone + RouterDataConversion<T, Req, Resp> + 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
>(
    state: &dyn ApiClientWrapper,
    connector_integration: BoxedConnectorIntegrationInterface<T, ResourceCommonData, Req, Resp>,
    req: &'b RouterData<T, Req, Resp>,
    call_connector_action: common_enums::CallConnectorAction,
    connector_request: Option<Request>,
    return_raw_connector_response: Option<bool>,
) -> CustomResult<RouterData<T, Req, Resp>, ConnectorError>
where
    T: Clone + Debug + 'static,
    // BoxedConnectorIntegration<T, Req, Resp>: 'b,
{
    // If needed add an error stack as follows
    // connector_integration.build_request(req).attach_printable("Failed to build request");
    tracing::Span::current().record("connector_name", &req.connector);
    tracing::Span::current().record("payment_method", req.payment_method.to_string());
    logger::debug!(connector_request=?connector_request);
    let mut router_data = req.clone();
    match call_connector_action {
        common_enums::CallConnectorAction::HandleResponse(res) => {
            let response = types::Response {
                headers: None,
                response: res.into(),
                status_code: 200,
            };
            connector_integration.handle_response(req, None, response)
        }
        common_enums::CallConnectorAction::UCSConsumeResponse(_)
        | common_enums::CallConnectorAction::UCSHandleResponse(_) => {
            Err(ConnectorError::ProcessingStepFailed(Some(
                "CallConnectorAction UCSHandleResponse/UCSConsumeResponse used in Direct gateway system flow. These actions are only valid in UCS gateway system"
                    .to_string()
                    .into(),
            ))
            .into())
        }
        common_enums::CallConnectorAction::Avoid => Ok(router_data),
        common_enums::CallConnectorAction::StatusUpdate {
            status,
            error_code,
            error_message,
        } => {
            router_data.status = status;
            let error_response = if error_code.is_some() | error_message.is_some() {
                Some(ErrorResponse {
                    code: error_code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: error_message.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    status_code: 200, // This status code is ignored in redirection response it will override with 302 status code.
                    reason: None,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            } else {
                None
            };
            router_data.response = error_response.map(Err).unwrap_or(router_data.response);
            Ok(router_data)
        }
        common_enums::CallConnectorAction::Trigger => {
            metrics::CONNECTOR_CALL_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("connector", req.connector.to_string()),
                    (
                        "flow",
                        get_flow_name::<T>().unwrap_or_else(|_| "UnknownFlow".to_string())
                    ),
                ),
            );

            let connector_request = match connector_request {
                Some(connector_request) => Some(connector_request),
                None => connector_integration
                    .build_request(req, &state.get_connectors())
                    .inspect_err(|error| {
                        if matches!(
                            error.current_context(),
                            &ConnectorError::RequestEncodingFailed
                                | &ConnectorError::RequestEncodingFailedWithReason(_)
                        ) {
                            metrics::REQUEST_BUILD_FAILURE.add(
                                1,
                                router_env::metric_attributes!((
                                    "connector",
                                    req.connector.clone()
                                )),
                            )
                        }
                    })?,
            };

            match connector_request {
                Some(mut request) => {
                    let masked_request_body = match &request.body {
                        Some(request) => match request {
                            RequestContent::Json(i)
                            | RequestContent::FormUrlEncoded(i)
                            | RequestContent::Xml(i) => i
                                .masked_serialize()
                                .unwrap_or(json!({ "error": "failed to mask serialize"})),
                            RequestContent::FormData((_, i)) => i
                                .masked_serialize()
                                .unwrap_or(json!({ "error": "failed to mask serialize"})),
                            RequestContent::RawBytes(_) => json!({"request_type": "RAW_BYTES"}),
                        },
                        None => serde_json::Value::Null,
                    };
                    let flow_name =
                        get_flow_name::<T>().unwrap_or_else(|_| "UnknownFlow".to_string());
                    request.headers.insert((
                        X_FLOW_NAME.to_string(),
                        Maskable::Masked(masking::Secret::new(flow_name.to_string())),
                    ));
                    let connector_name = req.connector.clone();
                    request.headers.insert((
                        X_CONNECTOR_NAME.to_string(),
                        Maskable::Masked(masking::Secret::new(connector_name.clone().to_string())),
                    ));
                    state.get_request_id().as_ref().map(|id| {
                        let request_id = id.to_string();
                        request.headers.insert((
                            X_REQUEST_ID.to_string(),
                            Maskable::Normal(request_id.clone()),
                        ));
                        request_id
                    });
                    let request_url = request.url.clone();
                    let request_method = request.method;
                    let current_time = Instant::now();
                    let response =
                        call_connector_api(state, request, "execute_connector_processing_step")
                            .await;
                    let external_latency = current_time.elapsed().as_millis();
                    logger::info!(raw_connector_request=?masked_request_body);
                    let status_code = response
                        .as_ref()
                        .map(|i| {
                            i.as_ref()
                                .map_or_else(|value| value.status_code, |value| value.status_code)
                        })
                        .unwrap_or_default();
                    let mut connector_event = ConnectorEvent::new(
                        state.get_tenant().tenant_id.clone(),
                        req.connector.clone(),
                        std::any::type_name::<T>(),
                        masked_request_body,
                        request_url,
                        request_method,
                        req.payment_id.clone(),
                        req.merchant_id.clone(),
                        state.get_request_id().as_ref(),
                        external_latency,
                        req.refund_id.clone(),
                        req.dispute_id.clone(),
                        status_code,
                    );

                    match response {
                        Ok(body) => {
                            let response = match body {
                                Ok(body) => {
                                    let connector_http_status_code = Some(body.status_code);
                                    let handle_response_result = connector_integration
                                        .handle_response(
                                            req,
                                            Some(&mut connector_event),
                                            body.clone(),
                                        )
                                        .inspect_err(|error| {
                                            if error.current_context()
                                                == &ConnectorError::ResponseDeserializationFailed
                                            {
                                                metrics::RESPONSE_DESERIALIZATION_FAILURE.add(
                                                    1,
                                                    router_env::metric_attributes!((
                                                        "connector",
                                                        req.connector.clone(),
                                                    )),
                                                )
                                            }
                                        });
                                    match handle_response_result {
                                        Ok(mut data) => {
                                            state
                                                .event_handler()
                                                .log_connector_event(&connector_event);
                                            data.connector_http_status_code =
                                                connector_http_status_code;
                                            // Add up multiple external latencies in case of multiple external calls within the same request.
                                            data.external_latency = Some(
                                                data.external_latency
                                                    .map_or(external_latency, |val| {
                                                        val + external_latency
                                                    }),
                                            );

                                            store_raw_connector_response_if_required(
                                                return_raw_connector_response,
                                                &mut data,
                                                &body,
                                            )?;

                                            Ok(data)
                                        }
                                        Err(err) => {
                                            connector_event
                                                .set_error(json!({"error": err.to_string()}));

                                            state
                                                .event_handler()
                                                .log_connector_event(&connector_event);
                                            Err(err)
                                        }
                                    }?
                                }
                                Err(body) => {
                                    router_data.connector_http_status_code = Some(body.status_code);
                                    router_data.external_latency = Some(
                                        router_data
                                            .external_latency
                                            .map_or(external_latency, |val| val + external_latency),
                                    );
                                    metrics::CONNECTOR_ERROR_RESPONSE_COUNT.add(
                                        1,
                                        router_env::metric_attributes!((
                                            "connector",
                                            req.connector.clone(),
                                        )),
                                    );

                                    store_raw_connector_response_if_required(
                                        return_raw_connector_response,
                                        &mut router_data,
                                        &body,
                                    )?;

                                    let error = match body.status_code {
                                        500..=511 => {
                                            let error_res = connector_integration
                                                .get_5xx_error_response(
                                                    body,
                                                    Some(&mut connector_event),
                                                )?;
                                            state
                                                .event_handler()
                                                .log_connector_event(&connector_event);
                                            error_res
                                        }
                                        _ => {
                                            let error_res = connector_integration
                                                .get_error_response(
                                                    body,
                                                    Some(&mut connector_event),
                                                )?;
                                            if let Some(status) = error_res.attempt_status {
                                                router_data.status = status;
                                            };
                                            state
                                                .event_handler()
                                                .log_connector_event(&connector_event);
                                            error_res
                                        }
                                    };

                                    router_data.response = Err(error);

                                    router_data
                                }
                            };
                            Ok(response)
                        }
                        Err(error) => {
                            connector_event.set_error(json!({"error": error.to_string()}));
                            state.event_handler().log_connector_event(&connector_event);
                            if error.current_context().is_upstream_timeout() {
                                let error_response = ErrorResponse {
                                    code: consts::REQUEST_TIMEOUT_ERROR_CODE.to_string(),
                                    message: consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string(),
                                    reason: Some(consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string()),
                                    status_code: 504,
                                    attempt_status: None,
                                    connector_transaction_id: None,
                                    network_advice_code: None,
                                    network_decline_code: None,
                                    network_error_message: None,
                                    connector_metadata: None,
                                };
                                router_data.response = Err(error_response);
                                router_data.connector_http_status_code = Some(504);
                                router_data.external_latency = Some(
                                    router_data
                                        .external_latency
                                        .map_or(external_latency, |val| val + external_latency),
                                );
                                Ok(router_data)
                            } else {
                                Err(error
                                    .change_context(ConnectorError::ProcessingStepFailed(None)))
                            }
                        }
                    }
                }
                None => Ok(router_data),
            }
        }
    }
}

/// Calls the connector API and handles the response
#[instrument(skip_all)]
pub async fn call_connector_api(
    state: &dyn ApiClientWrapper,
    request: Request,
    flow_name: &str,
) -> CustomResult<Result<types::Response, types::Response>, ApiClientError> {
    let current_time = Instant::now();
    let headers = request.headers.clone();
    let url = request.url.clone();
    let response = state
        .get_api_client()
        .send_request(state, request, None, true)
        .await;

    match response.as_ref() {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let elapsed_time = current_time.elapsed();
            logger::info!(
                ?headers,
                url,
                status_code,
                flow=?flow_name,
                ?elapsed_time
            );
        }
        Err(err) => {
            logger::info!(
                call_connector_api_error=?err
            );
        }
    }

    handle_response(response).await
}

/// Handle the response from the API call
#[instrument(skip_all)]
pub async fn handle_response(
    response: CustomResult<reqwest::Response, ApiClientError>,
) -> CustomResult<Result<types::Response, types::Response>, ApiClientError> {
    response
        .map(|response| async {
            logger::info!(?response);
            let status_code = response.status().as_u16();
            let headers = Some(response.headers().to_owned());
            match status_code {
                200..=202 | 302 | 204 => {
                    // If needed add log line
                    // logger:: error!( error_parsing_response=?err);
                    let response = response
                        .bytes()
                        .await
                        .change_context(ApiClientError::ResponseDecodingFailed)
                        .attach_printable("Error while waiting for response")?;
                    Ok(Ok(types::Response {
                        headers,
                        response,
                        status_code,
                    }))
                }

                status_code @ 500..=599 => {
                    let bytes = response.bytes().await.map_err(|error| {
                        report!(error)
                            .change_context(ApiClientError::ResponseDecodingFailed)
                            .attach_printable("Client error response received")
                    })?;
                    // let error = match status_code {
                    //     500 => ApiClientError::InternalServerErrorReceived,
                    //     502 => ApiClientError::BadGatewayReceived,
                    //     503 => ApiClientError::ServiceUnavailableReceived,
                    //     504 => ApiClientError::GatewayTimeoutReceived,
                    //     _ => ApiClientError::UnexpectedServerResponse,
                    // };
                    Ok(Err(types::Response {
                        headers,
                        response: bytes,
                        status_code,
                    }))
                }

                status_code @ 400..=499 => {
                    let bytes = response.bytes().await.map_err(|error| {
                        report!(error)
                            .change_context(ApiClientError::ResponseDecodingFailed)
                            .attach_printable("Client error response received")
                    })?;
                    /* let error = match status_code {
                        400 => ApiClientError::BadRequestReceived(bytes),
                        401 => ApiClientError::UnauthorizedReceived(bytes),
                        403 => ApiClientError::ForbiddenReceived,
                        404 => ApiClientError::NotFoundReceived(bytes),
                        405 => ApiClientError::MethodNotAllowedReceived,
                        408 => ApiClientError::RequestTimeoutReceived,
                        422 => ApiClientError::UnprocessableEntityReceived(bytes),
                        429 => ApiClientError::TooManyRequestsReceived,
                        _ => ApiClientError::UnexpectedServerResponse,
                    };
                    Err(report!(error).attach_printable("Client error response received"))
                        */
                    Ok(Err(types::Response {
                        headers,
                        response: bytes,
                        status_code,
                    }))
                }

                _ => Err(report!(ApiClientError::UnexpectedServerResponse)
                    .attach_printable("Unexpected response from server")),
            }
        })?
        .await
}

/// Store the raw connector response in the router data if required
pub fn store_raw_connector_response_if_required<T, Req, Resp>(
    return_raw_connector_response: Option<bool>,
    router_data: &mut RouterData<T, Req, Resp>,
    body: &types::Response,
) -> CustomResult<(), ConnectorError>
where
    T: Clone + Debug + 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
{
    if return_raw_connector_response == Some(true) {
        let mut decoded = String::from_utf8(body.response.as_ref().to_vec())
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        if decoded.starts_with('\u{feff}') {
            decoded = decoded.trim_start_matches('\u{feff}').to_string();
        }
        router_data.raw_connector_response = Some(masking::Secret::new(decoded));
    }
    Ok(())
}

/// Get the flow name from the type
#[inline]
pub fn get_flow_name<F>() -> CustomResult<String, api_error_response::ApiErrorResponse> {
    Ok(std::any::type_name::<F>()
        .to_string()
        .rsplit("::")
        .next()
        .ok_or(api_error_response::ApiErrorResponse::InternalServerError)
        .attach_printable("Flow stringify failed")?
        .to_string())
}
