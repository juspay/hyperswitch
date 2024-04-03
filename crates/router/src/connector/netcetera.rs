pub mod transformers;

use std::fmt::Debug;

use common_utils::request::RequestContent;
use error_stack::{report, ResultExt};
use masking::ExposeInterface;
use transformers as netcetera;

use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Netcetera;

impl api::Payment for Netcetera {}
impl api::PaymentSession for Netcetera {}
impl api::ConnectorAccessToken for Netcetera {}
impl api::MandateSetup for Netcetera {}
impl api::PaymentAuthorize for Netcetera {}
impl api::PaymentSync for Netcetera {}
impl api::PaymentCapture for Netcetera {}
impl api::PaymentVoid for Netcetera {}
impl api::Refund for Netcetera {}
impl api::RefundExecute for Netcetera {}
impl api::RefundSync for Netcetera {}
impl api::PaymentToken for Netcetera {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Netcetera
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Netcetera
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Netcetera {
    fn id(&self) -> &'static str {
        "netcetera"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.netcetera.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: netcetera::NetceteraErrorResponse = res
            .response
            .parse_struct("NetceteraErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_details.error_code,
            message: response
                .error_details
                .error_description
                .unwrap_or(consts::NO_ERROR_MESSAGE.into()),
            reason: response.error_details.error_detail,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Netcetera {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Netcetera
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Netcetera
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Netcetera
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Netcetera
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Netcetera
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Netcetera
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Netcetera
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Netcetera
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Netcetera
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Netcetera {
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

fn build_endpoint(
    base_url: &str,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, errors::ConnectorError> {
    let metadata = netcetera::NetceteraMetaData::try_from(connector_metadata)?;
    let endpoint_prefix = metadata.endpoint_prefix;
    Ok(base_url.replace("{{merchant_endpoint_prefix}}", &endpoint_prefix))
}

impl api::ConnectorPreAuthentication for Netcetera {}
impl api::ExternalAuthentication for Netcetera {}
impl api::ConnectorAuthentication for Netcetera {}
impl api::ConnectorPostAuthentication for Netcetera {}

impl
    ConnectorIntegration<
        api::PreAuthentication,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for Netcetera
{
    fn get_headers(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!("{}/3ds/versioning", base_url,))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = netcetera::NetceteraRouterData::try_from((0, req))?;
        let req_obj =
            netcetera::NetceteraPreAuthenticationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let netcetera_auth_type = netcetera::NetceteraAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(
                    &types::authentication::ConnectorPreAuthenticationType::get_url(
                        self, req, connectors,
                    )?,
                )
                .attach_default_headers()
                .headers(
                    types::authentication::ConnectorPreAuthenticationType::get_headers(
                        self, req, connectors,
                    )?,
                )
                .set_body(
                    types::authentication::ConnectorPreAuthenticationType::get_request_body(
                        self, req, connectors,
                    )?,
                )
                .add_certificate(Some(netcetera_auth_type.certificate.expose()))
                .add_certificate_key(Some(netcetera_auth_type.private_key.expose()))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::authentication::PreAuthNRouterData, errors::ConnectorError> {
        let response: netcetera::NetceteraPreAuthenticationResponse = res
            .response
            .parse_struct("netcetera NetceteraPreAuthenticationResponse")
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

impl
    ConnectorIntegration<
        api::Authentication,
        types::authentication::ConnectorAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Netcetera
{
}

impl
    ConnectorIntegration<
        api::PostAuthentication,
        types::authentication::ConnectorPostAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Netcetera
{
}
