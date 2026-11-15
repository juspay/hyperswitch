pub mod client;
pub mod generic_link_response;
pub mod request;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::Future,
    str,
    sync::Arc,
    time::{Duration, Instant},
};

use actix_http::header::HeaderMap;
use actix_web::{
    body,
    http::header::{HeaderName, HeaderValue},
    web, FromRequest, HttpRequest, HttpResponse, Responder, ResponseError,
};
pub use client::{ApiClient, MockApiClient, ProxyClient};
pub use common_enums::enums::PaymentAction;
pub use common_utils::request::{ContentType, Method, Request, RequestBuilder};
use common_utils::{
    consts::{DEFAULT_TENANT, TENANT_HEADER, X_HS_LATENCY},
    errors::{ErrorSwitch, ReportSwitchExt},
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::router_data_v2::flow_common_types as common_types;
pub use hyperswitch_domain_models::{
    api::{
        ApplicationResponse, GenericExpiredLinkData, GenericLinkFormData, GenericLinkStatusData,
        GenericLinks, PaymentLinkAction, RedirectionFormData, WebhookResponse,
    },
    payment_method_data::PaymentMethodData,
    router_response_types::RedirectForm,
};
pub use hyperswitch_interfaces::{
    api::{
        BoxedConnectorIntegration, CaptureSyncMethod, ConnectorIntegration,
        ConnectorIntegrationAny, ConnectorRedirectResponse, ConnectorSpecifications,
        ConnectorValidation,
    },
    api_client::{
        call_connector_api, execute_connector_processing_step, handle_response,
        store_raw_connector_response_if_required,
    },
    connector_integration_v2::{
        BoxedConnectorIntegrationV2, ConnectorIntegrationAnyV2, ConnectorIntegrationV2,
    },
};
use hyperswitch_masking::{Maskable, PeekInterface};
pub use payment_link::{PaymentLinkFormData, PaymentLinkStatusData};
use router_env::{instrument, tracing, RequestId, Tag};
use serde::Serialize;

use super::{
    authentication::AuthenticateAndFetch,
    connector_integration_interface::BoxedConnectorIntegrationInterface,
};
use crate::{
    configs::Settings,
    core::{
        api_locking,
        errors::{self, CustomResult},
    },
    events::api_logs::{ApiEvent, ApiEventMetric, ApiEventsType},
    headers, logger,
    routes::{
        app::{AppStateInfo, ReqState, SessionStateInfo},
        metrics, AppState, SessionState,
    },
    services::generic_link_response::build_generic_link_html,
    types::api,
    utils,
};

pub type BoxedPaymentConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::PaymentFlowData, Req, Resp>;
pub type BoxedRefundConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::RefundFlowData, Req, Resp>;
#[cfg(feature = "frm")]
pub type BoxedFrmConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::FrmFlowData, Req, Resp>;
pub type BoxedDisputeConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::DisputesFlowData, Req, Resp>;
pub type BoxedMandateRevokeConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::MandateRevokeFlowData, Req, Resp>;
#[cfg(feature = "payouts")]
pub type BoxedPayoutConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::PayoutFlowData, Req, Resp>;
pub type BoxedWebhookSourceVerificationConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::WebhookSourceVerifyData, Req, Resp>;
pub type BoxedExternalAuthenticationConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::ExternalAuthenticationFlowData, Req, Resp>;
pub type BoxedAuthenticationTokenConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::AuthenticationTokenFlowData, Req, Resp>;
pub type BoxedAccessTokenConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::AccessTokenFlowData, Req, Resp>;
pub type BoxedFilesConnectorIntegrationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::FilesFlowData, Req, Resp>;
pub type BoxedRevenueRecoveryRecordBackInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::InvoiceRecordBackData, Req, Res>;
pub type BoxedRevenueRecoveryDisputeRecordBackInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::DisputeRecordBackData, Req, Res>;
pub type BoxedGetSubscriptionPlansInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::GetSubscriptionItemsData, Req, Res>;
pub type BoxedGetSubscriptionPlanPricesInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::GetSubscriptionItemPricesData, Req, Res>;
pub type BoxedGetSubscriptionEstimateInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::GetSubscriptionEstimateData, Req, Res>;
pub type BoxedSubscriptionPauseInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::SubscriptionPauseData, Req, Res>;
pub type BoxedSubscriptionResumeInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::SubscriptionResumeData, Req, Res>;
pub type BoxedSubscriptionCancelInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::SubscriptionCancelData, Req, Res>;
pub type BoxedBillingConnectorInvoiceSyncIntegrationInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<
        T,
        common_types::BillingConnectorInvoiceSyncFlowData,
        Req,
        Res,
    >;

pub type BoxedUnifiedAuthenticationServiceInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<T, common_types::UasFlowData, Req, Resp>;

pub type BoxedBillingConnectorPaymentsSyncIntegrationInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<
        T,
        common_types::BillingConnectorPaymentsSyncFlowData,
        Req,
        Res,
    >;
pub type BoxedVaultConnectorIntegrationInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::VaultConnectorFlowData, Req, Res>;

pub type BoxedGiftCardBalanceCheckIntegrationInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::GiftCardBalanceCheckFlowData, Req, Res>;

pub type BoxedSubscriptionConnectorIntegrationInterface<T, Req, Res> =
    BoxedConnectorIntegrationInterface<T, common_types::SubscriptionCreateData, Req, Res>;

pub type BoxedConnectorWebhookConfigurationInterface<T, Req, Resp> =
    BoxedConnectorIntegrationInterface<
        T,
        common_types::ConnectorWebhookConfigurationFlowData,
        Req,
        Resp,
    >;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ApplicationRedirectResponse {
    pub url: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthFlow {
    Client,
    Merchant,
}

#[allow(clippy::too_many_arguments)]
#[instrument(
    skip(request, payload, state, func, api_auth, incoming_request_header),
    fields(merchant_id)
)]
pub async fn server_wrap_util<'a, 'b, U, T, Q, F, Fut, E, OErr>(
    flow: &'a impl router_env::types::FlowMetric,
    state: web::Data<AppState>,
    incoming_request_header: &HeaderMap,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn AuthenticateAndFetch<U, SessionState>,
    lock_action: api_locking::LockAction,
) -> CustomResult<ApplicationResponse<Q>, OErr>
where
    F: Fn(SessionState, U, T, ReqState) -> Fut,
    'b: 'a,
    Fut: Future<Output = CustomResult<ApplicationResponse<Q>, E>>,
    Q: Serialize + Debug + 'a + ApiEventMetric,
    T: Debug + Serialize + ApiEventMetric,
    E: ErrorSwitch<OErr> + error_stack::Context,
    OErr: ResponseError + error_stack::Context + Serialize,
    errors::ApiErrorResponse: ErrorSwitch<OErr>,
{
    let request_id = RequestId::extract(request)
        .await
        .attach_printable("Unable to extract request id from request")
        .change_context(errors::ApiErrorResponse::InternalServerError.switch())?;

    let mut app_state = state.get_ref().clone();

    let start_instant = Instant::now();
    let mut serialized_request = hyperswitch_masking::masked_serialize(&payload)
        .attach_printable("Failed to serialize json request")
        .change_context(errors::ApiErrorResponse::InternalServerError.switch())?;

    let mut event_type = payload.get_api_event_type();
    let tenant_id = if !state.conf.multitenancy.enabled {
        common_utils::id_type::TenantId::try_from_string(DEFAULT_TENANT.to_owned())
            .attach_printable("Unable to get default tenant id")
            .change_context(errors::ApiErrorResponse::InternalServerError.switch())?
    } else {
        let request_tenant_id = incoming_request_header
            .get(TENANT_HEADER)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| errors::ApiErrorResponse::MissingTenantId.switch())
            .and_then(|header_value| {
                common_utils::id_type::TenantId::try_from_string(header_value.to_string()).map_err(






                )
            })?;

        state
            .conf
            .multitenancy
            .get_tenant(&request_tenant_id)
            .map(|tenant| tenant.tenant_id.clone())
            .ok_or(
                errors::ApiErrorResponse::InvalidTenant {

                }
                .switch(),
            )?
    };
    let locale = utils::get_locale_from_header(&incoming_request_header.clone());
    let mut session_state =
        Arc::new(app_state.clone()).get_session_state(&tenant_id, Some(locale), || {
            errors::ApiErrorResponse::InvalidTenant {
                tenant_id: tenant_id.get_string_repr().to_string(),
            }
            .switch()
        })?;
    session_state.add_request_id(request_id.clone());
    let mut request_state = session_state.get_req_state();

    request_state.event_context.record_info(request_id.clone());
    request_state
        .event_context
        .record_info(("flow".to_string(), flow.to_string()));

    request_state.event_context.record_info((
        "tenant_id".to_string(),
        tenant_id.get_string_repr().to_string(),
    ));

    // Currently auth failures are not recorded as API events
    let (auth_out, auth_type) = api_auth
        .authenticate_and_fetch(request.headers(), &session_state)
        .await
        .switch()?;

    request_state.event_context.record_info(auth_type.clone());

    let merchant_id = auth_type
        .get_merchant_id()
        .cloned()
        .unwrap_or(common_utils::id_type::MerchantId::get_merchant_id_not_found());

    app_state.add_flow_name(flow.to_string());

    tracing::Span::current().record("merchant_id", merchant_id.get_string_repr().to_owned());

    let output = {
        lock_action
            .clone()
            .perform_locking_action(&session_state, merchant_id.to_owned())
            .await
            .switch()?;
        let res = func(session_state.clone(), auth_out, payload, request_state)
            .await
            .switch();
        lock_action
            .free_lock_action(&session_state, merchant_id.to_owned())
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
            let mut extracted_status_code: Option<http::StatusCode> = None;

            if let ApplicationResponse::Json(data) = res {
                serialized_response.replace(



                );
            } else if let ApplicationResponse::JsonWithHeaders((data, headers)) = res {
                serialized_response.replace(



                );

                if let Some((_, value)) = headers.iter().find(|(key, _)| key == X_HS_LATENCY) {



                }

                // Extract connector HTTP status code for ApiEvent logging
                extracted_status_code = state



            } else if let ApplicationResponse::IncomingWebhookEvent { response, metadata } = res {
                serialized_request = metadata.serialized_request.peek().clone();

                if let Some(tracker_data) = &metadata.webhook_tracker_data {

                }

                if let WebhookResponse::JsonWithHeaders((_, headers)) = response.as_ref() {









                }
            }
            event_type = res.get_api_event_type().or(event_type);

            // Use extracted status code if available, otherwise fall back to default
            extracted_status_code
                .map(|code| code.as_u16().into())
                .unwrap_or_else(|| metrics::request::track_response_status_code(res))
        }
        Err(err) => {
            error.replace(
                serde_json::to_value(err.current_context())




            );
            err.current_context().status_code().as_u16().into()
        }
    };

    let values: Vec<&serde_json::Value> = [Some(&serialized_request), serialized_response.as_ref()]
        .into_iter()
        .flatten()
        .collect();

    let infra = extract_mapped_fields(
        &values,
        state.enhancement.as_ref(),
        state.infra_components.as_ref(),
    );

    let auth_user_id = auth_type.get_user_id();

    // The header still holds raw connector time here; it is only turned into Hyperswitch's own
    // overhead later, on the wire.
    let hs_latency =
        overhead_latency.map(|overhead_latency| request_duration.saturating_sub(overhead_latency));

    let api_event = ApiEvent::new(
        tenant_id,
        Some(merchant_id.clone()),
        flow,
        &request_id,
        request_duration,
        status_code,
        serialized_request,
        serialized_response,
        hs_latency,
        auth_type,
        auth_user_id,
        error,
        event_type.unwrap_or(ApiEventsType::Miscellaneous),
        request,
        request.method(),
        infra.clone(),
    );

    state.event_handler().log_event(&api_event);

    output
}

