pub mod client;
pub mod request;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Debug,
    future::Future,
    str,
    time::{Duration, Instant},
};

use actix_web::{body, web, FromRequest, HttpRequest, HttpResponse, Responder, ResponseError};
use api_models::enums::{CaptureMethod, PaymentMethodType};
pub use client::{proxy_bypass_urls, ApiClient, MockApiClient, ProxyClient};
use common_enums::Currency;
pub use common_utils::request::{ContentType, Method, Request, RequestBuilder};
use common_utils::{
    consts::X_HS_LATENCY,
    errors::{ErrorSwitch, ReportSwitchExt},
    request::RequestContent,
};
use error_stack::{report, IntoReport, Report, ResultExt};
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing, tracing_actix_web::RequestId, Tag};
use serde::Serialize;
use serde_json::json;
use tera::{Context, Tera};

use self::request::{HeaderExt, RequestBuilderExt};
use super::authentication::AuthenticateAndFetch;
use crate::{
    configs::{settings::Connectors, Settings},
    consts,
    core::{
        api_locking,
        errors::{self, CustomResult},
        payments,
    },
    events::{
        api_logs::{ApiEvent, ApiEventMetric, ApiEventsType},
        connector_api_logs::ConnectorEvent,
    },
    logger,
    routes::{
        app::AppStateInfo,
        metrics::{self, request as metrics_request},
        AppState,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        ErrorResponse,
    },
};

pub type BoxedConnectorIntegration<'a, T, Req, Resp> =
    Box<&'a (dyn ConnectorIntegration<T, Req, Resp> + Send + Sync)>;

pub trait ConnectorIntegrationAny<T, Req, Resp>: Send + Sync + 'static {
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp>;
}

impl<S, T, Req, Resp> ConnectorIntegrationAny<T, Req, Resp> for S
where
    S: ConnectorIntegration<T, Req, Resp> + Send + Sync,
{
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp> {
        Box::new(self)
    }
}

