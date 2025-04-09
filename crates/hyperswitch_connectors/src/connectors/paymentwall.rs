pub mod transformers;

use common_enums::enums::{self, PaymentMethodType};
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, OptionExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        payments::{Authorize, Capture, PSync, PaymentMethodToken, PreProcessing, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsPreProcessingRouterData, PaymentsSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation, CaptureSyncMethod,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType, PaymentsVoidType, RefundExecuteType, Response,
        SetupMandateType,
    },
};
use masking::{ExposeInterface, Mask, Maskable, Secret};
use transformers as paymentwall;

use crate::{
    capture_method_not_supported,
    constants::{self, headers},
    types::ResponseRouterData,
    utils::{
        self as connector_utils, convert_payment_authorize_router_response,
        convert_setup_mandate_router_data_to_authorize_router_data, is_mandate_supported, ForeignTryFrom,
        PaymentMethodDataType,
    },
};

#[derive(Clone)]
pub struct Paymentwall {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Paymentwall {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl ConnectorCommon for Paymentwall {
    fn id(&self) -> &'static str {
        "paymentwall"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = paymentwall::PaymentwallAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.private_key.into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.paymentwall.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: paymentwall::PaymentwallErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.code.unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response.error.message.unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
            reason: response.error.message,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Paymentwall {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        let connector = self.id();
        match pmt {
            Some(payment_method_type) => match payment_method_type {
                PaymentMethodType::Credit | PaymentMethodType::Debit | PaymentMethodType::Card => {
                    match capture_method {
                        enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
                        enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled | enums::CaptureMethod::SequentialAutomatic => {
                            capture_method_not_supported!(connector, capture_method, payment_method_type)
                        }
                    }
                }
                _ => Err(errors::ConnectorError::NotImplemented(format!(
                    "Payment Method: {payment_method_type} not supported by {connector}"
                ))
                .into()),
            },
            None => match capture_method {
                enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
                enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled | enums::CaptureMethod::SequentialAutomatic => {
                    capture_method_not_supported!(connector, capture_method)
                }
            },
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
        ]);
        is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl api::ConnectorSpecifications for Paymentwall {
    fn get_capture_sync_method(&self) -> CaptureSyncMethod {
        CaptureSyncMethod::Manual
    }
}

impl ConnectorIntegration<api::Session, PaymentsSessionData, PaymentsResponseData> for Paymentwall {
    fn get_headers(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Session flow for Paymentwall".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Session flow for Paymentwall".to_string()).into())
    }

    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Session flow for Paymentwall".to_string()).into())
    }

    fn handle_response(
        &self,
        _data: &PaymentsSessionRouterData,
        _res: Response,
    ) -> CustomResult<PaymentsSessionRouterData, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Session flow for Paymentwall".to_string()).into())
    }

    fn get_error_response(
        &self,
        _res: Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Session flow for Paymentwall".to_string()).into())
    }
}

impl ConnectorIntegration<api::PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Paymentwall
{
    fn get_headers(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PreProcessing flow for Paymentwall".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PreProcessing flow for Paymentwall".to_string()).into())
    }

    fn build_request(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PreProcessing flow for Paymentwall".to_string()).into())
    }

    fn handle_response(
        &self,
        _data: &PaymentsPreProcessingRouterData,
        _res: Response,
    ) -> CustomResult<PaymentsPreProcessingRouterData, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PreProcessing flow for Paymentwall".to_string()).into())
    }

    fn get_error_response(
        &self,
        _res: Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("PreProcessing flow for Paymentwall".to_string()).into())
    }
}