#[instrument(
    skip(request, state, func, api_auth, payload),
    fields(request_method, request_url_path, status_code)
)]
pub async fn server_wrap<'a, T, U, Q, F, Fut, E>(
    flow: impl router_env::types::FlowMetric,
    state: web::Data<AppState>,
    request: &'a HttpRequest,
    payload: T,
    func: F,
    api_auth: &dyn AuthenticateAndFetch<U, SessionState>,
    lock_action: api_locking::LockAction,
) -> HttpResponse
where
    F: Fn(SessionState, U, T, ReqState) -> Fut,
    Fut: Future<Output = CustomResult<ApplicationResponse<Q>, E>>,
    Q: Serialize + Debug + ApiEventMetric + 'a,
    T: Debug + Serialize + ApiEventMetric,
    ApplicationResponse<Q>: Debug,
    E: ErrorSwitch<api_models::errors::types::ApiErrorResponse> + error_stack::Context,
{
    let request_method = request.method().as_str();
    let url_path = request.path();

    let unmasked_incoming_header_keys = state.conf().unmasked_headers.keys;

    let incoming_request_header = request.headers();

    let incoming_header_to_log: HashMap<String, HeaderValue> =
        incoming_request_header
            .iter()
            .fold(HashMap::new(), |mut acc, (key, value)| {
                let key = key.to_string();
                if unmasked_incoming_header_keys.contains(&key.as_str().to_lowercase()) {

                } else {

                }
                acc
            });

    tracing::Span::current().record("request_method", request_method);
    tracing::Span::current().record("request_url_path", url_path);

    let start_instant = Instant::now();

    logger::info!(
        tag = ?Tag::BeginRequest, payload = ?payload,
    headers = ?incoming_header_to_log);

    let server_wrap_util_res = server_wrap_util(
        &flow,
        state.clone(),
        incoming_request_header,
        request,
        payload,
        func,
        api_auth,
        lock_action,
    )
    .await
    .map(|response| {
        let response = match response {
            ApplicationResponse::IncomingWebhookEvent {
                response: inner, ..
            } => ApplicationResponse::from(*inner),
            other => other,
        };
        logger::info!(api_response =? response);
        response
    });

    let res = match server_wrap_util_res {
        Ok(ApplicationResponse::Json(response)) => match serde_json::to_string(&response) {
            Ok(res) => http_response_json(res),
            Err(_) => http_response_err(
                r#"{



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

        Ok(ApplicationResponse::GenericLinkForm(boxed_generic_link_data)) => {
            let link_type = boxed_generic_link_data.data.to_string();
            match build_generic_link_html(
                boxed_generic_link_data.data,
                boxed_generic_link_data.locale,
            ) {
                Ok(rendered_html) => {












                }
                Err(_) => http_response_err(format!("Error while rendering {link_type} HTML page")),
            }
        }

        Ok(ApplicationResponse::PaymentLinkForm(boxed_payment_link_data)) => {
            let mut headers = HashSet::new();
            headers.insert((
                "content-security-policy",
                "default-src 'self' http: https:; script-src 'self' 'unsafe-inline' http: https:; style-src 'self' 'unsafe-inline' http: https:; img-src * data: blob:; font-src * data:; connect-src *; frame-src *; object-src 'none';".to_string(),
            ));
            match *boxed_payment_link_data {
                PaymentLinkAction::PaymentLinkFormData(payment_link_data) => {










                }
                PaymentLinkAction::PaymentLinkStatus(payment_link_data) => {










                }
            }
        }

        Ok(ApplicationResponse::JsonWithHeaders((response, headers))) => {
            let request_elapsed_time = request.headers().get(X_HS_LATENCY).and_then(|value| {
                if value == "true" {

                } else {

                }
            });
            let proxy_connector_http_status_code = state
                .conf
                .proxy_status_mapping
                .extract_connector_http_status_code(&headers);
            match serde_json::to_string(&response) {
                Ok(res) => http_response_json_with_headers(




                ),
                Err(_) => http_response_err(





                ),
            }
        }
        Ok(ApplicationResponse::IncomingWebhookEvent { .. }) => http_response_ok(),
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
{
    logger::error!(?error);
    HttpResponse::from_error(error.current_context().clone())
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
    headers: Vec<(String, Maskable<String>)>,
    request_duration: Option<Duration>,
    status_code: Option<http::StatusCode>,
) -> HttpResponse {
    let mut response_builder = HttpResponse::build(status_code.unwrap_or(http::StatusCode::OK));
    for (header_name, header_value) in headers {
        let is_sensitive_header = header_value.is_masked();
        let mut header_value = header_value.into_inner();
        if header_name == X_HS_LATENCY {
            if let Some(request_duration) = request_duration {
                if let Ok(external_latency) = header_value.parse::<u128>() {




                }
            }
        }
        let mut header_value = match HeaderValue::from_str(header_value.as_str()) {
            Ok(header_value) => header_value,
            Err(error) => {
                logger::error!(?error);
                return http_server_error_json_response("Something Went Wrong");
            }
        };

        if is_sensitive_header {
            header_value.set_sensitive(true);
        }
        response_builder.append_header((header_name, header_value));
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

pub fn http_response_html_data<T: body::MessageBody + 'static>(
    res: T,
    optional_headers: Option<HashSet<(&'static str, String)>>,
) -> HttpResponse {
    let mut res_builder = HttpResponse::Ok();
    res_builder.content_type(mime::TEXT_HTML);

    if let Some(headers) = optional_headers {
        for (key, value) in headers {
            if let Ok(header_val) = HeaderValue::try_from(value) {
                res_builder.insert_header((HeaderName::from_static(key), header_val));
            }
        }
    }

    res_builder.body(res)
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

pub trait Authenticate {
    fn get_client_secret(&self) -> Option<&String> {
        None
    }

    fn should_return_raw_response(&self) -> Option<bool> {
        None
    }

    fn is_external_three_ds_data_passed_by_merchant(&self) -> bool {
        false
    }

    fn get_payment_method_data(&self) -> Option<api_models::payments::PaymentMethodData> {
        None
    }
}

#[cfg(feature = "v2")]
impl Authenticate for api_models::payments::PaymentsConfirmIntentRequest {
    fn should_return_raw_response(&self) -> Option<bool> {
        self.return_raw_connector_response
    }
}
#[cfg(feature = "v2")]
impl Authenticate for api_models::payments::ProxyPaymentsRequest {}

#[cfg(feature = "v2")]
impl Authenticate for api_models::payments::ExternalVaultProxyPaymentsRequest {}

#[cfg(feature = "v1")]
impl Authenticate for api_models::payments::PaymentsRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }

    fn should_return_raw_response(&self) -> Option<bool> {
        // In v1, this maps to `all_keys_required` to retain backward compatibility.
        // The equivalent field in v2 is `return_raw_connector_response`.
        self.all_keys_required
    }

    fn is_external_three_ds_data_passed_by_merchant(&self) -> bool {
        self.three_ds_data.is_some()
    }

    fn get_payment_method_data(&self) -> Option<api_models::payments::PaymentMethodData> {
        self.payment_method_data
            .as_ref()
            .and_then(|pmd| pmd.payment_method_data.clone())
    }
}

#[cfg(feature = "v1")]
impl Authenticate for api_models::payment_methods::PaymentMethodListRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}

#[cfg(feature = "v1")]
impl Authenticate for api_models::payments::PaymentsSessionRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret.as_ref()
    }
}
impl Authenticate for api_models::payments::PaymentsDynamicTaxCalculationRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl Authenticate for api_models::payments::PaymentsPostSessionTokensRequest {
    fn get_client_secret(&self) -> Option<&String> {
        self.client_secret
            .as_ref()
            .map(|client_secret| client_secret.peek())
    }
}

impl Authenticate for api_models::payments::PaymentsUpdateMetadataRequest {}
impl Authenticate for api_models::payments::PaymentsRetrieveRequest {
    #[cfg(feature = "v2")]
    fn should_return_raw_response(&self) -> Option<bool> {
        self.return_raw_connector_response
    }

    #[cfg(feature = "v1")]
    fn should_return_raw_response(&self) -> Option<bool> {
        // In v1, this maps to `all_keys_required` to retain backward compatibility.
        // The equivalent field in v2 is `return_raw_connector_response`.
        self.all_keys_required
    }
}
impl Authenticate for api_models::payments::PaymentsCancelRequest {
    #[cfg(feature = "v2")]
    fn should_return_raw_response(&self) -> Option<bool> {
        self.return_raw_connector_response
    }

    #[cfg(feature = "v1")]
    fn should_return_raw_response(&self) -> Option<bool> {
        // In v1, this maps to `all_keys_required` to retain backward compatibility.
        // The equivalent field in v2 is `return_raw_connector_response`.
        self.all_keys_required
    }
}
impl Authenticate for api_models::payments::PaymentsCancelPostCaptureRequest {}
impl Authenticate for common_utils::id_type::PaymentId {}
impl Authenticate for api_models::payments::PaymentsCaptureRequest {
    #[cfg(feature = "v2")]
    fn should_return_raw_response(&self) -> Option<bool> {
        self.return_raw_connector_response
    }

    #[cfg(feature = "v1")]
    fn should_return_raw_response(&self) -> Option<bool> {
        // In v1, this maps to `all_keys_required` to retain backward compatibility.
        // The equivalent field in v2 is `return_raw_connector_response`.
        self.all_keys_required
    }
}
impl Authenticate for api_models::payments::PaymentsIncrementalAuthorizationRequest {}
impl Authenticate for api_models::payments::PaymentsExtendAuthorizationRequest {}
impl Authenticate for api_models::payments::PaymentsStartRequest {}
// impl Authenticate for api_models::payments::PaymentsApproveRequest {}
impl Authenticate for api_models::payments::PaymentsRejectRequest {}
// #[cfg(feature = "v2")]
// impl Authenticate for api_models::payments::PaymentsIntentResponse {}

pub fn build_redirection_form(
    form: &RedirectForm,
    payment_method_data: Option<PaymentMethodData>,
    amount: String,
    currency: String,
    config: Settings,
) -> maud::Markup {
    use maud::PreEscaped;
    let logging_template =
        include_str!("redirection/assets/redirect_error_logs_push.js").to_string();
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



                }
                (PreEscaped(r##"
                <style>








                </style>
                "##))
            }

            body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {

                div id="loader1" class="lottie" style="height: 150px; display: block; position: relative; margin-left: auto; margin-right: auto;" { "" }

                (PreEscaped(r#"<script src="https://cdnjs.cloudflare.com/ajax/libs/bodymovin/5.7.4/lottie.min.js"></script>"#))

                (PreEscaped(r#"
                <script>
                var anime = bodymovin.loadAnimation({






                })
                </script>
                "#))

                h3 style="text-align: center;" { "Please wait while we process your payment..." }




                }
                (PreEscaped(format!(r#"
















            }
        }
        },
        RedirectForm::Html { html_data } => {
            PreEscaped(format!("{html_data} <script>{logging_template}</script>"))
        }
        RedirectForm::BarclaycardAuthSetup {
            access_token,
            ddc_url,
            reference_id,
        } => {
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {

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
                {logging_template}
                window.addEventListener(\"message\", function(event) {{



                  }}, false);
                </script>
                ")))
            }}
        }
        RedirectForm::BarclaycardConsumerAuth {
            access_token,
            step_up_url,
        } => {
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {

                }






















                // This is the iframe recommended by cybersource but the redirection happens inside this iframe once otp
                // is received and we lose control of the redirection on user client browser, so to avoid that we have removed this iframe and directly consumed it.
                // (PreEscaped(r#"<iframe id="step_up_iframe" style="border: none; margin-left: auto; margin-right: auto; display: block" height="800px" width="400px" name="stepUpIframe"></iframe>"#))
                (PreEscaped(format!("<form id=\"step-up-form\" method=\"POST\" action=\"{step_up_url}\">
                <input type=\"hidden\" name=\"JWT\" value=\"{access_token}\">
              </form>")))
              (PreEscaped(format!("<script>
              {logging_template}
              window.onload = function() {{
              var stepUpForm = document.querySelector('#step-up-form'); if(stepUpForm) stepUpForm.submit();
              }}
              </script>")))
            }}
        }
        RedirectForm::BlueSnap {
            payment_fields_token,
        } => {
            let card_details = if let Some(PaymentMethodData::Card(ccard)) = payment_method_data {
                format!(




                )
            } else {
                "".to_string()
            };
            let bluesnap_sdk_url = config.connectors.bluesnap.secondary_base_url;
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {


                }






















                (PreEscaped(format!("<script>

















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
                {logging_template}
                window.addEventListener(\"message\", function(event) {{



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

                }






















                // This is the iframe recommended by cybersource but the redirection happens inside this iframe once otp
                // is received and we lose control of the redirection on user client browser, so to avoid that we have removed this iframe and directly consumed it.
                // (PreEscaped(r#"<iframe id="step_up_iframe" style="border: none; margin-left: auto; margin-right: auto; display: block" height="800px" width="400px" name="stepUpIframe"></iframe>"#))
                (PreEscaped(format!("<form id=\"step-up-form\" method=\"POST\" action=\"{step_up_url}\">
                <input type=\"hidden\" name=\"JWT\" value=\"{access_token}\">
              </form>")))
              (PreEscaped(format!("<script>
              {logging_template}
              window.onload = function() {{
              var stepUpForm = document.querySelector('#step-up-form'); if(stepUpForm) stepUpForm.submit();
              }}
              </script>")))
            }}
        }
        RedirectForm::DeutschebankThreeDSChallengeFlow { acs_url, creq } => {
            maud::html! {
                (maud::DOCTYPE)
                html {

































                }
            }
        }
        RedirectForm::Payme => {
            maud::html! {
                (maud::DOCTYPE)
                head {

                }
                (PreEscaped(format!("<script>

















            </script>
                ")))
            }
        }
        RedirectForm::Braintree {
            client_token,
            card_token,
            bin,
            acs_url,
        } => {
            maud::html! {
            (maud::DOCTYPE)
            html {
                head {




                }






















                (PreEscaped(format!("<script>

























































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































































































































            </script>"
            )))
                }
        }
        RedirectForm::Mifinity {
            initialization_token,
        } => {
            let mifinity_base_url = config.connectors.mifinity.base_url;
            maud::html! {






	  <script>
		  var widget = showPaymentIframe('widget-container', {{
			  token: '{initialization_token}',
			  complete: function() {{
                var f = document.createElement('form');
                f.action=window.location.pathname.replace(/payments\\/redirect\\/(\\w+)\\/(\\w+)\\/\\w+/, \"payments/$1/$2/redirect/response/mifinity\");
                f.method='GET';
                document.body.appendChild(f);
                f.submit();
			  }}
		   }});
	   </script>")))

            }
        }
        RedirectForm::WorldpayDDCForm {
            endpoint,
            method,
            form_fields,
            collection_id,
        } => maud::html! {
            (maud::DOCTYPE)
            html {
                meta name="viewport" content="width=device-width, initial-scale=1";
                head {












                }

                body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {





























































































































                }
            }
        },
        RedirectForm::WorldpayxmlDDCForm { bin, jwt } => {
            let base_url = config.connectors.worldpayxml.secondary_base_url;
            maud::html! {
                (maud::DOCTYPE)
                html {



                body style="background-color: #ffffff; padding: 20px; font-family: Arial, Helvetica, Sans-Serif;" {










































































                }
            }
        }
        RedirectForm::WorldpayxmlRedirectForm { jwt } => {
            let base_url = config.connectors.worldpayxml.secondary_base_url;
            maud::html! {
                (maud::DOCTYPE)
                html {




















                "#))









                </form>"#)))









                </script>")))

                }
            }
        }
    }
}

pub fn build_payment_link_html(
    payment_link_data: PaymentLinkFormData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    payment_link::build_payment_link_html(payment_link_data)
        .map_err(|e| {
            logger::error!("Failed to build payment link HTML: {:?}", e);
            errors::ApiErrorResponse::InternalServerError
        })
        .attach_printable("Error while rendering open payment link's HTML template")
}

pub fn build_secure_payment_link_html(
    payment_link_data: PaymentLinkFormData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    payment_link::build_secure_payment_link_html(payment_link_data)
        .map_err(|e| {
            logger::error!("Failed to build secure payment link HTML: {:?}", e);
            errors::ApiErrorResponse::InternalServerError
        })
        .attach_printable("Error while rendering secure payment link's HTML template")
}

pub fn get_payment_link_status(
    payment_link_data: PaymentLinkStatusData,
) -> CustomResult<String, errors::ApiErrorResponse> {
    payment_link::get_payment_link_status(payment_link_data)
        .map_err(|e| {
            logger::error!("Failed to get payment link status: {:?}", e);
            errors::ApiErrorResponse::InternalServerError
        })
        .attach_printable("Error while rendering payment link status page")
}

pub fn extract_mapped_fields(
    values: &[&serde_json::Value],
    mapping: Option<&HashMap<String, String>>,
    existing_enhancement: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let mapping = mapping?;

    if mapping.is_empty() {
        return existing_enhancement.cloned();
    }

    let mut enhancement = match existing_enhancement {
        Some(existing) if existing.is_object() => existing.clone(),
        _ => serde_json::json!({}),
    };

    for (dot_path, output_key) in mapping {
        let mut extracted_value = None;

        // Try to extract from values in order of priority
        for value in values {
            if let Some(found_value) = extract_field_by_dot_path(value, dot_path) {
                extracted_value = Some(found_value);
                break;
            }
        }

        if let Some(value) = extracted_value {
            if let Some(obj) = enhancement.as_object_mut() {
                obj.insert(output_key.clone(), value);
            }
        }
    }

    if enhancement.as_object().is_some_and(|obj| !obj.is_empty()) {
        Some(enhancement)
    } else {
        None
    }
}

pub fn extract_field_by_dot_path(
    value: &serde_json::Value,
    path: &str,
) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            serde_json::Value::Object(obj) => {
                current = obj.get(part)?;
            }
            serde_json::Value::Array(arr) => {
                // Try to parse part as array index
                if let Ok(index) = part.parse::<usize>() {

                } else {

                }
            }
            _ => return None,
        }
    }

    Some(current.clone())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mime_essence() {
        assert_eq!(mime::APPLICATION_JSON.essence_str(), "application/json");
    }
}
