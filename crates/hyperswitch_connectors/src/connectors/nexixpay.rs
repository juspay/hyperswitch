pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, PreProcessing, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void, CompleteAuthorize},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsPreProcessingData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData, CompleteAuthorizeData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData, PaymentsCancelRouterData
    },
};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorValidation},
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask};
use transformers as nexixpay;
use uuid::Uuid;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Nexixpay {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Nexixpay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Nexixpay {}
impl api::PaymentsPreProcessing for Nexixpay{}
impl api::PaymentSession for Nexixpay {}
impl api::ConnectorAccessToken for Nexixpay {}
impl api::MandateSetup for Nexixpay {}
impl api::PaymentAuthorize for Nexixpay {}
impl api::PaymentSync for Nexixpay {}
impl api::PaymentCapture for Nexixpay {}
impl api::PaymentVoid for Nexixpay {}
impl api::Refund for Nexixpay {}
impl api::RefundExecute for Nexixpay {}
impl api::RefundSync for Nexixpay {}
impl api::PaymentToken for Nexixpay {}
impl api::PaymentsCompleteAuthorize for Nexixpay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nexixpay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nexixpay
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
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Nexixpay {
    fn id(&self) -> &'static str {
        "nexixpay"
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
        connectors.nexixpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = nexixpay::NexixpayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.api_key.expose().into_masked(),
        ),
        (
            headers::CORRELATION_ID.to_string(),
            Uuid::new_v4().to_string().into_masked()
        )
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        ////TODO: error handling -> struct
        let resp = res.clone();
        println!("*******redirect_payload{:?}",resp);
        let response: nexixpay::NexixpayErrorResponse = resp
            .response
            .parse_struct("NexixpayErrorResponse")
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

impl ConnectorValidation for Nexixpay {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nexixpay {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nexixpay {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Nexixpay
{
}

impl
    ConnectorIntegration< PreProcessing, PaymentsPreProcessingData, PaymentsResponseData,> for Nexixpay
    {
        fn get_headers(
            &self,
            req: &PaymentsPreProcessingRouterData,
            connectors: &Connectors,
        ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
            self.build_headers(req, connectors)
        }
    
        fn get_content_type(&self) -> &'static str {
            self.common_get_content_type()
        }
    
        fn get_url(
            &self,
            _req: &PaymentsPreProcessingRouterData,
            connectors: &Connectors,
        ) -> CustomResult<String, errors::ConnectorError> {
            Ok(format!(
                "{}/orders/3steps/validation",
                self.base_url(connectors)
            ))
        }
    
        fn get_request_body(
            &self,
            req: &PaymentsPreProcessingRouterData,
            _connectors: &Connectors,
        ) -> CustomResult<RequestContent, errors::ConnectorError> {
            let connector_req = nexixpay::NexixpayPreProcessingRequest::try_from(req)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        }
    
        fn build_request(
            &self,
            req: &PaymentsPreProcessingRouterData,
            connectors: &Connectors,
        ) -> CustomResult<Option<Request>, errors::ConnectorError> {
            Ok(Some(
                RequestBuilder::new()
                    .method(Method::Post)
                    .url(&types::PaymentsPreProcessingType::get_url(
                        self, req, connectors,
                    )?)
                    .attach_default_headers()
                    .headers(types::PaymentsPreProcessingType::get_headers(
                        self, req, connectors,
                    )?)
                    .set_body(types::PaymentsPreProcessingType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            ))
        }
    
        fn handle_response(
            &self,
            data: &PaymentsPreProcessingRouterData,
            event_builder: Option<&mut ConnectorEvent>,
            res: Response,
        ) -> CustomResult<PaymentsPreProcessingRouterData, errors::ConnectorError> {
            let response: nexixpay::NexixpayPreProcessingResponse = res
                .response
                .parse_struct("NexixpayPreProcessingResponse")
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

    impl ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData> for Nexixpay {
        fn get_headers(
            &self,
            req: &PaymentsCompleteAuthorizeRouterData,
            connectors: &Connectors,
        ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
            self.build_headers(req, connectors)
        }
    
        fn get_content_type(&self) -> &'static str {
            self.common_get_content_type()
        }
    
        fn get_url(
            &self,
            _req: &PaymentsCompleteAuthorizeRouterData,
            connectors: &Connectors,
        ) -> CustomResult<String, errors::ConnectorError> {
            Ok(format!(
                "{}/orders/3steps/payment",
                self.base_url(connectors)
            ))
        }
    
        fn get_request_body(
            &self,
            req: &PaymentsCompleteAuthorizeRouterData,
            _connectors: &Connectors,
        ) -> CustomResult<RequestContent, errors::ConnectorError> {
            let amount = utils::convert_amount(
                self.amount_converter,
                req.request.minor_amount,
                req.request.currency,
            )?;
            let connector_router_data = nexixpay::NexixpayRouterData::from((amount, req));
            let connector_req = nexixpay::NexixpayCompleteAuthorizeRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        }
    
        fn build_request(
            &self,
            req: &PaymentsCompleteAuthorizeRouterData,
            connectors: &Connectors,
        ) -> CustomResult<Option<Request>, errors::ConnectorError> {
            Ok(Some(
                RequestBuilder::new()
                    .method(Method::Post)
                    .url(&types::PaymentsCompleteAuthorizeType::get_url(
                        self, req, connectors,
                    )?)
                    .attach_default_headers()
                    .headers(types::PaymentsCompleteAuthorizeType::get_headers(
                        self, req, connectors,
                    )?)
                    .set_body(types::PaymentsCompleteAuthorizeType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            ))
        }
    
        fn handle_response(
            &self,
            data: &PaymentsCompleteAuthorizeRouterData,
            event_builder: Option<&mut ConnectorEvent>,
            res: Response,
        ) -> CustomResult<PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
            let response: nexixpay::NexixpayCompleteAuthorizeResponse = res
                .response
                .parse_struct("NexixpayCompleteAuthorizeResponse")
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nexixpay {
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
        Ok(format!(
            "{}/orders/3steps/init",
            self.base_url(connectors)
        ))
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

        let connector_router_data = nexixpay::NexixpayRouterData::from((amount, req));
        let connector_req = nexixpay::NexixpayPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: nexixpay::NexixpayPaymentsResponse = res
            .response
            .parse_struct("NexixpayPaymentsResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nexixpay {
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
        let connector_payment_id = req.request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/operations/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
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
        let response: nexixpay::NexixpayTransactionResponse = res
            .response
            .parse_struct("NexixpayTransactionResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nexixpay {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request
            .connector_transaction_id.clone();
        Ok(format!(
            "{}/operations/{}/captures",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data = nexixpay::NexixpayRouterData::from((amount, req));
        let connector_req =
            nexixpay::NexixpayPaymentsCaptureRequest::try_from(&connector_router_data)?;
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
        let response: nexixpay::NexixpayPaymentsCaptureResponse = res
            .response
            .parse_struct("NexixpayPaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nexixpay {
    // fn get_headers(
    //     &self,
    //     req: &PaymentsCancelRouterData,
    //     connectors: &Connectors,
    // ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    //     self.build_headers(req, connectors)
    // }

    // fn get_content_type(&self) -> &'static str {
    //     self.common_get_content_type()
    // }

    // fn get_url(
    //     &self,
    //     req: &PaymentsCancelRouterData,
    //     connectors: &Connectors,
    // ) -> CustomResult<String, errors::ConnectorError> {
    //     let connector_payment_id = req.request
    //         .connector_transaction_id.clone();
    //     Ok(format!(
    //         "{}/operations/{}/cancels",
    //         self.base_url(connectors),
    //         connector_payment_id
    //     ))
    // }

    // fn get_request_body(
    //     &self,
    //     req: &PaymentsCancelRouterData,
    //     _connectors: &Connectors,
    // ) -> CustomResult<RequestContent, errors::ConnectorError> {
    //     let description = req.request.cancellation_reason;
    //     let connector_req =
    //         nexixpay::NexixpayPaymentsCancleRequest::try_from(&description)?;
    //     Ok(RequestContent::Json(Box::new(connector_req)))
    // }

    // fn build_request(
    //     &self,
    //     req: &PaymentsCancelRouterData,
    //     connectors: &Connectors,
    // ) -> CustomResult<Option<Request>, errors::ConnectorError> {
    //     Ok(Some(
    //         RequestBuilder::new()
    //             .method(Method::Post)
    //             .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
    //             .attach_default_headers()
    //             .headers(types::PaymentsCaptureType::get_headers(
    //                 self, req, connectors,
    //             )?)
    //             .set_body(types::PaymentsCaptureType::get_request_body(
    //                 self, req, connectors,
    //             )?)
    //             .build(),
    //     ))
    // }

    // fn handle_response(
    //     &self,
    //     data: &PaymentsCancelRouterData,
    //     event_builder: Option<&mut ConnectorEvent>,
    //     res: Response,
    // ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
    //     let response: nexixpay::NexixpayPaymentsCaptureResponse = res
    //         .response
    //         .parse_struct("NexixpayPaymentsCaptureResponse")
    //         .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    //     event_builder.map(|i| i.set_response_body(&response));
    //     router_env::logger::info!(connector_response=?response);
    //     RouterData::try_from(ResponseRouterData {
    //         response,
    //         data: data.clone(),
    //         http_code: res.status_code,
    //     })
    // }

    // fn get_error_response(
    //     &self,
    //     res: Response,
    //     event_builder: Option<&mut ConnectorEvent>,
    // ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    //     self.build_error_response(res, event_builder)
    // }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nexixpay {
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

        let connector_router_data = nexixpay::NexixpayRouterData::from((refund_amount, req));
        let connector_req = nexixpay::NexixpayRefundRequest::try_from(&connector_router_data)?;
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
        let response: nexixpay::RefundResponse = res
            .response
            .parse_struct("nexixpay RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nexixpay {
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
        let response: nexixpay::RefundResponse = res
            .response
            .parse_struct("nexixpay RefundSyncResponse")
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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Nexixpay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
