pub mod transformers;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{revenue_recovery, router_data_v2::RouterDataV2};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::flow_common_types::{
        GetSubscriptionPlanPricesData, GetSubscriptionPlansData, SubscriptionCreateData,
        SubscriptionCustomerData,
    },
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        revenue_recovery::InvoiceRecordBack,
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionPlanPrices, GetSubscriptionPlans,
            SubscriptionCancel, SubscriptionCreate, SubscriptionPause, SubscriptionResume,
        },
        CreateConnectorCustomer,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCancelRequest, SubscriptionCreateRequest,
            SubscriptionPauseRequest, SubscriptionResumeRequest,
        },
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCancelResponse, SubscriptionCreateResponse,
            SubscriptionPauseResponse, SubscriptionResumeResponse,
        },
        ConnectorInfo, PaymentsResponseData, RefundsResponseData,
    },
    types::{
        ConnectorCustomerRouterData, GetSubscriptionEstimateRouterData,
        GetSubscriptionPlanPricesRouterData, GetSubscriptionPlansRouterData,
        InvoiceRecordBackRouterData, PaymentsAuthorizeRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
        SubscriptionCancelRouterData, SubscriptionCreateRouterData, SubscriptionPauseRouterData,
        SubscriptionResumeRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self,
        payments::ConnectorCustomer,
        subscriptions_v2::{GetSubscriptionPlanPricesV2, GetSubscriptionPlansV2},
        ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    connector_integration_v2::ConnectorIntegrationV2,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{Mask, PeekInterface, Secret};
use transformers as chargebee;

use crate::{
    connectors::chargebee::transformers::{
        ChargebeeGetPlanPricesResponse, ChargebeeListPlansResponse,
    },
    constants::{self, headers},
    types::ResponseRouterData,
    utils,
};

#[derive(Clone)]
pub struct Chargebee {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Chargebee {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}
impl ConnectorCustomer for Chargebee {}
impl api::Payment for Chargebee {}
impl api::PaymentSession for Chargebee {}
impl api::ConnectorAccessToken for Chargebee {}
impl api::MandateSetup for Chargebee {}
impl api::PaymentAuthorize for Chargebee {}
impl api::PaymentSync for Chargebee {}
impl api::PaymentCapture for Chargebee {}
impl api::PaymentVoid for Chargebee {}
impl api::Refund for Chargebee {}
impl api::RefundExecute for Chargebee {}
impl api::RefundSync for Chargebee {}
impl api::PaymentToken for Chargebee {}
impl api::subscriptions::Subscriptions for Chargebee {}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery::RevenueRecoveryRecordBack for Chargebee {}

fn build_chargebee_url<Flow, Request, Response>(
    connector: &Chargebee,
    req: &RouterData<Flow, Request, Response>,
    connectors: &Connectors,
    path: &str,
) -> CustomResult<String, errors::ConnectorError> {
    let metadata: chargebee::ChargebeeMetadata =
        utils::to_connector_meta_from_secret(req.connector_meta_data.clone())?;

    let site = metadata.site.peek();
    let mut base = connector.base_url(connectors).to_string();

    base = base.replace("{{merchant_endpoint_prefix}}", site);
    base = base.replace("$", site);

    if base.contains("{{merchant_endpoint_prefix}}") || base.contains('$') {
        return Err(errors::ConnectorError::InvalidConnectorConfig {
            config: "Chargebee base_url has an unresolved placeholder",
        }
        .into());
    }

    if !base.ends_with('/') {
        base.push('/');
    }

    Ok(format!("{}{}", base, path))
}

macro_rules! impl_chargebee_integration {
    (
        flow: $flow:ty,
        flow_type: $flow_type:ty,
        request: $request:ty,
        response: $response:ty,
        router_data: $router_data:ty,
        connector_response: $connector_response:ty,
        url_path: |$req_param:ident| $url_path:expr,
        method: $method:expr
        $(, request_body: $request_body:expr)?
        $(, query_params: $query_fn:expr)?
    ) => {
        impl ConnectorIntegration<$flow, $request, $response> for Chargebee {
            fn get_headers(
                &self,
                req: &$router_data,
                connectors: &Connectors,
            ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
                self.build_headers(req, connectors)
            }

            #[allow(unreachable_code)]
            fn get_url(
                &self,
                req: &$router_data,
                _connectors: &Connectors,
            ) -> CustomResult<String, errors::ConnectorError> {
                let $req_param = req;
                let _base_path_opt: Option<String> = $url_path;
                let _base_path = _base_path_opt.ok_or_else(|| {
                    errors::ConnectorError::NotImplemented(
                        format!("{} operation is not supported by Chargebee", stringify!($flow))
                    )
                })?;
                $(
                    let query = $query_fn(req)?;
                    let path = format!("{}{}", _base_path, query);
                    return build_chargebee_url(self, req, _connectors, &path);
                )?
                build_chargebee_url(self, req, _connectors, &_base_path)
            }

            fn get_content_type(&self) -> &'static str {
                self.common_get_content_type()
            }

            $(
                // Only include get_request_body if request_body is specified
                fn get_request_body(
                    &self,
                    req: &$router_data,
                    _connectors: &Connectors,
                ) -> CustomResult<RequestContent, errors::ConnectorError> {
                    $request_body(self, req)
                }
            )?

            fn build_request(
                &self,
                req: &$router_data,
                connectors: &Connectors,
            ) -> CustomResult<Option<Request>, errors::ConnectorError> {
                #[allow(unused_mut)]
                let mut builder = RequestBuilder::new()
                    .method($method)
                    .url(&<$flow_type>::get_url(
                        self, req, connectors,
                    )?)
                    .attach_default_headers()
                    .headers(<$flow_type>::get_headers(
                        self, req, connectors,
                    )?);

                $(
                    let _ = $request_body; // Use the token to satisfy the macro
                    builder = builder.set_body(<$flow_type>::get_request_body(
                        self, req, connectors,
                    )?);
                )?

                Ok(Some(builder.build()))
            }

            fn handle_response(
                &self,
                data: &$router_data,
                event_builder: Option<&mut ConnectorEvent>,
                res: Response,
            ) -> CustomResult<$router_data, errors::ConnectorError> {
                let response: $connector_response = res
                    .response
                    .parse_struct(stringify!($connector_response))
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                RouterData::try_from(ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }

            fn get_error_response(
                &self,
                res: Response,
                event_builder: Option<&mut ConnectorEvent>,
            ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
                self.build_error_response(res, event_builder)
            }
        }
    };
}

impl
    ConnectorIntegrationV2<
        SubscriptionCreate,
        SubscriptionCreateData,
        SubscriptionCreateRequest,
        SubscriptionCreateResponse,
    > for Chargebee
{
    // Not Implemented (R)
}

impl
    ConnectorIntegrationV2<
        CreateConnectorCustomer,
        SubscriptionCustomerData,
        ConnectorCustomerData,
        PaymentsResponseData,
    > for Chargebee
{
    // Not Implemented (R)
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Chargebee
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Chargebee
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Chargebee {
    fn id(&self) -> &'static str {
        "chargebee"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.chargebee.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = chargebee::ChargebeeAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key = BASE64_ENGINE.encode(auth.full_access_key_v1.peek());
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {encoded_api_key}").into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: chargebee::ChargebeeErrorResponse = res
            .response
            .parse_struct("ChargebeeErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.api_error_code.clone(),
            message: response.api_error_code.clone(),
            reason: Some(response.message),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Chargebee {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Chargebee {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Chargebee {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Chargebee
{
}

// Payment flows are not implemented for Chargebee as it's a subscription billing connector
fn build_payments_authorize_request_body(
    connector: &Chargebee,
    req: &PaymentsAuthorizeRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let amount = utils::convert_amount(
        connector.amount_converter,
        req.request.minor_amount,
        req.request.currency,
    )?;

    let connector_router_data = chargebee::ChargebeeRouterData::from((amount, req));
    let connector_req = chargebee::ChargebeePaymentsRequest::try_from(&connector_router_data)?;
    Ok(RequestContent::Json(Box::new(connector_req)))
}

impl_chargebee_integration!(
    flow: Authorize,
    flow_type: types::PaymentsAuthorizeType,
    request: PaymentsAuthorizeData,
    response: PaymentsResponseData,
    router_data: PaymentsAuthorizeRouterData,
    connector_response: chargebee::ChargebeePaymentsResponse,
    url_path: |_req| None,
    method: Method::Post,
    request_body: build_payments_authorize_request_body
);

impl_chargebee_integration!(
    flow: PSync,
    flow_type: types::PaymentsSyncType,
    request: PaymentsSyncData,
    response: PaymentsResponseData,
    router_data: PaymentsSyncRouterData,
    connector_response: chargebee::ChargebeePaymentsResponse,
    url_path: |_req| None,
    method: Method::Get
);

fn build_payments_capture_request_body(
    _connector: &Chargebee,
    _req: &PaymentsCaptureRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    Err(errors::ConnectorError::NotImplemented(
        "Payment capture not supported by Chargebee".to_string(),
    )
    .into())
}

impl_chargebee_integration!(
    flow: Capture,
    flow_type: types::PaymentsCaptureType,
    request: PaymentsCaptureData,
    response: PaymentsResponseData,
    router_data: PaymentsCaptureRouterData,
    connector_response: chargebee::ChargebeePaymentsResponse,
    url_path: |_req| None,
    method: Method::Post,
    request_body: build_payments_capture_request_body
);

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Chargebee {}

fn build_refund_execute_request_body(
    connector: &Chargebee,
    req: &RefundsRouterData<Execute>,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let refund_amount = utils::convert_amount(
        connector.amount_converter,
        req.request.minor_refund_amount,
        req.request.currency,
    )?;

    let connector_router_data = chargebee::ChargebeeRouterData::from((refund_amount, req));
    let connector_req = chargebee::ChargebeeRefundRequest::try_from(&connector_router_data)?;
    Ok(RequestContent::Json(Box::new(connector_req)))
}

impl_chargebee_integration!(
    flow: Execute,
    flow_type: types::RefundExecuteType,
    request: RefundsData,
    response: RefundsResponseData,
    router_data: RefundsRouterData<Execute>,
    connector_response: chargebee::RefundResponse,
    url_path: |_req| None,
    method: Method::Post,
    request_body: build_refund_execute_request_body
);

impl_chargebee_integration!(
    flow: RSync,
    flow_type: types::RefundSyncType,
    request: RefundsData,
    response: RefundsResponseData,
    router_data: RefundSyncRouterData,
    connector_response: chargebee::RefundResponse,
    url_path: |_req| None,
    method: Method::Get
);

fn build_subscription_create_request_body(
    _connector: &Chargebee,
    req: &SubscriptionCreateRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let connector_router_data = chargebee::ChargebeeRouterData::from((MinorUnit::new(0), req));
    let connector_req =
        chargebee::ChargebeeSubscriptionCreateRequest::try_from(&connector_router_data)?;
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

fn build_connector_customer_request_body(
    _connector: &Chargebee,
    req: &ConnectorCustomerRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let connector_router_data = chargebee::ChargebeeRouterData::from((MinorUnit::new(0), req));
    let connector_req =
        chargebee::ChargebeeCustomerCreateRequest::try_from(&connector_router_data)?;
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

fn build_invoice_record_back_request_body(
    connector: &Chargebee,
    req: &InvoiceRecordBackRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let amount = utils::convert_amount(
        connector.amount_converter,
        req.request.amount,
        req.request.currency,
    )?;
    let connector_router_data = chargebee::ChargebeeRouterData::from((amount, req));
    let connector_req = chargebee::ChargebeeRecordPaymentRequest::try_from(&connector_router_data)?;
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

fn build_subscription_estimate_request_body(
    _connector: &Chargebee,
    req: &GetSubscriptionEstimateRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let connector_req = chargebee::ChargebeeSubscriptionEstimateRequest::try_from(req)?;
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

fn build_subscription_pause_request_body(
    _connector: &Chargebee,
    req: &SubscriptionPauseRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let connector_req = chargebee::ChargebeePauseSubscriptionRequest::from(req);
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

fn build_subscription_resume_request_body(
    _connector: &Chargebee,
    req: &SubscriptionResumeRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let connector_req = chargebee::ChargebeeResumeSubscriptionRequest::from(req);
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

fn build_subscription_cancel_request_body(
    _connector: &Chargebee,
    req: &SubscriptionCancelRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let connector_req = chargebee::ChargebeeCancelSubscriptionRequest::from(req);
    Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
}

impl api::subscriptions::SubscriptionCreate for Chargebee {}

impl_chargebee_integration!(
    flow: SubscriptionCreate,
    flow_type: types::SubscriptionCreateType,
    request: SubscriptionCreateRequest,
    response: SubscriptionCreateResponse,
    router_data: SubscriptionCreateRouterData,
    connector_response: chargebee::ChargebeeSubscriptionCreateResponse,
    url_path: |req| Some(format!("v2/customers/{}/subscription_for_items", req.request.customer_id.get_string_repr())),
    method: Method::Post,
    request_body: build_subscription_create_request_body
);

impl_chargebee_integration!(
    flow: InvoiceRecordBack,
    flow_type: types::InvoiceRecordBackType,
    request: InvoiceRecordBackRequest,
    response: InvoiceRecordBackResponse,
    router_data: InvoiceRecordBackRouterData,
    connector_response: chargebee::ChargebeeRecordbackResponse,
    url_path: |req| Some(format!("v2/invoices/{}/record_payment", req.request.merchant_reference_id.get_string_repr())),
    method: Method::Post,
    request_body: build_invoice_record_back_request_body
);

fn get_chargebee_plans_query_params(
    req: &GetSubscriptionPlansRouterData,
) -> CustomResult<String, errors::ConnectorError> {
    // Try to get limit from request, else default to 10
    let limit = req.request.limit.unwrap_or(10);
    let offset = req.request.offset.unwrap_or(0);
    let param = format!(
        "?limit={}&offset={}&type[is]={}",
        limit,
        offset,
        constants::PLAN_ITEM_TYPE
    );
    Ok(param)
}

impl api::subscriptions::GetSubscriptionPlansFlow for Chargebee {}
impl api::subscriptions::SubscriptionRecordBackFlow for Chargebee {}

impl GetSubscriptionPlansV2 for Chargebee {}

impl
    ConnectorIntegrationV2<
        GetSubscriptionPlans,
        GetSubscriptionPlansData,
        GetSubscriptionPlansRequest,
        GetSubscriptionPlansResponse,
    > for Chargebee
{
    // Not implemented (R)
}

impl_chargebee_integration!(
    flow: GetSubscriptionPlans,
    flow_type: types::GetSubscriptionPlansType,
    request: GetSubscriptionPlansRequest,
    response: GetSubscriptionPlansResponse,
    router_data: GetSubscriptionPlansRouterData,
    connector_response: ChargebeeListPlansResponse,
    url_path: |_req| Some("v2/items".to_string()),
    method: Method::Get,
    query_params: get_chargebee_plans_query_params
);

impl_chargebee_integration!(
    flow: CreateConnectorCustomer,
    flow_type: types::ConnectorCustomerType,
    request: ConnectorCustomerData,
    response: PaymentsResponseData,
    router_data: ConnectorCustomerRouterData,
    connector_response: chargebee::ChargebeeCustomerCreateResponse,
    url_path: |_req| Some("v2/customers".to_string()),
    method: Method::Post,
    request_body: build_connector_customer_request_body
);

impl api::subscriptions::GetSubscriptionPlanPricesFlow for Chargebee {}

fn get_chargebee_plan_prices_query_params(
    req: &GetSubscriptionPlanPricesRouterData,
) -> CustomResult<String, errors::ConnectorError> {
    let item_id = req.request.plan_price_id.to_string();
    let params = format!("?item_id[is]={item_id}");
    Ok(params)
}

impl_chargebee_integration!(
    flow: GetSubscriptionPlanPrices,
    flow_type: types::GetSubscriptionPlanPricesType,
    request: GetSubscriptionPlanPricesRequest,
    response: GetSubscriptionPlanPricesResponse,
    router_data: GetSubscriptionPlanPricesRouterData,
    connector_response: ChargebeeGetPlanPricesResponse,
    url_path: |_req| Some("v2/item_prices".to_string()),
    method: Method::Get,
    query_params: get_chargebee_plan_prices_query_params
);

impl GetSubscriptionPlanPricesV2 for Chargebee {}

impl
    ConnectorIntegrationV2<
        GetSubscriptionPlanPrices,
        GetSubscriptionPlanPricesData,
        GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlanPricesResponse,
    > for Chargebee
{
    // TODO: implement functions when support enabled
}

impl api::subscriptions::GetSubscriptionEstimateFlow for Chargebee {}

impl_chargebee_integration!(
    flow: GetSubscriptionEstimate,
    flow_type: types::GetSubscriptionEstimateType,
    request: GetSubscriptionEstimateRequest,
    response: GetSubscriptionEstimateResponse,
    router_data: GetSubscriptionEstimateRouterData,
    connector_response: chargebee::SubscriptionEstimateResponse,
    url_path: |_req| Some("v2/estimates/create_subscription_for_items".to_string()),
    method: Method::Post,
    request_body: build_subscription_estimate_request_body
);

// Pause Subscription Implementation
impl api::subscriptions::SubscriptionPauseFlow for Chargebee {}

impl_chargebee_integration!(
    flow: SubscriptionPause,
    flow_type: types::SubscriptionPauseType,
    request: SubscriptionPauseRequest,
    response: SubscriptionPauseResponse,
    router_data: SubscriptionPauseRouterData,
    connector_response: chargebee::ChargebeePauseSubscriptionResponse,
    url_path: |req| Some(format!("v2/subscriptions/{}/pause", req.request.subscription_id.get_string_repr())),
    method: Method::Post,
    request_body: build_subscription_pause_request_body
);

// Resume Subscription Implementation
impl api::subscriptions::SubscriptionResumeFlow for Chargebee {}

impl_chargebee_integration!(
    flow: SubscriptionResume,
    flow_type: types::SubscriptionResumeType,
    request: SubscriptionResumeRequest,
    response: SubscriptionResumeResponse,
    router_data: SubscriptionResumeRouterData,
    connector_response: chargebee::ChargebeeResumeSubscriptionResponse,
    url_path: |req| Some(format!("v2/subscriptions/{}/resume", req.request.subscription_id.get_string_repr())),
    method: Method::Post,
    request_body: build_subscription_resume_request_body
);

// Cancel Subscription Implementation
impl api::subscriptions::SubscriptionCancelFlow for Chargebee {}

impl_chargebee_integration!(
    flow: SubscriptionCancel,
    flow_type: types::SubscriptionCancelType,
    request: SubscriptionCancelRequest,
    response: SubscriptionCancelResponse,
    router_data: SubscriptionCancelRouterData,
    connector_response: chargebee::ChargebeeCancelSubscriptionResponse,
    url_path: |req| Some(format!("v2/subscriptions/{}/cancel_for_items", req.request.subscription_id.get_string_repr())),
    method: Method::Post,
    request_body: build_subscription_cancel_request_body
);

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Chargebee {
    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let base64_signature = utils::get_header_key_value("authorization", request.headers)?;
        let signature = base64_signature.as_bytes().to_owned();
        Ok(signature)
    }
    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: common_utils::crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let password = connector_webhook_secrets
            .additional_secret
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Failed to get additional secrets")?;
        let username = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        let secret_auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!(
                "{}:{}",
                username,
                password.peek()
            ))
        );
        let signature_auth = String::from_utf8(signature.to_vec())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            .attach_printable("Could not convert secret to UTF-8")?;
        Ok(signature_auth == secret_auth)
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::InvoiceId(
            api_models::webhooks::InvoiceIdType::ConnectorInvoiceId(
                webhook.content.invoice.id.get_string_repr().to_string(),
            ),
        ))
    }
    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        let subscription_id = webhook.content.invoice.subscription_id;
        Ok(api_models::webhooks::ObjectReferenceId::SubscriptionId(
            subscription_id,
        ))
    }
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let event = api_models::webhooks::IncomingWebhookEvent::from(webhook.event_type);
        Ok(event)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook =
            chargebee::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_attempt_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryAttemptData, errors::ConnectorError> {
        let webhook =
            transformers::ChargebeeWebhookBody::get_webhook_object_from_body(request.body)?;
        revenue_recovery::RevenueRecoveryAttemptData::try_from(webhook)
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_invoice_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryInvoiceData, errors::ConnectorError> {
        let webhook =
            transformers::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)?;
        revenue_recovery::RevenueRecoveryInvoiceData::try_from(webhook)
    }

    fn get_subscription_mit_payment_data(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        let webhook_body =
            transformers::ChargebeeInvoiceBody::get_invoice_webhook_data_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
                .attach_printable("Failed to parse Chargebee invoice webhook body")?;

        let chargebee_mit_data = transformers::ChargebeeMitPaymentData::try_from(webhook_body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Failed to extract MIT payment data from Chargebee webhook")?;

        // Convert Chargebee-specific data to generic domain model
        Ok(
            hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData {
                invoice_id: chargebee_mit_data.invoice_id,
                amount_due: chargebee_mit_data.amount_due,
                currency_code: chargebee_mit_data.currency_code,
                status: chargebee_mit_data.status.map(|s| s.into()),
                customer_id: chargebee_mit_data.customer_id,
                subscription_id: chargebee_mit_data.subscription_id,
                first_invoice: chargebee_mit_data.first_invoice,
            },
        )
    }
}

static CHARGEBEE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Chargebee",
    description: "Chargebee is a Revenue Growth Management (RGM) platform that helps subscription businesses manage subscriptions, billing, revenue recognition, collections, and customer retention, essentially streamlining the entire subscription lifecycle.",
    connector_type: enums::HyperswitchConnectorCategory::RevenueGrowthManagementPlatform,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static CHARGEBEE_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Chargebee {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&CHARGEBEE_CONNECTOR_INFO)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&CHARGEBEE_SUPPORTED_WEBHOOK_FLOWS)
    }
}
