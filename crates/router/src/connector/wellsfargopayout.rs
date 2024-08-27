pub mod transformers;

use base64::Engine;
use error_stack::{report, ResultExt};
use masking::PeekInterface;
use router_env::{instrument, tracing};
use transformers as wellsfargopayout;

use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
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

#[derive(Debug, Clone)]
pub struct Wellsfargopayout;

impl api::Payment for Wellsfargopayout {}
impl api::PaymentSession for Wellsfargopayout {}
impl api::MandateSetup for Wellsfargopayout {}
impl api::PaymentAuthorize for Wellsfargopayout {}
impl api::PaymentSync for Wellsfargopayout {}
impl api::PaymentCapture for Wellsfargopayout {}
impl api::PaymentVoid for Wellsfargopayout {}
impl api::Refund for Wellsfargopayout {}
impl api::RefundExecute for Wellsfargopayout {}
impl api::RefundSync for Wellsfargopayout {}
impl api::PaymentToken for Wellsfargopayout {}

impl api::Payouts for Wellsfargopayout {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Wellsfargopayout {}
#[cfg(feature = "payouts")]
impl api::PayoutSync for Wellsfargopayout {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Wellsfargopayout
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Wellsfargopayout
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "payouts")]
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth = wellsfargopayout::WellsfargopayoutAuthType::try_from(&req.connector_auth_type)?;
        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into_masked(),
            ),
            (
                "gateway-entity-id".to_string(),
                auth.gateway_entity_id.peek().clone().into_masked(),
            ),
            (
                "client-request-id".to_string(),
                req.payment_id.clone().into_masked(),
            ),
        ];

        Ok(headers)
    }
}

impl ConnectorCommon for Wellsfargopayout {
    fn id(&self) -> &'static str {
        "wellsfargopayout"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.wellsfargopayout.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: wellsfargopayout::WellsfargopayoutErrorResponse = res
            .response
            .parse_struct("wellsfargopayout::WellsfargopayoutErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let error = response
            .errors
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: error.error_code.clone(),
            message: error.description.clone(),
            reason: Some(error.description.clone()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Wellsfargopayout {
    //TODO: implement functions when support enabled
}

impl api::ConnectorAccessToken for Wellsfargopayout {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Wellsfargopayout
{
    fn get_url(
        &self,
        _req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "oauth2/v1/token"
        ))
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_headers(
        &self,
        req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefreshTokenType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = wellsfargopayout::WellsfargopayoutAuthType::try_from(&req.connector_auth_type)?;
        let auth_key = format!(
            "{}:{}",
            auth.consumer_key.peek(),
            auth.consumer_secret.peek()
        );
        let auth_header = (
            headers::AUTHORIZATION.to_string(),
            format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key)).into_masked(),
        );

        header.push(auth_header);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = wellsfargopayout::WellsfargopayoutAuthpdateRequest::try_from(req)?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let req = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .set_body(types::RefreshTokenType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );
        Ok(req)
    }
    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::RefreshTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefreshTokenRouterData, errors::ConnectorError> {
        let response: wellsfargopayout::WellsfargopayoutAuthUpdateResponse = res
            .response
            .parse_struct("wellsfargopayout WellsfargopayoutAuthUpdateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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
        let response: wellsfargopayout::AccessTokenErrorResponse = res
            .response
            .parse_struct("wellsfargopayout AccessTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error,
            message: response.error_description.clone(),
            reason: Some(response.error_description),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Wellsfargopayout
{
    //TODO: implement sessions flow
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Wellsfargopayout
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Wellsfargopayout
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Wellsfargopayout
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Wellsfargopayout
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Wellsfargopayout
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Wellsfargopayout
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Wellsfargopayout
{
}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Wellsfargopayout
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ach/v2/payments/credit-transfers",
            connectors.wellsfargopayout.base_url,
        ))
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
        let connector_router_data = wellsfargopayout::WellsfargopayoutRouterData::try_from((
            &self.get_currency_unit(),
            req.request.source_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = wellsfargopayout::WellsfargopayoutPayoutCreateRequest::try_from(
            &connector_router_data,
        )?;
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
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: wellsfargopayout::WellsfargopayoutPayoutResponse = res
            .response
            .parse_struct("WellsfargopayoutPayoutResponse")
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
impl api::IncomingWebhook for Wellsfargopayout {
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

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoSync, types::PayoutsData, types::PayoutsResponseData>
    for Wellsfargopayout
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_id = req.request.connector_payout_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_payout_id",
            },
        )?;
        Ok(format!(
            "{}ach/v2/payments/{}",
            self.base_url(connectors),
            payment_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoSync>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Get)
            .url(&types::PayoutSyncType::get_url(self, req, connectors)?)
            .headers(types::PayoutSyncType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoSync>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoSync>, errors::ConnectorError> {
        let response: wellsfargopayout::WellsFargoPayoutSyncResponse = res
            .response
            .parse_struct("WellsFargoPayoutSyncResponse")
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
