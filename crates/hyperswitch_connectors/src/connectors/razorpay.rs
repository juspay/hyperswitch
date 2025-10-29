pub mod transformers;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, CreateOrder, PSync, PaymentMethodToken, Session, SetupMandate, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, CreateOrderRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        CreateOrderRouterData, PaymentsAuthorizeRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::NO_ERROR_CODE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, CreateOrderType, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use lazy_static::lazy_static;
use masking::{Mask, Maskable, PeekInterface};
use router_env::logger;
use transformers as razorpay;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{convert_amount, handle_json_response_deserialization_failure},
};

#[derive(Clone)]
pub struct Razorpay {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Razorpay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Razorpay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Razorpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
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

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.razorpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = razorpay::RazorpayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key = common_utils::consts::BASE64_ENGINE.encode(format!(
            "{}:{}",
            auth.razorpay_id.peek(),
            auth.razorpay_secret.peek()
        ));
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
                            code: error_response.error.code,
                            message: error_response.error.description,
                            reason: error_response.error.reason,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    }
                    razorpay::ErrorResponse::RazorpayError(error_response) => Ok(ErrorResponse {
                        status_code: res.status_code,
                        code: error_response.message.clone(),
                        message: error_response.message.clone(),
                        reason: Some(error_response.message),
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    razorpay::ErrorResponse::RazorpayStringError(error_string) => {
                        Ok(ErrorResponse {
                            status_code: res.status_code,
                            code: NO_ERROR_CODE.to_string(),
                            message: error_string.clone(),
                            reason: Some(error_string.clone()),
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    }
                }
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                logger::error!(deserialization_error =? error_msg);
                handle_json_response_deserialization_failure(res, "razorpay")
            }
        }
    }
}

impl ConnectorValidation for Razorpay {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Razorpay {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Razorpay {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Razorpay
{
}

impl api::PaymentsCreateOrder for Razorpay {}

impl ConnectorIntegration<CreateOrder, CreateOrderRequestData, PaymentsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &CreateOrderRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &CreateOrderRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/orders", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &CreateOrderRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount = req.request.minor_amount;
        let currency = req.request.currency;
        let amount = convert_amount(self.amount_converter, minor_amount, currency)?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req = razorpay::RazorpayOrderRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &CreateOrderRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(CreateOrderType::get_headers(self, req, connectors)?)
                .url(&CreateOrderType::get_url(self, req, connectors)?)
                .set_body(CreateOrderType::get_request_body(self, req, connectors)?)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &CreateOrderRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<CreateOrderRouterData, errors::ConnectorError> {
        let response: razorpay::RazorpayOrderResponse = res
            .response
            .parse_struct("RazorpayOrderResponse")
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match _req.request.payment_method_data {
            PaymentMethodData::Upi(_) => Ok(format!(
                "{}v1/payments/create/upi",
                self.base_url(connectors)
            )),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not implemented for Razorpay".to_string(),
            )
            .into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req = razorpay::RazorpayPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: razorpay::RazorpayPaymentsResponse = res
            .response
            .parse_struct("Razorpay PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let order_id = req
            .request
            .connector_reference_id
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(format!(
            "{}v1/orders/{}/payments",
            self.base_url(connectors),
            order_id,
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
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
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: razorpay::RazorpaySyncResponse = res
            .response
            .parse_struct("razorpay PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        _data: &PaymentsCaptureRouterData,
        _event_builder: Option<&mut ConnectorEvent>,
        _res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Razorpay {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v1/payments/{}/refund",
            self.base_url(connectors),
            req.request.connector_transaction_id,
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_router_data = razorpay::RazorpayRouterData::try_from((amount, req))?;
        let connector_req = razorpay::RazorpayRefundRequest::try_from(&connector_router_data)?;
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
        let response: razorpay::RazorpayRefundResponse = res
            .response
            .parse_struct("razorpay RazorpayRefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Razorpay {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req
            .request
            .connector_refund_id
            .to_owned()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;
        Ok(format!(
            "{}v1/payments/{}/refunds/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id,
            refund_id
        ))
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
        let response: razorpay::RazorpayRefundResponse = res
            .response
            .parse_struct("razorpay RazorpayRefundResponse")
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

// This code can be used later when Razorpay webhooks are implemented

// #[async_trait::async_trait]
// impl IncomingWebhook for Razorpay {
//     fn get_webhook_object_reference_id(
//         &self,
//         request: &IncomingWebhookRequestDetails<'_>,
//     ) -> CustomResult<webhooks::ObjectReferenceId, errors::ConnectorError> {
//         let webhook_resource_object: razorpay::RazorpayWebhookPayload = request
//             .body
//             .parse_struct("RazorpayWebhookPayload")
//             .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

//         match webhook_resource_object.payload.refund {
//             Some(refund_data) => Ok(webhooks::ObjectReferenceId::RefundId(
//                 webhooks::RefundIdType::ConnectorRefundId(refund_data.entity.id),
//             )),
//             None => Ok(webhooks::ObjectReferenceId::PaymentId(
//                 api_models::payments::PaymentIdType::ConnectorTransactionId(
//                     webhook_resource_object.payload.payment.entity.id,
//                 ),
//             )),
//         }
//     }

//     async fn verify_webhook_source(
//         &self,
//         _request: &IncomingWebhookRequestDetails<'_>,
//         _merchant_id: &common_utils::id_type::MerchantId,
//         _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
//         _connector_account_details: common_utils::crypto::Encryptable<
//             masking::Secret<serde_json::Value>,
//         >,
//         _connector_label: &str,
//     ) -> CustomResult<bool, errors::ConnectorError> {
//         Ok(false)
//     }

//     fn get_webhook_event_type(
//         &self,
//         request: &IncomingWebhookRequestDetails<'_>,
//     ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
//         let payload: razorpay::RazorpayWebhookEventType = request
//             .body
//             .parse_struct("RazorpayWebhookEventType")
//             .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
//         Ok(IncomingWebhookEvent::try_from(payload)?)
//     }

//     fn get_webhook_resource_object(
//         &self,
//         request: &IncomingWebhookRequestDetails<'_>,
//     ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
//         let details: razorpay::RazorpayWebhookPayload = request
//             .body
//             .parse_struct("RazorpayWebhookPayload")
//             .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
//         Ok(Box::new(details))
//     }
// }

#[async_trait::async_trait]
impl IncomingWebhook for Razorpay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

lazy_static! {
    static ref RAZORPAY_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut razorpay_supported_payment_methods = SupportedPaymentMethods::new();
        razorpay_supported_payment_methods.add(
            enums::PaymentMethod::Upi,
            enums::PaymentMethodType::UpiCollect,
            PaymentMethodDetails{
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        razorpay_supported_payment_methods
    };

    static ref RAZORPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "RAZORPAY",
        description:
            "Razorpay helps you accept online payments from customers across Desktop, Mobile web, Android & iOS. Additionally by using Razorpay Payment Links, you can collect payments across multiple channels like SMS, Email, Whatsapp, Chatbots & Messenger.",
        connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
        integration_status: enums::ConnectorIntegrationStatus::Sandbox,
    };

    static ref RAZORPAY_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = vec![enums::EventClass::Payments, enums::EventClass::Refunds];

}

impl ConnectorSpecifications for Razorpay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*RAZORPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*RAZORPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*RAZORPAY_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "v2")]
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> String {
        // The length of receipt for Razorpay order request should not exceed 40 characters.
        payment_intent
            .merchant_reference_id
            .as_ref()
            .map(|id| id.get_string_repr().to_owned())
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string())
    }
}
