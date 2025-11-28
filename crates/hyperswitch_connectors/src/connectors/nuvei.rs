pub mod transformers;
use std::sync::LazyLock;

#[cfg(feature = "payouts")]
use api_models::webhooks::PayoutIdType;
use api_models::{
    payments::PaymentIdType,
    webhooks::{IncomingWebhookEvent, RefundIdType},
};
use common_enums::{enums, CallConnectorAction, PaymentAction};
use common_utils::{
    crypto,
    errors::{CustomResult, ReportSwitchExt},
    ext_traits::{ByteSliceExt, BytesExt, ValueExt},
    id_type,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{
        AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector, StringMajorUnit,
        StringMajorUnitForConnector, StringMinorUnit, StringMinorUnitForConnector,
    },
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        AuthorizeSessionToken, CompleteAuthorize, PostCaptureVoid, PreProcessing,
    },
    router_request_types::{
        AccessTokenRequestData, AuthorizeSessionTokenData, CompleteAuthorizeData,
        PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCancelPostCaptureData, PaymentsCaptureData, PaymentsPreProcessingData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsAuthorizeSessionTokenRouterData,
        PaymentsCancelPostCaptureRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData,
        PaymentsSyncRouterData, RefundsRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::PoFulfill, router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData, types::PayoutsRouterData,
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    disputes, errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::ExposeInterface;
use transformers as nuvei;

use crate::{
    connectors::nuvei::transformers::{NuveiPaymentsResponse, NuveiTransactionSyncResponse},
    constants::headers,
    types::ResponseRouterData,
    utils::{self, is_mandate_supported, PaymentMethodDataType, RouterData as _},
};

#[derive(Clone)]
pub struct Nuvei {
    pub amount_convertor: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
    amount_converter_string_minor_unit:
        &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
    amount_converter_float_major_unit:
        &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}
impl Nuvei {
    pub fn new() -> &'static Self {
        &Self {
            amount_convertor: &StringMajorUnitForConnector,
            amount_converter_string_minor_unit: &StringMinorUnitForConnector,
            amount_converter_float_major_unit: &FloatMajorUnitForConnector,
        }
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nuvei
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(headers)
    }
}

impl ConnectorCommon for Nuvei {
    fn id(&self) -> &'static str {
        "nuvei"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nuvei.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }
}

impl ConnectorValidation for Nuvei {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::GooglePay,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::NetworkTransactionIdAndCardDetails,
        ]);
        is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl api::Payment for Nuvei {}

