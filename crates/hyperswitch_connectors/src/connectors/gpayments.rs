pub mod gpayments_types;
pub mod transformers;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use gpayments_types::GpaymentsConnectorMetaData;
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
        self,
        authentication::{
            ConnectorAuthentication, ConnectorPostAuthentication, ConnectorPreAuthentication,
            ConnectorPreAuthenticationVersionCall, ExternalAuthentication,
        },
        ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::Maskable;
use transformers as gpayments;

use crate::{
    constants::headers,
    types::{
        ConnectorAuthenticationRouterData, ConnectorAuthenticationType,
        ConnectorPostAuthenticationRouterData, ConnectorPostAuthenticationType,
        ConnectorPreAuthenticationType, ConnectorPreAuthenticationVersionCallType,
        PreAuthNRouterData, PreAuthNVersionCallRouterData, ResponseRouterData,
    },
    utils::to_connector_meta,
};

#[derive(Clone)]
pub struct Gpayments {
    _amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Gpayments {
    pub fn new() -> &'static Self {
        &Self {
            _amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Gpayments {}
impl api::PaymentSession for Gpayments {}
impl api::ConnectorAccessToken for Gpayments {}
impl api::MandateSetup for Gpayments {}
impl api::PaymentAuthorize for Gpayments {}
impl api::PaymentSync for Gpayments {}
impl api::PaymentCapture for Gpayments {}
impl api::PaymentVoid for Gpayments {}
impl api::Refund for Gpayments {}
impl api::RefundExecute for Gpayments {}
impl api::RefundSync for Gpayments {}
impl api::PaymentToken for Gpayments {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Gpayments
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Gpayments
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Gpayments {
    fn id(&self) -> &'static str {
        "gpayments"
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

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.gpayments.base_url.as_ref()
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
        let response: gpayments_types::TDS2ApiError = res
            .response
            .parse_struct("gpayments_types TDS2ApiError")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.error_description,
            reason: response.error_detail,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Gpayments {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Gpayments {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Gpayments {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Gpayments {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Gpayments {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Gpayments {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Gpayments {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Gpayments {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Gpayments {}

#[async_trait::async_trait]
impl IncomingWebhook for Gpayments {
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

impl ExternalAuthentication for Gpayments {}
impl ConnectorAuthentication for Gpayments {}
impl ConnectorPreAuthentication for Gpayments {}
impl ConnectorPreAuthenticationVersionCall for Gpayments {}
impl ConnectorPostAuthentication for Gpayments {}

fn build_endpoint(
    base_url: &str,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, ConnectorError> {
    let metadata = gpayments::GpaymentsMetaData::try_from(connector_metadata)?;
    let endpoint_prefix = metadata.endpoint_prefix;
    Ok(base_url.replace("{{merchant_endpoint_prefix}}", &endpoint_prefix))
}

impl
    ConnectorIntegration<
        Authentication,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for Gpayments
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
        _connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let connector_metadata: GpaymentsConnectorMetaData = to_connector_meta(
            req.request
                .pre_authentication_data
                .connector_metadata
                .clone(),
        )?;
        Ok(connector_metadata.authentication_url)
    }

    fn get_request_body(
        &self,
        req: &ConnectorAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = gpayments::GpaymentsRouterData::from((MinorUnit::zero(), req));
        let req_obj =
            gpayments_types::GpaymentsAuthenticationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }
    fn build_request(
        &self,
        req: &ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
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
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorAuthenticationRouterData, ConnectorError> {
        let response: gpayments_types::GpaymentsAuthenticationSuccessResponse = res
            .response
            .parse_struct("gpayments GpaymentsAuthenticationResponse")
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
    > for Gpayments
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
        req: &ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!(
            "{}/api/v2/auth/brw/result?threeDSServerTransID={}",
            base_url, req.request.threeds_server_transaction_id,
        ))
    }

    fn build_request(
        &self,
        req: &ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&ConnectorPostAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(ConnectorPostAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorPostAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorPostAuthenticationRouterData, ConnectorError> {
        let response: gpayments_types::GpaymentsPostAuthenticationResponse = res
            .response
            .parse_struct("gpayments PaymentsSyncResponse")
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

impl ConnectorIntegration<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
    for Gpayments
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
        Ok(format!("{base_url}/api/v2/auth/brw/init?mode=custom"))
    }

    fn get_request_body(
        &self,
        req: &PreAuthNRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = gpayments::GpaymentsRouterData::from((MinorUnit::zero(), req));
        let req_obj =
            gpayments_types::GpaymentsPreAuthenticationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
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
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PreAuthNRouterData, ConnectorError> {
        let response: gpayments_types::GpaymentsPreAuthenticationResponse = res
            .response
            .parse_struct("gpayments GpaymentsPreAuthenticationResponse")
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
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > for Gpayments
{
    fn get_headers(
        &self,
        req: &PreAuthNVersionCallRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PreAuthNVersionCallRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!("{base_url}/api/v2/auth/enrol"))
    }

    fn get_request_body(
        &self,
        req: &PreAuthNVersionCallRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = gpayments::GpaymentsRouterData::from((MinorUnit::zero(), req));
        let req_obj =
            gpayments_types::GpaymentsPreAuthVersionCallRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &PreAuthNVersionCallRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&ConnectorPreAuthenticationVersionCallType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(ConnectorPreAuthenticationVersionCallType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(ConnectorPreAuthenticationVersionCallType::get_request_body(
                    self, req, connectors,
                )?)
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PreAuthNVersionCallRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PreAuthNVersionCallRouterData, ConnectorError> {
        let response: gpayments_types::GpaymentsPreAuthVersionCallResponse = res
            .response
            .parse_struct("gpayments GpaymentsPreAuthVersionCallResponse")
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

static GPAYMENTS_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "GPayments",
    description: "GPayments authentication connector for 3D Secure MPI/ACS services supporting Visa Secure, Mastercard SecureCode, and global card authentication standards",
    connector_type: common_enums::HyperswitchConnectorCategory::AuthenticationProvider,
    integration_status: common_enums::ConnectorIntegrationStatus::Alpha,
};

impl ConnectorSpecifications for Gpayments {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&GPAYMENTS_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