impl ConnectorIntegration<api::PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Paymentwall
{
    fn get_headers(
        &self,
        req: &RouterData<
            api::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<
            api::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/api/brick/onetime-token", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<
            api::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = paymentwall::PaymentwallTokenRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(connector_req))
    }

    fn build_request(
        &self,
        req: &RouterData<
            api::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&self.get_url(req, connectors)?)
            .headers(self.get_headers(req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RouterData<
            api::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        res: Response,
    ) -> CustomResult<RouterData<api::PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>, errors::ConnectorError> {
        let response: paymentwall::PaymentwallTokenResponse = res
            .response
            .parse_struct("PaymentwallTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let token_response = paymentwall::PaymentwallRouterData::try_from(response)?;
        let response_data = RouterData::from(data.clone()).with_response(token_response);
        Ok(response_data)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Paymentwall {
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
        Ok(format!("{}/api/brick/charge", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = paymentwall::PaymentwallChargeRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(connector_req))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&self.get_url(req, connectors)?)
            .headers(self.get_headers(req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: paymentwall::PaymentwallChargeResponse = res
            .response
            .parse_struct("PaymentwallChargeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let is_manual_capture = data.request.capture_method == Some(enums::CaptureMethod::Manual);
        let response_data = paymentwall::PaymentwallRouterData::try_from((response, is_manual_capture))?;
        Ok(convert_payment_authorize_router_response(data, response_data))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::PSync, PaymentsSyncData, PaymentsResponseData> for Paymentwall {
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
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/api/brick/charge/{}",
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
                .url(&self.get_url(req, connectors)?)
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: paymentwall::PaymentwallChargeResponse = res
            .response
            .parse_struct("PaymentwallChargeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let is_manual_capture = data.request.capture_method == Some(enums::CaptureMethod::Manual);
        let response_data = paymentwall::PaymentwallRouterData::try_from((response, is_manual_capture))?;
        Ok(PaymentsSyncRouterData {
            response: data.response.clone(),
            data: data.data.clone(),
            http_code: res.status_code,
            amount_captured: response_data.amount_captured,
            status: response_data.status,
            connector_metadata: response_data.connector_metadata,
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

impl ConnectorIntegration<api::Capture, PaymentsCaptureData, PaymentsResponseData> for Paymentwall {
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/api/brick/charge/{}/capture",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&self.get_url(req, connectors)?)
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: paymentwall::PaymentwallChargeResponse = res
            .response
            .parse_struct("PaymentwallChargeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let response_data = paymentwall::PaymentwallRouterData::try_from((response, false))?;
        Ok(PaymentsCaptureRouterData {
            response: data.response.clone(),
            data: data.data.clone(),
            http_code: res.status_code,
            amount_captured: response_data.amount_captured,
            status: response_data.status,
            connector_metadata: response_data.connector_metadata,
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

impl ConnectorIntegration<api::Void, PaymentsCancelData, PaymentsResponseData> for Paymentwall {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/api/brick/charge/{}/void",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&self.get_url(req, connectors)?)
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: paymentwall::PaymentwallChargeResponse = res
            .response
            .parse_struct("PaymentwallChargeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let response_data = paymentwall::PaymentwallRouterData::try_from((response, false))?;
        Ok(PaymentsCancelRouterData {
            response: data.response.clone(),
            data: data.data.clone(),
            http_code: res.status_code,
            status: response_data.status,
            connector_metadata: response_data.connector_metadata,
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

impl ConnectorIntegration<api::Execute, RefundsData, RefundsResponseData> for Paymentwall {
    fn get_headers(
        &self,
        req: &RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}/api/brick/charge/{}/refund",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<api::Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = paymentwall::PaymentwallRefundRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(connector_req))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<api::Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&self.get_url(req, connectors)?)
            .headers(self.get_headers(req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: paymentwall::PaymentwallRefundResponse = res
            .response
            .parse_struct("PaymentwallRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let refund_status = enums::RefundStatus::from(response.refund.status.clone());
        Ok(RefundsRouterData {
            response: data.response.clone(),
            data: data.data.clone(),
            http_code: res.status_code,
            status: refund_status,
            connector_refund_id: Some(response.refund.id),
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

impl ConnectorIntegration<api::RSync, RefundsData, RefundsResponseData> for Paymentwall {
    fn get_headers(
        &self,
        req: &RefundsRouterData<api::RSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<api::RSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}/api/brick/refund/{}",
            self.base_url(connectors),
            refund_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<api::RSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&self.get_url(req, connectors)?)
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<api::RSync>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<api::RSync>, errors::ConnectorError> {
        let response: paymentwall::PaymentwallRefundResponse = res
            .response
            .parse_struct("PaymentwallRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let refund_status = enums::RefundStatus::from(response.refund.status.clone());
        Ok(RefundsRouterData {
            response: data.response.clone(),
            data: data.data.clone(),
            http_code: res.status_code,
            status: refund_status,
            connector_refund_id: Some(response.refund.id),
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

impl ConnectorIntegration<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Paymentwall
{
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/api/brick/charge", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let authorize_data = convert_setup_mandate_router_data_to_authorize_router_data(req)?;
        let connector_req = paymentwall::PaymentwallChargeRequest::try_from(&authorize_data)?;
        Ok(RequestContent::FormUrlEncoded(connector_req))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&self.get_url(req, connectors)?)
            .headers(self.get_headers(req, connectors)?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: paymentwall::PaymentwallChargeResponse = res
            .response
            .parse_struct("PaymentwallChargeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        router_env::logger::info!(connector_response=?response);
        
        let response_data = paymentwall::PaymentwallRouterData::try_from((response, false))?;
        Ok(SetupMandateRouterData {
            response: data.response.clone(),
            data: data.data.clone(),
            http_code: res.status_code,
            status: response_data.status,
            connector_mandate_id: response_data.connector_mandate_id.clone(),
            mandate_reference: response_data.mandate_reference.clone(),
            connector_metadata: response_data.connector_metadata,
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Paymentwall {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }

    fn get_webhook_source_verification_signature(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }

    fn get_webhook_source_verification_message(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _secret: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }

    async fn verify_webhook_source(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _secret: &[u8],
    ) -> CustomResult<bool, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }

    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into()
    }
}