pub mod transformers;

use std::fmt::Debug;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        authentication::{
            Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
        },
        AccessTokenAuth, Authorize, Capture, Execute, PSync, PaymentMethodToken, RSync, Session,
        SetupMandate, Void,
    },
    router_request_types::{
        authentication::{
            ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
            PreAuthNRequestData,
        },
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        AuthenticationResponseData, ConnectorInfo, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods,
    },
};
use hyperswitch_interfaces::{
    api::{
        authentication::{
            ConnectorAuthentication, ConnectorPostAuthentication, ConnectorPreAuthentication,
            ConnectorPreAuthenticationVersionCall, ExternalAuthentication,
        },
        ConnectorAccessToken, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        ConnectorSpecifications, ConnectorValidation, CurrencyUnit, MandateSetup, Payment,
        PaymentAuthorize, PaymentCapture, PaymentSession, PaymentSync, PaymentToken, PaymentVoid,
        Refund, RefundExecute, RefundSync,
    },
    configs::Connectors,
    consts::NO_ERROR_MESSAGE,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{ExposeInterface, Mask as _, Maskable};
use transformers as threedsecureio;

use crate::{
    constants::headers,
    types::{
        ConnectorAuthenticationRouterData, ConnectorAuthenticationType,
        ConnectorPostAuthenticationRouterData, ConnectorPostAuthenticationType,
        ConnectorPreAuthenticationType, PreAuthNRouterData, ResponseRouterData,
    },
    utils::handle_json_response_deserialization_failure,
};
#[derive(Debug, Clone)]
pub struct Threedsecureio;

impl Payment for Threedsecureio {}
impl PaymentSession for Threedsecureio {}
impl ConnectorAccessToken for Threedsecureio {}
impl MandateSetup for Threedsecureio {}
impl PaymentAuthorize for Threedsecureio {}
impl PaymentSync for Threedsecureio {}
impl PaymentCapture for Threedsecureio {}
impl PaymentVoid for Threedsecureio {}
impl Refund for Threedsecureio {}
impl RefundExecute for Threedsecureio {}
impl RefundSync for Threedsecureio {}
impl PaymentToken for Threedsecureio {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Threedsecureio
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Threedsecureio
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
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

    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.threedsecureio.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = threedsecureio::ThreedsecureioAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::APIKEY.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
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
                        .unwrap_or(NO_ERROR_MESSAGE.to_owned()),
                    reason: response.error_description,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(err) => {
                router_env::logger::error!(deserialization_error =? err);
                handle_json_response_deserialization_failure(res, "threedsecureio")
            }
        }
    }
}

impl ConnectorValidation for Threedsecureio {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Threedsecureio {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Threedsecureio {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Threedsecureio
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Threedsecureio {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Threedsecureio {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Threedsecureio {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Threedsecureio {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Threedsecureio {}

#[async_trait::async_trait]
impl IncomingWebhook for Threedsecureio {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorPreAuthentication for Threedsecureio {}
impl ConnectorPreAuthenticationVersionCall for Threedsecureio {}
impl ExternalAuthentication for Threedsecureio {}
impl ConnectorAuthentication for Threedsecureio {}
impl ConnectorPostAuthentication for Threedsecureio {}

impl
    ConnectorIntegration<
        Authentication,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for Threedsecureio
{
    fn get_headers(
        &self,
        req: &ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}/auth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &ConnectorAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = threedsecureio::ThreedsecureioRouterData::try_from((
            &self.get_currency_unit(),
            req.request
                .currency
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?,
            req.request
                .amount
                .ok_or(ConnectorError::MissingRequiredField {
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
        req: &ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&ConnectorAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(ConnectorAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(ConnectorAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorAuthenticationRouterData, ConnectorError> {
        let response: threedsecureio::ThreedsecureioAuthenticationResponse = res
            .response
            .parse_struct("ThreedsecureioAuthenticationResponse")
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

impl ConnectorIntegration<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
    for Threedsecureio
{
    fn get_headers(
        &self,
        req: &PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}/preauth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &PreAuthNRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = threedsecureio::ThreedsecureioRouterData::try_from((0, req))?;
        let req_obj = threedsecureio::ThreedsecureioPreAuthenticationRequest::try_from(
            &connector_router_data,
        )?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&ConnectorPreAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(ConnectorPreAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(ConnectorPreAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PreAuthNRouterData, ConnectorError> {
        let response: threedsecureio::ThreedsecureioPreAuthenticationResponse = res
            .response
            .parse_struct("threedsecureio ThreedsecureioPreAuthenticationResponse")
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

impl
    ConnectorIntegration<
        PostAuthentication,
        ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    > for Threedsecureio
{
    fn get_headers(
        &self,
        req: &ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}/postauth", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &ConnectorPostAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = threedsecureio::ThreedsecureioPostAuthenticationRequest {
            three_ds_server_trans_id: req.request.threeds_server_transaction_id.clone(),
        };
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&ConnectorPostAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(ConnectorPostAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(ConnectorPostAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorPostAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorPostAuthenticationRouterData, ConnectorError> {
        let response: threedsecureio::ThreedsecureioPostAuthenticationResponse = res
            .response
            .parse_struct("threedsecureio PaymentsSyncResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(ConnectorPostAuthenticationRouterData {
            response: Ok(AuthenticationResponseData::PostAuthNResponse {
                trans_status: response.trans_status.into(),
                authentication_value: response.authentication_value,
                eci: response.eci,
                challenge_cancel: None,
                challenge_code_reason: None,
            }),
            ..data.clone()
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

impl
    ConnectorIntegration<
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > for Threedsecureio
{
}

static THREEDSECUREIO_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "3dsecure.io",
    description: "3DSecure.io is a service that facilitates 3-D Secure verifications for online credit and debit card transactions through a simple JSON API, enhancing payment security for merchants.docs.3dsecure.io3dsecure.io",
    connector_type: common_enums::HyperswitchConnectorCategory::AuthenticationProvider,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Threedsecureio {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&THREEDSECUREIO_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
