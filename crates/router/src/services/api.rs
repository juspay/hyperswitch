mod client;
pub(crate) mod request;

use std::{borrow::Cow, collections::HashMap, fmt::Debug, future::Future, str, time::Instant};

use actix_web::{body, HttpRequest, HttpResponse, Responder};
use bytes::Bytes;
use error_stack::{report, IntoReport, Report, ResultExt};
use masking::ExposeOptionInterface;
use router_env::{
    tracing::{self, instrument},
    Tag,
};
use serde::Serialize;

use self::request::{ContentType, HeaderExt, RequestBuilderExt};
pub use self::request::{Method, Request, RequestBuilder};
use crate::{
    configs::settings::Connectors,
    core::{
        errors::{
            self, ApiClientErrorExt, CustomResult, RouterResponse, RouterResult, StorageErrorExt,
        },
        payments,
    },
    db::StorageInterface,
    logger, routes,
    routes::AppState,
    types::{
        self, api,
        storage::{self, enums},
        ErrorResponse, Response,
    },
    utils::{self, OptionExt},
};

pub type BoxedConnectorIntegration<'a, T, Req, Resp> =
    Box<&'a (dyn ConnectorIntegration<T, Req, Resp> + Send + Sync)>;

pub trait ConnectorIntegrationExt<T, Req, Resp>: Send + Sync + 'static {
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<T, Req, Resp>;
}

impl<S, T, Req, Resp> ConnectorIntegrationExt<T, Req, Resp> for S
where
    S: ConnectorIntegration<T, Req, Resp> + Send + Sync,
{
    fn get_connector_integration(&self) -> BoxedConnectorIntegration<'_, T, Req, Resp> {
        Box::new(self)
    }
}

pub trait ConnectorIntegration<T, Req, Resp>: ConnectorIntegrationExt<T, Req, Resp> {
    fn get_headers(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

    fn get_url(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
        _connectors: Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(String::new())
    }

    fn get_request_body(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request(
        &self,
        _req: &types::RouterData<T, Req, Resp>,
        _connectors: Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &types::RouterData<T, Req, Resp>,
        _res: Response,
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
        _res: Bytes,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Ok(ErrorResponse::get_not_implemented())
    }
}

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
                response: res.into(),
                status_code: 200,
            };

            connector_integration.handle_response(req, response)
        }
        payments::CallConnectorAction::Avoid => Ok(router_data),
        payments::CallConnectorAction::StatusUpdate(status) => {
            router_data.status = status;
            Ok(router_data)
        }
        payments::CallConnectorAction::Trigger => {
            match connector_integration.build_request(req, state.conf.connectors.clone())? {
                Some(request) => {
                    let response = call_connector_api(state, request).await;
                    match response {
                        Ok(body) => {
                            let response = match body {
                                Ok(body) => connector_integration.handle_response(req, body)?,
                                Err(body) => {
                                    let error =
                                        connector_integration.get_error_response(body.response)?;
                                    router_data.response = Err(error);

                                    router_data
                                }
                            };
                            logger::debug!(?response);
                            Ok(response)
                        }
                        Err(error) => Err(error.to_unsuccessful_processing_step_response()),
                    }
                }
                None => Ok(router_data),
            }
        }
    }
}

#[instrument(skip_all)]
pub(crate) async fn call_connector_api(
    state: &AppState,
    request: Request,
) -> CustomResult<Result<Response, Response>, errors::ApiClientError> {
    let current_time = Instant::now();

    let response = send_request(state, request).await;

    let elapsed_time = current_time.elapsed();
    logger::info!(request_time=?elapsed_time);

    handle_response(response).await
}

