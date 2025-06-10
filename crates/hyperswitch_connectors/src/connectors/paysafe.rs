pub mod transformers;

use base64::Engine;
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
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask, PeekInterface};
use transformers as paysafe;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Paysafe {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Paysafe {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Paysafe {}
impl api::PaymentSession for Paysafe {}
impl api::ConnectorAccessToken for Paysafe {}
impl api::MandateSetup for Paysafe {}
impl api::PaymentAuthorize for Paysafe {}
impl api::PaymentSync for Paysafe {}
impl api::PaymentCapture for Paysafe {}
impl api::PaymentVoid for Paysafe {}
impl api::Refund for Paysafe {}
impl api::RefundExecute for Paysafe {}
impl api::RefundSync for Paysafe {}
impl api::PaymentToken for Paysafe {}
// PaymentsPreProcessing is already implemented in default_implementations.rs

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Paysafe
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Paysafe
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

impl ConnectorCommon for Paysafe {
    fn id(&self) -> &'static str {
        "paysafe"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
        // Paysafe accepts amount in minor units (cents for USD)
        // Example: $14.07 is represented as 1407
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.paysafe.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = paysafe::PaysafeAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key = common_utils::consts::BASE64_ENGINE.encode(format!("{}:{}", auth.api_key.peek(), auth.api_password.peek()));
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
        let response: paysafe::PaysafeErrorResponse = res
            .response
            .parse_struct("PaysafeErrorResponse")
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
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Paysafe {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Paysafe {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Paysafe {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Paysafe {}

impl ConnectorIntegration<hyperswitch_domain_models::router_flow_types::payments::PreProcessing, PaymentsAuthorizeData, PaymentsResponseData> for Paysafe {
    fn get_headers(
        &self,
        req: &RouterData<hyperswitch_domain_models::router_flow_types::payments::PreProcessing, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<hyperswitch_domain_models::router_flow_types::payments::PreProcessing, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/paymenthub/v1/paymenthandles", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<hyperswitch_domain_models::router_flow_types::payments::PreProcessing, PaymentsAuthorizeData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = paysafe::PaysafeRouterData::from((amount, req));
        let connector_req = paysafe::PaysafePaymentHandleRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &RouterData<hyperswitch_domain_models::router_flow_types::payments::PreProcessing, PaymentsAuthorizeData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<hyperswitch_domain_models::router_flow_types::payments::PreProcessing, PaymentsAuthorizeData, PaymentsResponseData>, errors::ConnectorError> {
        let response: paysafe::PaysafePaymentHandleResponse = res
            .response
            .parse_struct("Paysafe PaymentHandleResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        
        let connector_metadata = serde_json::to_value(response)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(RouterData {
            connector_meta_data: Some(connector_metadata.into()),
            ..data.clone()
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Paysafe {
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
        Ok(format!("{}/paymenthub/v1/payments", self.base_url(connectors)))
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

        let connector_meta_data = req
            .connector_meta_data
            .as_ref()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        
        let payment_handle: paysafe::PaysafePaymentHandleResponse = serde_json::from_value(connector_meta_data.clone().expose())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        
        let connector_router_data = paysafe::PaysafeRouterData::from((amount, req));
        let connector_req = paysafe::PaysafePaymentsRequest::try_from((&payment_handle, &connector_router_data))?;
        
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
        let response: paysafe::PaysafePaymentsResponse = res
            .response
            .parse_struct("Paysafe PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Paysafe {
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
        let connector_id = req.request.connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!("{}/v1/payments/{}", 
            self.base_url(connectors), 
            connector_id
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
        let response: paysafe::PaysafePaymentsResponse = res
            .response
            .parse_struct("paysafe PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Paysafe {
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
        Ok(format!(
            "{}/paymenthub/v1/payments/{}/settlements",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let _amount = utils::convert_amount(
            self.amount_converter,
            common_utils::types::MinorUnit::new(req.request.amount_to_capture),
            req.request.currency,
        )?;
        
        let settle_request = serde_json::json!({
            "merchantRefNum": format!("settle-{}", req.payment_id),
            "amount": req.request.amount_to_capture,
            "dupCheck": false
        });
        
        Ok(RequestContent::Json(Box::new(settle_request)))
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
        let response: paysafe::PaysafePaymentsResponse = res
            .response
            .parse_struct("Paysafe PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Paysafe {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Paysafe {
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
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let settlement_id = req.request.connector_transaction_id.to_string();
        Ok(format!(
            "{}/paymenthub/v1/settlements/{}/refunds",
            self.base_url(connectors),
            settlement_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let _refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let refund_request = serde_json::json!({
            "merchantRefNum": format!("refund-{}", req.request.refund_id),
            "amount": req.request.minor_refund_amount,
            "dupCheck": false
        });
        
        Ok(RequestContent::Json(Box::new(refund_request)))
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
        let response: paysafe::RefundResponse = res
            .response
            .parse_struct("paysafe RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Paysafe {
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
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let settlement_id = req.request.connector_transaction_id.to_string();
        let refund_id = req.request.connector_refund_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        Ok(format!(
            "{}/paymenthub/v1/settlements/{}/refunds/{}",
            self.base_url(connectors),
            settlement_id,
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
        let response: paysafe::RefundResponse = res
            .response
            .parse_struct("paysafe RefundSyncResponse")
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
impl webhooks::IncomingWebhook for Paysafe {
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

impl ConnectorSpecifications for Paysafe {}