pub trait ConnectorValidation: ConnectorCommon {
    fn validate_capture_method(
        &self,
        capture_method: Option<CaptureMethod>,
        _pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            CaptureMethod::Automatic => Ok(()),
            CaptureMethod::Manual | CaptureMethod::ManualMultiple | CaptureMethod::Scheduled => {
                Err(errors::ConnectorError::NotSupported {
                    message: capture_method.to_string(),
                    connector: self.id(),
                }
                .into())
            }
        }
    }

    fn validate_psync_reference_id(
        &self,
        data: &types::PaymentsSyncRouterData,
    ) -> CustomResult<(), errors::ConnectorError> {
        data.request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)
            .map(|_| ())
    }

    fn is_webhook_source_verification_mandatory(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
pub trait ConnectorIntegration<T, Req, Resp>: ConnectorIntegrationAny<T, Req, Resp> + Sync {
    fn get_headers(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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
        _req: &types::RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(String::new())
    }

    fn get_request_body(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::Json(Box::new(json!(r#"{}"#))))
    }

    fn get_request_form_data(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<reqwest::multipart::Form>, errors::ConnectorError> {
        Ok(None)
    }

    /// This module can be called before executing a payment flow where a pre-task is needed
    /// Eg: Some connectors requires one-time session token before making a payment, we can add the session token creation logic in this block
    async fn execute_pretasks(
        &self,
        _router_data: &mut types::RouterData<T, Req, Resp>,
        _app_state: &AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }

    /// This module can be called after executing a payment flow where a post-task needed
    /// Eg: Some connectors require payment sync to happen immediately after the authorize call to complete the transaction, we can add that logic in this block
    async fn execute_posttasks(
        &self,
        _router_data: &mut types::RouterData<T, Req, Resp>,
        _app_state: &AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }

    fn build_request(
        &self,
        req: &types::RouterData<T, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        metrics::UNIMPLEMENTED_FLOW.add(
            &metrics::CONTEXT,
            1,
            &[metrics::request::add_attributes(
                "connector",
                req.connector.clone(),
            )],
        );
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::RouterData<T, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        _res: types::Response,
    ) -> CustomResult<types::RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        T: Clone,
        Req: Clone,
        Resp: Clone,
    {
        event_builder.map(|e| e.set_error(json!({"error": "Not Implemented"})));
        Ok(data.clone())
    }

    fn get_error_response(
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
        _req: &types::RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn get_certificate_key(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }
}

pub enum CaptureSyncMethod {
    Individual,
    Bulk,
}

/// Handle the flow by interacting with connector module
/// `connector_request` is applicable only in case if the `CallConnectorAction` is `Trigger`
/// In other cases, It will be created if required, even if it is not passed
#[instrument(skip_all, fields(connector_name, payment_method))]
pub async fn execute_connector_processing_step<
    'b,
    'a,
    T: 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
>(
    state: &'b AppState,
    connector_integration: BoxedConnectorIntegration<'a, T, Req, Resp>,
    req: &'b types::RouterData<T, Req, Resp>,
    call_connector_action: payments::CallConnectorAction,
    connector_request: Option<Request>,
) -> CustomResult<types::RouterData<T, Req, Resp>, errors::ConnectorError>
where
    T: Clone + Debug,
    // BoxedConnectorIntegration<T, Req, Resp>: 'b,
{
    // If needed add an error stack as follows
    // connector_integration.build_request(req).attach_printable("Failed to build request");
    tracing::Span::current().record("connector_name", &req.connector);
    tracing::Span::current().record("payment_method", &req.payment_method.to_string());
    logger::debug!(connector_request=?connector_request);
    let mut router_data = req.clone();
    match call_connector_action {
        payments::CallConnectorAction::HandleResponse(res) => {
            let response = types::Response {
                headers: None,
                response: res.into(),
                status_code: 200,
            };
            connector_integration.handle_response(req, None, response)
        }
        payments::CallConnectorAction::Avoid => Ok(router_data),
        payments::CallConnectorAction::StatusUpdate {
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
                })
            } else {
                None
            };
            router_data.response = error_response.map(Err).unwrap_or(router_data.response);
            Ok(router_data)
        }
        payments::CallConnectorAction::Trigger => {
            metrics::CONNECTOR_CALL_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[
                    metrics::request::add_attributes("connector", req.connector.to_string()),
                    metrics::request::add_attributes(
                        "flow",
                        std::any::type_name::<T>()
                            .split("::")
                            .last()
                            .unwrap_or_default()
                            .to_string(),
                    ),
                ],
            );

            let connector_request = match connector_request {
                Some(connector_request) => Some(connector_request),
                None => connector_integration
                    .build_request(req, &state.conf.connectors)
                    .map_err(|error| {
                        if matches!(
                            error.current_context(),
                            &errors::ConnectorError::RequestEncodingFailed
                                | &errors::ConnectorError::RequestEncodingFailedWithReason(_)
                        ) {
                            metrics::REQUEST_BUILD_FAILURE.add(
                                &metrics::CONTEXT,
                                1,
                                &[metrics::request::add_attributes(
                                    "connector",
                                    req.connector.to_string(),
                                )],
                            )
                        }
                        error
                    })?,
            };

            match connector_request {
                Some(request) => {
                    let masked_request_body = match &request.body {
                        Some(request) => match request {
                            RequestContent::Json(i)
                            | RequestContent::FormUrlEncoded(i)
                            | RequestContent::Xml(i) => i
                                .masked_serialize()
                                .unwrap_or(json!({ "error": "failed to mask serialize"})),
                            RequestContent::FormData(_) => json!({"request_type": "FORM_DATA"}),
                        },
                        None => serde_json::Value::Null,
                    };
                    let request_url = request.url.clone();
                    let request_method = request.method;

                    let current_time = Instant::now();
                    let response = call_connector_api(state, request).await;
                    let external_latency = current_time.elapsed().as_millis();
                    logger::debug!(connector_response=?response);
                    let status_code = response
                        .as_ref()
                        .map(|i| {
                            i.as_ref()
                                .map_or_else(|value| value.status_code, |value| value.status_code)
                        })
                        .unwrap_or_default();
                    let mut connector_event = ConnectorEvent::new(
                        req.connector.clone(),
                        std::any::type_name::<T>(),
                        masked_request_body,
                        request_url,
                        request_method,
                        req.payment_id.clone(),
                        req.merchant_id.clone(),
                        state.request_id.as_ref(),
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
                                        .handle_response(req, Some(&mut connector_event), body)
                                        .map_err(|error| {
                                            if error.current_context()
                                            == &errors::ConnectorError::ResponseDeserializationFailed
                                        {
                                            metrics::RESPONSE_DESERIALIZATION_FAILURE.add(
                                                &metrics::CONTEXT,
                                                1,
                                                &[metrics::request::add_attributes(
                                                    "connector",
                                                    req.connector.to_string(),
                                                )],
                                            )
                                        }
                                            error
                                        });
                                    match handle_response_result {
                                        Ok(mut data) => {
                                            match connector_event.try_into() {
                                                Ok(event) => {
                                                    state.event_handler().log_event(event);
                                                }
                                                Err(err) => {
                                                    logger::error!(error=?err, "Error Logging Connector Event");
                                                }
                                            };
                                            data.connector_http_status_code =
                                                connector_http_status_code;
                                            // Add up multiple external latencies in case of multiple external calls within the same request.
                                            data.external_latency = Some(
                                                data.external_latency
                                                    .map_or(external_latency, |val| {
                                                        val + external_latency
                                                    }),
                                            );
                                            Ok(data)
                                        }
                                        Err(err) => {
                                            connector_event
                                                .set_error(json!({"error": err.to_string()}));

                                            match connector_event.try_into() {
                                                Ok(event) => {
                                                    state.event_handler().log_event(event);
                                                }
                                                Err(err) => {
                                                    logger::error!(error=?err, "Error Logging Connector Event");
                                                }
                                            }
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
                                        &metrics::CONTEXT,
                                        1,
                                        &[metrics::request::add_attributes(
                                            "connector",
                                            req.connector.clone(),
                                        )],
                                    );

                                    let error = match body.status_code {
                                        500..=511 => {
                                            let error_res = connector_integration
                                                .get_5xx_error_response(
                                                    body,
                                                    Some(&mut connector_event),
                                                )?;
                                            match connector_event.try_into() {
                                                Ok(event) => {
                                                    state.event_handler().log_event(event);
                                                }
                                                Err(err) => {
                                                    logger::error!(error=?err, "Error Logging Connector Event");
                                                }
                                            };
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
                            match connector_event.try_into() {
                                Ok(event) => {
                                    state.event_handler().log_event(event);
                                }
                                Err(err) => {
                                    logger::error!(error=?err, "Error Logging Connector Event");
                                }
                            };
                            if error.current_context().is_upstream_timeout() {
                                let error_response = ErrorResponse {
                                    code: consts::REQUEST_TIMEOUT_ERROR_CODE.to_string(),
                                    message: consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string(),
                                    reason: Some(consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string()),
                                    status_code: 504,
                                    attempt_status: None,
                                    connector_transaction_id: None,
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
                                Err(error.change_context(
                                    errors::ConnectorError::ProcessingStepFailed(None),
                                ))
                            }
                        }
                    }
                }
                None => Ok(router_data),
            }
        }
    }
}

#[instrument(skip_all)]
pub async fn call_connector_api(
    state: &AppState,
    request: Request,
) -> CustomResult<Result<types::Response, types::Response>, errors::ApiClientError> {
    let current_time = Instant::now();

    let response = state
        .api_client
        .send_request(state, request, None, true)
        .await;

    let elapsed_time = current_time.elapsed();
    logger::info!(request_time=?elapsed_time);

    handle_response(response).await
}

#[instrument(skip_all)]
pub async fn send_request(
    state: &AppState,
    request: Request,
    option_timeout_secs: Option<u64>,
) -> CustomResult<reqwest::Response, errors::ApiClientError> {
    logger::debug!(method=?request.method, headers=?request.headers, payload=?request.body, ?request);

    let url = reqwest::Url::parse(&request.url)
        .into_report()
        .change_context(errors::ApiClientError::UrlEncodingFailed)?;

    #[cfg(feature = "dummy_connector")]
    let should_bypass_proxy = url
        .as_str()
        .starts_with(&state.conf.connectors.dummyconnector.base_url)
        || proxy_bypass_urls(&state.conf.locker).contains(&url.to_string());
    #[cfg(not(feature = "dummy_connector"))]
    let should_bypass_proxy = proxy_bypass_urls(&state.conf.locker).contains(&url.to_string());
    let client = client::create_client(
        &state.conf.proxy,
        should_bypass_proxy,
        request.certificate,
        request.certificate_key,
    )?;

    let headers = request.headers.construct_header_map()?;
    let metrics_tag = router_env::opentelemetry::KeyValue {
        key: consts::METRICS_HOST_TAG_NAME.into(),
        value: url.host_str().unwrap_or_default().to_string().into(),
    };
    let request = {
        match request.method {
            Method::Get => client.get(url),
            Method::Post => {
                let client = client.post(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData(form)) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload)) => {
                        let body = quick_xml::se::to_string(&payload)
                            .into_report()
                            .change_context(errors::ApiClientError::BodySerializationFailed)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    None => client,
                }
            }
            Method::Put => {
                let client = client.put(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData(form)) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload)) => {
                        let body = quick_xml::se::to_string(&payload)
                            .into_report()
                            .change_context(errors::ApiClientError::BodySerializationFailed)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    None => client,
                }
            }
            Method::Patch => {
                let client = client.patch(url);
                match request.body {
                    Some(RequestContent::Json(payload)) => client.json(&payload),
                    Some(RequestContent::FormData(form)) => client.multipart(form),
                    Some(RequestContent::FormUrlEncoded(payload)) => client.form(&payload),
                    Some(RequestContent::Xml(payload)) => {
                        let body = quick_xml::se::to_string(&payload)
                            .into_report()
                            .change_context(errors::ApiClientError::BodySerializationFailed)?;
                        client.body(body).header("Content-Type", "application/xml")
                    }
                    None => client,
                }
            }
            Method::Delete => client.delete(url),
        }
        .add_headers(headers)
        .timeout(Duration::from_secs(
            option_timeout_secs.unwrap_or(crate::consts::REQUEST_TIME_OUT),
        ))
    };

    // We cannot clone the request type, because it has Form trait which is not clonable. So we are cloning the request builder here.
    let cloned_send_request = request.try_clone().map(|cloned_request| async {
        cloned_request
            .send()
            .await
            .map_err(|error| match error {
                error if error.is_timeout() => {
                    metrics::REQUEST_BUILD_FAILURE.add(&metrics::CONTEXT, 1, &[]);
                    errors::ApiClientError::RequestTimeoutReceived
                }
                error if is_connection_closed_before_message_could_complete(&error) => {
                    metrics::REQUEST_BUILD_FAILURE.add(&metrics::CONTEXT, 1, &[]);
                    errors::ApiClientError::ConnectionClosedIncompleteMessage
                }
                _ => errors::ApiClientError::RequestNotSent(error.to_string()),
            })
            .into_report()
            .attach_printable("Unable to send request to connector")
    });

    let send_request = async {
        request
            .send()
            .await
            .map_err(|error| match error {
                error if error.is_timeout() => {
                    metrics::REQUEST_BUILD_FAILURE.add(&metrics::CONTEXT, 1, &[]);
                    errors::ApiClientError::RequestTimeoutReceived
                }
                error if is_connection_closed_before_message_could_complete(&error) => {
                    metrics::REQUEST_BUILD_FAILURE.add(&metrics::CONTEXT, 1, &[]);
                    errors::ApiClientError::ConnectionClosedIncompleteMessage
                }
                _ => errors::ApiClientError::RequestNotSent(error.to_string()),
            })
            .into_report()
            .attach_printable("Unable to send request to connector")
    };

    let response = metrics_request::record_operation_time(
        send_request,
        &metrics::EXTERNAL_REQUEST_TIME,
        &[metrics_tag.clone()],
    )
    .await;
    // Retry once if the response is connection closed.
    //
    // This is just due to the racy nature of networking.
    // hyper has a connection pool of idle connections, and it selected one to send your request.
    // Most of the time, hyper will receive the server’s FIN and drop the dead connection from its pool.
    // But occasionally, a connection will be selected from the pool
    // and written to at the same time the server is deciding to close the connection.
    // Since hyper already wrote some of the request,
    // it can’t really retry it automatically on a new connection, since the server may have acted already
    match response {
        Ok(response) => Ok(response),
        Err(error)
            if error.current_context()
                == &errors::ApiClientError::ConnectionClosedIncompleteMessage =>
        {
            metrics::AUTO_RETRY_CONNECTION_CLOSED.add(&metrics::CONTEXT, 1, &[]);
            match cloned_send_request {
                Some(cloned_request) => {
                    logger::info!(
                        "Retrying request due to connection closed before message could complete"
                    );
                    metrics_request::record_operation_time(
                        cloned_request,
                        &metrics::EXTERNAL_REQUEST_TIME,
                        &[metrics_tag],
                    )
                    .await
                }
                None => {
                    logger::info!("Retrying request due to connection closed before message could complete failed as request is not clonable");
                    Err(error)
                }
            }
        }
        err @ Err(_) => err,
    }
}

fn is_connection_closed_before_message_could_complete(error: &reqwest::Error) -> bool {
    let mut source = error.source();
    while let Some(err) = source {
        if let Some(hyper_err) = err.downcast_ref::<hyper::Error>() {
            if hyper_err.is_incomplete_message() {
                return true;
            }
        }
        source = err.source();
    }
    false
}

#[instrument(skip_all)]
async fn handle_response(
    response: CustomResult<reqwest::Response, errors::ApiClientError>,
) -> CustomResult<Result<types::Response, types::Response>, errors::ApiClientError> {
    response
        .map(|response| async {
            logger::info!(?response);
            let status_code = response.status().as_u16();
            let headers = Some(response.headers().to_owned());

            match status_code {
                200..=202 | 302 | 204 => {
                    logger::debug!(response=?response);
                    // If needed add log line
                    // logger:: error!( error_parsing_response=?err);
                    let response = response
                        .bytes()
                        .await
                        .into_report()
                        .change_context(errors::ApiClientError::ResponseDecodingFailed)
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
                            .change_context(errors::ApiClientError::ResponseDecodingFailed)
                            .attach_printable("Client error response received")
                    })?;
                    // let error = match status_code {
                    //     500 => errors::ApiClientError::InternalServerErrorReceived,
                    //     502 => errors::ApiClientError::BadGatewayReceived,
                    //     503 => errors::ApiClientError::ServiceUnavailableReceived,
                    //     504 => errors::ApiClientError::GatewayTimeoutReceived,
                    //     _ => errors::ApiClientError::UnexpectedServerResponse,
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
                            .change_context(errors::ApiClientError::ResponseDecodingFailed)
                            .attach_printable("Client error response received")
                    })?;
                    /* let error = match status_code {
                        400 => errors::ApiClientError::BadRequestReceived(bytes),
                        401 => errors::ApiClientError::UnauthorizedReceived(bytes),
                        403 => errors::ApiClientError::ForbiddenReceived,
                        404 => errors::ApiClientError::NotFoundReceived(bytes),
                        405 => errors::ApiClientError::MethodNotAllowedReceived,
                        408 => errors::ApiClientError::RequestTimeoutReceived,
                        422 => errors::ApiClientError::UnprocessableEntityReceived(bytes),
                        429 => errors::ApiClientError::TooManyRequestsReceived,
                        _ => errors::ApiClientError::UnexpectedServerResponse,
                    };
                    Err(report!(error).attach_printable("Client error response received"))
                        */
                    Ok(Err(types::Response {
                        headers,
                        response: bytes,
                        status_code,
                    }))
                }

                _ => Err(report!(errors::ApiClientError::UnexpectedServerResponse)
                    .attach_printable("Unexpected response from server")),
            }
        })?
        .await
}