#[instrument(skip_all)]
async fn send_request(
    state: &AppState,
    request: Request,
) -> CustomResult<reqwest::Response, errors::ApiClientError> {
    logger::debug!(method=?request.method, headers=?request.headers, payload=?request.payload, ?request);
    let url = &request.url;
    let should_bypass_proxy = client::proxy_bypass_urls(&state.conf.locker).contains(url);
    // TODO propogate error for request timeout
    let client = client::create_client(
        &state.conf.proxy,
        should_bypass_proxy,
        crate::consts::REQUEST_TIME_OUT,
    )?;
    let headers = request.headers.construct_header_map()?;

    match request.method {
        Method::Get => client.get(url).add_headers(headers).send().await,
        Method::Post => {
            let client = client.post(url).add_headers(headers);
            match request.content_type {
                Some(ContentType::Json) => client.json(&request.payload).send(),

                // Currently this is not used remove this if not required
                // If using this then handle the serde_part
                Some(ContentType::FormUrlEncoded) => {
                    let url_encoded_payload = serde_urlencoded::to_string(&request.payload)
                        .into_report()
                        .change_context(errors::ApiClientError::UrlEncodingFailed)
                        .attach_printable_lazy(|| {
                            format!(
                                "Unable to do url encoding on request: {:?}",
                                &request.payload
                            )
                        })?;

                    logger::debug!(?url_encoded_payload);
                    client.body(url_encoded_payload).send()
                }
                None => client
                    .body(request.payload.expose_option().unwrap_or_default())
                    .send(),
            }
            .await
        }

        Method::Put => client.put(url).add_headers(headers).send().await,
        Method::Delete => client.delete(url).add_headers(headers).send().await,
    }
    .into_report()
    .change_context(errors::ApiClientError::RequestNotSent)
    .attach_printable("Unable to send request to connector")
}

#[instrument(skip_all)]
async fn handle_response(
    response: CustomResult<reqwest::Response, errors::ApiClientError>,
) -> CustomResult<Result<Response, Response>, errors::ApiClientError> {
    response
        .map(|response| async {
            logger::info!(?response);
            let status_code = response.status().as_u16();
            match status_code {
                200..=202 => {
                    logger::debug!(response=?response);
                    // If needed add log line
                    // logger:: error!( error_parsing_response=?err);
                    let response = response
                        .bytes()
                        .await
                        .into_report()
                        .change_context(errors::ApiClientError::ResponseDecodingFailed)
                        .attach_printable("Error while waiting for response")?;
                    Ok(Ok(Response {
                        response,
                        status_code,
                    }))
                }

                status_code @ 500..=599 => {
                    let error = match status_code {
                        500 => errors::ApiClientError::InternalServerErrorReceived,
                        502 => errors::ApiClientError::BadGatewayReceived,
                        503 => errors::ApiClientError::ServiceUnavailableReceived,
                        504 => errors::ApiClientError::GatewayTimeoutReceived,
                        _ => errors::ApiClientError::UnexpectedServerResponse,
                    };
                    Err(Report::new(error).attach_printable("Server error response received"))
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
                    Ok(Err(Response {
                        response: bytes,
                        status_code,
                    }))
                }

                _ => {
                    // FIXME: may need to understand redirects
                    Err(report!(errors::ApiClientError::UnexpectedServerResponse)
                        .attach_printable("Unexpected response from server"))
                }
            }
        })?
        .await
}

