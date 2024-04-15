pub mod transformers;

use std::fmt::Debug;

#[cfg(feature = "payouts")]
use common_utils::request::RequestContent;
use error_stack::{report, ResultExt};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};
use transformers as ebanx;

#[cfg(feature = "payouts")]
use crate::services;
#[cfg(feature = "payouts")]
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Ebanx;

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
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.ebanx.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: ebanx::EbanxErrorResponse =
            res.response
                .parse_struct("EbanxErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.status_code,
            message: response.code,
            reason: response.message,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

#[async_trait::async_trait]
#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoCreate, types::PayoutsData, types::PayoutsResponseData> for Ebanx {
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ws/payout/create", connectors.ebanx.base_url))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = ebanx::EbanxRouterData::try_from((
            &self.get_currency_unit(),
            req.request.source_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = ebanx::EbanxPayoutCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        data: &types::PayoutsRouterData<api::PoCreate>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCreate>, errors::ConnectorError> {
        let response: ebanx::EbanxPayoutResponse = res
            .response
            .parse_struct("EbanxPayoutResponse")
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
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Ebanx
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ws/payout/commit", connectors.ebanx.base_url,))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = ebanx::EbanxRouterData::try_from((
            &self.get_currency_unit(),
            req.request.source_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = ebanx::EbanxPayoutFulfillRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        data: &types::PayoutsRouterData<api::PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: ebanx::EbanxFulfillResponse = res
            .response
            .parse_struct("EbanxFulfillResponse")
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
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoCancel, types::PayoutsData, types::PayoutsResponseData> for Ebanx {
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ws/payout/cancel", connectors.ebanx.base_url,))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, _connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = ebanx::EbanxPayoutCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Put)
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
        data: &types::PayoutsRouterData<api::PoCancel>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCancel>, errors::ConnectorError> {
        let response: ebanx::EbanxCancelResponse = res
            .response
            .parse_struct("EbanxCancelResponse")
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
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoQuote, types::PayoutsData, types::PayoutsResponseData> for Ebanx {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoRecipient, types::PayoutsData, types::PayoutsResponseData>
    for Ebanx
{
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoEligibility, types::PayoutsData, types::PayoutsResponseData>
    for Ebanx
{
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Ebanx
{
    // Not Implemented (R)
}

impl ConnectorValidation for Ebanx {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Ebanx
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Ebanx
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Ebanx
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Ebanx
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Ebanx
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Ebanx
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Ebanx
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Ebanx {}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Ebanx {}

#[async_trait::async_trait]
impl api::IncomingWebhook for Ebanx {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