#[derive(Debug, Eq, PartialEq)]
pub enum ApplicationResponse<R> {
    Json(R),
    StatusOk,
    TextPlain(String),
    JsonForRedirection(api::RedirectionResponse),
    Form(Box<RedirectionFormData>),
    PaymentLinkForm(Box<PaymentLinkAction>),
    FileData((Vec<u8>, mime::Mime)),
    JsonWithHeaders((R, Vec<(String, String)>)),
}

#[derive(Debug, Eq, PartialEq)]
pub enum PaymentLinkAction {
    PaymentLinkFormData(PaymentLinkFormData),
    PaymentLinkStatus(PaymentLinkStatusData),
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkFormData {
    pub js_script: String,
    pub css_script: String,
    pub sdk_url: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkStatusData {
    pub js_script: String,
    pub css_script: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RedirectionFormData {
    pub redirect_form: RedirectForm,
    pub payment_method_data: Option<api::PaymentMethodData>,
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PaymentAction {
    PSync,
    CompleteAuthorize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ApplicationRedirectResponse {
    pub url: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum RedirectForm {
    Form {
        endpoint: String,
        method: Method,
        form_fields: HashMap<String, String>,
    },
    Html {
        html_data: String,
    },
    BlueSnap {
        payment_fields_token: String, // payment-field-token
    },
    CybersourceAuthSetup {
        access_token: String,
        ddc_url: String,
        reference_id: String,
    },
    CybersourceConsumerAuth {
        access_token: String,
        step_up_url: String,
    },
    Payme,
    Braintree {
        client_token: String,
        card_token: String,
        bin: String,
    },
    Nmi {
        amount: String,
        currency: Currency,
        public_key: Secret<String>,
        customer_vault_id: String,
        order_id: String,
    },
}

impl From<(url::Url, Method)> for RedirectForm {
    fn from((mut redirect_url, method): (url::Url, Method)) -> Self {
        let form_fields = std::collections::HashMap::from_iter(
            redirect_url
                .query_pairs()
                .map(|(key, value)| (key.to_string(), value.to_string())),
        );

        // Do not include query params in the endpoint
        redirect_url.set_query(None);

        Self::Form {
            endpoint: redirect_url.to_string(),
            method,
            form_fields,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthFlow {
    Client,
    Merchant,
}

#[instrument(skip(request, payload, state, func, api_auth), fields(merchant_id))]
pub async fn server_wrap_util<'a, 'b, A, U, T, Q, F, Fut, E, OErr>(
    flow: &'a impl router_env::types::FlowMetric,
    state: web::Data<A>,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn AuthenticateAndFetch<U, A>,
    lock_action: api_locking::LockAction,
) -> CustomResult<ApplicationResponse<Q>, OErr>
where
    F: Fn(A, U, T) -> Fut,
    'b: 'a,
    Fut: Future<Output = CustomResult<ApplicationResponse<Q>, E>>,
    Q: Serialize + Debug + 'a + ApiEventMetric,
    T: Debug + Serialize + ApiEventMetric,
    A: AppStateInfo + Clone,
    E: ErrorSwitch<OErr> + error_stack::Context,
    OErr: ResponseError + error_stack::Context + Serialize,
    errors::ApiErrorResponse: ErrorSwitch<OErr>,
{
    let request_id = RequestId::extract(request)
        .await
        .into_report()
        .attach_printable("Unable to extract request id from request")
        .change_context(errors::ApiErrorResponse::InternalServerError.switch())?;

    let mut request_state = state.get_ref().clone();

    request_state.add_request_id(request_id);
    let start_instant = Instant::now();
    let serialized_request = masking::masked_serialize(&payload)
        .into_report()
        .attach_printable("Failed to serialize json request")
        .change_context(errors::ApiErrorResponse::InternalServerError.switch())?;

    let mut event_type = payload.get_api_event_type();

    // Currently auth failures are not recorded as API events
    let (auth_out, auth_type) = api_auth
        .authenticate_and_fetch(request.headers(), &request_state)
        .await
        .switch()?;

    let merchant_id = auth_type
        .get_merchant_id()
        .unwrap_or("MERCHANT_ID_NOT_FOUND")
        .to_string();

    request_state.add_merchant_id(Some(merchant_id.clone()));

    request_state.add_flow_name(flow.to_string());

    tracing::Span::current().record("merchant_id", &merchant_id);

    let output = {
        lock_action
            .clone()
            .perform_locking_action(&request_state, merchant_id.to_owned())
            .await
            .switch()?;
        let res = func(request_state.clone(), auth_out, payload)
            .await
            .switch();
        lock_action
            .free_lock_action(&request_state, merchant_id.to_owned())
            .await
            .switch()?;
        res
    };
    let request_duration = Instant::now()
        .saturating_duration_since(start_instant)
        .as_millis();

    let mut serialized_response = None;
    let mut error = None;
    let mut overhead_latency = None;

    let status_code = match output.as_ref() {
        Ok(res) => {
            if let ApplicationResponse::Json(data) = res {
                serialized_response.replace(
                    masking::masked_serialize(&data)
                        .into_report()
                        .attach_printable("Failed to serialize json response")
                        .change_context(errors::ApiErrorResponse::InternalServerError.switch())?,
                );
            } else if let ApplicationResponse::JsonWithHeaders((data, headers)) = res {
                serialized_response.replace(
                    masking::masked_serialize(&data)
                        .into_report()
                        .attach_printable("Failed to serialize json response")
                        .change_context(errors::ApiErrorResponse::InternalServerError.switch())?,
                );

                if let Some((_, value)) = headers.iter().find(|(key, _)| key == X_HS_LATENCY) {
                    if let Ok(external_latency) = value.parse::<u128>() {
                        overhead_latency.replace(external_latency);
                    }
                }
            }
            event_type = res.get_api_event_type().or(event_type);

            metrics::request::track_response_status_code(res)
        }
        Err(err) => {
            error.replace(
                serde_json::to_value(err.current_context())
                    .into_report()
                    .attach_printable("Failed to serialize json response")
                    .change_context(errors::ApiErrorResponse::InternalServerError.switch())
                    .ok()
                    .into(),
            );
            err.current_context().status_code().as_u16().into()
        }
    };

    let api_event = ApiEvent::new(
        Some(merchant_id.clone()),
        flow,
        &request_id,
        request_duration,
        status_code,
        serialized_request,
        serialized_response,
        overhead_latency,
        auth_type,
        error,
        event_type.unwrap_or(ApiEventsType::Miscellaneous),
        request,
        request.method(),
    );
    match api_event.clone().try_into() {
        Ok(event) => {
            state.event_handler().log_event(event);
        }
        Err(err) => {
            logger::error!(error=?err, event=?api_event, "Error Logging API Event");
        }
    }

    metrics::request::status_code_metrics(status_code, flow.to_string(), merchant_id.to_string());

    output
}

#[instrument(
    skip(request, state, func, api_auth, payload),
    fields(request_method, request_url_path, status_code)
)]
pub async fn server_wrap<'a, A, T, U, Q, F, Fut, E>(
    flow: impl router_env::types::FlowMetric,
    state: web::Data<A>,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn AuthenticateAndFetch<U, A>,
    lock_action: api_locking::LockAction,
) -> HttpResponse
where
    F: Fn(A, U, T) -> Fut,
    Fut: Future<Output = CustomResult<ApplicationResponse<Q>, E>>,
    Q: Serialize + Debug + ApiEventMetric + 'a,
    T: Debug + Serialize + ApiEventMetric,
    A: AppStateInfo + Clone,
    ApplicationResponse<Q>: Debug,
    E: ErrorSwitch<api_models::errors::types::ApiErrorResponse> + error_stack::Context,
{
    let request_method = request.method().as_str();
    let url_path = request.path();
    tracing::Span::current().record("request_method", request_method);
    tracing::Span::current().record("request_url_path", url_path);

    let start_instant = Instant::now();
    logger::info!(tag = ?Tag::BeginRequest, payload = ?payload);

    let server_wrap_util_res = metrics::request::record_request_time_metric(
        server_wrap_util(
            &flow,
            state.clone(),
            request,
            payload,
            func,
            api_auth,
            lock_action,
        ),
        &flow,
    )
    .await
    .map(|response| {
        logger::info!(api_response =? response);
        response
    });

    let res = match server_wrap_util_res {
        Ok(ApplicationResponse::Json(response)) => match serde_json::to_string(&response) {
            Ok(res) => http_response_json(res),
            Err(_) => http_response_err(
                r#"{
                    "error": {
                        "message": "Error serializing response from connector"
                    }
                }"#,
            ),
        },
        Ok(ApplicationResponse::StatusOk) => http_response_ok(),
        Ok(ApplicationResponse::TextPlain(text)) => http_response_plaintext(text),
        Ok(ApplicationResponse::FileData((file_data, content_type))) => {
            http_response_file_data(file_data, content_type)
        }
        Ok(ApplicationResponse::JsonForRedirection(response)) => {
            match serde_json::to_string(&response) {
                Ok(res) => http_redirect_response(res, response),
                Err(_) => http_response_err(
                    r#"{
                    "error": {
                        "message": "Error serializing response from connector"
                    }
                }"#,
                ),
            }
        }
        Ok(ApplicationResponse::Form(redirection_data)) => {
            let config = state.conf();
            build_redirection_form(
                &redirection_data.redirect_form,
                redirection_data.payment_method_data,
                redirection_data.amount,
                redirection_data.currency,
                config,
            )
            .respond_to(request)
            .map_into_boxed_body()
        }

        Ok(ApplicationResponse::PaymentLinkForm(boxed_payment_link_data)) => {
            match *boxed_payment_link_data {
                PaymentLinkAction::PaymentLinkFormData(payment_link_data) => {
                    match build_payment_link_html(payment_link_data) {
                        Ok(rendered_html) => http_response_html_data(rendered_html),
                        Err(_) => http_response_err(
                            r#"{
                                "error": {
                                    "message": "Error while rendering payment link html page"
                                }
                            }"#,
                        ),
                    }
                }
                PaymentLinkAction::PaymentLinkStatus(payment_link_data) => {
                    match get_payment_link_status(payment_link_data) {
                        Ok(rendered_html) => http_response_html_data(rendered_html),
                        Err(_) => http_response_err(
                            r#"{
                                "error": {
                                    "message": "Error while rendering payment link status page"
                                }
                            }"#,
                        ),
                    }
                }
            }
        }

        Ok(ApplicationResponse::JsonWithHeaders((response, headers))) => {
            let request_elapsed_time = request.headers().get(X_HS_LATENCY).and_then(|value| {
                if value == "true" {
                    Some(start_instant.elapsed())
                } else {
                    None
                }
            });
            match serde_json::to_string(&response) {
                Ok(res) => http_response_json_with_headers(res, headers, request_elapsed_time),
                Err(_) => http_response_err(
                    r#"{
                        "error": {
                            "message": "Error serializing response from connector"
                        }
                    }"#,
                ),
            }
        }
        Err(error) => log_and_return_error_response(error),
    };

    let response_code = res.status().as_u16();
    tracing::Span::current().record("status_code", response_code);

    let end_instant = Instant::now();
    let request_duration = end_instant.saturating_duration_since(start_instant);
    logger::info!(
        tag = ?Tag::EndRequest,
        time_taken_ms = request_duration.as_millis(),
    );
    res
}

pub fn log_and_return_error_response<T>(error: Report<T>) -> HttpResponse
where
    T: error_stack::Context + Clone + ResponseError,
    Report<T>: EmbedError,
{
    logger::error!(?error);
    HttpResponse::from_error(error.embed().current_context().clone())
}

pub trait EmbedError: Sized {
    fn embed(self) -> Self {
        self
    }
}

impl EmbedError for Report<api_models::errors::types::ApiErrorResponse> {
    fn embed(self) -> Self {
        #[cfg(feature = "detailed_errors")]
        {
            let mut report = self;
            let error_trace = serde_json::to_value(&report).ok().and_then(|inner| {
                serde_json::from_value::<Vec<errors::NestedErrorStack<'_>>>(inner)
                    .ok()
                    .map(Into::<errors::VecLinearErrorStack<'_>>::into)
                    .map(serde_json::to_value)
                    .transpose()
                    .ok()
                    .flatten()
            });

            match report.downcast_mut::<api_models::errors::types::ApiErrorResponse>() {
                None => {}
                Some(inner) => {
                    inner.get_internal_error_mut().stacktrace = error_trace;
                }
            }
            report
        }

        #[cfg(not(feature = "detailed_errors"))]
        self
    }
}

pub fn http_response_json<T: body::MessageBody + 'static>(response: T) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(mime::APPLICATION_JSON)
        .body(response)
}

pub fn http_server_error_json_response<T: body::MessageBody + 'static>(
    response: T,
) -> HttpResponse {
    HttpResponse::InternalServerError()
        .content_type(mime::APPLICATION_JSON)
        .body(response)
}

