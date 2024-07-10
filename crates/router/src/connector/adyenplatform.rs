pub mod transformers;
use std::fmt::Debug;

#[cfg(feature = "payouts")]
use common_utils::request::RequestContent;
use error_stack::{report, ResultExt};
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};

use self::transformers as adyenplatform;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
    },
};
#[cfg(feature = "payouts")]
use crate::{events::connector_api_logs::ConnectorEvent, utils::BytesExt};

#[derive(Debug, Clone)]
pub struct Adyenplatform;

impl ConnectorCommon for Adyenplatform {
    fn id(&self) -> &'static str {
        "adyenplatform"
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = adyenplatform::AdyenplatformAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.adyenplatform.base_url.as_ref()
    }

    #[cfg(feature = "payouts")]
    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyenplatform::AdyenTransferErrorResponse = res
            .response
            .parse_struct("AdyenTransferErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.title,
            reason: response.detail,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl api::Payment for Adyenplatform {}
impl api::PaymentAuthorize for Adyenplatform {}
impl api::PaymentSync for Adyenplatform {}
impl api::PaymentVoid for Adyenplatform {}
impl api::PaymentCapture for Adyenplatform {}
impl api::MandateSetup for Adyenplatform {}
impl api::ConnectorAccessToken for Adyenplatform {}
impl api::PaymentToken for Adyenplatform {}
impl ConnectorValidation for Adyenplatform {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl api::PaymentSession for Adyenplatform {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Adyenplatform
{
}

impl api::Payouts for Adyenplatform {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Adyenplatform {}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Adyenplatform
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}btl/v4/transfers",
            connectors.adyenplatform.base_url,
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutFulfillType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = adyenplatform::AdyenplatformAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyenplatform::AdyenTransferRequest::try_from(req)?;
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
        let response: adyenplatform::AdyenTransferResponse = res
            .response
            .parse_struct("AdyenTransferResponse")
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

impl api::Refund for Adyenplatform {}
impl api::RefundExecute for Adyenplatform {}
impl api::RefundSync for Adyenplatform {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Adyenplatform
{
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Adyenplatform
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Adyenplatform {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
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