#[derive(Debug, Eq, PartialEq)]
pub enum BachResponse<R> {
    Json(R),
    StatusOk,
    TextPlain(String),
    /*
    redirect form not used https://juspay.atlassian.net/browse/ORCA-301
    RedirectResponse(BachRedirectResponse),
    Form(BachRedirectForm),
    */
    JsonForRedirection(api::RedirectionResponse),
    // RedirectResponse(BachRedirectResponse),
    Form(RedirectForm),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BachRedirectResponse {
    pub url: String,
}

impl From<&storage::PaymentAttempt> for BachRedirectResponse {
    fn from(payment_attempt: &storage::PaymentAttempt) -> Self {
        Self {
            url: format!(
                "/payments/start/{}/{}/{}",
                &payment_attempt.payment_id, &payment_attempt.merchant_id, &payment_attempt.txn_id
            ),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedirectForm {
    pub url: String,
    pub method: Method,
    pub form_fields: HashMap<String, String>,
}

impl RedirectForm {
    pub fn new(url: String, method: Method, form_fields: HashMap<String, String>) -> Self {
        Self {
            url,
            method,
            form_fields,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ApiAuthentication<'a> {
    Merchant(MerchantAuthentication<'a>),
    Connector(ConnectorAuthentication<'a>),
}

#[derive(Clone, Debug)]
pub enum MerchantAuthentication<'a> {
    ApiKey,
    MerchantId(Cow<'a, str>),
    AdminApiKey,
    PublishableKey,
}

#[derive(Clone, Debug)]
pub enum ConnectorAuthentication<'a> {
    MerchantId(&'a str),
}

impl<'a> From<MerchantAuthentication<'a>> for ApiAuthentication<'a> {
    fn from(merchant_auth: MerchantAuthentication<'a>) -> Self {
        ApiAuthentication::Merchant(merchant_auth)
    }
}

impl<'a> From<ConnectorAuthentication<'a>> for ApiAuthentication<'a> {
    fn from(connector_auth: ConnectorAuthentication<'a>) -> Self {
        ApiAuthentication::Connector(connector_auth)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthFlow {
    Client,
    Merchant,
}

pub(crate) fn get_auth_flow(auth_type: &MerchantAuthentication) -> AuthFlow {
    match auth_type {
        MerchantAuthentication::ApiKey => AuthFlow::Merchant,
        _ => AuthFlow::Client,
    }
}

pub(crate) fn get_auth_type(request: &HttpRequest) -> RouterResult<MerchantAuthentication> {
    let api_key = get_api_key(request).change_context(errors::ApiErrorResponse::Unauthorized)?;
    if api_key.starts_with("pk_") {
        Ok(MerchantAuthentication::PublishableKey)
    } else {
        Ok(MerchantAuthentication::ApiKey)
    }
}

#[instrument(skip(request, payload, state, func))]
pub(crate) async fn server_wrap_util<'a, 'b, T, Q, F, Fut>(
    state: &'b routes::AppState,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_authentication: ApiAuthentication<'a>,
) -> RouterResult<BachResponse<Q>>
where
    F: Fn(&'b routes::AppState, storage::MerchantAccount, T) -> Fut,
    Fut: Future<Output = RouterResponse<Q>>,
    Q: Serialize + Debug + 'a,
    T: std::fmt::Debug,
{
    let merchant_account = match api_authentication {
        ApiAuthentication::Merchant(merchant_auth) => {
            authenticate_merchant(request, &*state.store, merchant_auth).await?
        }
        ApiAuthentication::Connector(connector_auth) => {
            authenticate_connector(request, &*state.store, connector_auth).await?
        }
    };
    logger::debug!(request=?payload);
    func(state, merchant_account, payload).await
}

#[instrument(
    skip(request, payload, state, func),
    fields(request_method, request_url_path)
)]
pub(crate) async fn server_wrap<'a, 'b, A, T, Q, F, Fut>(
    state: &'b routes::AppState,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_authentication: A,
) -> HttpResponse
where
    A: Into<ApiAuthentication<'a>> + Debug,
    F: Fn(&'b routes::AppState, storage::MerchantAccount, T) -> Fut,
    Fut: Future<Output = RouterResult<BachResponse<Q>>>,
    Q: Serialize + Debug + 'a,
    T: std::fmt::Debug,
{
    let api_authentication = api_authentication.into();
    let request_method = request.method().as_str();
    let url_path = request.path();
    tracing::Span::current().record("request_method", request_method);
    tracing::Span::current().record("request_url_path", url_path);

    let start_instant = Instant::now();
    logger::info!(tag = ?Tag::BeginRequest);

    let res = match server_wrap_util(state, request, payload, func, api_authentication).await {
        Ok(BachResponse::Json(response)) => match serde_json::to_string(&response) {
            Ok(res) => http_response_json(res),
            Err(_) => http_response_err(
                r#"{
                    "error": {
                        "message": "Error serializing response from connector"
                    }
                }"#,
            ),
        },
        Ok(BachResponse::StatusOk) => http_response_ok(),
        Ok(BachResponse::TextPlain(text)) => http_response_plaintext(text),
        Ok(BachResponse::JsonForRedirection(response)) => match serde_json::to_string(&response) {
            Ok(res) => http_redirect_response(res, response),
            Err(_) => http_response_err(
                r#"{
                    "error": {
                        "message": "Error serializing response from connector"
                    }
                }"#,
            ),
        },
        Ok(BachResponse::Form(response)) => build_redirection_form(&response)
            .respond_to(request)
            .map_into_boxed_body(),

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

pub(crate) fn log_and_return_error_response<T>(error: Report<T>) -> HttpResponse
where
    T: actix_web::ResponseError + error_stack::Context,
{
    logger::error!(?error);
    error.current_context().error_response()
}

pub async fn authenticate_merchant<'a>(
    request: &HttpRequest,
    store: &dyn StorageInterface,
    merchant_authentication: MerchantAuthentication<'a>,
) -> RouterResult<storage::MerchantAccount> {
    match merchant_authentication {
        MerchantAuthentication::ApiKey => {
            let api_key =
                get_api_key(request).change_context(errors::ApiErrorResponse::Unauthorized)?;
            authenticate_by_api_key(store, api_key).await
        }

        MerchantAuthentication::MerchantId(merchant_id) => {
            store
                .find_merchant_account_by_merchant_id(&merchant_id)
                .await
                .map_err(|error| {
                    // TODO: The BadCredentials error is too specific for api keys, and inappropriate for AdminApiKey/MerchantID
                    // https://juspay.atlassian.net/browse/ORCA-366
                    error.to_not_found_response(errors::ApiErrorResponse::BadCredentials)
                })
        }

        MerchantAuthentication::AdminApiKey => {
            let admin_api_key =
                get_api_key(request).change_context(errors::ApiErrorResponse::Unauthorized)?;
            if admin_api_key != "test_admin" {
                // TODO: The BadCredentials error is too specific for api keys, and inappropriate
                // for AdminApiKey/MerchantID
                Err(report!(errors::ApiErrorResponse::BadCredentials)
                    .attach_printable("Admin Authentication Failure"))?;
            }

            Ok(storage::MerchantAccount {
                id: -1,
                merchant_id: String::from("juspay"),
                merchant_name: None,
                api_key: None,
                merchant_details: None,
                return_url: None,
                webhook_details: None,
                routing_algorithm: None,
                custom_routing_rules: None,
                sub_merchants_enabled: None,
                parent_merchant_id: None,
                enable_payment_response_hash: false,
                payment_response_hash_key: None,
                redirect_to_merchant_with_http_post: false,
                publishable_key: None,
                storage_scheme: enums::MerchantStorageScheme::PostgresOnly,
            })
        }

        MerchantAuthentication::PublishableKey => {
            let api_key =
                get_api_key(request).change_context(errors::ApiErrorResponse::Unauthorized)?;
            authenticate_by_publishable_key(store, api_key).await
        }
    }
}

pub async fn authenticate_connector<'a>(
    _request: &HttpRequest,
    store: &dyn StorageInterface,
    connector_authentication: ConnectorAuthentication<'a>,
) -> RouterResult<storage::MerchantAccount> {
    match connector_authentication {
        ConnectorAuthentication::MerchantId(merchant_id) => store
            .find_merchant_account_by_merchant_id(merchant_id)
            .await
            .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::BadCredentials)),
    }
}

pub(crate) fn get_auth_type_and_check_client_secret<P>(
    req: &actix_web::HttpRequest,
    payload: P,
) -> RouterResult<(P, MerchantAuthentication)>
where
    P: Authenticate,
{
    let auth_type = get_auth_type(req)?;
    Ok((
        payments::helpers::client_secret_auth(payload, &auth_type)?,
        auth_type,
    ))
}

pub(crate) async fn authenticate_eph_key<'a>(
    req: &'a actix_web::HttpRequest,
    store: &dyn StorageInterface,
    customer_id: String,
) -> RouterResult<MerchantAuthentication<'a>> {
    let api_key = get_api_key(req)?;
    if api_key.starts_with("epk") {
        let ek = store
            .get_ephemeral_key(api_key)
            .await
            .change_context(errors::ApiErrorResponse::BadCredentials)?;
        utils::when(
            ek.customer_id.ne(&customer_id),
            Err(report!(errors::ApiErrorResponse::InvalidEphermeralKey)),
        )?;
        Ok(MerchantAuthentication::MerchantId(Cow::Owned(
            ek.merchant_id,
        )))
    } else {
        Ok(MerchantAuthentication::ApiKey)
    }
}

fn get_api_key(req: &HttpRequest) -> RouterResult<&str> {
    req.headers()
        .get("api-key")
        .get_required_value("api-key")?
        .to_str()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert API key to string")
}

pub async fn authenticate_by_api_key(
    store: &dyn StorageInterface,
    api_key: &str,
) -> RouterResult<storage::MerchantAccount> {
    store
        .find_merchant_account_by_api_key(api_key)
        .await
        .change_context(errors::ApiErrorResponse::BadCredentials)
        .attach_printable("Merchant not authenticated")
}

async fn authenticate_by_publishable_key(
    store: &dyn StorageInterface,
    publishable_key: &str,
) -> RouterResult<storage::MerchantAccount> {
    store
        .find_merchant_account_by_publishable_key(publishable_key)
        .await
        .change_context(errors::ApiErrorResponse::BadCredentials)
        .attach_printable("Merchant not authenticated")
}

pub(crate) fn http_response_json<T: body::MessageBody + 'static>(response: T) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .append_header(("Via", "Juspay_router"))
        .body(response)
}

pub(crate) fn http_response_plaintext<T: body::MessageBody + 'static>(res: T) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain")
        .append_header(("Via", "Juspay_router"))
        .body(res)
}

