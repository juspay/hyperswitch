pub mod transformers;

use common_utils::{
    ext_traits::ByteSliceExt,
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{Report, ResultExt};
use masking::ExposeInterface;
use transformers as razorpay;

use super::utils::{self as connector_utils};
use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers, logger,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, RequestContent, Response,
    },
    utils::BytesExt,
};

#[derive(Clone)]
pub struct Razorpay {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Razorpay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for Razorpay {}
impl api::PaymentSession for Razorpay {}
impl api::ConnectorAccessToken for Razorpay {}
impl api::MandateSetup for Razorpay {}
impl api::PaymentAuthorize for Razorpay {}
impl api::PaymentSync for Razorpay {}
impl api::PaymentCapture for Razorpay {}
impl api::PaymentVoid for Razorpay {}
impl api::Refund for Razorpay {}
impl api::RefundExecute for Razorpay {}
impl api::RefundSync for Razorpay {}
impl api::PaymentToken for Razorpay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Razorpay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Razorpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Basic {}", connectors.razorpay.api_key.clone().expose()).into_masked(),
            ),
        ];
        Ok(header)
    }
}

impl ConnectorCommon for Razorpay {
    fn id(&self) -> &'static str {
        "razorpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.razorpay.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<razorpay::ErrorResponse, Report<common_utils::errors::ParsingError>> =
            res.response.parse_struct("Razorpay ErrorResponse");

        match response {
            Ok(response_data) => {
                event_builder.map(|i| i.set_error_response_body(&response_data));
                router_env::logger::info!(connector_response=?response_data);
                match response_data {
                    razorpay::ErrorResponse::RazorpayErrorResponse(error_response) => {
                        Ok(ErrorResponse {
                            status_code: res.status_code,
                            code: error_response.error_info.code,
                            message: error_response
                                .error_info
                                .fields
                                .iter()
                                .map(|error| error.field_name.clone())
                                .collect::<Vec<String>>()
                                .join(" & "),
                            reason: Some(
                                error_response
                                    .error_info
                                    .fields
                                    .iter()
                                    .map(|error| error.field_name.clone())
                                    .collect::<Vec<String>>()
                                    .join(" & "),
                            ),
                            attempt_status: None,
                            connector_transaction_id: None,
                        })
                    }
                    razorpay::ErrorResponse::RazorpayStringError(error_string) => {
                        Ok(ErrorResponse {
                            status_code: res.status_code,
                            code: consts::NO_ERROR_CODE.to_string(),
                            message: error_string.clone(),
                            reason: Some(error_string.clone()),
                            attempt_status: None,
                            connector_transaction_id: None,
                        })
                    }
                }
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                logger::error!(deserialization_error =? error_msg);
                crate::utils::handle_json_response_deserialization_failure(res, "razorpay")
            }
        }
    }
}

impl ConnectorValidation for Razorpay {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Razorpay
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Razorpay
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Razorpay
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Razorpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}gatewayProxy/txn/sendCollect",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req =
            razorpay::RazorpayPaymentsRequest::try_from((&connector_router_data, connectors))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: razorpay::RazorpayPaymentsResponse = res
            .response
            .parse_struct("Razorpay PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Razorpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}gatewayProxy/sync/transaction",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.amount,
            req.request.currency,
        )?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req =
            razorpay::RazorpayCreateSyncRequest::try_from((connector_router_data, connectors))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
            .set_body(types::PaymentsSyncType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: razorpay::RazorpaySyncResponse = res
            .response
            .parse_struct("razorpay PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Razorpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: razorpay::RazorpayPaymentsResponse = res
            .response
            .parse_struct("Razorpay PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Razorpay
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Razorpay
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}gatewayProxy/refund", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req =
            razorpay::RazorpayRefundRequest::try_from((&connector_router_data, connectors))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        data: &types::RefundsRouterData<api::Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: razorpay::RefundResponse = res
            .response
            .parse_struct("razorpay RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}gatewayProxy/sync/refund",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = connector_utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req =
            razorpay::RazorpayRefundRequest::try_from((&connector_router_data, connectors))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: razorpay::RefundResponse = res
            .response
            .parse_struct("razorpay RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
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
impl api::IncomingWebhook for Razorpay {
    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_resource_object = get_webhook_object_from_body(request.body)?;
        match webhook_resource_object.refund {
            Some(refund_data) => Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(refund_data.entity.id),
            )),
            None => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    webhook_resource_object.payment.entity.id,
                ),
            )),
        }
    }

    async fn verify_webhook_source(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: common_utils::crypto::Encryptable<
            masking::Secret<serde_json::Value>,
        >,
        _connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        Ok(false)
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook_resource_object = get_webhook_object_from_body(request.body)?;
        Ok(api_models::webhooks::IncomingWebhookEvent::try_from(
            webhook_resource_object,
        )?)
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_resource_object = get_webhook_object_from_body(request.body)?;
        Ok(Box::new(webhook_resource_object))
    }
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<razorpay::RazorpayWebhookPayload, errors::ConnectorError> {
    let details: razorpay::RazorpayWebhookEvent = body
        .parse_struct("RazorpayWebhookEvent")
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    Ok(details.payload)
}