pub fn http_response_json_with_headers<T: body::MessageBody + 'static>(
    response: T,
    mut headers: Vec<(String, String)>,
    request_duration: Option<Duration>,
) -> HttpResponse {
    let mut response_builder = HttpResponse::Ok();

    for (name, value) in headers.iter_mut() {
        if name == X_HS_LATENCY {
            if let Some(request_duration) = request_duration {
                if let Ok(external_latency) = value.parse::<u128>() {
                    let updated_duration = request_duration.as_millis() - external_latency;
                    *value = updated_duration.to_string();
                }
            }
        }
        response_builder.append_header((name.clone(), value.clone()));
    }

    response_builder
        .content_type(mime::APPLICATION_JSON)
        .body(response)
}

pub fn http_response_plaintext<T: body::MessageBody + 'static>(res: T) -> HttpResponse {
    HttpResponse::Ok().content_type(mime::TEXT_PLAIN).body(res)
}

pub fn http_response_file_data<T: body::MessageBody + 'static>(
    res: T,
    content_type: mime::Mime,
) -> HttpResponse {
    HttpResponse::Ok().content_type(content_type).body(res)
}

pub fn http_response_html_data<T: body::MessageBody + 'static>(res: T) -> HttpResponse {
    HttpResponse::Ok().content_type(mime::TEXT_HTML).body(res)
}

