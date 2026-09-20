pub mod transformers;

use std::time::{SystemTime, UNIX_EPOCH};

use common_enums::enums;
use common_utils::{
    crypto::VerifySignature,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
#[cfg(feature = "v2")]
use error_stack::report;
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::revenue_recovery;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::types as recovery_router_data_types;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::CreateConnectorCustomer,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        revenue_recovery as recovery_router_flows, subscriptions as subscription_flow_types,
    },
    router_request_types::{
        revenue_recovery as recovery_request_types, subscriptions as subscription_request_types,
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        revenue_recovery as recovery_response_types, subscriptions as subscription_response_types,
        ConnectorInfo, PaymentsResponseData, RefundsResponseData,
    },
    types::{
        ConnectorCustomerRouterData, GetSubscriptionEstimateRouterData,
        GetSubscriptionItemsRouterData, GetSubscriptionPlanPricesRouterData,
        InvoiceRecordBackRouterData, PaymentsAuthorizeRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
        SubscriptionCancelRouterData, SubscriptionCreateRouterData, SubscriptionPauseRouterData,
        SubscriptionResumeRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, payments::ConnectorCustomer, subscriptions as subscriptions_api, ConnectorCommon,
        ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use hyperswitch_masking::{Mask, PeekInterface};
use stripebilling::auth_headers;
use transformers as stripebilling;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Stripebilling {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

const STRIPE_WEBHOOK_TOLERANCE_SECONDS: i64 = 300;

#[derive(Debug)]
struct StripeBillingWebhookVerifier;

impl VerifySignature for StripeBillingWebhookVerifier {
    fn verify_signature(
        &self,
        secret: &[u8],
        signatures: &[u8],
        message: &[u8],
    ) -> CustomResult<bool, common_utils::errors::CryptoError> {
        Ok(signatures.chunks_exact(32).any(|signature| {
            common_utils::crypto::HmacSha256
                .verify_signature(secret, signature, message)
                .unwrap_or(false)
        }))
    }
}

#[derive(Debug)]
struct StripeSignatureHeader {
    timestamp: i64,
    signatures: Vec<Vec<u8>>,
}

fn parse_stripe_signature_header(
    value: &str,
    now: i64,
) -> CustomResult<StripeSignatureHeader, errors::ConnectorError> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for element in value.split(',') {
        let (key, value) = element
            .split_once('=')
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        match key {
            "t" if timestamp.is_none() => {
                timestamp = Some(
                    value
                        .parse::<i64>()
                        .change_context(errors::ConnectorError::WebhookSignatureNotFound)?,
                );
            }
            "v1" => signatures.push(
                hex::decode(value)
                    .change_context(errors::ConnectorError::WebhookSignatureNotFound)?,
            ),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
    if signatures.is_empty()
        || signatures.iter().any(|signature| signature.len() != 32)
        || now.abs_diff(timestamp) > STRIPE_WEBHOOK_TOLERANCE_SECONDS as u64
    {
        return Err(errors::ConnectorError::WebhookSourceVerificationFailed.into());
    }
    Ok(StripeSignatureHeader {
        timestamp,
        signatures,
    })
}

fn get_stripe_signature_header(
    headers: &actix_web::http::header::HeaderMap,
) -> CustomResult<StripeSignatureHeader, errors::ConnectorError> {
    let value = headers
        .get("stripe-signature")
        .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?
        .to_str()
        .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?
        .as_secs() as i64;
    parse_stripe_signature_header(value, now)
}

impl Stripebilling {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl ConnectorCustomer for Stripebilling {}

impl api::Payment for Stripebilling {}
impl api::PaymentSession for Stripebilling {}
impl api::ConnectorAccessToken for Stripebilling {}
impl api::MandateSetup for Stripebilling {}
impl api::PaymentAuthorize for Stripebilling {}
impl api::PaymentSync for Stripebilling {}
impl api::PaymentCapture for Stripebilling {}
impl api::PaymentVoid for Stripebilling {}
impl api::Refund for Stripebilling {}
impl api::RefundExecute for Stripebilling {}
impl api::RefundSync for Stripebilling {}
impl api::PaymentToken for Stripebilling {}
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl api::revenue_recovery::RevenueRecoveryRecordBack for Stripebilling {}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery::BillingConnectorPaymentsSyncIntegration for Stripebilling {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Stripebilling
{
    // Not Implemented (R)
}

macro_rules! impl_stripebilling_integration {
    (
        flow: $flow:ty,
        flow_type: $flow_type:ty,
        request: $request:ty,
        response: $response:ty,
        router_data: $router_data:ty,
        connector_response: $connector_response:ty,
        url: $url:expr,
        method: $method:expr
        $(, request_body: $request_body:expr)?
    ) => {
        impl ConnectorIntegration<$flow, $request, $response> for Stripebilling {
            fn get_headers(
                &self,
                req: &$router_data,
                connectors: &Connectors,
            ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError> {
                self.build_headers(req, connectors)
            }

            fn get_content_type(&self) -> &'static str {
                "application/x-www-form-urlencoded"
            }

            fn get_url(
                &self,
                req: &$router_data,
                connectors: &Connectors,
            ) -> CustomResult<String, errors::ConnectorError> {
                $url(self, req, connectors)
            }

            $(
                fn get_request_body(
                    &self,
                    req: &$router_data,
                    _connectors: &Connectors,
                ) -> CustomResult<RequestContent, errors::ConnectorError> {
                    $request_body(req)
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
                    .url(&<$flow_type>::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(<$flow_type>::get_headers(self, req, connectors)?);
                $(
                    let _ = $request_body;
                    builder = builder.set_body(<$flow_type>::get_request_body(self, req, connectors)?);
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
                event_builder.map(|event| event.set_response_body(&response));
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

impl subscriptions_api::Subscriptions for Stripebilling {}
impl subscriptions_api::GetSubscriptionItemsFlow for Stripebilling {}
impl subscriptions_api::SubscriptionRecordBackFlow for Stripebilling {}
impl subscriptions_api::SubscriptionCreate for Stripebilling {}
impl subscriptions_api::GetSubscriptionPlanPricesFlow for Stripebilling {}
impl subscriptions_api::GetSubscriptionEstimateFlow for Stripebilling {}
impl subscriptions_api::SubscriptionCancelFlow for Stripebilling {}
impl subscriptions_api::SubscriptionPauseFlow for Stripebilling {}
impl subscriptions_api::SubscriptionResumeFlow for Stripebilling {}

fn stripebilling_customer_body(
    req: &ConnectorCustomerRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    Ok(RequestContent::FormUrlEncoded(Box::new(
        stripebilling::StripebillingCustomerCreateRequest::try_from(req)?,
    )))
}

fn stripebilling_subscription_create_body(
    req: &SubscriptionCreateRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    Ok(RequestContent::FormUrlEncoded(Box::new(
        stripebilling::StripebillingSubscriptionCreateRequest::try_from(req)?,
    )))
}

fn stripebilling_pause_body(
    req: &SubscriptionPauseRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    match req.request.pause_option {
        None | Some(api_models::subscription::PauseOption::Immediately) => {
            Ok(RequestContent::FormUrlEncoded(Box::new(
                stripebilling::StripebillingPauseRequest { behavior: "void" },
            )))
        }
        Some(_) => Err(errors::ConnectorError::NotImplemented(
            "Stripe Billing currently supports immediate pause only".to_string(),
        )
        .into()),
    }
}

fn stripebilling_resume_body(
    req: &SubscriptionResumeRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    match req.request.resume_option {
        None | Some(api_models::subscription::ResumeOption::Immediately) => Ok(
            RequestContent::FormUrlEncoded(Box::new(stripebilling::StripebillingResumeRequest {
                pause_collection: "",
            })),
        ),
        Some(_) => Err(errors::ConnectorError::NotImplemented(
            "Stripe Billing currently supports immediate resume only".to_string(),
        )
        .into()),
    }
}

impl_stripebilling_integration!(
    flow: CreateConnectorCustomer,
    flow_type: types::ConnectorCustomerType,
    request: ConnectorCustomerData,
    response: PaymentsResponseData,
    router_data: ConnectorCustomerRouterData,
    connector_response: stripebilling::StripebillingCustomerResponse,
    url: |connector: &Stripebilling, _req: &ConnectorCustomerRouterData, connectors: &Connectors| Ok(format!("{}v1/customers", connector.base_url(connectors))),
    method: Method::Post,
    request_body: stripebilling_customer_body
);

impl_stripebilling_integration!(
    flow: subscription_flow_types::GetSubscriptionItems,
    flow_type: types::GetSubscriptionPlansType,
    request: subscription_request_types::GetSubscriptionItemsRequest,
    response: subscription_response_types::GetSubscriptionItemsResponse,
    router_data: GetSubscriptionItemsRouterData,
    connector_response: stripebilling::StripebillingListResponse<stripebilling::StripebillingProduct>,
    url: |connector: &Stripebilling, req: &GetSubscriptionItemsRouterData, connectors: &Connectors| {
        let requested = req.request.limit.unwrap_or(10).saturating_add(req.request.offset.unwrap_or(0)).clamp(1, 100);
        Ok(format!("{}v1/products?active=true&limit={requested}", connector.base_url(connectors)))
    },
    method: Method::Get
);

impl_stripebilling_integration!(
    flow: subscription_flow_types::GetSubscriptionItemPrices,
    flow_type: types::GetSubscriptionPlanPricesType,
    request: subscription_request_types::GetSubscriptionItemPricesRequest,
    response: subscription_response_types::GetSubscriptionItemPricesResponse,
    router_data: GetSubscriptionPlanPricesRouterData,
    connector_response: stripebilling::StripebillingListResponse<stripebilling::StripebillingPrice>,
    url: |connector: &Stripebilling, req: &GetSubscriptionPlanPricesRouterData, connectors: &Connectors| Ok(format!(
        "{}v1/prices?active=true&type=recurring&product={}",
        connector.base_url(connectors),
        req.request.item_price_id
    )),
    method: Method::Get
);

impl_stripebilling_integration!(
    flow: subscription_flow_types::GetSubscriptionEstimate,
    flow_type: types::GetSubscriptionEstimateType,
    request: subscription_request_types::GetSubscriptionEstimateRequest,
    response: subscription_response_types::GetSubscriptionEstimateResponse,
    router_data: GetSubscriptionEstimateRouterData,
    connector_response: stripebilling::StripebillingPrice,
    url: |connector: &Stripebilling, req: &GetSubscriptionEstimateRouterData, connectors: &Connectors| Ok(format!(
        "{}v1/prices/{}", connector.base_url(connectors), req.request.price_id
    )),
    method: Method::Get
);

impl_stripebilling_integration!(
    flow: subscription_flow_types::SubscriptionCreate,
    flow_type: types::SubscriptionCreateType,
    request: subscription_request_types::SubscriptionCreateRequest,
    response: subscription_response_types::SubscriptionCreateResponse,
    router_data: SubscriptionCreateRouterData,
    connector_response: stripebilling::StripebillingSubscriptionResponse,
    url: |connector: &Stripebilling, _req: &SubscriptionCreateRouterData, connectors: &Connectors| Ok(format!("{}v1/subscriptions", connector.base_url(connectors))),
    method: Method::Post,
    request_body: stripebilling_subscription_create_body
);

impl_stripebilling_integration!(
    flow: subscription_flow_types::SubscriptionPause,
    flow_type: types::SubscriptionPauseType,
    request: subscription_request_types::SubscriptionPauseRequest,
    response: subscription_response_types::SubscriptionPauseResponse,
    router_data: SubscriptionPauseRouterData,
    connector_response: stripebilling::StripebillingSubscriptionResponse,
    url: |connector: &Stripebilling, req: &SubscriptionPauseRouterData, connectors: &Connectors| Ok(format!(
        "{}v1/subscriptions/{}", connector.base_url(connectors), req.request.connector_subscription_id
    )),
    method: Method::Post,
    request_body: stripebilling_pause_body
);

impl_stripebilling_integration!(
    flow: subscription_flow_types::SubscriptionResume,
    flow_type: types::SubscriptionResumeType,
    request: subscription_request_types::SubscriptionResumeRequest,
    response: subscription_response_types::SubscriptionResumeResponse,
    router_data: SubscriptionResumeRouterData,
    connector_response: stripebilling::StripebillingSubscriptionResponse,
    url: |connector: &Stripebilling, req: &SubscriptionResumeRouterData, connectors: &Connectors| Ok(format!(
        "{}v1/subscriptions/{}", connector.base_url(connectors), req.request.connector_subscription_id
    )),
    method: Method::Post,
    request_body: stripebilling_resume_body
);

impl
    ConnectorIntegration<
        subscription_flow_types::SubscriptionCancel,
        subscription_request_types::SubscriptionCancelRequest,
        subscription_response_types::SubscriptionCancelResponse,
    > for Stripebilling
{
    fn get_headers(
        &self,
        req: &SubscriptionCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &SubscriptionCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/subscriptions/{}",
            self.base_url(connectors),
            req.request.connector_subscription_id
        ))
    }

    fn build_request(
        &self,
        req: &SubscriptionCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let mut builder = RequestBuilder::new()
            .url(&types::SubscriptionCancelType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::SubscriptionCancelType::get_headers(
                self, req, connectors,
            )?);

        match req.request.cancel_option.as_ref() {
            None | Some(api_models::subscription::CancelOption::Immediately) => {
                builder = builder.method(Method::Delete);
            }
            Some(api_models::subscription::CancelOption::EndOfTerm) => {
                builder = builder
                    .method(Method::Post)
                    .set_body(RequestContent::FormUrlEncoded(Box::new(
                        stripebilling::StripebillingCancelAtPeriodEndRequest {
                            cancel_at_period_end: true,
                        },
                    )));
            }
            Some(api_models::subscription::CancelOption::SpecificDate) => {
                let cancel_at = req.request.cancel_date.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "cancel_date".into(),
                    },
                )?;
                builder = builder
                    .method(Method::Post)
                    .set_body(RequestContent::FormUrlEncoded(Box::new(
                        stripebilling::StripebillingCancelAtRequest {
                            cancel_at: cancel_at.assume_utc().unix_timestamp(),
                        },
                    )));
            }
        }
        Ok(Some(builder.build()))
    }

    fn handle_response(
        &self,
        data: &SubscriptionCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SubscriptionCancelRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingSubscriptionResponse = res
            .response
            .parse_struct("StripebillingSubscriptionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|event| event.set_response_body(&response));
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

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Stripebilling
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Stripebilling {
    fn id(&self) -> &'static str {
        "stripebilling"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.stripebilling.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = stripebilling::StripebillingAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", auth.api_key.peek()).into_masked(),
            ),
            (
                auth_headers::STRIPE_API_VERSION.to_string(),
                auth_headers::STRIPE_VERSION.to_string().into_masked(),
            ),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: stripebilling::StripebillingErrorResponse = res
            .response
            .parse_struct("StripebillingErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .or(response.error.error_type)
                .unwrap_or_else(|| "stripe_billing_error".to_string()),
            message: response
                .error
                .message
                .clone()
                .unwrap_or_else(|| "Stripe Billing request failed".to_string()),
            reason: response.error.message,
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Stripebilling {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Stripebilling {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Stripebilling {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Stripebilling
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Stripebilling
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = stripebilling::StripebillingRouterData::from((amount, req));
        let connector_req =
            stripebilling::StripebillingPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingPaymentsResponse = res
            .response
            .parse_struct("Stripebilling PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingPaymentsResponse = res
            .response
            .parse_struct("stripebilling PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingPaymentsResponse = res
            .response
            .parse_struct("Stripebilling PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Stripebilling {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data =
            stripebilling::StripebillingRouterData::from((refund_amount, req));
        let connector_req =
            stripebilling::StripebillingRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::RefundExecuteType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: stripebilling::RefundResponse = res
            .response
            .parse_struct("stripebilling RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Stripebilling {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: stripebilling::RefundResponse = res
            .response
            .parse_struct("stripebilling RefundSyncResponse")
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

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegration<
        recovery_router_flows::BillingConnectorPaymentsSync,
        recovery_request_types::BillingConnectorPaymentsSyncRequest,
        recovery_response_types::BillingConnectorPaymentsSyncResponse,
    > for Stripebilling
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/charges/{}",
            self.base_url(connectors),
            req.request.billing_connector_psync_id
        ))
    }

    fn build_request(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::BillingConnectorPaymentsSyncType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::BillingConnectorPaymentsSyncType::get_headers(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        recovery_router_data_types::BillingConnectorPaymentsSyncRouterData,
        errors::ConnectorError,
    > {
        let response: stripebilling::StripebillingRecoveryDetailsData = res
            .response
            .parse_struct::<stripebilling::StripebillingRecoveryDetailsData>(
                "StripebillingRecoveryDetailsData",
            )
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        recovery_router_data_types::BillingConnectorPaymentsSyncRouterData::try_from(
            ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
        )
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "v1")]
impl
    ConnectorIntegration<
        recovery_router_flows::InvoiceRecordBack,
        recovery_request_types::InvoiceRecordBackRequest,
        recovery_response_types::InvoiceRecordBackResponse,
    > for Stripebilling
{
    fn get_headers(
        &self,
        req: &InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        req: &InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let invoice_id = req.request.merchant_reference_id.get_string_repr();
        match req.request.attempt_status {
            common_enums::AttemptStatus::Charged => Ok(format!(
                "{}v1/invoices/{invoice_id}/pay?paid_out_of_band=true",
                self.base_url(connectors)
            )),
            common_enums::AttemptStatus::Failure => Ok(format!(
                "{}v1/invoices/{invoice_id}/void",
                self.base_url(connectors)
            )),
            _ => Err(errors::ConnectorError::FailedToObtainIntegrationUrl.into()),
        }
    }

    fn build_request(
        &self,
        req: &InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::InvoiceRecordBackType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::InvoiceRecordBackType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &InvoiceRecordBackRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<InvoiceRecordBackRouterData, errors::ConnectorError> {
        let response: stripebilling::StripebillingRecordBackResponse = res
            .response
            .parse_struct("StripebillingRecordBackResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|event| event.set_response_body(&response));
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

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegration<
        recovery_router_flows::InvoiceRecordBack,
        recovery_request_types::InvoiceRecordBackRequest,
        recovery_response_types::InvoiceRecordBackResponse,
    > for Stripebilling
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let invoice_id = req
            .request
            .merchant_reference_id
            .get_string_repr()
            .to_string();
        match req.request.attempt_status {
            common_enums::AttemptStatus::Charged => Ok(format!(
                "{}/v1/invoices/{invoice_id}/pay?paid_out_of_band=true",
                self.base_url(connectors),
            )),
            common_enums::AttemptStatus::Failure => Ok(format!(
                "{}/v1/invoices/{invoice_id}/void",
                self.base_url(connectors),
            )),
            _ => Err(errors::ConnectorError::FailedToObtainIntegrationUrl.into()),
        }
    }

    fn build_request(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::InvoiceRecordBackType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::InvoiceRecordBackType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &recovery_router_data_types::InvoiceRecordBackRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<recovery_router_data_types::InvoiceRecordBackRouterData, errors::ConnectorError>
    {
        let response = res
            .response
            .parse_struct::<stripebilling::StripebillingRecordBackResponse>(
                "StripebillingRecordBackResponse",
            )
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        recovery_router_data_types::InvoiceRecordBackRouterData::try_from(ResponseRouterData {
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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Stripebilling {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(StripeBillingWebhookVerifier))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(get_stripe_signature_header(request.headers)?
            .signatures
            .into_iter()
            .flatten()
            .collect())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let timestamp = get_stripe_signature_header(request.headers)?.timestamp;
        Ok(format!("{}.{}", timestamp, String::from_utf8_lossy(request.body)).into_bytes())
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        //  For Stripe billing, we need an additional call to fetch the required recovery data. So, instead of the Invoice ID, we send the Charge ID.
        let webhook =
            stripebilling::StripebillingWebhookBody::get_webhook_object_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(
                webhook
                    .data
                    .object
                    .charge
                    .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
            ),
        ))
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let event = request
            .body
            .parse_struct::<stripebilling::StripebillingWebhookEventBody>(
                "StripebillingWebhookEventBody",
            )
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let hyperswitch_subscription_id = match event.event_type {
            stripebilling::StripebillingEventType::SubscriptionUpdated
            | stripebilling::StripebillingEventType::SubscriptionDeleted => {
                let webhook = request
                    .body
                    .parse_struct::<stripebilling::StripebillingSubscriptionWebhookBody>(
                        "StripebillingSubscriptionWebhookBody",
                    )
                    .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
                let object = webhook.data.object;
                object.metadata.get("hyperswitch_subscription_id").cloned()
            }
            _ => {
                let webhook =
                    stripebilling::StripebillingInvoiceBody::get_invoice_webhook_data_from_body(
                        request.body,
                    )?;
                let webhook_object = webhook.data.object;
                webhook_object
                    .subscription_details
                    .as_ref()
                    .or_else(|| {
                        webhook_object
                            .parent
                            .as_ref()
                            .and_then(|parent| parent.subscription_details.as_ref())
                    })
                    .and_then(|details| details.metadata.get("hyperswitch_subscription_id"))
                    .cloned()
            }
        };
        let subscription_id =
            common_utils::id_type::SubscriptionId::try_from(std::borrow::Cow::Owned(
                hyperswitch_subscription_id
                    .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
            ))
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::SubscriptionId(
            subscription_id,
        ))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook =
            stripebilling::StripebillingWebhookBody::get_webhook_object_from_body(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let event = match webhook.event_type {
            stripebilling::StripebillingEventType::PaymentSucceeded => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess
            }
            stripebilling::StripebillingEventType::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentFailure
            }
            stripebilling::StripebillingEventType::InvoiceDeleted => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryInvoiceCancel
            }
            stripebilling::StripebillingEventType::InvoiceGenerated => {
                api_models::webhooks::IncomingWebhookEvent::InvoiceGenerated
            }
            stripebilling::StripebillingEventType::SubscriptionUpdated
            | stripebilling::StripebillingEventType::SubscriptionDeleted => {
                api_models::webhooks::IncomingWebhookEvent::EventNotSupported
            }
        };
        Ok(event)
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook = request
            .body
            .parse_struct::<stripebilling::StripebillingWebhookEventBody>(
                "StripebillingWebhookEventBody",
            )
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(match webhook.event_type {
            stripebilling::StripebillingEventType::InvoiceGenerated
            | stripebilling::StripebillingEventType::PaymentSucceeded
            | stripebilling::StripebillingEventType::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::InvoiceGenerated
            }
            stripebilling::StripebillingEventType::InvoiceDeleted => {
                api_models::webhooks::IncomingWebhookEvent::EventNotSupported
            }
            stripebilling::StripebillingEventType::SubscriptionUpdated => {
                api_models::webhooks::IncomingWebhookEvent::SubscriptionUpdated
            }
            stripebilling::StripebillingEventType::SubscriptionDeleted => {
                api_models::webhooks::IncomingWebhookEvent::SubscriptionDeleted
            }
        })
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let webhook = request
            .body
            .parse_struct::<serde_json::Value>("StripebillingWebhookResource")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let webhook = stripebilling::StripebillingInvoiceBody::get_invoice_webhook_data_from_body(
            request.body,
        )
        .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_attempt_details(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryAttemptData, errors::ConnectorError> {
        // since stripe requires an additional call we dont need to implement this function because we get the recovery data from additional call itself
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_invoice_details(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<revenue_recovery::RevenueRecoveryInvoiceData, errors::ConnectorError> {
        let webhook = stripebilling::StripebillingInvoiceBody::get_invoice_webhook_data_from_body(
            request.body,
        )?;
        revenue_recovery::RevenueRecoveryInvoiceData::try_from(webhook)
    }

    fn get_subscription_mit_payment_data(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        let webhook = stripebilling::StripebillingInvoiceBody::get_invoice_webhook_data_from_body(
            request.body,
        )?;
        webhook.subscription_mit_payment_data()
    }

    fn get_subscription_webhook_data(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionWebhookData,
        errors::ConnectorError,
    > {
        request
            .body
            .parse_struct::<stripebilling::StripebillingSubscriptionWebhookBody>(
                "StripebillingSubscriptionWebhookBody",
            )
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?
            .subscription_webhook_data()
    }
}

static STRIPEBILLING_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Stripebilling",
    description: "Stripe Billing manages subscriptions, recurring payments, and invoicing. It supports trials, usage-based billing, coupons, and automated retries.",
    connector_type: enums::HyperswitchConnectorCategory::RevenueGrowthManagementPlatform,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 2] = [
    enums::EventClass::Payments,
    enums::EventClass::Subscriptions,
];

#[cfg(test)]
mod webhook_signature_tests {
    use common_utils::crypto::{SignMessage, VerifySignature};

    use super::{
        parse_stripe_signature_header, StripeBillingWebhookVerifier,
        STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS,
    };

    #[test]
    fn accepts_all_v1_signatures_during_secret_rotation() {
        let parsed = parse_stripe_signature_header(
            "t=1000,v1=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,v1=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            1100,
        )
        .expect("signature header should be parsed");

        assert_eq!(parsed.signatures.len(), 2);
    }

    #[test]
    fn rejects_stale_webhook_timestamp() {
        assert!(parse_stripe_signature_header(
            "t=1000,v1=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1301,
        )
        .is_err());
    }

    #[test]
    fn verifies_when_a_rotated_signature_is_not_the_first_v1_value() {
        let secret = b"whsec_test";
        let message = b"1000.{}";
        let valid = common_utils::crypto::HmacSha256
            .sign_message(secret, message)
            .expect("test signature should be generated");
        let mut signatures = vec![0_u8; 32];
        signatures.extend(valid);

        assert!(StripeBillingWebhookVerifier
            .verify_signature(secret, &signatures, message)
            .expect("signature verification should succeed"));
    }

    #[test]
    fn declares_payment_and_subscription_webhook_capabilities() {
        assert!(STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS
            .contains(&common_enums::enums::EventClass::Payments));
        assert!(STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS
            .contains(&common_enums::enums::EventClass::Subscriptions));
    }
}

impl ConnectorSpecifications for Stripebilling {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&STRIPEBILLING_CONNECTOR_INFO)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&STRIPEBILLING_SUPPORTED_WEBHOOK_FLOWS)
    }
}
