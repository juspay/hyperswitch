pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::errors::ParsingError;
use error_stack::{report, IntoReport, ResultExt};
use masking::ExposeInterface;
use serde_json::{json, to_string};
use transformers as tokenex;

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
pub struct Tokenex;

impl api::Payment for Tokenex {}
impl api::PaymentSession for Tokenex {}
impl api::ConnectorAccessToken for Tokenex {}
impl api::MandateSetup for Tokenex {}
impl api::PaymentAuthorize for Tokenex {}
impl api::PaymentSync for Tokenex {}
impl api::PaymentCapture for Tokenex {}
impl api::PaymentVoid for Tokenex {}
impl api::Refund for Tokenex {}
impl api::RefundExecute for Tokenex {}
impl api::RefundSync for Tokenex {}
impl api::PaymentToken for Tokenex {}
impl api::ExternalAuthentication for Tokenex {}
impl api::ConnectorPreAuthentication for Tokenex {}
impl api::ConnectorPostAuthentication for Tokenex {}
impl api::ConnectorAuthentication for Tokenex {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Tokenex
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Tokenex
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            ("tx-token-scheme".to_string(), "PCI".to_string().into()),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Tokenex {
    fn id(&self) -> &'static str {
        "tokenex"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.tokenex.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = tokenex::TokenexAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            ("tx-apikey".to_string(), auth.api_key.expose().into_masked()),
            (
                "tx-tokenex-id".to_string(),
                auth.tokenex_id.expose().into_masked(),
            ),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: tokenex::TokenexAuthenticationResponse = res
            .response
            .parse_struct("TokenexAuthenticationResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let error_response =
            tokenex::get_router_response_from_tokenex_authn_response(&response, res.status_code);
        match error_response {
            Ok(_) => Err(errors::ConnectorError::ParsingFailed.into()),
            Err(error_response) => Ok(error_response),
        }
    }
}

impl ConnectorValidation for Tokenex {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Tokenex
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Tokenex
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Tokenex
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Tokenex
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = tokenex::TokenexRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let req_obj = tokenex::TokenexPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: tokenex::TokenexPaymentsResponse = res
            .response
            .parse_struct("Tokenex PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Tokenex
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: tokenex::TokenexPaymentsResponse = res
            .response
            .parse_struct("tokenex PaymentsSyncResponse")
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Tokenex
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: tokenex::TokenexPaymentsResponse = res
            .response
            .parse_struct("Tokenex PaymentsCaptureResponse")
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

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Tokenex
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Tokenex
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = tokenex::TokenexRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let req_obj = tokenex::TokenexRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: tokenex::RefundResponse = res
            .response
            .parse_struct("tokenex RefundResponse")
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Tokenex {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: tokenex::RefundResponse = res
            .response
            .parse_struct("tokenex RefundSyncResponse")
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Tokenex {
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

impl
    ConnectorIntegration<
        api::PreAuthentication,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for Tokenex
{
    fn get_headers(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = self.build_headers(req, connectors)?;
        headers.push(("tx-tokenize".to_string(), "true".to_string().into()));
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v2/ThreeDSecure/SupportedVersions",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = tokenex::TokenexRouterData::try_from((
            &self.get_currency_unit(),
            common_enums::Currency::USD,
            1000,
            req,
        ))?;
        let req_obj = tokenex::TokenexPreAuthenticationRequest::try_from(&connector_router_data)?;
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
        let response: tokenex::TokenexPreAuthenticationResponse = res
            .response
            .parse_struct("TokenexPreAuthenticationResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let acs_response = response.three_d_secure_response.first().ok_or(report!(
            errors::ConnectorError::ResponseDeserializationFailed
        ))?;
        let creq = json!({
            "threeDSServerTransID": acs_response.threeds_server_trans_id,
            "threeDSMethodNotificationURL": "https://webhook.site/8e2e1fd3-1ab0-4ffd-84b7-0c01daf2e2b0"
        });
        let creq_str = to_string(&creq)
            .ok()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        let creq_base64 = BASE64_ENGINE.encode(creq_str);
        Ok(types::authentication::PreAuthNRouterData {
            response: Ok(
                types::authentication::AuthenticationResponseData::PreAuthNResponse {
                    threeds_server_transaction_id: acs_response.threeds_server_trans_id.clone(),
                    maximum_supported_3ds_version: ForeignTryFrom::foreign_try_from(
                        acs_response.acs_end_protocol_version.clone(),
                    )?,
                    authentication_connector_id: response.token.clone(),
                    three_ds_method_data: creq_base64,
                    three_ds_method_url: acs_response.threeds_method_url.clone(),
                    message_version: acs_response.acs_end_protocol_version.clone(),
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
        api::Authentication,
        types::ConnectorAuthenticationRequestData,
        types::ConnectorAuthenticationResponse,
    > for Tokenex
{
    fn get_headers(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = self.build_headers(req, connectors)?;
        headers.push(("tx-tokenize".to_string(), "false".to_string().into()));
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v2/ThreeDSecure/Authentications",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = tokenex::TokenexRouterData::try_from((
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
        let req_obj = tokenex::TokenexAuthenticationRequest::try_from(&connector_router_data);
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
        let response: Result<
            tokenex::TokenexAuthenticationResponse,
            error_stack::Report<ParsingError>,
        > = res
            .response
            .parse_struct("tokenex TokenexAuthenticationResponse");
        let tokenex_response =
            response.change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let response = tokenex::get_router_response_from_tokenex_authn_response(
            &tokenex_response,
            res.status_code,
        );
        Ok(types::ConnectorAuthenticationRouterData {
            response,
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
    > for Tokenex
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
        Ok(format!(
            "{}/v2/ThreeDSecure/ChallengeResults",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = tokenex::TokenexPostAuthenticationRequest {
            server_transaction_id: req
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
        let response: Result<
            tokenex::TokenexPostAuthenticationResponse,
            error_stack::Report<ParsingError>,
        > = res.response.parse_struct("tokenex PaymentsSyncResponse");
        println!("response post-authn {:?}", response);
        let response =
            response.change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ConnectorPostAuthenticationRouterData {
            response: Ok(
                types::authentication::AuthenticationResponseData::PostAuthNResponse {
                    trans_status: response.three_d_secure_response.trans_status.into(),
                    authentication_value: response.three_d_secure_response.authentication_value,
                    eci: response.three_d_secure_response.eci,
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
