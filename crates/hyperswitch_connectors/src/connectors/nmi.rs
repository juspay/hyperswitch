pub mod transformers;
use std::sync::LazyLock;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::{enums, CallConnectorAction, PaymentAction};
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        AccessTokenAuth, Authorize, Capture, CompleteAuthorize, Execute, PSync, PaymentMethodToken,
        PreProcessing, RSync, Session, SetupMandate, Void,
    },
    router_request_types::{
        AccessTokenRequestData, CompleteAuthorizeData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsPreProcessingData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData,
        PaymentsSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsCompleteAuthorizeType,
        PaymentsPreProcessingType, PaymentsSyncType, PaymentsVoidType, RefundExecuteType,
        RefundSyncType, Response, SetupMandateType,
    },
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::Maskable;
use regex::Regex;
use transformers as nmi;

use crate::{
    types::ResponseRouterData,
    utils::{self, convert_amount, get_header_key_value},
};

#[derive(Clone)]
pub struct Nmi {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Nmi {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for Nmi {}
impl api::PaymentSession for Nmi {}
impl api::ConnectorAccessToken for Nmi {}
impl api::MandateSetup for Nmi {}
impl api::PaymentAuthorize for Nmi {}
impl api::PaymentSync for Nmi {}
impl api::PaymentCapture for Nmi {}
impl api::PaymentVoid for Nmi {}
impl api::Refund for Nmi {}
impl api::RefundExecute for Nmi {}
impl api::RefundSync for Nmi {}
impl api::PaymentToken for Nmi {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nmi
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(vec![(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string().into(),
        )])
    }
}

impl ConnectorCommon for Nmi {
    fn id(&self) -> &'static str {
        "nmi"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nmi.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: nmi::StandardResponse = res
            .response
            .parse_struct("StandardResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            message: response.responsetext.to_owned(),
            status_code: res.status_code,
            reason: Some(response.responsetext),
            code: response.response_code,
            attempt_status: None,
            connector_transaction_id: Some(response.transactionid),
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Nmi {
    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), ConnectorError> {
        // in case we dont have transaction id, we can make psync using attempt id
        Ok(())
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    ) -> CustomResult<(), ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            utils::PaymentMethodDataType::Card,
            utils::PaymentMethodDataType::ApplePay,
        ]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nmi
{
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nmi {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nmi {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = nmi::NmiValidateRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&SetupMandateType::get_url(self, req, connectors)?)
                .headers(SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(SetupMandateType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, ConnectorError> {
        let response: nmi::StandardResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentsPreProcessing for Nmi {}

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = nmi::NmiVaultRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .attach_default_headers()
                .headers(PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .url(&PaymentsPreProcessingType::get_url(self, req, connectors)?)
                .set_body(PaymentsPreProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &PaymentsPreProcessingRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPreProcessingRouterData, ConnectorError> {
        let response: nmi::NmiVaultResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = nmi::NmiRouterData::from((amount, req));
        let connector_req = nmi::NmiPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
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
    ) -> CustomResult<PaymentsAuthorizeRouterData, ConnectorError> {
        let response: nmi::StandardResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::PaymentsCompleteAuthorize for Nmi {}

impl ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = nmi::NmiRouterData::from((amount, req));
        let connector_req = nmi::NmiCompleteRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCompleteAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(PaymentsCompleteAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsCompleteAuthorizeType::get_request_body(
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
    ) -> CustomResult<PaymentsCompleteAuthorizeRouterData, ConnectorError> {
        let response: nmi::NmiCompleteResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/query.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = nmi::NmiSyncRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .set_body(PaymentsSyncType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, ConnectorError> {
        let response = nmi::SyncResponse::try_from(res.response.to_vec())?;

        event_builder.map(|i| i.set_response_body(&response));

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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;
        let connector_router_data = nmi::NmiRouterData::from((amount, req));
        let connector_req = nmi::NmiCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCaptureType::get_url(self, req, connectors)?)
                .headers(PaymentsCaptureType::get_headers(self, req, connectors)?)
                .set_body(PaymentsCaptureType::get_request_body(
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
    ) -> CustomResult<PaymentsCaptureRouterData, ConnectorError> {
        let response: nmi::StandardResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = nmi::NmiCancelRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, ConnectorError> {
        let response: nmi::StandardResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/transact.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let refund_amount = convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = nmi::NmiRouterData::from((refund_amount, req));
        let connector_req = nmi::NmiRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&RefundExecuteType::get_url(self, req, connectors)?)
                .headers(RefundExecuteType::get_headers(self, req, connectors)?)
                .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, ConnectorError> {
        let response: nmi::StandardResponse = serde_urlencoded::from_bytes(&res.response)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nmi {
    fn get_headers(
        &self,
        req: &RefundsRouterData<RSync>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<RSync>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}api/query.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<RSync>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = nmi::NmiSyncRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<RSync>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(RefundSyncType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<RSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<RSync>, ConnectorError> {
        let response = nmi::NmiRefundSyncResponse::try_from(res.response.to_vec())?;

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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl IncomingWebhook for Nmi {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let sig_header = get_header_key_value("webhook-signature", request.headers)?;

        let regex_pattern = r"t=(.*),s=(.*)";

        if let Some(captures) = Regex::new(regex_pattern)
            .change_context(ConnectorError::WebhookSignatureNotFound)?
            .captures(sig_header)
        {
            let signature = captures
                .get(2)
                .ok_or(ConnectorError::WebhookSignatureNotFound)?
                .as_str();

            // Decode hex signature to bytes
            hex::decode(signature).change_context(ConnectorError::WebhookSignatureNotFound)
        } else {
            Err(report!(ConnectorError::WebhookSignatureNotFound))
        }
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let sig_header = get_header_key_value("webhook-signature", request.headers)?;

        let regex_pattern = r"t=(.*),s=(.*)";

        if let Some(captures) = Regex::new(regex_pattern)
            .change_context(ConnectorError::WebhookSignatureNotFound)?
            .captures(sig_header)
        {
            let nonce = captures
                .get(1)
                .ok_or(ConnectorError::WebhookSignatureNotFound)?
                .as_str();

            let message = format!("{}.{}", nonce, String::from_utf8_lossy(request.body));

            Ok(message.into_bytes())
        } else {
            Err(report!(ConnectorError::WebhookSignatureNotFound))
        }
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        let reference_body: nmi::NmiWebhookObjectReference = request
            .body
            .parse_struct("nmi NmiWebhookObjectReference")
            .change_context(ConnectorError::WebhookResourceObjectNotFound)?;

        let object_reference_id = match reference_body.event_body.action.action_type {
            nmi::NmiActionType::Sale => api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(
                    reference_body.event_body.order_id,
                ),
            ),
            nmi::NmiActionType::Auth => api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(
                    reference_body.event_body.order_id,
                ),
            ),
            nmi::NmiActionType::Capture => api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(
                    reference_body.event_body.order_id,
                ),
            ),
            nmi::NmiActionType::Void => api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(
                    reference_body.event_body.order_id,
                ),
            ),
            nmi::NmiActionType::Refund => api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::RefundId(reference_body.event_body.order_id),
            ),
            _ => Err(ConnectorError::WebhooksNotImplemented)?,
        };

        Ok(object_reference_id)
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        let event_type_body: nmi::NmiWebhookEventBody = request
            .body
            .parse_struct("nmi NmiWebhookEventType")
            .change_context(ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(transformers::get_nmi_webhook_event(
            event_type_body.event_type,
        ))
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        let webhook_body: nmi::NmiWebhookBody = request
            .body
            .parse_struct("nmi NmiWebhookBody")
            .change_context(ConnectorError::WebhookResourceObjectNotFound)?;

        match webhook_body.event_body.action.action_type {
            nmi::NmiActionType::Sale
            | nmi::NmiActionType::Auth
            | nmi::NmiActionType::Capture
            | nmi::NmiActionType::Void
            | nmi::NmiActionType::Credit => {
                Ok(Box::new(nmi::SyncResponse::try_from(&webhook_body)?))
            }
            nmi::NmiActionType::Refund => Ok(Box::new(webhook_body)),
        }
    }
}

impl ConnectorRedirectResponse for Nmi {
    fn get_flow_type(
        &self,
        _query_params: &str,
        json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<CallConnectorAction, ConnectorError> {
        match action {
            PaymentAction::CompleteAuthorize => {
                let payload_data = json_payload.ok_or(ConnectorError::MissingRequiredField {
                    field_name: "connector_metadata",
                })?;

                let redirect_res: nmi::NmiRedirectResponse = serde_json::from_value(payload_data)
                    .change_context(
                    ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "redirect_res",
                    },
                )?;

                match redirect_res {
                    transformers::NmiRedirectResponse::NmiRedirectResponseData(_) => {
                        Ok(CallConnectorAction::Trigger)
                    }
                    transformers::NmiRedirectResponse::NmiErrorResponseData(error_res) => {
                        Ok(CallConnectorAction::StatusUpdate {
                            status: enums::AttemptStatus::Failure,
                            error_code: Some(error_res.code),
                            error_message: Some(error_res.message),
                        })
                    }
                }
            }
            PaymentAction::PSync | PaymentAction::PaymentAuthenticateCompleteAuthorize => {
                Ok(CallConnectorAction::Trigger)
            }
        }
    }
}

static NMI_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::Interac,
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::CartesBancaires,
        common_enums::CardNetwork::UnionPay,
        common_enums::CardNetwork::Maestro,
    ];

    let mut nmi_supported_payment_methods = SupportedPaymentMethods::new();

    nmi_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    nmi_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network,
                    }
                }),
            ),
        },
    );

    nmi_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    nmi_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods,
            specific_features: None,
        },
    );

    nmi_supported_payment_methods
});

static NMI_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "NMI",
    description: "NMI is a global leader in embedded payments, powering more than $200 billion in payment volumes every year.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static NMI_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 2] =
    [enums::EventClass::Payments, enums::EventClass::Refunds];

impl ConnectorSpecifications for Nmi {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&NMI_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*NMI_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&NMI_SUPPORTED_WEBHOOK_FLOWS)
    }
}
