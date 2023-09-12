mod client;
pub mod request;

use std::{
    collections::HashMap,
    error::Error,
    fmt::Debug,
    future::Future,
    str,
    time::{Duration, Instant},
};

use actix_web::{body, HttpRequest, HttpResponse, Responder, ResponseError};
use api_models::enums::CaptureMethod;
pub use client::{proxy_bypass_urls, ApiClient, MockApiClient, ProxyClient};
use common_utils::errors::ReportSwitchExt;
use error_stack::{report, IntoReport, Report, ResultExt};
use masking::{ExposeOptionInterface, PeekInterface};
use router_env::{instrument, tracing, Tag};
use serde::Serialize;
use serde_json::json;

use self::request::{ContentType, HeaderExt, RequestBuilderExt};
pub use self::request::{Method, Request, RequestBuilder};
use crate::{
    configs::settings::Connectors,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    logger,
    routes::{
        app::AppStateInfo,
        metrics::{self, request as metrics_request},
        AppState,
    },
    services::authentication as auth,
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        Ok(None)
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
        _res: types::Response,
    ) -> CustomResult<types::RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        T: Clone,
        Req: Clone,
        Resp: Clone,
    {
        Ok(data.clone())
    }

    fn get_error_response(
        &self,
        _res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Ok(ErrorResponse::get_not_implemented())
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
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
#[instrument(skip_all)]
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
    let mut router_data = req.clone();
    match call_connector_action {
        payments::CallConnectorAction::HandleResponse(res) => {
            let response = types::Response {
                headers: None,
                response: res.into(),
                status_code: 200,
            };

            connector_integration.handle_response(req, response)
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

            let connector_request = connector_request.or(connector_integration
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
                })?);

            match connector_request {
                Some(request) => {
                    logger::debug!(connector_request=?request);
                    let response = call_connector_api(state, request).await;
                    logger::debug!(connector_response=?response);
                    match response {
                        Ok(body) => {
                            let response = match body {
                                Ok(body) => {
                                    let connector_http_status_code = Some(body.status_code);
                                    let mut data = connector_integration
                                        .handle_response(req, body)
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
                                        })?;
                                    data.connector_http_status_code = connector_http_status_code;
                                    data
                                }
                                Err(body) => {
                                    router_data.connector_http_status_code = Some(body.status_code);
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
                                            connector_integration.get_5xx_error_response(body)?
                                        }
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
                                let error_response = ErrorResponse {
                                    code: consts::REQUEST_TIMEOUT_ERROR_CODE.to_string(),
                                    message: consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string(),
                                    reason: Some(consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string()),
                                    status_code: 504,
                                };
                                router_data.response = Err(error_response);
                                router_data.connector_http_status_code = Some(504);
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

    let response = send_request(state, request, None).await;

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
    logger::debug!(method=?request.method, headers=?request.headers, payload=?request.payload, ?request);

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

    let send_request = async {
        match request.method {
            Method::Get => client.get(url),
            Method::Post => {
                let client = client.post(url);
                match request.content_type {
                    Some(ContentType::Json) => client.json(&request.payload),

                    Some(ContentType::FormData) => client.multipart(
                        request
                            .form_data
                            .unwrap_or_else(reqwest::multipart::Form::new),
                    ),

                    // Currently this is not used remove this if not required
                    // If using this then handle the serde_part
                    Some(ContentType::FormUrlEncoded) => {
                        let payload = match request.payload.clone() {
                            Some(req) => serde_json::from_str(req.peek())
                                .into_report()
                                .change_context(errors::ApiClientError::UrlEncodingFailed)?,
                            _ => json!(r#""#),
                        };
                        let url_encoded_payload = serde_urlencoded::to_string(&payload)
                            .into_report()
                            .change_context(errors::ApiClientError::UrlEncodingFailed)
                            .attach_printable_lazy(|| {
                                format!(
                                    "Unable to do url encoding on request: {:?}",
                                    &request.payload
                                )
                            })?;

                        logger::debug!(?url_encoded_payload);
                        client.body(url_encoded_payload)
                    }
                    // If payload needs processing the body cannot have default
                    None => client.body(request.payload.expose_option().unwrap_or_default()),
                }
            }

            Method::Put => client
                .put(url)
                .body(request.payload.expose_option().unwrap_or_default()), // If payload needs processing the body cannot have default
            Method::Delete => client.delete(url),
        }
        .add_headers(headers)
        .timeout(Duration::from_secs(
            option_timeout_secs.unwrap_or(crate::consts::REQUEST_TIME_OUT),
        ))
        .send()
        .await
        .map_err(|error| match error {
            error if error.is_timeout() => {
                metrics::REQUEST_BUILD_FAILURE.add(&metrics::CONTEXT, 1, &[]);
                errors::ApiClientError::RequestTimeoutReceived
            }
            error if is_connection_closed(&error) => {
                metrics::REQUEST_BUILD_FAILURE.add(&metrics::CONTEXT, 1, &[]);
                errors::ApiClientError::ConnectionClosed
            }
            _ => errors::ApiClientError::RequestNotSent(error.to_string()),
        })
        .into_report()
        .attach_printable("Unable to send request to connector")
    };

    metrics_request::record_operation_time(
        send_request,
        &metrics::EXTERNAL_REQUEST_TIME,
        &[metrics_tag],
    )
    .await
}

fn is_connection_closed(error: &reqwest::Error) -> bool {
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
    PaymenkLinkForm(Box<PaymentLinkFormData>),
    FileData((Vec<u8>, mime::Mime)),
    JsonWithHeaders((R, Vec<(String, String)>)),
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkFormData {
    pub client_secret: String,
    pub base_url: String,
    pub amount : i64,
    pub currency: String,
    pub pub_key: String

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
    Payme,
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
    state: &'b A,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn auth::AuthenticateAndFetch<U, A>,
) -> CustomResult<ApplicationResponse<Q>, OErr>
where
    F: Fn(&'b A, U, T) -> Fut,
    'b: 'a,
    Fut: Future<Output = CustomResult<ApplicationResponse<Q>, E>>,
    Q: Serialize + Debug + 'a,
    T: Debug,
    A: AppStateInfo,
    U: auth::AuthInfo,
    CustomResult<ApplicationResponse<Q>, E>: ReportSwitchExt<ApplicationResponse<Q>, OErr>,
    CustomResult<U, errors::ApiErrorResponse>: ReportSwitchExt<U, OErr>,
    OErr: ResponseError + Sync + Send + 'static,
{
    let auth_out = api_auth
        .authenticate_and_fetch(request.headers(), state)
        .await
        .switch()?;
    let merchant_id = auth_out.get_merchant_id().unwrap_or("").to_string();
    tracing::Span::current().record("merchant_id", &merchant_id);

    let output = func(state, auth_out, payload).await.switch();

    let status_code = match output.as_ref() {
        Ok(res) => metrics::request::track_response_status_code(res),
        Err(err) => err.current_context().status_code().as_u16().into(),
    };

    metrics::request::status_code_metrics(status_code, flow.to_string(), merchant_id.to_string());

    output
}

#[instrument(
    skip(request, state, func, api_auth, payload),
    fields(request_method, request_url_path)
)]
pub async fn server_wrap<'a, 'b, A, T, U, Q, F, Fut, E>(
    flow: impl router_env::types::FlowMetric,
    state: &'b A,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn auth::AuthenticateAndFetch<U, A>,
) -> HttpResponse
where
    F: Fn(&'b A, U, T) -> Fut,
    Fut: Future<Output = CustomResult<ApplicationResponse<Q>, E>>,
    Q: Serialize + Debug + 'a,
    T: Debug,
    U: auth::AuthInfo,
    A: AppStateInfo,
    ApplicationResponse<Q>: Debug,
    CustomResult<ApplicationResponse<Q>, E>:
        ReportSwitchExt<ApplicationResponse<Q>, api_models::errors::types::ApiErrorResponse>,
{
    let request_method = request.method().as_str();
    let url_path = request.path();
    tracing::Span::current().record("request_method", request_method);
    tracing::Span::current().record("request_url_path", url_path);

    let start_instant = Instant::now();
    logger::info!(tag = ?Tag::BeginRequest, payload = ?payload);

    let res = match metrics::request::record_request_time_metric(
        server_wrap_util(&flow, state, request, payload, func, api_auth),
        &flow,
    )
    .await
    .map(|response| {
        logger::info!(api_response =? response);
        response
    }) {
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
        Ok(ApplicationResponse::Form(redirection_data)) => build_redirection_form(
            &redirection_data.redirect_form,
            redirection_data.payment_method_data,
            redirection_data.amount,
            redirection_data.currency,
        )
        .respond_to(request)
        .map_into_boxed_body(),

        Ok(ApplicationResponse::PaymenkLinkForm(payment_link_data)) => build_payment_link_html(
            *payment_link_data
        )
        .respond_to(request)
        .map_into_boxed_body(),

        Ok(ApplicationResponse::JsonWithHeaders((response, headers))) => {
            match serde_json::to_string(&response) {
                Ok(res) => http_response_json_with_headers(res, headers),
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
    let end_instant = Instant::now();
    let request_duration = end_instant.saturating_duration_since(start_instant);
    logger::info!(
        tag = ?Tag::EndRequest,
        status_code = response_code,
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

pub fn http_response_json_with_headers<T: body::MessageBody + 'static>(
    response: T,
    headers: Vec<(String, String)>,
) -> HttpResponse {
    let mut response_builder = HttpResponse::Ok();
    for (name, value) in headers {
        response_builder.append_header((name, value));
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
impl Authenticate for api_models::payments::PaymentsStartRequest {}

pub fn build_redirection_form(
    form: &RedirectForm,
    payment_method_data: Option<api_models::payments::PaymentMethodData>,
    amount: String,
    currency: String,
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
            let card_details = if let Some(api::PaymentMethodData::Card(ccard)) =
                payment_method_data
            {
                format!(
                    "var newCard={{ccNumber: \"{}\",cvv: \"{}\",expDate: \"{}/{}\",amount: {},currency: \"{}\"}};",
                    ccard.card_number.peek(),
                    ccard.card_cvc.peek(),
                    ccard.card_exp_month.peek(),
                    ccard.card_exp_year.peek(),
                    amount,
                    currency
                )
            } else {
                "".to_string()
            };
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    (PreEscaped(r#"<script src="https://sandpay.bluesnap.com/web-sdk/5/bluesnap.js"></script>"#))
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
                        console.log(sdkResponse);
                        var f = document.createElement('form');
                        f.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/complete/bluesnap\");
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
                    bluesnap.threeDsPaymentsSubmitData(newCard);
                </script>
                ")))
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
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mime_essence() {
        assert_eq!(mime::APPLICATION_JSON.essence_str(), "application/json");
    }
}


pub fn build_payment_link_html(
    payment_link_data: PaymentLinkFormData,
) -> maud::Markup {
    use maud::PreEscaped;
    maud::html!{
        (maud::DOCTYPE)
        html {
            head {
                (PreEscaped(r#"<script src="https://beta.hyperswitch.io/v1/HyperLoader.js"></script>"#))
                (PreEscaped(r##"
                <style>
                html, body {
                    height: 100%;
                }
                
                body {
                    display: flex;
                    flex-flow: column;
                    align-items: center;
                    justify-content: flex-start;
                    margin: 0;
                    background-color: #fafafa;
                    color: #292929;
                }
                
                .hidden {
                    display: none !important;
                }
                
                .hyper-checkout {
                    display: flex;
                    background-color: #fafafa;
                    margin-top: 50px;
                }
                
                .main {
                    padding: 15px 15px 15px 25px;
                    display: flex;
                    flex-flow: column;
                    background-color: #fdfdfd;
                    margin: 20px 0;
                    box-shadow: 0px 1px 10px #f2f2f2;
                    width: 500px;
                }
                
                .hyper-checkout-details-header {
                    font-weight: 600;
                    font-size: 23px;
                    font-family: "Montserrat";
                }
                
                .hyper-checkout-item {
                    margin-top: 20px;
                }
                
                .hyper-checkout-item-header {
                    font-family: "Montserrat";
                    font-weight: 500;
                    font-size: 12px;
                    color: #53655c;
                }
                
                .hyper-checkout-item-value {
                    margin-top: 2px;
                    font-family: "Montserrat";
                    font-weight: 500;
                    font-size: 18px;
                }
                
                .hyper-checkout-item-amount {
                    font-weight: 600;
                    font-size: 23px;
                }
                
                .hyper-checkout-sdk {
                    z-index: 2;
                    background-color: #fdfdfd;
                    margin: 20px 30px 20px 0;
                    box-shadow: 0px 1px 10px #f2f2f2;
                }
                
                #hyper-checkout-sdk-header {
                    padding: 10px 10px 10px 22px;
                    display: flex;
                    align-items: flex-start;
                    justify-content: flex-start;
                    border-bottom: 1px solid #f2f2f2;
                }
                
                .hyper-checkout-sdk-header-logo {
                    height: 60px;
                    width: 60px;
                    background-color: white;
                    border-radius: 2px;
                }
                
                .hyper-checkout-sdk-header-logo>img {
                    height: 56px;
                    width: 56px;
                    margin: 2px;
                }
                
                .hyper-checkout-sdk-header-items {
                    display: flex;
                    flex-flow: column;
                    color: white;
                    font-size: 20px;
                    font-weight: 700;
                }
                
                .hyper-checkout-sdk-items {
                    margin-left: 10px;
                }
                
                .hyper-checkout-sdk-header-brand-name,
                .hyper-checkout-sdk-header-amount {
                    font-size: 18px;
                    font-weight: 600;
                    display: flex;
                    align-items: center;
                    font-family: "Montserrat";
                    justify-self: flex-start;
                }
                
                .hyper-checkout-sdk-header-amount {
                    font-weight: 800;
                    font-size: 25px;
                }
                
                .payNow {
                    margin-top: 10px;
                }
                
                .checkoutButton {
                    height: 48px;
                    border-radius: 25px;
                    width: 100%;
                    border: transparent;
                    background: #006df9;
                    color: #ffffff;
                    font-weight: 600;
                    cursor: pointer;
                }
                
                .page-spinner,
                .page-spinner::before,
                .page-spinner::after,
                .spinner,
                .spinner:before,
                .spinner:after {
                    border-radius: 50%;
                }
                
                .page-spinner,
                .spinner {
                    color: #ffffff;
                    font-size: 22px;
                    text-indent: -99999px;
                    margin: 0px auto;
                    position: relative;
                    width: 20px;
                    height: 20px;
                    box-shadow: inset 0 0 0 2px;
                    -webkit-transform: translateZ(0);
                    -ms-transform: translateZ(0);
                    transform: translateZ(0);
                }
                
                .page-spinner::before,
                .page-spinner::after,
                .spinner:before,
                .spinner:after {
                    position: absolute;
                    content: "";
                }
                
                .page-spinner {
                    color: #006df9 !important;
                    height: 50px !important;
                    width: 50px !important;
                    box-shadow: inset 0 0 0 4px !important;
                    margin: auto !important;
                
                }
                
                #hyper-checkout-status {
                    margin: 40px !important;
                }
                
                .hyper-checkout-status-header {
                    display: flex;
                    align-items: center;
                    font-family: "Montserrat";
                    font-size: 24px;
                    font-weight: 600;
                }
                
                #status-img {
                    height: 70px;
                }
                
                #status-date {
                    font-size: 13px;
                    font-weight: 500;
                    color: #53655c;
                }
                
                #status-details {
                    margin-left: 10px;
                    justify-content: center;
                    display: flex;
                    flex-flow: column;
                }
                
                @keyframes loading {
                    0% {
                    -webkit-transform: rotate(0deg);
                    transform: rotate(0deg);
                    }
                
                    100% {
                    -webkit-transform: rotate(360deg);
                    transform: rotate(360deg);
                    }
                }
                
                .spinner:before {
                    width: 10.4px;
                    height: 20.4px;
                    background: #016df9;
                    border-radius: 20.4px 0 0 20.4px;
                    top: -0.2px;
                    left: -0.2px;
                    -webkit-transform-origin: 10.4px 10.2px;
                    transform-origin: 10.4px 10.2px;
                    -webkit-animation: loading 2s infinite ease 1.5s;
                    animation: loading 2s infinite ease 1.5s;
                }
                
                #payment-message {
                    font-size: 12px;
                    font-weight: 500;
                    padding: 2%;
                    color: #ff0000;
                    font-family: "Montserrat";
                }
                
                .spinner:after {
                    width: 10.4px;
                    height: 10.2px;
                    background: #016df9;
                    border-radius: 0 10.2px 10.2px 0;
                    top: -0.1px;
                    left: 10.2px;
                    -webkit-transform-origin: 0px 10.2px;
                    transform-origin: 0px 10.2px;
                    -webkit-animation: loading 2s infinite ease;
                    animation: loading 2s infinite ease;
                }
                
                #payment-form-wrap {
                    margin: 30px;
                }
                
                #payment-form-wrap {
                    margin: 30px;
                }
                
                #payment-form {
                    max-width: 560px;
                    width: 100%;
                    margin: 0 auto;
                    text-align: center;
                }
                
                @media only screen and (max-width: 765px) {
                    .checkoutButton {
                    width: 95%;
                    }
                
                    .hyper-checkout {
                    flex-flow: column;
                    margin: 0;
                    flex-direction: column-reverse;
                    }
                
                    .main {
                    width: auto;
                    }
                
                    .hyper-checkout-sdk {
                    margin: 0;
                    }
                
                    #hyper-checkout-status {
                    padding: 15px;
                    }
                
                    #status-img {
                    height: 60px;
                    }
                
                    #status-text {
                    font-size: 19px;
                    }
                
                    #status-date {
                    font-size: 12px;
                    }
                
                    .hyper-checkout-item-header {
                    font-size: 11px;
                    }
                
                    .hyper-checkout-item-value {
                    font-size: 17px;
                    }
                }
                
                </style>

                "##))
                link rel = "stylesheet" href = "https://fonts.googleapis.com/css2?family=Mukta:wght@400;500;600;700;800";
            }
    (PreEscaped(r##" <body onload="showSDK()">
    <div class="page-spinner hidden" id="page-spinner"></div>
    <div class="hyper-checkout">
      <div class="main hidden" id="hyper-checkout-status">
        <div class="hyper-checkout-status-header">
          <img id="status-img" />
          <div id="status-details">
            <div id="status-text"></div>
            <div id="status-date"></div>
          </div>
        </div>
        <div id="hyper-checkout-status-items"></div>
      </div>
      <div class="main hidden" id="hyper-checkout-details"></div>
      <div class="hyper-checkout-sdk hidden" id="hyper-checkout-sdk">
        <div id="hyper-checkout-sdk-header"></div>
        <div id="payment-form-wrap">
          <form id="payment-form" onsubmit="handleSubmit(); return false;">
            <div id="unified-checkout">
              <!--HyperLoader injects the Unified Checkout-->
            </div>
            <button id="submit" class="checkoutButton payNow">
              <div class="spinner hidden" id="spinner"></div>
              <span id="button-text">Pay now</span>
            </button>
            <div id="payment-message" class="hidden"></div>
          </form>
        </div>
      </div>
    </div>
    <div id="hyper-footer" class="hidden">
      <svg class="fill-current " height="18px" width="130px" transform="">
        <path opacity="0.4"
          d="M0.791016 11.7578H1.64062V9.16992H1.71875C2.00684 9.73145 2.63672 10.0928 3.35938 10.0928C4.69727 10.0928 5.56641 9.02344 5.56641 7.37305V7.36328C5.56641 5.72266 4.69238 4.64355 3.35938 4.64355C2.62695 4.64355 2.04102 4.99023 1.71875 5.57617H1.64062V4.73633H0.791016V11.7578ZM3.16406 9.34082C2.20703 9.34082 1.62109 8.58887 1.62109 7.37305V7.36328C1.62109 6.14746 2.20703 5.39551 3.16406 5.39551C4.12598 5.39551 4.69727 6.1377 4.69727 7.36328V7.37305C4.69727 8.59863 4.12598 9.34082 3.16406 9.34082ZM8.85762 10.0928C10.3566 10.0928 11.2844 9.05762 11.2844 7.37305V7.36328C11.2844 5.67383 10.3566 4.64355 8.85762 4.64355C7.35859 4.64355 6.43086 5.67383 6.43086 7.36328V7.37305C6.43086 9.05762 7.35859 10.0928 8.85762 10.0928ZM8.85762 9.34082C7.86152 9.34082 7.3 8.61328 7.3 7.37305V7.36328C7.3 6.11816 7.86152 5.39551 8.85762 5.39551C9.85371 5.39551 10.4152 6.11816 10.4152 7.36328V7.37305C10.4152 8.61328 9.85371 9.34082 8.85762 9.34082ZM13.223 10H14.0727L15.2445 5.92773H15.3227L16.4994 10H17.3539L18.8285 4.73633H17.9838L16.9486 8.94531H16.8705L15.6938 4.73633H14.8881L13.7113 8.94531H13.6332L12.598 4.73633H11.7484L13.223 10ZM21.7047 10.0928C22.9449 10.0928 23.6969 9.38965 23.8775 8.67676L23.8873 8.6377H23.0377L23.0182 8.68164C22.8766 8.99902 22.4371 9.33594 21.7242 9.33594C20.7867 9.33594 20.1861 8.70117 20.1617 7.6123H23.9508V7.28027C23.9508 5.70801 23.0816 4.64355 21.651 4.64355C20.2203 4.64355 19.2926 5.75684 19.2926 7.38281V7.3877C19.2926 9.03809 20.2008 10.0928 21.7047 10.0928ZM21.6461 5.40039C22.4225 5.40039 22.9986 5.89355 23.0865 6.93359H20.1764C20.2691 5.93262 20.8648 5.40039 21.6461 5.40039ZM25.0691 10H25.9188V6.73828C25.9188 5.9668 26.4949 5.4541 27.3055 5.4541C27.491 5.4541 27.6521 5.47363 27.8279 5.50293V4.67773C27.7449 4.66309 27.5643 4.64355 27.4031 4.64355C26.6902 4.64355 26.1971 4.96582 25.9969 5.51758H25.9188V4.73633H25.0691V10ZM30.6797 10.0928C31.9199 10.0928 32.6719 9.38965 32.8525 8.67676L32.8623 8.6377H32.0127L31.9932 8.68164C31.8516 8.99902 31.4121 9.33594 30.6992 9.33594C29.7617 9.33594 29.1611 8.70117 29.1367 7.6123H32.9258V7.28027C32.9258 5.70801 32.0566 4.64355 30.626 4.64355C29.1953 4.64355 28.2676 5.75684 28.2676 7.38281V7.3877C28.2676 9.03809 29.1758 10.0928 30.6797 10.0928ZM30.6211 5.40039C31.3975 5.40039 31.9736 5.89355 32.0615 6.93359H29.1514C29.2441 5.93262 29.8398 5.40039 30.6211 5.40039ZM35.9875 10.0928C36.7199 10.0928 37.3059 9.74609 37.6281 9.16016H37.7062V10H38.5559V2.64648H37.7062V5.56641H37.6281C37.34 5.00488 36.7102 4.64355 35.9875 4.64355C34.6496 4.64355 33.7805 5.71289 33.7805 7.36328V7.37305C33.7805 9.01367 34.6545 10.0928 35.9875 10.0928ZM36.1828 9.34082C35.2209 9.34082 34.6496 8.59863 34.6496 7.37305V7.36328C34.6496 6.1377 35.2209 5.39551 36.1828 5.39551C37.1398 5.39551 37.7258 6.14746 37.7258 7.36328V7.37305C37.7258 8.58887 37.1398 9.34082 36.1828 9.34082ZM45.2164 10.0928C46.5494 10.0928 47.4234 9.01367 47.4234 7.37305V7.36328C47.4234 5.71289 46.5543 4.64355 45.2164 4.64355C44.4938 4.64355 43.8639 5.00488 43.5758 5.56641H43.4977V2.64648H42.648V10H43.4977V9.16016H43.5758C43.898 9.74609 44.484 10.0928 45.2164 10.0928ZM45.0211 9.34082C44.0641 9.34082 43.4781 8.58887 43.4781 7.37305V7.36328C43.4781 6.14746 44.0641 5.39551 45.0211 5.39551C45.983 5.39551 46.5543 6.1377 46.5543 7.36328V7.37305C46.5543 8.59863 45.983 9.34082 45.0211 9.34082ZM48.7957 11.8457C49.7283 11.8457 50.1629 11.5039 50.5975 10.3223L52.6531 4.73633H51.7596L50.3191 9.06738H50.241L48.7957 4.73633H47.8875L49.8357 10.0049L49.7381 10.3174C49.5477 10.9229 49.2547 11.1426 48.7713 11.1426C48.6541 11.1426 48.5223 11.1377 48.4197 11.1182V11.8164C48.5369 11.8359 48.6834 11.8457 48.7957 11.8457Z"
          fill="currentColor"></path>
        <g opacity="0.6">
          <path
            d="M78.42 6.9958C78.42 9.15638 77.085 10.4444 75.2379 10.4444C74.2164 10.4444 73.3269 10.0276 72.9206 9.33816V12.9166H71.4929V3.65235H72.8018L72.9193 4.66772C73.3256 3.97825 74.189 3.5225 75.2366 3.5225C77.017 3.5225 78.4186 4.75861 78.4186 6.9971L78.42 6.9958ZM76.94 6.9958C76.94 5.62985 76.1288 4.78328 74.9492 4.78328C73.8232 4.77029 72.9598 5.62855 72.9598 7.00878C72.9598 8.38901 73.8246 9.18235 74.9492 9.18235C76.0739 9.18235 76.94 8.36304 76.94 6.9958Z"
            fill="currentColor"></path>
          <path
            d="M86.0132 7.3736H80.8809C80.9071 8.62268 81.7313 9.2732 82.7789 9.2732C83.564 9.2732 84.2197 8.90834 84.494 8.17992H85.9479C85.5939 9.53288 84.3895 10.4444 82.7528 10.4444C80.749 10.4444 79.4271 9.06545 79.4271 6.96978C79.4271 4.87412 80.749 3.50818 82.7397 3.50818C84.7305 3.50818 86.0132 4.83517 86.0132 6.83994V7.3736ZM80.894 6.38419H84.5594C84.481 5.226 83.709 4.6404 82.7397 4.6404C81.7705 4.6404 80.9985 5.226 80.894 6.38419Z"
            fill="currentColor"></path>
          <path
            d="M88.5407 3.65204C87.8745 3.65204 87.335 4.18829 87.335 4.85048V10.3156H88.7758V5.22703C88.7758 5.06213 88.9104 4.92709 89.0776 4.92709H91.2773V3.65204H88.5407Z"
            fill="currentColor"></path> -
          <path
            d="M69.1899 3.63908L67.3442 9.17039L65.3535 3.65207H63.8082L66.3606 10.2247C66.439 10.4325 66.4782 10.6026 66.4782 10.7713C66.4782 10.8635 66.469 10.9479 66.4533 11.0258L66.4494 11.0401C66.4403 11.0817 66.4298 11.1206 66.4168 11.1583L66.3201 11.5102C66.2966 11.5971 66.2169 11.6569 66.1268 11.6569H64.0956V12.9189H65.5755C66.5709 12.9189 67.3952 12.6852 67.8667 11.3829L70.6817 3.65207L69.1886 3.63908H69.1899Z"
            fill="currentColor"></path>
          <path
            d="M57 10.3144H58.4264V6.72299C58.4264 5.60375 59.0417 4.82339 60.1807 4.82339C61.1761 4.81041 61.7913 5.396 61.7913 6.68404V10.3144H63.2191V6.46201C63.2191 4.18457 61.8188 3.50809 60.5478 3.50809C59.5785 3.50809 58.8196 3.88593 58.4264 4.51047V0.919022H57V10.3144Z"
            fill="currentColor"></path>
          <path
            d="M93.1623 8.29808C93.1753 8.98755 93.8167 9.39136 94.6945 9.39136C95.5723 9.39136 96.0948 9.06545 96.0948 8.47986C96.0948 7.97218 95.8336 7.69951 95.0733 7.58135L93.7253 7.34763C92.4164 7.1269 91.9057 6.44912 91.9057 5.49997C91.9057 4.30282 93.097 3.52246 94.6161 3.52246C96.2529 3.52246 97.4442 4.30282 97.4572 5.63111H96.0439C96.0308 4.95463 95.4417 4.57679 94.6174 4.57679C93.7932 4.57679 93.3347 4.90269 93.3347 5.44933C93.3347 5.93105 93.6756 6.15178 94.4215 6.28162L95.7434 6.51534C96.987 6.73607 97.563 7.34763 97.563 8.35002C97.563 9.72895 96.2803 10.4457 94.722 10.4457C92.9546 10.4457 91.7633 9.60041 91.7372 8.29808H93.1649H93.1623Z"
            fill="currentColor"></path>
          <path
            d="M100.808 8.75352L102.327 3.652H103.82L105.313 8.75352L106.583 3.652H108.089L106.191 10.3155H104.58L103.061 5.23997L101.529 10.3155H99.9052L97.9941 3.652H99.5002L100.809 8.75352H100.808Z"
            fill="currentColor"></path>
          <path d="M108.926 0.918945H110.511V2.40305H108.926V0.918945ZM109.005 3.65214H110.431V10.3157H109.005V3.65214Z"
            fill="currentColor"></path>
          <path
            d="M119.504 4.7452C118.391 4.7452 117.632 5.55152 117.632 6.9707C117.632 8.46779 118.417 9.19621 119.465 9.19621C120.302 9.19621 120.919 8.72748 121.193 7.84325H122.712C122.371 9.45719 121.141 10.4466 119.491 10.4466C117.502 10.4466 116.165 9.06767 116.165 6.972C116.165 4.87634 117.5 3.51039 119.504 3.51039C121.141 3.51039 122.358 4.43487 122.712 6.04752H121.167C120.932 5.21523 120.289 4.7465 119.504 4.7465V4.7452Z"
            fill="currentColor"></path>
          <path
            d="M113.959 9.05208C113.875 9.05208 113.809 8.98456 113.809 8.90276V4.91399H115.367V3.65191H113.809V1.86787H112.382V3.02607C112.382 3.44287 112.252 3.65062 111.833 3.65062H111.256V4.91269H112.382V8.50414C112.382 9.66234 113.024 10.3128 114.189 10.3128H115.354V9.05078H113.96L113.959 9.05208Z"
            fill="currentColor"></path>
          <path
            d="M127.329 3.50801C126.359 3.50801 125.601 3.88585 125.207 4.5104V0.918945H123.781V10.3144H125.207V6.72292C125.207 5.60367 125.823 4.82332 126.962 4.82332C127.957 4.81033 128.572 5.39592 128.572 6.68397V10.3144H130V6.46193C130 4.18449 128.6 3.50801 127.329 3.50801Z"
            fill="currentColor"></path>
        </g>
      </svg>
    </div>
  </body>"##))
    (PreEscaped(format!(r##"
                <script>
                    window.__PAYMENT_DETAILS_STR = `{{"client_secret": "pay_zpnxSIFpqrIRa8y0PTkA_secret_c9C3MKMst7zjxYsj0jwc", "return_url":"http://localhost:5500/public/index.html","merchant_logo":"https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg","merchant":"Steam","amount":{} ,"currency":{},"purchased_item":"F1 '23","payment_id":"pay_42dfeb3a0ee"}}`;
                    const hyper = Hyper("pk_snd_bc58f95dab324ac196c7d2c15a09fbe2");
                    var widgets = null;
                    window.__PAYMENT_DETAILS = {{}};
                    try {{
                        window.__PAYMENT_DETAILS = JSON.parse(window.__PAYMENT_DETAILS_STR);
                      }} catch (error) {{
                        console.error("Failed to parse payment details");
                      }}
                    async function initialize() {{
                        var paymentDetails = window.__PAYMENT_DETAILS;
                        var client_secret = paymentDetails.client_secret;
                        const appearance = {{
                            theme: "default",
                         }};
                        widgets = hyper.widgets({{
                          appearance,
                          clientSecret: client_secret,
                        }});
                      
                        const unifiedCheckoutOptions = {{
                          layout: "tabs",
                          wallets: {{
                            walletReturnUrl: paymentDetails.return_url,
                          }},
                        }};
                        const unifiedCheckout = widgets.create("payment", unifiedCheckoutOptions);
                        unifiedCheckout.mount("#unified-checkout");
                    }}
                    initialize();
                    async function handleSubmit(e) {{
                        setLoading(true);
                        var paymentDetails = window.__PAYMENT_DETAILS;
                        const {{ error, data, status }} = await hyper.confirmPayment({{
                          widgets,
                          confirmParams: {{
                            // Make sure to change this to your payment completion page
                            return_url: paymentDetails.return_url,
                          }},
                        }});
                        // This point will only be reached if there is an immediate error occurring while confirming the payment. Otherwise, your customer will be redirected to your `return_url`.
                        // For some payment flows such as Sofort, iDEAL, your customer will be redirected to an intermediate page to complete authorization of the payment, and then redirected to the `return_url`.
                      
                        if (error) {{
                          if (error.type === "validation_error") {{
                            showMessage(error.message);
                          }} else {{
                            showMessage("An unexpected error occurred.");
                          }}
                        }} else {{
                          const {{ paymentIntent }} = await hyper.retrievePaymentIntent(paymentDetails.client_secret);
                          if (paymentIntent && paymentIntent.status) {{
                            hide("#hyper-checkout-sdk");
                            hide("#hyper-checkout-details");
                            show("#hyper-checkout-status");
                            show("#hyper-footer");
                            showStatus(paymentIntent);
                          }}
                        }}
                      
                        setLoading(false);
                      }}

                    async function checkStatus() {{
                    const clientSecret = new URLSearchParams(window.location.search).get(
                        "payment_intent_client_secret"
                    );
                    const res = {{
                        showSdk: true,
                    }};
                    
                    if (!clientSecret) {{
                        return res;
                    }}
                    
                    const {{ paymentIntent }} = await hyper.retrievePaymentIntent(clientSecret);
                    
                    if (!paymentIntent || !paymentIntent.status) {{
                        return res;
                    }}
                    
                    showStatus(paymentIntent);
                    res.showSdk = false;
                    
                    return res;
                    }}

                    function setPageLoading(showLoader) {{
                    if (showLoader) {{
                        show(".page-spinner");
                    }} else {{
                        hide(".page-spinner");
                    }}
                    }}
                    
                    function setLoading(showLoader) {{
                    if (showLoader) {{
                        show(".spinner");
                        hide("#button-text");
                    }} else {{
                        hide(".spinner");
                        show("#button-text");
                    }}
                    }}
                    
                    function show(id) {{
                    removeClass(id, "hidden");
                    }}
                    function hide(id) {{
                    addClass(id, "hidden");
                    }}

                    function showMessage(msg) {{
                        show("#payment-message");
                        addText("#payment-message", msg);
                      }}
                      function showStatus(paymentDetails) {{
                        const status = paymentDetails.status;
                        let statusDetails = {{
                          imageSource: "",
                          message: "",
                          status: status,
                          amountText: "",
                          items: [],
                        }};
                      
                        switch (status) {{
                          case "succeeded":
                            statusDetails.imageSource = "http://www.clipartbest.com/cliparts/4ib/oRa/4iboRa7RT.png";
                            statusDetails.message = "Payment successful";
                            statusDetails.status = "Succeeded";
                            statusDetails.amountText = new Date(paymentDetails.created).toTimeString();
                      
                            // Payment details
                            var amountNode = createItem("AMOUNT PAID", paymentDetails.currency + " " + paymentDetails.amount);
                            var paymentId = createItem("PAYMENT ID", paymentDetails.payment_id);
                            // @ts-ignore
                            statusDetails.items.push(amountNode, paymentId);
                            break;
                      
                          case "processing":
                            statusDetails.imageSource = "http://www.clipartbest.com/cliparts/4ib/oRa/4iboRa7RT.png";
                            statusDetails.message = "Payment in progress";
                            statusDetails.status = "Processing";
                            // Payment details
                            var amountNode = createItem("AMOUNT PAID", paymentDetails.currency + " " + paymentDetails.amount);
                            var paymentId = createItem("PAYMENT ID", paymentDetails.payment_id);
                            // @ts-ignore
                            statusDetails.items.push(amountNode, paymentId);
                            break;
                      
                          case "failed":
                            statusDetails.imageSource = "";
                            statusDetails.message = "Payment failed";
                            statusDetails.status = "Failed";
                            // Payment details
                            var amountNode = createItem("AMOUNT PAID", paymentDetails.currency + " " + paymentDetails.amount);
                            var paymentId = createItem("PAYMENT ID", paymentDetails.payment_id);
                            // @ts-ignore
                            statusDetails.items.push(amountNode, paymentId);
                            break;
                      
                          case "cancelled":
                            statusDetails.imageSource = "";
                            statusDetails.message = "Payment cancelled";
                            statusDetails.status = "Cancelled";
                            // Payment details
                            var amountNode = createItem("AMOUNT PAID", paymentDetails.currency + " " + paymentDetails.amount);
                            var paymentId = createItem("PAYMENT ID", paymentDetails.payment_id);
                            // @ts-ignore
                            statusDetails.items.push(amountNode, paymentId);
                            break;
                      
                          case "requires_merchant_action":
                            statusDetails.imageSource = "";
                            statusDetails.message = "Payment under review";
                            statusDetails.status = "Under review";
                            // Payment details
                            var amountNode = createItem("AMOUNT PAID", paymentDetails.currency + " " + paymentDetails.amount);
                            var paymentId = createItem("PAYMENT ID", paymentDetails.payment_id);
                            var paymentId = createItem("MESSAGE", "Your payment is under review by the merchant.");
                            // @ts-ignore
                            statusDetails.items.push(amountNode, paymentId);
                            break;
                      
                          default:
                            statusDetails.imageSource = "http://www.clipartbest.com/cliparts/4ib/oRa/4iboRa7RT.png";
                            statusDetails.message = "Something went wrong";
                            statusDetails.status = "Something went wrong";
                            // Error details
                            if (typeof paymentDetails.error === "object") {{
                              var errorCodeNode = createItem("ERROR CODE", paymentDetails.error.code);
                              var errorMessageNode = createItem("ERROR MESSAGE", paymentDetails.error.message);
                              // @ts-ignore
                              statusDetails.items.push(errorMessageNode, errorCodeNode);
                            }}
                            break;
                        }}
                      
                        // Append status
                        var statusTextNode = document.getElementById("status-text");
                        if (statusTextNode !== null) {{
                          statusTextNode.innerText = statusDetails.message;
                        }}
                      
                        // Append image
                        var statusImageNode = document.getElementById("status-img");
                        if (statusImageNode !== null) {{
                          statusImageNode.src = statusDetails.imageSource;
                        }}
                      
                        // Append status details
                        var statusDateNode = document.getElementById("status-date");
                        if (statusDateNode !== null) {{
                          statusDateNode.innerText = statusDetails.amountText;
                        }}
                      
                        // Append items
                        var statusItemNode = document.getElementById("hyper-checkout-status-items");
                        if (statusItemNode !== null) {{
                          statusDetails.items.map((item) => statusItemNode?.append(item));
                        }}
                      }}
                      
                      function createItem(heading, value) {{
                        var itemNode = document.createElement("div");
                        itemNode.className = "hyper-checkout-item";
                        var headerNode = document.createElement("div");
                        headerNode.className = "hyper-checkout-item-header";
                        headerNode.innerText = heading;
                        var valueNode = document.createElement("div");
                        valueNode.className = "hyper-checkout-item-value";
                        valueNode.innerText = value;
                        itemNode.append(headerNode);
                        itemNode.append(valueNode);
                        return itemNode;
                      }}
                      
                      function addText(id, msg) {{
                        var element = document.querySelector(id);
                        element.innerText = msg;
                      }}
                      
                      function addClass(id, className) {{
                        var element = document.querySelector(id);
                        element.classList.add(className);
                      }}
                      
                      function removeClass(id, className) {{
                        var element = document.querySelector(id);
                        element.classList.remove(className);
                      }}
                      
                      function renderPaymentDetails() {{
                        var paymentDetails = window.__PAYMENT_DETAILS;
                      
                        // Payment details header
                        var paymentDetailsHeaderNode = document.createElement("div");
                        paymentDetailsHeaderNode.className = "hyper-checkout-details-header";
                        paymentDetailsHeaderNode.innerText = "Payment request for " + paymentDetails.merchant;
                      
                        // Payment details
                        var purchasedItemNode = createItem("PAYMENT FOR", paymentDetails.purchased_item);
                        var paymentIdNode = createItem("PAYMENT ID", paymentDetails.payment_id);
                        var orderAmountNode = createItem("AMOUNT PAYABLE", paymentDetails.currency + " " + paymentDetails.amount);
                      
                        // Append to PaymentDetails node
                        var paymentDetailsNode = document.getElementById("hyper-checkout-details");
                        if (paymentDetailsNode !== null) {{
                          paymentDetailsNode.append(paymentDetailsHeaderNode);
                          paymentDetailsNode.append(purchasedItemNode);
                          paymentDetailsNode.append(paymentIdNode);
                          paymentDetailsNode.append(orderAmountNode);
                        }}
                      }}
                      
                      function renderSDKHeader() {{
                        var paymentDetails = window.__PAYMENT_DETAILS;
                      
                        // SDK header's logo
                        var sdkHeaderLogoNode = document.createElement("div");
                        sdkHeaderLogoNode.className = "hyper-checkout-sdk-header-logo";
                        var sdkHeaderLogoImageNode = document.createElement("img");
                        sdkHeaderLogoImageNode.src = paymentDetails.merchant_logo;
                        sdkHeaderLogoImageNode.alt = paymentDetails.merchant;
                        sdkHeaderLogoNode.append(sdkHeaderLogoImageNode);
                      
                        // SDK headers' items
                        var sdkHeaderItemNode = document.createElement("div");
                        sdkHeaderItemNode.className = "hyper-checkout-sdk-items";
                        var sdkHeaderMerchantNameNode = document.createElement("div");
                        sdkHeaderMerchantNameNode.className = "hyper-checkout-sdk-header-brand-name";
                        sdkHeaderMerchantNameNode.innerText = paymentDetails.merchant;
                        var sdkHeaderAmountNode = document.createElement("div");
                        sdkHeaderAmountNode.className = "hyper-checkout-sdk-header-amount";
                        sdkHeaderAmountNode.innerText = paymentDetails.currency + " " + paymentDetails.amount;
                        sdkHeaderItemNode.append(sdkHeaderMerchantNameNode);
                        sdkHeaderItemNode.append(sdkHeaderAmountNode);
                      
                        // Append to SDK header's node
                        var sdkHeaderNode = document.getElementById("hyper-checkout-sdk-header");
                        if (sdkHeaderNode !== null) {{
                          sdkHeaderNode.append(sdkHeaderLogoNode);
                          sdkHeaderNode.append(sdkHeaderItemNode);
                        }}
                      }}
                      
                      function showSDK(e) {{
                        setPageLoading(true);
                        checkStatus().then((res) => {{
                          if (res.showSdk) {{
                            renderPaymentDetails();
                            renderSDKHeader();
                            show("#hyper-checkout-sdk");
                            show("#hyper-checkout-details")
                          }} else {{
                            show("#hyper-checkout-status");
                            show("#hyper-footer");
                          }}
                        }}).catch((err) => {{
                      
                        }}).finally(() => {{
                          setPageLoading(false);
                        }})
                      }}
                      
                      
                </script>
                "##, payment_link_data.amount, payment_link_data.currency)))
    }
        
}
}