pub fn http_response_ok() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn http_redirect_response<T: body::MessageBody + 'static>(
    response: T,
    redirection_response: api::RedirectionResponse,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(mime::APPLICATION_JSON)
        .append_header((
            "Location",
            redirection_response.return_url_with_query_params,
        ))
        .status(http::StatusCode::FOUND)
        .body(response)
}

pub fn http_response_err<T: body::MessageBody + 'static>(response: T) -> HttpResponse {
    HttpResponse::BadRequest()
        .content_type(mime::APPLICATION_JSON)
        .body(response)
}

pub trait ConnectorRedirectResponse {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Avoid)
    }
}

pub trait Authenticate {
    fn get_client_secret(&self) -> Option<&String> {
        None
    }
}

impl Authenticate for api_models::payments::PaymentsRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl Authenticate for api_models::payment_methods::PaymentMethodListRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl Authenticate for api_models::payments::PaymentsSessionRequest {
    fn get_client_secret(&self) -> Option<&String> {
        Some(&self.client_secret)
    }
}

impl Authenticate for api_models::payments::PaymentsRetrieveRequest {}
impl Authenticate for api_models::payments::PaymentsCancelRequest {}
impl Authenticate for api_models::payments::PaymentsCaptureRequest {}
impl Authenticate for api_models::payments::PaymentsIncrementalAuthorizationRequest {}
impl Authenticate for api_models::payments::PaymentsStartRequest {}
// impl Authenticate for api_models::payments::PaymentsApproveRequest {}
impl Authenticate for api_models::payments::PaymentsRejectRequest {}