pub(crate) fn http_response_ok() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub(crate) fn http_redirect_response<T: body::MessageBody + 'static>(
    response: T,
    redirection_response: api::RedirectionResponse,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .append_header(("Via", "Juspay_router"))
        .append_header((
            "Location",
            redirection_response.return_url_with_query_params,
        ))
        .status(http::StatusCode::FOUND)
        .body(response)
}

pub(crate) fn http_response_err<T: body::MessageBody + 'static>(response: T) -> HttpResponse {
    HttpResponse::BadRequest()
        .content_type("application/json")
        .append_header(("Via", "Juspay_router"))
        .body(response)
}

pub trait ConnectorRedirectResponse {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Avoid)
    }
}

pub trait Authenticate {
    fn get_client_secret(&self) -> Option<&String>;
}

impl Authenticate for api_models::payments::PaymentsRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

impl Authenticate for api_models::payment_methods::ListPaymentMethodRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

pub fn build_redirection_form(form: &RedirectForm) -> maud::Markup {
    use maud::PreEscaped;

    maud::html! {
        (maud::DOCTYPE)
        html {
            meta name="viewport" content="width=device-width, initial-scale=1";
            head {
                style { "#loading { -webkit-animation: rotation 2s infinite linear; } @-webkit-keyframes rotation{ from { -webkit-transform: rotate(0deg); } to { -webkit-transform: rotate(359deg); } }" }
            }
            body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {
                img #loading
                style="width: 60px;  display: block; position: relative; margin-left: auto; margin-right: auto;"
                src="data:image/png;base64,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";

                h3 style="text-align: center;" { "Please wait while we process your payment..." }
                form action=(PreEscaped(&form.url)) method=(form.method.to_string()) #payment_form {
                    @for (field, value) in &form.form_fields {
                        input type="hidden" name=(field) value=(value);
                    }
                }

                (maud::PreEscaped(r#"<script type="text/javascript"> var frm = document.getElementById("payment_form"); window.setTimeout(function () { frm.submit(); }, 500); </script>"#))
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
