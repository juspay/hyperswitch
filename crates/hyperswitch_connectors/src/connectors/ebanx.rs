pub mod transformers;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
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
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::{PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient},
    types::{PayoutsData, PayoutsResponseData, PayoutsRouterData},
};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorValidation},
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};
use transformers as ebanx;

#[cfg(feature = "payouts")]
use crate::utils::convert_amount;
use crate::{constants::headers, types::ResponseRouterData};

#[derive(Clone)]
pub struct Ebanx {
    #[cfg(feature = "payouts")]
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Ebanx {
    pub fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "payouts")]
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for Ebanx {}
impl api::PaymentSession for Ebanx {}
impl api::ConnectorAccessToken for Ebanx {}
impl api::MandateSetup for Ebanx {}
impl api::PaymentAuthorize for Ebanx {}
impl api::PaymentSync for Ebanx {}
impl api::PaymentCapture for Ebanx {}
impl api::PaymentVoid for Ebanx {}
impl api::Refund for Ebanx {}
impl api::RefundExecute for Ebanx {}
impl api::RefundSync for Ebanx {}
impl api::PaymentToken for Ebanx {}

impl api::Payouts for Ebanx {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Ebanx {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Ebanx {}
#[cfg(feature = "payouts")]
impl api::PayoutEligibility for Ebanx {}
#[cfg(feature = "payouts")]
impl api::PayoutQuote for Ebanx {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipient for Ebanx {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Ebanx {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Ebanx
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "payouts")]
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Ebanx {
    fn id(&self) -> &'static str {
        "ebanx"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.ebanx.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: ebanx::EbanxErrorResponse =
            res.response
                .parse_struct("EbanxErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status_code,
            message: response.code,
            reason: response.message,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData> for Ebanx {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ws/payout/create", connectors.ebanx.base_url))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.source_currency,
        )?;
        let connector_router_data = ebanx::EbanxRouterData::from((amount, req));
        let connector_req = ebanx::EbanxPayoutCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCreate>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCreateType::get_headers(self, req, connectors)?)
            .set_body(types::PayoutCreateType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCreate>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCreate>, errors::ConnectorError> {
        let response: ebanx::EbanxPayoutResponse = res
            .response
            .parse_struct("EbanxPayoutResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Ebanx {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ws/payout/commit", connectors.ebanx.base_url,))
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
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.source_currency,
        )?;
        let connector_router_data = ebanx::EbanxRouterData::from((amount, req));
        let connector_req = ebanx::EbanxPayoutFulfillRequest::try_from(&connector_router_data)?;
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

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, errors::ConnectorError> {
        let response: ebanx::EbanxFulfillResponse = res
            .response
            .parse_struct("EbanxFulfillResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData> for Ebanx {
    fn get_url(
        &self,
        _req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ws/payout/cancel", connectors.ebanx.base_url,))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, _connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = ebanx::EbanxPayoutCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoCancel>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Put)
            .url(&types::PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCancelType::get_headers(self, req, connectors)?)
            .set_body(types::PayoutCancelType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoCancel>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoCancel>, errors::ConnectorError> {
        let response: ebanx::EbanxCancelResponse = res
            .response
            .parse_struct("EbanxCancelResponse")
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData> for Ebanx {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData> for Ebanx {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData> for Ebanx {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Ebanx
{
    // Not Implemented (R)
}

impl ConnectorValidation for Ebanx {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Ebanx {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Ebanx {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Ebanx {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Ebanx {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Ebanx {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Ebanx {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Ebanx {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Ebanx {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Ebanx {}

impl IncomingWebhook for Ebanx {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
