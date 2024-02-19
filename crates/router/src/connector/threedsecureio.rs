pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use pm_auth::consts::NO_ERROR_MESSAGE;
use serde_json::{json, to_string};
use transformers as threedsecureio;

use crate::{
    configs::settings,
    consts::BASE64_ENGINE,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        transformers::ForeignTryFrom,
        ErrorResponse, RequestContent, Response,
    },
    utils::BytesExt,
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
    // Not Implemented (R)
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: threedsecureio::ThreedsecureioErrorResponse = res
            .response
            .parse_struct("ThreedsecureioErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response
                .error_description
                .clone()
                .unwrap_or(NO_ERROR_MESSAGE.to_owned()),
            reason: response.error_detail,
            attempt_status: None,
            connector_transaction_id: None,
        })
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
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl api::ConnectorPreAuthentication for Threedsecureio {}
impl api::ExternalAuthentication for Threedsecureio {}
impl api::ConnectorAuthentication for Threedsecureio {}
impl api::ConnectorPostAuthentication for Threedsecureio {}

impl
    ConnectorIntegration<
        api::Authentication,
        types::ConnectorAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Threedsecureio
{
    fn get_headers(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/auth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
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
        println!("req_obj authn {:?}", req_obj);
        Ok(RequestContent::Json(Box::new(req_obj?)))
    }

    fn build_request(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::ConnectorAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ConnectorAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::ConnectorAuthenticationRouterData,
        res: Response,
    ) -> CustomResult<types::ConnectorAuthenticationRouterData, errors::ConnectorError> {
        let response = res
            .response
            .parse_struct("ThreedsecureioAuthenticationResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
                .url(&types::ConnectorPreAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorPreAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ConnectorPreAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::PreAuthNRouterData,
        res: Response,
    ) -> CustomResult<types::authentication::PreAuthNRouterData, errors::ConnectorError> {
        let response: threedsecureio::ThreedsecureioPreAuthenticationResponse = res
            .response
            .parse_struct("threedsecureio ThreedsecureioPreAuthenticationResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let three_ds_method_data = json!({
            "threeDSServerTransID": response.threeds_server_trans_id,
        });
        let three_ds_method_data_str = to_string(&three_ds_method_data)
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
            .attach_printable("error while constructing three_ds_method_data_str")?;
        let three_ds_method_data_base64 = BASE64_ENGINE.encode(three_ds_method_data_str);
        Ok(types::authentication::PreAuthNRouterData {
            response: Ok(
                types::authentication::AuthenticationResponseData::PreAuthNResponse {
                    threeds_server_transaction_id: response.threeds_server_trans_id.clone(),
                    maximum_supported_3ds_version: ForeignTryFrom::foreign_try_from(
                        response.acs_end_protocol_version.clone(),
                    )?,
                    authentication_connector_id: response.threeds_server_trans_id,
                    three_ds_method_data: three_ds_method_data_base64,
                    three_ds_method_url: response.threeds_method_url,
                    message_version: response.acs_end_protocol_version.clone(),
                },
            ),
            status: common_enums::AttemptStatus::AuthenticationPending,
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<
        api::PostAuthentication,
        types::ConnectorPostAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Threedsecureio
{
    fn get_headers(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/postauth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = threedsecureio::ThreedsecureioPostAuthenticationRequest {
            three_ds_server_trans_id: req
                .request
                .authentication_data
                .threeds_server_transaction_id
                .clone(),
        };
        println!("req_obj post-authn {:?}", req_obj);
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::ConnectorPostAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorPostAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ConnectorPostAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::ConnectorPostAuthenticationRouterData,
        res: Response,
    ) -> CustomResult<types::ConnectorPostAuthenticationRouterData, errors::ConnectorError> {
        let response: threedsecureio::ThreedsecureioPostAuthenticationResponse = res
            .response
            .parse_struct("threedsecureio PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ConnectorPostAuthenticationRouterData {
            response: Ok(
                types::authentication::AuthenticationResponseData::PostAuthNResponse {
                    trans_status: response.trans_status.into(),
                    authentication_value: response.authentication_value,
                    eci: response.eci,
                },
            ),
            ..data.clone()
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