impl api::PaymentToken for Nuvei {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nuvei
{
    // Not Implemented (R)
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nuvei {}

impl api::MandateSetup for Nuvei {}
impl api::PaymentVoid for Nuvei {}
impl api::PaymentSync for Nuvei {}
impl api::PaymentCapture for Nuvei {}
impl api::PaymentSession for Nuvei {}
impl api::PaymentAuthorize for Nuvei {}
impl api::PaymentAuthorizeSessionToken for Nuvei {}
impl api::Refund for Nuvei {}
impl api::RefundExecute for Nuvei {}
impl api::RefundSync for Nuvei {}
impl api::PaymentsCompleteAuthorize for Nuvei {}
impl api::ConnectorAccessToken for Nuvei {}
impl api::PaymentsPreProcessing for Nuvei {}
impl api::PaymentPostCaptureVoid for Nuvei {}

impl api::Payouts for Nuvei {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Nuvei {}

#[async_trait::async_trait]
#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Nuvei {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/payout.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPayoutRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutFulfillType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutFulfillType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: nuvei::NuveiPayoutResponse =
            res.response.parse_struct("NuveiPayoutResponse").switch()?;
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Nuvei {
    fn get_headers(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/payment.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPaymentsRequest::try_from((req, req.get_session_token()?))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(types::SetupMandateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nuvei {
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
        Ok(format!(
            "{}ppp/api/v1/voidTransaction.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .set_body(types::PaymentsVoidType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
    for Nuvei
{
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
            "{}ppp/api/v1/payment.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsCompleteAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let meta: nuvei::NuveiMeta = utils::to_connector_meta(req.request.connector_meta.clone())?;
        let connector_req = nuvei::NuveiPaymentsRequest::try_from((req, meta.session_token))?;
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
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &PaymentsCancelPostCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelPostCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/voidTransaction.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelPostCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiVoidRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelPostCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PaymentsPostCaptureVoidType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PaymentsPostCaptureVoidType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PaymentsPostCaptureVoidType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelPostCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelPostCaptureRouterData, errors::ConnectorError> {
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
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
        self.build_error_response(res, event_builder)
    }
}
impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nuvei {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nuvei {
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
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/getTransactionDetails.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPaymentSyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let nuvie_psync_common_response: nuvei::NuveiPaymentSyncResponse = res
            .response
            .parse_struct("NuveiPaymentSyncResponse")
            .switch()?;

        event_builder.map(|i| i.set_response_body(&nuvie_psync_common_response));
        router_env::logger::info!(connector_response=?nuvie_psync_common_response);
        let response = NuveiTransactionSyncResponse::from(nuvie_psync_common_response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nuvei {
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
        Ok(format!(
            "{}ppp/api/v1/settleTransaction.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
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
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;

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
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nuvei {
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
            "{}ppp/api/v1/payment.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPaymentsRequest::try_from((req, req.get_session_token()?))?;
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
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;

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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeSessionTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeSessionTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/getSessionToken.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeSessionTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiSessionRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeSessionTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsPreAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPreAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsPreAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeSessionTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeSessionTokenRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiSessionResponse =
            res.response.parse_struct("NuveiSessionResponse").switch()?;

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

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Nuvei
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
            "{}ppp/api/v1/initPayment.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            nuvei::NuveiThreeDSInitPaymentRequest::try_from((req, req.get_session_token()?))?;
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
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nuvei {
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
        Ok(format!(
            "{}ppp/api/v1/refundTransaction.do",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
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
        let response: NuveiPaymentsResponse = res
            .response
            .parse_struct("NuveiPaymentsResponse")
            .switch()?;
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nuvei {
    fn handle_response(
        &self,
        data: &RefundsRouterData<RSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<RSync, RefundsData, RefundsResponseData>, errors::ConnectorError>
    {
        let nuvie_rsync_common_response: nuvei::PaymentDmnNotification = res
            .response
            .parse_struct("PaymentDmnNotification")
            .switch()?;
        event_builder.map(|i| i.set_response_body(&nuvie_rsync_common_response));
        router_env::logger::info!(connector_response=?nuvie_rsync_common_response);
        let response = NuveiTransactionSyncResponse::from(nuvie_rsync_common_response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

fn has_payout_prefix(id_option: &Option<String>) -> bool {
    // - Default value returns false if the Option is `None`.
    // - The argument is a closure that runs if the Option is `Some`.
    //   It takes the contained value (`s`) and its result is returned.
    id_option
        .as_deref()
        .is_some_and(|s| s.starts_with("payout_"))
}

#[async_trait::async_trait]
impl IncomingWebhook for Nuvei {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Sha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let webhook = get_webhook_object_from_body(request.body)?;

        let nuvei_notification_signature = match webhook {
            nuvei::NuveiWebhook::PaymentDmn(notification) => notification
                .advance_response_checksum
                .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?,
            nuvei::NuveiWebhook::Chargeback(_) => {
                utils::get_header_key_value("Checksum", request.headers)?.to_string()
            }
        };

        hex::decode(nuvei_notification_signature)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // Parse the webhook payload
        let webhook = get_webhook_object_from_body(request.body)?;
        let secret_str = std::str::from_utf8(&connector_webhook_secrets.secret)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        // Generate signature based on webhook type
        match webhook {
            nuvei::NuveiWebhook::PaymentDmn(notification) => {
                // For payment DMNs, use the same format as before
                let status = notification
                    .status
                    .as_ref()
                    .map(|s| format!("{s:?}").to_uppercase())
                    .unwrap_or_default();

                let to_sign = transformers::concat_strings(&[
                    secret_str.to_string(),
                    notification.total_amount,
                    notification.currency,
                    notification.response_time_stamp,
                    notification.ppp_transaction_id,
                    status,
                    notification.product_id.unwrap_or("NA".to_string()),
                ]);
                Ok(to_sign.into_bytes())
            }
            nuvei::NuveiWebhook::Chargeback(notification) => {
                // For chargeback notifications, use a different format based on Nuvei's documentation
                // Note: This is a placeholder - you'll need to adjust based on Nuvei's actual chargeback signature format
                let response = serde_json::to_string(&notification)
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

                let to_sign = format!("{secret_str}{response}");
                Ok(to_sign.into_bytes())
            }
        }
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        // Parse the webhook payload
        let webhook = get_webhook_object_from_body(request.body)?;
        // Extract transaction ID from the webhook
        match &webhook {
            nuvei::NuveiWebhook::PaymentDmn(notification) => {
                // if prefix contains 'payout_' then it is a payout related webhook
                if has_payout_prefix(&notification.client_request_id) {
                    #[cfg(feature = "payouts")]
                    {
                        Ok(api_models::webhooks::ObjectReferenceId::PayoutId(
                            PayoutIdType::PayoutAttemptId(
                                notification
                                    .client_request_id
                                    .clone()
                                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                            ),
                        ))
                    }
                    #[cfg(not(feature = "payouts"))]
                    {
                        Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
                    }
                } else {
                    match notification.transaction_type {
                        Some(nuvei::NuveiTransactionType::Auth)
                        | Some(nuvei::NuveiTransactionType::Sale)
                        | Some(nuvei::NuveiTransactionType::Settle)
                        | Some(nuvei::NuveiTransactionType::Void)
                        | Some(nuvei::NuveiTransactionType::Auth3D)
                        | Some(nuvei::NuveiTransactionType::InitAuth3D) => {
                            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                                PaymentIdType::ConnectorTransactionId(
                                    notification.transaction_id.clone().ok_or(
                                        errors::ConnectorError::MissingConnectorTransactionID,
                                    )?,
                                ),
                            ))
                        }
                        Some(nuvei::NuveiTransactionType::Credit) => {
                            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                                RefundIdType::ConnectorRefundId(
                                    notification
                                        .transaction_id
                                        .clone()
                                        .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
                                ),
                            ))
                        }
                        None => Err(errors::ConnectorError::WebhookEventTypeNotFound.into()),
                    }
                }
            }
            nuvei::NuveiWebhook::Chargeback(notification) => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    PaymentIdType::ConnectorTransactionId(
                        notification.transaction_details.transaction_id.to_string(),
                    ),
                ))
            }
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        // Parse the webhook payload
        let webhook = get_webhook_object_from_body(request.body)?;

        // Map webhook type to event type
        match webhook {
            nuvei::NuveiWebhook::PaymentDmn(notification) => {
                if has_payout_prefix(&notification.client_request_id) {
                    #[cfg(feature = "payouts")]
                    {
                        if let Some((status, transaction_type)) =
                            notification.status.zip(notification.transaction_type)
                        {
                            nuvei::map_notification_to_event_for_payout(status, transaction_type)
                        } else {
                            Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
                        }
                    }
                    #[cfg(not(feature = "payouts"))]
                    {
                        Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
                    }
                } else if let Some((status, transaction_type)) =
                    notification.status.zip(notification.transaction_type)
                {
                    nuvei::map_notification_to_event(status, transaction_type)
                } else {
                    Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
                }
            }
            nuvei::NuveiWebhook::Chargeback(notification) => {
                nuvei::map_dispute_notification_to_event(&notification.chargeback)
            }
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notification = get_webhook_object_from_body(request.body)?;
        Ok(Box::new(notification))
    }

    fn get_dispute_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<disputes::DisputePayload, errors::ConnectorError> {
        let webhook = request
            .body
            .parse_struct::<nuvei::ChargebackNotification>("ChargebackNotification")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let currency = webhook
            .chargeback
            .reported_currency
            .to_uppercase()
            .parse::<enums::Currency>()
            .map_err(|_| errors::ConnectorError::ResponseDeserializationFailed)?;
        let amount_minorunit = utils::convert_back_amount_to_minor_units(
            self.amount_converter_float_major_unit,
            webhook.chargeback.reported_amount,
            currency,
        )?;

        let amount = utils::convert_amount(
            self.amount_converter_string_minor_unit,
            amount_minorunit,
            currency,
        )?;
        let dispute_unified_status_code = webhook
            .chargeback
            .dispute_unified_status_code
            .clone()
            .ok_or(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let connector_dispute_id = webhook
            .chargeback
            .dispute_id
            .clone()
            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(disputes::DisputePayload {
            amount,
            currency,
            dispute_stage: nuvei::get_dispute_stage(&webhook.chargeback)?,
            connector_dispute_id,
            connector_reason: webhook.chargeback.chargeback_reason,
            connector_reason_code: webhook.chargeback.chargeback_reason_category,
            challenge_required_by: webhook.chargeback.dispute_due_date,
            connector_status: dispute_unified_status_code.to_string(),
            created_at: webhook.chargeback.date,
            updated_at: None,
        })
    }
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<nuvei::NuveiWebhook, errors::ConnectorError> {
    let payments_response = serde_urlencoded::from_bytes::<nuvei::NuveiWebhook>(body)
        .change_context(errors::ConnectorError::ResponseDeserializationFailed);

    match payments_response {
        Ok(webhook) => Ok(webhook),
        Err(_) => body
            .parse_struct::<nuvei::NuveiWebhook>("NuveiWebhook")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed),
    }
}

impl ConnectorRedirectResponse for Nuvei {
    fn get_flow_type(
        &self,
        _query_params: &str,
        json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<CallConnectorAction, errors::ConnectorError> {
        match action {
            PaymentAction::PSync | PaymentAction::PaymentAuthenticateCompleteAuthorize => {
                Ok(CallConnectorAction::Trigger)
            }
            PaymentAction::CompleteAuthorize => {
                if let Some(payload) = json_payload {
                    let redirect_response: nuvei::NuveiRedirectionResponse =
                        payload.parse_value("NuveiRedirectionResponse").switch()?;
                    let acs_response: nuvei::NuveiACSResponse =
                        utils::base64_decode(redirect_response.cres.expose())?
                            .as_slice()
                            .parse_struct("NuveiACSResponse")
                            .switch()?;
                    match acs_response.trans_status {
                        None | Some(nuvei::LiabilityShift::Failed) => {
                            Ok(CallConnectorAction::StatusUpdate {
                                status: enums::AttemptStatus::AuthenticationFailed,
                                error_code: None,
                                error_message: Some("3ds Authentication failed".to_string()),
                            })
                        }
                        _ => Ok(CallConnectorAction::Trigger),
                    }
                } else {
                    Ok(CallConnectorAction::Trigger)
                }
            }
        }
    }
}

static NUVEI_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::UnionPay,
        common_enums::CardNetwork::Interac,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::CartesBancaires,
    ];

    let mut nuvei_supported_payment_methods = SupportedPaymentMethods::new();

    nuvei_supported_payment_methods.add(
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
    nuvei_supported_payment_methods.add(
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
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        enums::PaymentMethodType::Klarna,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );

    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::PayLater,
        enums::PaymentMethodType::AfterpayClearpay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Ideal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Giropay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Sofort,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::BankRedirect,
        enums::PaymentMethodType::Eps,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::ApplePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::GooglePay,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::Supported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::Paypal,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    nuvei_supported_payment_methods
});

static NUVEI_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "Nuvei",
        description: "Nuvei is the Canadian fintech company accelerating the business of clients around the world.",
        connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
        integration_status: enums::ConnectorIntegrationStatus::Live,
    };

static NUVEI_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 2] =
    [enums::EventClass::Payments, enums::EventClass::Disputes];

impl ConnectorSpecifications for Nuvei {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&NUVEI_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*NUVEI_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&NUVEI_SUPPORTED_WEBHOOK_FLOWS)
    }
}
