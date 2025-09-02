pub mod netcetera_types;
pub mod transformers;

use std::fmt::Debug;

use api_models::webhooks::{AuthenticationIdType, IncomingWebhookEvent, ObjectReferenceId};
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
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
        *,
    },
    authentication::ExternalAuthenticationPayload,
    configs::Connectors,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::Maskable;
use transformers as netcetera;

use crate::{
    constants::headers,
    types::{
        ConnectorAuthenticationRouterData, ConnectorAuthenticationType,
        ConnectorPreAuthenticationType, PreAuthNRouterData, ResponseRouterData,
    },
};

#[derive(Debug, Clone)]
pub struct Netcetera;

impl Payment for Netcetera {}
impl PaymentSession for Netcetera {}
impl ConnectorAccessToken for Netcetera {}
impl MandateSetup for Netcetera {}
impl PaymentAuthorize for Netcetera {}
impl PaymentSync for Netcetera {}
impl PaymentCapture for Netcetera {}
impl PaymentVoid for Netcetera {}
impl Refund for Netcetera {}
impl RefundExecute for Netcetera {}
impl RefundSync for Netcetera {}
impl PaymentToken for Netcetera {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Netcetera
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Netcetera
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

    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.netcetera.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: netcetera::NetceteraErrorResponse = res
            .response
            .parse_struct("NetceteraErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_details.error_code,
            message: response.error_details.error_description,
            reason: response.error_details.error_detail,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Netcetera {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Netcetera {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Netcetera {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Netcetera
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Netcetera {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Netcetera {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Netcetera {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Netcetera {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Netcetera {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Netcetera {}

#[async_trait::async_trait]
impl IncomingWebhook for Netcetera {
    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, ConnectorError> {
        let webhook_body: netcetera::ResultsResponseData = request
            .body
            .parse_struct("netcetera ResultsResponseData")
            .change_context(ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(ObjectReferenceId::ExternalAuthenticationID(
            AuthenticationIdType::ConnectorAuthenticationId(webhook_body.three_ds_server_trans_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        Ok(IncomingWebhookEvent::ExternalAuthenticationARes)
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        let webhook_body_value: netcetera::ResultsResponseData = request
            .body
            .parse_struct("netcetera ResultsResponseDatae")
            .change_context(ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Box::new(webhook_body_value))
    }

    fn get_external_authentication_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ExternalAuthenticationPayload, ConnectorError> {
        let webhook_body: netcetera::ResultsResponseData = request
            .body
            .parse_struct("netcetera ResultsResponseData")
            .change_context(ConnectorError::WebhookBodyDecodingFailed)?;

        let challenge_cancel = webhook_body
            .results_request
            .as_ref()
            .and_then(|v| v.get("challengeCancel").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        let challenge_code_reason = webhook_body
            .results_request
            .as_ref()
            .and_then(|v| v.get("transStatusReason").and_then(|v| v.as_str()))
            .map(|s| s.to_string());

        Ok(ExternalAuthenticationPayload {
            trans_status: webhook_body
                .trans_status
                .unwrap_or(common_enums::TransactionStatus::InformationOnly),
            authentication_value: webhook_body.authentication_value,
            eci: webhook_body.eci,
            challenge_cancel,
            challenge_code_reason,
        })
    }
}

fn build_endpoint(
    base_url: &str,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, ConnectorError> {
    let metadata = netcetera::NetceteraMetaData::try_from(connector_metadata)?;
    let endpoint_prefix = metadata.endpoint_prefix;
    Ok(base_url.replace("{{merchant_endpoint_prefix}}", &endpoint_prefix))
}

impl ConnectorPreAuthentication for Netcetera {}
impl ConnectorPreAuthenticationVersionCall for Netcetera {}
impl ExternalAuthentication for Netcetera {}
impl ConnectorAuthentication for Netcetera {}
impl ConnectorPostAuthentication for Netcetera {}

impl ConnectorIntegration<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
    for Netcetera
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
        req: &PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!("{base_url}/3ds/versioning"))
    }

    fn get_request_body(
        &self,
        req: &PreAuthNRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = netcetera::NetceteraRouterData::try_from((0, req))?;
        let req_obj =
            netcetera::NetceteraPreAuthenticationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let netcetera_auth_type = netcetera::NetceteraAuthType::try_from(&req.connector_auth_type)?;
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
                .add_certificate(Some(netcetera_auth_type.certificate))
                .add_certificate_key(Some(netcetera_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PreAuthNRouterData, ConnectorError> {
        let response: netcetera::NetceteraPreAuthenticationResponse = res
            .response
            .parse_struct("netcetera NetceteraPreAuthenticationResponse")
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
        Authentication,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for Netcetera
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
        req: &ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!("{base_url}/3ds/authentication"))
    }

    fn get_request_body(
        &self,
        req: &ConnectorAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = netcetera::NetceteraRouterData::try_from((
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
        let req_obj = netcetera::NetceteraAuthenticationRequest::try_from(&connector_router_data);
        Ok(RequestContent::Json(Box::new(req_obj?)))
    }

    fn build_request(
        &self,
        req: &ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let netcetera_auth_type = netcetera::NetceteraAuthType::try_from(&req.connector_auth_type)?;
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
                .add_certificate(Some(netcetera_auth_type.certificate))
                .add_certificate_key(Some(netcetera_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorAuthenticationRouterData, ConnectorError> {
        let response: netcetera::NetceteraAuthenticationResponse = res
            .response
            .parse_struct("NetceteraAuthenticationResponse")
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
    > for Netcetera
{
}

impl
    ConnectorIntegration<
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > for Netcetera
{
}

static NETCETERA_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Netcetera",
    description: "Netcetera authentication provider for comprehensive 3D Secure solutions including certified ACS, Directory Server, and multi-protocol EMV 3DS supports",
    connector_type: common_enums::HyperswitchConnectorCategory::AuthenticationProvider,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Netcetera {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&NETCETERA_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