pub fn build_redirection_form(
    form: &RedirectForm,
    payment_method_data: Option<api_models::payments::PaymentMethodData>,
    amount: String,
    currency: String,
    config: Settings,
) -> maud::Markup {
    use maud::PreEscaped;

    match form {
        RedirectForm::Form {
            endpoint,
            method,
            form_fields,
        } => maud::html! {
        (maud::DOCTYPE)
        html {
            meta name="viewport" content="width=device-width, initial-scale=1";
            head {
                style {
                    r##"

                    "##
                }
                (PreEscaped(r##"
                <style>
                    #loader1 {
                        width: 500px,
                    }
                    @media max-width: 600px {
                        #loader1 {
                            width: 200px
                        }
                    }
                </style>
                "##))
            }

            body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-left: auto; margin-right: auto;" { "" }

                (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                (PreEscaped(r#"
                <script>
                var anime = bodymovin.loadAnimation({
                    container: document.getElementById('loader1'),
                    renderer: 'svg',
                    loop: true,
                    autoplay: true,
                    name: 'hyperswitch loader',
                    animationData: {"v":"4.8.0","meta":{"g":"LottieFiles AE 3.1.1","a":"","k":"","d":"","tc":""},"fr":29.9700012207031,"ip":0,"op":31.0000012626559,"w":400,"h":250,"nm":"loader_shape","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"circle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[278.25,202.671,0],"ix":2},"a":{"a":0,"k":[23.72,23.72,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[12.935,0],[0,-12.936],[-12.935,0],[0,12.935]],"o":[[-12.952,0],[0,12.935],[12.935,0],[0,-12.936]],"v":[[0,-23.471],[-23.47,0.001],[0,23.471],[23.47,0.001]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":10,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":19.99,"s":[100]},{"t":29.9800012211104,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[23.72,23.721],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0},{"ddd":0,"ind":2,"ty":4,"nm":"square 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[196.25,201.271,0],"ix":2},"a":{"a":0,"k":[22.028,22.03,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[1.914,0],[0,0],[0,-1.914],[0,0],[-1.914,0],[0,0],[0,1.914],[0,0]],"o":[[0,0],[-1.914,0],[0,0],[0,1.914],[0,0],[1.914,0],[0,0],[0,-1.914]],"v":[[18.313,-21.779],[-18.312,-21.779],[-21.779,-18.313],[-21.779,18.314],[-18.312,21.779],[18.313,21.779],[21.779,18.314],[21.779,-18.313]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":5,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":14.99,"s":[100]},{"t":24.9800010174563,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[22.028,22.029],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":47.0000019143492,"st":0,"bm":0},{"ddd":0,"ind":3,"ty":4,"nm":"Triangle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[116.25,200.703,0],"ix":2},"a":{"a":0,"k":[27.11,21.243,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[0,0],[0.558,-0.879],[0,0],[-1.133,0],[0,0],[0.609,0.947],[0,0]],"o":[[-0.558,-0.879],[0,0],[-0.609,0.947],[0,0],[1.133,0],[0,0],[0,0]],"v":[[1.209,-20.114],[-1.192,-20.114],[-26.251,18.795],[-25.051,20.993],[25.051,20.993],[26.251,18.795],[1.192,-20.114]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":0,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":9.99,"s":[100]},{"t":19.9800008138021,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[27.11,21.243],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0}],"markers":[]}
                })
                </script>
                "#))


                h3 style="text-align: center;" { "Please wait while we process your payment..." }
                    form action=(PreEscaped(endpoint)) method=(method.to_string()) #payment_form {
                        @for (field, value) in form_fields {
                        input type="hidden" name=(field) value=(value);
                    }
                }

                (PreEscaped(r#"<script type="text/javascript"> var frm = document.getElementById("payment_form"); window.setTimeout(function () { frm.submit(); }, 300); </script>"#))
            }
        }
        },
        RedirectForm::Html { html_data } => PreEscaped(html_data.to_string()),
        RedirectForm::BlueSnap {
            payment_fields_token,
        } => {
            let card_details =
                if let Some(api::PaymentMethodData::Card(ccard)) = payment_method_data {
                    format!(
                        "var saveCardDirectly={{cvv: \"{}\",amount: {},currency: \"{}\"}};",
                        ccard.card_cvc.peek(),
                        amount,
                        currency
                    )
                } else {
                    "".to_string()
                };
            let bluesnap_sdk_url = config.connectors.bluesnap.secondary_base_url;
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    (PreEscaped(format!("<script src=\"{bluesnap_sdk_url}web-sdk/5/bluesnap.js\"></script>")))
                }
                    body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                        div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-top: 150px; margin-left: auto; margin-right: auto;" { "" }

                        (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                        (PreEscaped(r#"
                        <script>
                        var anime = bodymovin.loadAnimation({
                            container: document.getElementById('loader1'),
                            renderer: 'svg',
                            loop: true,
                            autoplay: true,
                            name: 'hyperswitch loader',
                            animationData: {"v":"4.8.0","meta":{"g":"LottieFiles AE 3.1.1","a":"","k":"","d":"","tc":""},"fr":29.9700012207031,"ip":0,"op":31.0000012626559,"w":400,"h":250,"nm":"loader_shape","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"circle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[278.25,202.671,0],"ix":2},"a":{"a":0,"k":[23.72,23.72,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[12.935,0],[0,-12.936],[-12.935,0],[0,12.935]],"o":[[-12.952,0],[0,12.935],[12.935,0],[0,-12.936]],"v":[[0,-23.471],[-23.47,0.001],[0,23.471],[23.47,0.001]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":10,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":19.99,"s":[100]},{"t":29.9800012211104,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[23.72,23.721],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0},{"ddd":0,"ind":2,"ty":4,"nm":"square 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[196.25,201.271,0],"ix":2},"a":{"a":0,"k":[22.028,22.03,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[1.914,0],[0,0],[0,-1.914],[0,0],[-1.914,0],[0,0],[0,1.914],[0,0]],"o":[[0,0],[-1.914,0],[0,0],[0,1.914],[0,0],[1.914,0],[0,0],[0,-1.914]],"v":[[18.313,-21.779],[-18.312,-21.779],[-21.779,-18.313],[-21.779,18.314],[-18.312,21.779],[18.313,21.779],[21.779,18.314],[21.779,-18.313]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":5,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":14.99,"s":[100]},{"t":24.9800010174563,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[22.028,22.029],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":47.0000019143492,"st":0,"bm":0},{"ddd":0,"ind":3,"ty":4,"nm":"Triangle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[116.25,200.703,0],"ix":2},"a":{"a":0,"k":[27.11,21.243,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[0,0],[0.558,-0.879],[0,0],[-1.133,0],[0,0],[0.609,0.947],[0,0]],"o":[[-0.558,-0.879],[0,0],[-0.609,0.947],[0,0],[1.133,0],[0,0],[0,0]],"v":[[1.209,-20.114],[-1.192,-20.114],[-26.251,18.795],[-25.051,20.993],[25.051,20.993],[26.251,18.795],[1.192,-20.114]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":0,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":9.99,"s":[100]},{"t":19.9800008138021,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[27.11,21.243],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0}],"markers":[]}
                        })
                        </script>
                        "#))


                        h3 style="text-align: center;" { "Please wait while we process your payment..." }
                    }

                (PreEscaped(format!("<script>
                    bluesnap.threeDsPaymentsSetup(\"{payment_fields_token}\",
                    function(sdkResponse) {{
                        // console.log(sdkResponse);
                        var f = document.createElement('form');
                        f.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/complete/bluesnap?paymentToken={payment_fields_token}\");
                        f.method='POST';
                        var i=document.createElement('input');
                        i.type='hidden';
                        i.name='authentication_response';
                        i.value=JSON.stringify(sdkResponse);
                        f.appendChild(i);
                        document.body.appendChild(f);
                        f.submit();
                    }});
                    {card_details}
                    bluesnap.threeDsPaymentsSubmitData(saveCardDirectly);
                </script>
                ")))
                }}
        }
        RedirectForm::CybersourceAuthSetup {
            access_token,
            ddc_url,
            reference_id,
        } => {
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {
                    meta name="viewport" content="width=device-width, initial-scale=1";
                }
                    body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                        div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-top: 150px; margin-left: auto; margin-right: auto;" { "" }

                        (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                        (PreEscaped(r#"
                            <script>
                            var anime = bodymovin.loadAnimation({
                                container: document.getElementById('loader1'),
                                renderer: 'svg',
                                loop: true,
                                autoplay: true,
                                name: 'hyperswitch loader',
                                animationData: {"v":"4.8.0","meta":{"g":"LottieFiles AE 3.1.1","a":"","k":"","d":"","tc":""},"fr":29.9700012207031,"ip":0,"op":31.0000012626559,"w":400,"h":250,"nm":"loader_shape","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"circle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[278.25,202.671,0],"ix":2},"a":{"a":0,"k":[23.72,23.72,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[12.935,0],[0,-12.936],[-12.935,0],[0,12.935]],"o":[[-12.952,0],[0,12.935],[12.935,0],[0,-12.936]],"v":[[0,-23.471],[-23.47,0.001],[0,23.471],[23.47,0.001]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":10,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":19.99,"s":[100]},{"t":29.9800012211104,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[23.72,23.721],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0},{"ddd":0,"ind":2,"ty":4,"nm":"square 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[196.25,201.271,0],"ix":2},"a":{"a":0,"k":[22.028,22.03,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[1.914,0],[0,0],[0,-1.914],[0,0],[-1.914,0],[0,0],[0,1.914],[0,0]],"o":[[0,0],[-1.914,0],[0,0],[0,1.914],[0,0],[1.914,0],[0,0],[0,-1.914]],"v":[[18.313,-21.779],[-18.312,-21.779],[-21.779,-18.313],[-21.779,18.314],[-18.312,21.779],[18.313,21.779],[21.779,18.314],[21.779,-18.313]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":5,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":14.99,"s":[100]},{"t":24.9800010174563,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[22.028,22.029],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":47.0000019143492,"st":0,"bm":0},{"ddd":0,"ind":3,"ty":4,"nm":"Triangle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[116.25,200.703,0],"ix":2},"a":{"a":0,"k":[27.11,21.243,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[0,0],[0.558,-0.879],[0,0],[-1.133,0],[0,0],[0.609,0.947],[0,0]],"o":[[-0.558,-0.879],[0,0],[-0.609,0.947],[0,0],[1.133,0],[0,0],[0,0]],"v":[[1.209,-20.114],[-1.192,-20.114],[-26.251,18.795],[-25.051,20.993],[25.051,20.993],[26.251,18.795],[1.192,-20.114]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":0,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":9.99,"s":[100]},{"t":19.9800008138021,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[27.11,21.243],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0}],"markers":[]}
                            })
                            </script>
                            "#))


                        h3 style="text-align: center;" { "Please wait while we process your payment..." }
                    }

                (PreEscaped(r#"<iframe id="cardinal_collection_iframe" name="collectionIframe" height="10" width="10" style="display: none;"></iframe>"#))
                (PreEscaped(format!("<form id=\"cardinal_collection_form\" method=\"POST\" target=\"collectionIframe\" action=\"{ddc_url}\">
                <input id=\"cardinal_collection_form_input\" type=\"hidden\" name=\"JWT\" value=\"{access_token}\">
              </form>")))
              (PreEscaped(r#"<script>
              window.onload = function() {
              var cardinalCollectionForm = document.querySelector('#cardinal_collection_form'); if(cardinalCollectionForm) cardinalCollectionForm.submit();
              }
              </script>"#))
              (PreEscaped(format!("<script>
                window.addEventListener(\"message\", function(event) {{
                    if (event.origin === \"https://centinelapistag.cardinalcommerce.com\" || event.origin === \"https://centinelapi.cardinalcommerce.com\") {{
                      window.location.href = window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/complete/cybersource?referenceId={reference_id}\");
                    }}
                  }}, false);
                </script>
                ")))
            }}
        }
        RedirectForm::CybersourceConsumerAuth {
            access_token,
            step_up_url,
        } => {
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {
                    meta name="viewport" content="width=device-width, initial-scale=1";
                }
                    body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                        div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-top: 150px; margin-left: auto; margin-right: auto;" { "" }

                        (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                        (PreEscaped(r#"
                            <script>
                            var anime = bodymovin.loadAnimation({
                                container: document.getElementById('loader1'),
                                renderer: 'svg',
                                loop: true,
                                autoplay: true,
                                name: 'hyperswitch loader',
                                animationData: {"v":"4.8.0","meta":{"g":"LottieFiles AE 3.1.1","a":"","k":"","d":"","tc":""},"fr":29.9700012207031,"ip":0,"op":31.0000012626559,"w":400,"h":250,"nm":"loader_shape","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"circle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[278.25,202.671,0],"ix":2},"a":{"a":0,"k":[23.72,23.72,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[12.935,0],[0,-12.936],[-12.935,0],[0,12.935]],"o":[[-12.952,0],[0,12.935],[12.935,0],[0,-12.936]],"v":[[0,-23.471],[-23.47,0.001],[0,23.471],[23.47,0.001]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":10,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":19.99,"s":[100]},{"t":29.9800012211104,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[23.72,23.721],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0},{"ddd":0,"ind":2,"ty":4,"nm":"square 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[196.25,201.271,0],"ix":2},"a":{"a":0,"k":[22.028,22.03,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[1.914,0],[0,0],[0,-1.914],[0,0],[-1.914,0],[0,0],[0,1.914],[0,0]],"o":[[0,0],[-1.914,0],[0,0],[0,1.914],[0,0],[1.914,0],[0,0],[0,-1.914]],"v":[[18.313,-21.779],[-18.312,-21.779],[-21.779,-18.313],[-21.779,18.314],[-18.312,21.779],[18.313,21.779],[21.779,18.314],[21.779,-18.313]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":5,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":14.99,"s":[100]},{"t":24.9800010174563,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[22.028,22.029],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":47.0000019143492,"st":0,"bm":0},{"ddd":0,"ind":3,"ty":4,"nm":"Triangle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[116.25,200.703,0],"ix":2},"a":{"a":0,"k":[27.11,21.243,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[0,0],[0.558,-0.879],[0,0],[-1.133,0],[0,0],[0.609,0.947],[0,0]],"o":[[-0.558,-0.879],[0,0],[-0.609,0.947],[0,0],[1.133,0],[0,0],[0,0]],"v":[[1.209,-20.114],[-1.192,-20.114],[-26.251,18.795],[-25.051,20.993],[25.051,20.993],[26.251,18.795],[1.192,-20.114]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":0,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":9.99,"s":[100]},{"t":19.9800008138021,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[27.11,21.243],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0}],"markers":[]}
                            })
                            </script>
                            "#))


                        h3 style="text-align: center;" { "Please wait while we process your payment..." }
                    }

                // This is the iframe recommended by cybersource but the redirection happens inside this iframe once otp
                // is received and we lose control of the redirection on user client browser, so to avoid that we have removed this iframe and directly consumed it.
                // (PreEscaped(r#"<iframe id="step_up_iframe" style="border: none; margin-left: auto; margin-right: auto; display: block" height="800px" width="400px" name="stepUpIframe"></iframe>"#))
                (PreEscaped(format!("<form id=\"step-up-form\" method=\"POST\" action=\"{step_up_url}\">
                <input type=\"hidden\" name=\"JWT\" value=\"{access_token}\">
              </form>")))
              (PreEscaped(r#"<script>
              window.onload = function() {
              var stepUpForm = document.querySelector('#step-up-form'); if(stepUpForm) stepUpForm.submit();
              }
              </script>"#))
            }}
        }
        RedirectForm::Payme => {
            maud::html! {
                (maud::DOCTYPE)
                head {
                    (PreEscaped(r#"<script src="https://cdn.paymeservice.com/hf/v1/hostedfields.js"></script>"#))
                }
                (PreEscaped("<script>
                    var f = document.createElement('form');
                    f.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/complete/payme\");
                    f.method='POST';
                    PayMe.clientData()
                    .then((data) => {{
                        var i=document.createElement('input');
                        i.type='hidden';
                        i.name='meta_data';
                        i.value=data.hash;
                        f.appendChild(i);
                        document.body.appendChild(f);
                        f.submit();
                    }})
                    .catch((error) => {{
                        f.submit();
                    }});
            </script>
                ".to_string()))
            }
        }
        RedirectForm::Braintree {
            client_token,
            card_token,
            bin,
        } => {
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    (PreEscaped(r#"<script src="https://js.braintreegateway.com/web/3.97.1/js/three-d-secure.js"></script>"#))
                    // (PreEscaped(r#"<script src="https://js.braintreegateway.com/web/3.97.1/js/hosted-fields.js"></script>"#))

                }
                    body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                        div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-top: 150px; margin-left: auto; margin-right: auto;" { "" }

                        (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                        (PreEscaped(r#"
                            <script>
                            var anime = bodymovin.loadAnimation({
                                container: document.getElementById('loader1'),
                                renderer: 'svg',
                                loop: true,
                                autoplay: true,
                                name: 'hyperswitch loader',
                                animationData: {"v":"4.8.0","meta":{"g":"LottieFiles AE 3.1.1","a":"","k":"","d":"","tc":""},"fr":29.9700012207031,"ip":0,"op":31.0000012626559,"w":400,"h":250,"nm":"loader_shape","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"circle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[278.25,202.671,0],"ix":2},"a":{"a":0,"k":[23.72,23.72,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[12.935,0],[0,-12.936],[-12.935,0],[0,12.935]],"o":[[-12.952,0],[0,12.935],[12.935,0],[0,-12.936]],"v":[[0,-23.471],[-23.47,0.001],[0,23.471],[23.47,0.001]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":10,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":19.99,"s":[100]},{"t":29.9800012211104,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[23.72,23.721],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0},{"ddd":0,"ind":2,"ty":4,"nm":"square 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[196.25,201.271,0],"ix":2},"a":{"a":0,"k":[22.028,22.03,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[1.914,0],[0,0],[0,-1.914],[0,0],[-1.914,0],[0,0],[0,1.914],[0,0]],"o":[[0,0],[-1.914,0],[0,0],[0,1.914],[0,0],[1.914,0],[0,0],[0,-1.914]],"v":[[18.313,-21.779],[-18.312,-21.779],[-21.779,-18.313],[-21.779,18.314],[-18.312,21.779],[18.313,21.779],[21.779,18.314],[21.779,-18.313]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":5,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":14.99,"s":[100]},{"t":24.9800010174563,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[22.028,22.029],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":47.0000019143492,"st":0,"bm":0},{"ddd":0,"ind":3,"ty":4,"nm":"Triangle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[116.25,200.703,0],"ix":2},"a":{"a":0,"k":[27.11,21.243,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[0,0],[0.558,-0.879],[0,0],[-1.133,0],[0,0],[0.609,0.947],[0,0]],"o":[[-0.558,-0.879],[0,0],[-0.609,0.947],[0,0],[1.133,0],[0,0],[0,0]],"v":[[1.209,-20.114],[-1.192,-20.114],[-26.251,18.795],[-25.051,20.993],[25.051,20.993],[26.251,18.795],[1.192,-20.114]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":0,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":9.99,"s":[100]},{"t":19.9800008138021,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[27.11,21.243],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0}],"markers":[]}
                            })
                            </script>
                            "#))


                        h3 style="text-align: center;" { "Please wait while we process your payment..." }
                    }

                    (PreEscaped(format!("<script>
                                var my3DSContainer;
                                var clientToken = \"{client_token}\";
                                braintree.threeDSecure.create({{
                                        authorization: clientToken,
                                        version: 2
                                    }}, function(err, threeDs) {{
                                        threeDs.verifyCard({{
                                            amount: \"{amount}\",
                                            nonce: \"{card_token}\",
                                            bin: \"{bin}\",
                                            addFrame: function(err, iframe) {{
                                                my3DSContainer = document.createElement('div');
                                                my3DSContainer.appendChild(iframe);
                                                document.body.appendChild(my3DSContainer);
                                            }},
                                            removeFrame: function() {{
                                                if(my3DSContainer && my3DSContainer.parentNode) {{
                                                    my3DSContainer.parentNode.removeChild(my3DSContainer);
                                                }}
                                            }},
                                            onLookupComplete: function(data, next) {{
                                                // console.log(\"onLookup Complete\", data);
                                                    next();
                                                }}
                                            }},
                                            function(err, payload) {{
                                                if(err) {{
                                                    console.error(err);
                                                    var f = document.createElement('form');
                                                    f.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/response/braintree\");
                                                    var i = document.createElement('input');
                                                    i.type = 'hidden';
                                                    f.method='POST';
                                                    i.name = 'authentication_response';
                                                    i.value = JSON.stringify(err);
                                                    f.appendChild(i);
                                                    f.body = JSON.stringify(err);
                                                    document.body.appendChild(f);
                                                    f.submit();
                                                }} else {{
                                                    // console.log(payload);
                                                    var f = document.createElement('form');
                                                    f.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/complete/braintree\");
                                                    var i = document.createElement('input');
                                                    i.type = 'hidden';
                                                    f.method='POST';
                                                    i.name = 'authentication_response';
                                                    i.value = JSON.stringify(payload);
                                                    f.appendChild(i);
                                                    f.body = JSON.stringify(payload);
                                                    document.body.appendChild(f);
                                                    f.submit();
                                                    }}
                                                }});
                                        }}); </script>"
                                    )))
                }}
        }
        RedirectForm::Nmi {
            amount,
            currency,
            public_key,
            customer_vault_id,
            order_id,
        } => {
            let public_key_val = public_key.peek();
            maud::html! {
                    (maud::DOCTYPE)
                    head {
                        (PreEscaped(r#"<script src="https://secure.networkmerchants.com/js/v1/Gateway.js"></script>"#))
                    }
                    body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                        div id="loader-wrapper" {
                            div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-top: 150px; margin-left: auto; margin-right: auto;" { "" }

                        (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                        (PreEscaped(r#"
                            <script>
                            var anime = bodymovin.loadAnimation({
                                container: document.getElementById('loader1'),
                                renderer: 'svg',
                                loop: true,
                                autoplay: true,
                                name: 'hyperswitch loader',
                                animationData: {"v":"4.8.0","meta":{"g":"LottieFiles AE 3.1.1","a":"","k":"","d":"","tc":""},"fr":29.9700012207031,"ip":0,"op":31.0000012626559,"w":400,"h":250,"nm":"loader_shape","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"circle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[278.25,202.671,0],"ix":2},"a":{"a":0,"k":[23.72,23.72,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[12.935,0],[0,-12.936],[-12.935,0],[0,12.935]],"o":[[-12.952,0],[0,12.935],[12.935,0],[0,-12.936]],"v":[[0,-23.471],[-23.47,0.001],[0,23.471],[23.47,0.001]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":10,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":19.99,"s":[100]},{"t":29.9800012211104,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[23.72,23.721],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0},{"ddd":0,"ind":2,"ty":4,"nm":"square 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[196.25,201.271,0],"ix":2},"a":{"a":0,"k":[22.028,22.03,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[1.914,0],[0,0],[0,-1.914],[0,0],[-1.914,0],[0,0],[0,1.914],[0,0]],"o":[[0,0],[-1.914,0],[0,0],[0,1.914],[0,0],[1.914,0],[0,0],[0,-1.914]],"v":[[18.313,-21.779],[-18.312,-21.779],[-21.779,-18.313],[-21.779,18.314],[-18.312,21.779],[18.313,21.779],[21.779,18.314],[21.779,-18.313]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":5,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":14.99,"s":[100]},{"t":24.9800010174563,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[22.028,22.029],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":47.0000019143492,"st":0,"bm":0},{"ddd":0,"ind":3,"ty":4,"nm":"Triangle 2","sr":1,"ks":{"o":{"a":0,"k":100,"ix":11},"r":{"a":0,"k":0,"ix":10},"p":{"a":0,"k":[116.25,200.703,0],"ix":2},"a":{"a":0,"k":[27.11,21.243,0],"ix":1},"s":{"a":0,"k":[100,100,100],"ix":6}},"ao":0,"shapes":[{"ty":"gr","it":[{"ind":0,"ty":"sh","ix":1,"ks":{"a":0,"k":{"i":[[0,0],[0.558,-0.879],[0,0],[-1.133,0],[0,0],[0.609,0.947],[0,0]],"o":[[-0.558,-0.879],[0,0],[-0.609,0.947],[0,0],[1.133,0],[0,0],[0,0]],"v":[[1.209,-20.114],[-1.192,-20.114],[-26.251,18.795],[-25.051,20.993],[25.051,20.993],[26.251,18.795],[1.192,-20.114]],"c":true},"ix":2},"nm":"Path 1","mn":"ADBE Vector Shape - Group","hd":false},{"ty":"fl","c":{"a":0,"k":[0,0.427451010311,0.976470648074,1],"ix":4},"o":{"a":1,"k":[{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":0,"s":[10]},{"i":{"x":[0.667],"y":[1]},"o":{"x":[0.333],"y":[0]},"t":9.99,"s":[100]},{"t":19.9800008138021,"s":[10]}],"ix":5},"r":1,"bm":0,"nm":"Fill 1","mn":"ADBE Vector Graphic - Fill","hd":false},{"ty":"tr","p":{"a":0,"k":[27.11,21.243],"ix":2},"a":{"a":0,"k":[0,0],"ix":1},"s":{"a":0,"k":[100,100],"ix":3},"r":{"a":0,"k":0,"ix":6},"o":{"a":0,"k":100,"ix":7},"sk":{"a":0,"k":0,"ix":4},"sa":{"a":0,"k":0,"ix":5},"nm":"Transform"}],"nm":"Group 1","np":2,"cix":2,"bm":0,"ix":1,"mn":"ADBE Vector Group","hd":false}],"ip":0,"op":48.0000019550801,"st":0,"bm":0}],"markers":[]}
                            })
                            </script>
                            "#))

                        h3 style="text-align: center;" { "Please wait while we process your payment..." }
                        }

                        div id="threeds-wrapper" style="display: flex; width: 100%; height: 100vh; align-items: center; justify-content: center;" {""}
                    }
                    (PreEscaped(format!("<script>
                    const gateway = Gateway.create('{public_key_val}');

                    // Initialize the ThreeDSService
                    const threeDS = gateway.get3DSecure();

                    const options = {{
                        customerVaultId: '{customer_vault_id}',
                        currency: '{currency}',
                        amount: '{amount}'
                    }};

                    var responseForm = document.createElement('form');
                    responseForm.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/complete/nmi\");
                    responseForm.method='POST';

                    const threeDSsecureInterface = threeDS.createUI(options);

                    threeDSsecureInterface.on('challenge', function(e) {{
                        console.log('Challenged');
                        document.getElementById('loader-wrapper').style.display = 'none';
                    }});

                    threeDSsecureInterface.on('complete', function(e) {{

                        var item1=document.createElement('input');
                        item1.type='hidden';
                        item1.name='cavv';
                        item1.value=e.cavv;
                        responseForm.appendChild(item1);

                        var item2=document.createElement('input');
                        item2.type='hidden';
                        item2.name='xid';
                        item2.value=e.xid;
                        responseForm.appendChild(item2);

                        var item6=document.createElement('input');
                        item6.type='hidden';
                        item6.name='eci';
                        item6.value=e.eci;
                        responseForm.appendChild(item6);

                        var item7=document.createElement('input');
                        item7.type='hidden';
                        item7.name='directoryServerId';
                        item7.value=e.directoryServerId;
                        responseForm.appendChild(item7);

                        var item3=document.createElement('input');
                        item3.type='hidden';
                        item3.name='cardHolderAuth';
                        item3.value=e.cardHolderAuth;
                        responseForm.appendChild(item3);

                        var item4=document.createElement('input');
                        item4.type='hidden';
                        item4.name='threeDsVersion';
                        item4.value=e.threeDsVersion;
                        responseForm.appendChild(item4);

                        var item5=document.createElement('input');
                        item5.type='hidden';
                        item5.name='orderId';
                        item5.value='{order_id}';
                        responseForm.appendChild(item5);

                        var item6=document.createElement('input');
                        item6.type='hidden';
                        item6.name='customerVaultId';
                        item6.value='{customer_vault_id}';
                        responseForm.appendChild(item6);

                        document.body.appendChild(responseForm);
                        responseForm.submit();
                    }});

                    threeDSsecureInterface.on('failure', function(e) {{
                        responseForm.submit();
                    }});

                    threeDSsecureInterface.start('#threeds-wrapper');
            </script>"
            )))
                }
        }
    }
}

pub fn build_payment_link_html(
    payment_link_data: PaymentLinkFormData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let mut tera = Tera::default();

    // Add modification to css template with dynamic data
    let css_template =
        include_str!("../core/payment_link/payment_link_initiate/payment_link.css").to_string();
    let _ = tera.add_raw_template("payment_link_css", &css_template);
    let mut context = Context::new();
    context.insert("css_color_scheme", &payment_link_data.css_script);

    let rendered_css = match tera.render("payment_link_css", &context) {
        Ok(rendered_css) => rendered_css,
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };

    // Add modification to js template with dynamic data
    let js_template =
        include_str!("../core/payment_link/payment_link_initiate/payment_link.js").to_string();
    let _ = tera.add_raw_template("payment_link_js", &js_template);

    context.insert("payment_details_js_script", &payment_link_data.js_script);

    let rendered_js = match tera.render("payment_link_js", &context) {
        Ok(rendered_js) => rendered_js,
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };

    // Modify Html template with rendered js and rendered css files
    let html_template =
        include_str!("../core/payment_link/payment_link_initiate/payment_link.html").to_string();

    let _ = tera.add_raw_template("payment_link", &html_template);

    context.insert(
        "hyperloader_sdk_link",
        &get_hyper_loader_sdk(&payment_link_data.sdk_url),
    );
    context.insert("rendered_css", &rendered_css);
    context.insert("rendered_js", &rendered_js);

    match tera.render("payment_link", &context) {
        Ok(rendered_html) => Ok(rendered_html),
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
}

fn get_hyper_loader_sdk(sdk_url: &str) -> String {
    format!("<script src=\"{sdk_url}\" onload=\"initializeSDK()\"></script>")
}

pub fn get_payment_link_status(
    payment_link_data: PaymentLinkStatusData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let mut tera = Tera::default();

    // Add modification to css template with dynamic data
    let css_template =
        include_str!("../core/payment_link/payment_link_status/status.css").to_string();
    let _ = tera.add_raw_template("payment_link_css", &css_template);
    let mut context = Context::new();
    context.insert("css_color_scheme", &payment_link_data.css_script);

    let rendered_css = match tera.render("payment_link_css", &context) {
        Ok(rendered_css) => rendered_css,
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };

    // Add modification to js template with dynamic data
    let js_template =
        include_str!("../core/payment_link/payment_link_status/status.js").to_string();
    let _ = tera.add_raw_template("payment_link_js", &js_template);
    context.insert("payment_details_js_script", &payment_link_data.js_script);

    let rendered_js = match tera.render("payment_link_js", &context) {
        Ok(rendered_js) => rendered_js,
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    };

    // Modify Html template with rendered js and rendered css files
    let html_template =
        include_str!("../core/payment_link/payment_link_status/status.html").to_string();
    let _ = tera.add_raw_template("payment_link_status", &html_template);

    context.insert("rendered_css", &rendered_css);

    context.insert("rendered_js", &rendered_js);

    match tera.render("payment_link_status", &context) {
        Ok(rendered_html) => Ok(rendered_html),
        Err(tera_error) => {
            crate::logger::warn!("{tera_error}");
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mime_essence() {
        assert_eq!(mime::APPLICATION_JSON.essence_str(), "application/json");
    }
}
