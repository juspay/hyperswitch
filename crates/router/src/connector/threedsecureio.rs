pub mod transformers;

use std::fmt::Debug;

use error_stack::{report, ResultExt};
use masking::ExposeInterface;
use pm_auth::consts;
use transformers as threedsecureio;

use crate::{
    configs::settings,
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
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Threedsecureio;

impl api::Payment for Threedsecureio {}
impl api::PaymentSession for Threedsecureio {}
impl api::ConnectorAccessToken for Threedsecureio {}
impl api::MandateSetup for Threedsecureio {}
impl api::PaymentAuthorize for Threedsecureio {}
impl api::PaymentSync for Threedsecureio {}
impl api::PaymentCapture for Threedsecureio {}
impl api::PaymentVoid for Threedsecureio {}
impl api::Refund for Threedsecureio {}
impl api::RefundExecute for Threedsecureio {}
impl api::RefundSync for Threedsecureio {}
impl api::PaymentToken for Threedsecureio {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Threedsecureio
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Threedsecureio
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
            "application/json; charset=utf-8".to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Threedsecureio {
    fn id(&self) -> &'static str {
        "threedsecureio"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.threedsecureio.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = threedsecureio::ThreedsecureioAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::APIKEY.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response_result: Result<
            threedsecureio::ThreedsecureioErrorResponse,
            error_stack::Report<common_utils::errors::ParsingError>,
        > = res.response.parse_struct("ThreedsecureioErrorResponse");

        match response_result {
            Ok(response) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: response.error_code,
                    message: response
                        .error_description
                        .clone()
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_owned()),
                    reason: response.error_description,
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
            Err(err) => {
                router_env::logger::error!(deserialization_error =? err);
                utils::handle_json_response_deserialization_failure(
                    res,
                    "threedsecureio".to_owned(),
                )
            }
        }
    }
}

impl ConnectorValidation for Threedsecureio {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Threedsecureio
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Threedsecureio
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Threedsecureio
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Threedsecureio {
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

impl api::ConnectorPreAuthentication for Threedsecureio {}
impl api::ExternalAuthentication for Threedsecureio {}
impl api::ConnectorAuthentication for Threedsecureio {}
impl api::ConnectorPostAuthentication for Threedsecureio {}

impl
    ConnectorIntegration<
        api::Authentication,
        types::authentication::ConnectorAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Threedsecureio
{
    fn get_headers(
        &self,
        req: &types::authentication::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::authentication::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/auth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::ConnectorAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = threedsecureio::ThreedsecureioRouterData::try_from((
            &self.get_currency_unit(),
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?,
            req.request
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                })?,
            req,
        ))?;
        let req_obj =
            threedsecureio::ThreedsecureioAuthenticationRequest::try_from(&connector_router_data);
        Ok(RequestContent::Json(Box::new(req_obj?)))
    }

    fn build_request(
        &self,
        req: &types::authentication::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(
                    &types::authentication::ConnectorAuthenticationType::get_url(
                        self, req, connectors,
                    )?,
                )
                .attach_default_headers()
                .headers(
                    types::authentication::ConnectorAuthenticationType::get_headers(
                        self, req, connectors,
                    )?,
                )
                .set_body(
                    types::authentication::ConnectorAuthenticationType::get_request_body(
                        self, req, connectors,
                    )?,
                )
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::ConnectorAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        types::authentication::ConnectorAuthenticationRouterData,
        errors::ConnectorError,
    > {
        let response: threedsecureio::ThreedsecureioAuthenticationResponse = res
            .response
            .parse_struct("ThreedsecureioAuthenticationResponse")
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
        api::PreAuthentication,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for Threedsecureio
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
        _req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/preauth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = threedsecureio::ThreedsecureioRouterData::try_from((0, req))?;
        let req_obj = threedsecureio::ThreedsecureioPreAuthenticationRequest::try_from(
            &connector_router_data,
        )?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::authentication::PreAuthNRouterData, errors::ConnectorError> {
        let response: threedsecureio::ThreedsecureioPreAuthenticationResponse = res
            .response
            .parse_struct("threedsecureio ThreedsecureioPreAuthenticationResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        // Ok(types::authentication::PreAuthNRouterData {
        //     response,
        //     ..data.clone()
        // })
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
        api::PostAuthentication,
        types::authentication::ConnectorPostAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Threedsecureio
{
    fn get_headers(
        &self,
        req: &types::authentication::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::authentication::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/postauth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::ConnectorPostAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = threedsecureio::ThreedsecureioPostAuthenticationRequest {
            three_ds_server_trans_id: req.request.threeds_server_transaction_id.clone(),
        };
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::authentication::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(
                    &types::authentication::ConnectorPostAuthenticationType::get_url(
                        self, req, connectors,
                    )?,
                )
                .attach_default_headers()
                .headers(
                    types::authentication::ConnectorPostAuthenticationType::get_headers(
                        self, req, connectors,
                    )?,
                )
                .set_body(
                    types::authentication::ConnectorPostAuthenticationType::get_request_body(
                        self, req, connectors,
                    )?,
                )
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::ConnectorPostAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        types::authentication::ConnectorPostAuthenticationRouterData,
        errors::ConnectorError,
    > {
        let response: threedsecureio::ThreedsecureioPostAuthenticationResponse = res
            .response
            .parse_struct("threedsecureio PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(
            types::authentication::ConnectorPostAuthenticationRouterData {
                response: Ok(
                    types::authentication::AuthenticationResponseData::PostAuthNResponse {
                        trans_status: response.trans_status.into(),
                        authentication_value: response.authentication_value,
                        eci: response.eci,
                    },
                ),
                ..data.clone()
            },
        )
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
