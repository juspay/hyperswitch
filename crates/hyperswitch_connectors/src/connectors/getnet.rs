pub mod transformers;

use actix_web::services;
use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::{self, BASE64_ENGINE},
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt, Encode},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{
        AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector, MinorUnit,
        MinorUnitForConnector,
    },
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors::{self, ConnectorError},
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks::{self, IncomingWebhookFlowError, IncomingWebhookRequestDetails},
};
use masking::{Mask, PeekInterface};
use transformers::{self as getnet, GetnetPaymentStatus};

use crate::{
    connectors::paybox::transformers::parse_url_encoded_to_struct, constants::headers,
    types::ResponseRouterData, utils,
};

#[derive(Clone)]
pub struct Getnet {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Getnet {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for Getnet {}
impl api::PaymentSession for Getnet {}
impl api::ConnectorAccessToken for Getnet {}
impl api::MandateSetup for Getnet {}
impl api::PaymentAuthorize for Getnet {}
impl api::PaymentSync for Getnet {}
impl api::PaymentCapture for Getnet {}
impl api::PaymentVoid for Getnet {}
impl api::Refund for Getnet {}
impl api::RefundExecute for Getnet {}
impl api::RefundSync for Getnet {}
impl api::PaymentToken for Getnet {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Getnet
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Getnet
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),               // Add the Accept header
                self.get_accept_type().to_string().into(), // Value for the Accept header
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Getnet {
    fn id(&self) -> &'static str {
        "getnet"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.getnet.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = getnet::GetnetAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        println!("$$$ auth_type {:?}", auth_type);
        let encoded_api_key = common_utils::consts::BASE64_ENGINE.encode(format!(
            "{}:{}",
            auth.username.peek(),
            auth.password.peek()
        ));
        println!("$$$ encoded_api_key {:?}", encoded_api_key);

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
        println!("$$$ error {:?} ", res.response);
        let response: getnet::GetnetErrorResponse = res
            .response
            .parse_struct("GetnetErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Getnet {
    //TODO: implement functions when support enabled
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::ManualMultiple => Ok(()),
            enums::CaptureMethod::Scheduled | enums::CaptureMethod::SequentialAutomatic => Err(
                utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Getnet {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Getnet {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Getnet {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Getnet {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        println!("inside connector");
        let endpoint = self.base_url(connectors);
        Ok(format!("{endpoint}/payments/"))
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
        // println!("$$$ req {:?} ", req);

        let connector_router_data = getnet::GetnetRouterData::from((amount, req));
        // println!(
        //     "$$$ connector_router_data {:?} ",
        //     connector_router_data.router_data
        // );

        let connector_req = getnet::GetnetPaymentsRequest::try_from(&connector_router_data)?;
        // println!("$$$ request {:?} ", connector_req);
        let res = RequestContent::Json(Box::new(connector_req));
        // println!("$$$ request2 {:?} ", res);

        Ok(res)
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let header = types::PaymentsAuthorizeType::get_headers(self, req, connectors)?;
        // println!("$$$ header {:?} ", header);
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
        // println!("$$$ response {:?} {:?}", res.status_code, res.response);

        let response: getnet::GetnetPaymentsResponse = res
            .response
            .parse_struct("Getnet PaymentsAuthorizeResponse")
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
        println!("$$$ error auth {:?} ", res.response);

        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Getnet {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        // let auth = getnet::GetnetAuthType::try_from(req.connector_auth_type)
        //     .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        // let auth_type=getnet::GetnetAuthType::try_from(&req.connector_auth_type);
        // let merchant_id=auth_type.merchant_id;
        // Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())

        let endpoint = self.base_url(connectors);
        println!("$$$ get_url psync {:?}", req);
        let transaction_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(format!(
            "{endpoint}/merchants/5c4a8a42-04a8-4970-a595-262f0ba0a108/payments/{}",
            transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let url = &types::PaymentsSyncType::get_url(self, req, connectors)?;
        println!("$$$ in psync endpoint {:?} ", url);
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
        let response: getnet::GetnetPaymentsResponse = res
            .response
            .parse_struct("getnet PaymentsSyncResponse")
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
        println!("$$$ error psync {:?} ", res.response);

        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Getnet {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        println!("inside capture");
        let endpoint = self.base_url(connectors);
        Ok(format!("{endpoint}/payments/"))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        // println!("$$$ request capture {:?} ", req);

        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;

        let connector_router_data = getnet::GetnetRouterData::from((amount, req));
        let connector_req = getnet::GetnetCaptureRequest::try_from(&connector_router_data)?;
        let printrequest = Encode::encode_to_string_of_json(&connector_req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        // println!("$$$$$ {:?}", printrequest);
        // println!("$$$ request capture 2 {:?} ", connector_req);

        Ok(RequestContent::Json(Box::new(connector_req)))
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
        // println!("$$$ response capture {:?} ", res.response);

        let response: getnet::GetnetCaptureResponse = res
            .response
            .parse_struct("Getnet PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        println!("$$$ error capture {:?} ", res.response);

        self.build_error_response(res, event_builder)
    }
}
// impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Getnet {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Getnet {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        println!("$$$ inside cancel");
        let endpoint = self.base_url(connectors);
        Ok(format!("{endpoint}/payments/"))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = getnet::GetnetCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: getnet::GetnetCancelResponse = res
            .response
            .parse_struct("GetnetPaymentsVoidResponse")
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
        println!("$$$ error cancel {:?} ", res.response);

        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Getnet {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        println!("$$$ inside refund");
        let endpoint = self.base_url(connectors);
        Ok(format!("{endpoint}/payments/"))
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

        let connector_router_data = getnet::GetnetRouterData::from((refund_amount, req));
        let connector_req = getnet::GetnetRefundRequest::try_from(&connector_router_data)?;
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
        let response: getnet::RefundResponse =
            res.response
                .parse_struct("getnet RefundResponse")
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
        println!("$$$ error refund {:?} ", res.response);

        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Getnet {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let response: getnet::RefundResponse = res
            .response
            .parse_struct("getnet RefundSyncResponse")
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
        println!("$$$ error rsync {:?} ", res.response);

        self.build_error_response(res, event_builder)
    }
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<getnet::GetnetWebhookNotificationResponseBody, errors::ConnectorError> {
    // println!("$$$ Received body: {:?}", body);
    println!("$$$ in get_webhook_object_from_body");

    let body_bytes = bytes::Bytes::copy_from_slice(body.into());
    let parsed_param: getnet::GetnetWebhookNotificationResponse =
        parse_url_encoded_to_struct(body_bytes)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
    // println!("$$$ Parsed Param: {:?}", parsed_param);
    let response_base64 = &parsed_param.response_base64;
    // println!("$$$ Response Base64: {:?}", response_base64);

    let decoded_response = BASE64_ENGINE
        .decode(response_base64)
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
    // println!("$$$ Decoded Response: {:?}", decoded_response);

    let decoded_str = String::from_utf8_lossy(&decoded_response);
    // println!("$$$ decoded string: {}", decoded_str);

    let getnet_webhook_notification_response: getnet::GetnetWebhookNotificationResponseBody =
        match serde_json::from_slice::<getnet::GetnetWebhookNotificationResponseBody>(
            &decoded_response,
        ) {
            Ok(response) => {
                // Successful parsing, just return the response
                response
            }
            Err(e) => {
                // Print error only when an error occurs
                println!("Error deserializing response: {:?}", e);
                return Err(errors::ConnectorError::WebhookBodyDecodingFailed)?; // Propagate the error
            }
        };

    // let getnet_webhook_notification_response: getnet::GetnetWebhookNotificationResponseBody = decoded_response
    //     .parse_struct("GetnetWebhookNotificationResponseBody")
    //     .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
    // println!("$$$ parsing done ");

    // println!(
    //     "$$$ Final Webhook Notification Response: {:?}",
    //     getnet_webhook_notification_response
    // );

    Ok(getnet_webhook_notification_response)
}

fn get_webhook_response(
    body: &[u8],
) -> CustomResult<getnet::GetnetWebhookNotificationResponse, errors::ConnectorError> {
    println!("$$$ in get_webhook_response");

    // println!("$$$ Received body: {:?}", body);

    let body_bytes = bytes::Bytes::copy_from_slice(body.into());
    let parsed_param: getnet::GetnetWebhookNotificationResponse =
        parse_url_encoded_to_struct(body_bytes)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
    // println!("$$$ Parsed Param: {:?}", parsed_param);

    Ok(parsed_param)
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Getnet {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        println!("$$$ in get_webhook_source_verification_algorithm");

        Ok(Box::new(crypto::Sha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        println!("$$$ in get_webhook_source_verification_signature");

        let notif_item = get_webhook_response(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        // to be corrected
        // hex::decode(notif_item.response_signature_base64)
        //     .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)
        // println!(
        //     "$$$ in notif get_webhook_source_verification_signature {:?}",
        //     notif_item
        // );
        // let sanitized_base64 = notif_item.response_signature_base64.replace(" ", "");
        // println!("$$$ notif_item get_webhook_source_verification_signature {:?}", notif_item);
        // println!("$$$ in notif sanitized_base64 {:?}",sanitized_base64);

        match consts::BASE64_ENGINE.decode(&notif_item.response_base64) {
            Ok(decoded_signature) => {
                println!("$$$ Decoding succeeded: {:?}", decoded_signature);
                Ok(decoded_signature) // Return the successfully decoded signature
            }
            Err(e) => {
                println!("$$$ Decoding failed: {:?}", e);
                consts::BASE64_ENGINE
                    .decode(&notif_item.response_base64)
                    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
            }
        }
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        println!("$$$ in get_webhook_source_verification_message");

        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        // to be corrected
        // println!("$$$ notif get_webhook_source_verification_message {:?}", notif);

        let serialized_notif = serde_json::to_vec(&notif)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        Ok(serialized_notif)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        println!("$$$ in get_webhook_object_reference_id");

        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        // println!("$$$ notif get_webhook_object_reference_id {:?}", notif);

        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(
                notif.payment.transaction_id.to_string(),
            ),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        println!("$$$ in get_webhook_event_type");
        // println!("$$$ request {:?}", request);

        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        // println!("$$$ notif get_webhook_event_type {:?}", notif);

        let incoming_webhook_event = getnet::get_incoming_webhook_event(
            notif.payment.transaction_type,
            notif.payment.transaction_state,
        );
        println!(
            "$$$ incoming_webhook_event get_webhook_event_type {:?}",
            incoming_webhook_event
        );

        Ok(incoming_webhook_event)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        println!("$$$ in get_webhook_resource_object");

        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        // println!("$$$ notif get_webhook_resource_object {:?}", notif);

        Ok(Box::new(notif))
    }
}

impl ConnectorSpecifications for Getnet {}
