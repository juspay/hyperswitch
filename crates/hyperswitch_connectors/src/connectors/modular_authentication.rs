pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        authentication::{
            Authentication, AuthenticationCreate, PostAuthentication, PreAuthentication,
            PreAuthenticationVersionCall,
        },
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        Authenticate, AuthenticationConfirmation, PostAuthenticate, PreAuthenticate,
        ProcessIncomingWebhook,
    },
    router_request_types::{
        authentication::{
            ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
            PreAuthNRequestData,
        },
        unified_authentication_service::{
            AuthenticationCreateRequestData, UasAuthenticationRequestData,
            UasAuthenticationResponseData, UasConfirmationRequestData,
            UasPostAuthenticationRequestData, UasPreAuthenticationRequestData,
            UasWebhookRequestData,
        },
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        AuthenticationCreateRouterData, UasAuthenticationRouterData,
        UasPostAuthenticationRouterData, UasPreAuthenticationRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        AuthenticationCreateType, Response, UasAuthenticationType, UasPostAuthenticationType,
        UasPreAuthenticationType,
    },
    webhooks,
};
use hyperswitch_masking::Mask;

use crate::{
    connectors::UnifiedAuthenticationService,
    constants::headers,
    types::{self, ResponseRouterData},
};
use transformers as modular_authentication;

#[derive(Clone)]
pub struct ModularAuthentication {
    _amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl ModularAuthentication {
    pub fn new() -> &'static Self {
        &Self {
            _amount_converter: &MinorUnitForConnector,
        }
    }
}

// impl api::authentication::ConnectorAuthentication for ModularAuthentication {}
// impl api::authentication::ConnectorAuthenticationCreate for ModularAuthentication {}
// impl api::authentication::ConnectorPreAuthentication for ModularAuthentication {}
// impl api::authentication::ConnectorPreAuthenticationVersionCall for ModularAuthentication
// impl api::authentication::ConnectorPostAuthentication for ModularAuthentication {}
impl api::Payment for ModularAuthentication {}
impl api::PaymentSession for ModularAuthentication {}
// impl api::ConnectorAccessToken for ModularAuthentication {}
impl api::MandateSetup for ModularAuthentication {}
impl api::PaymentAuthorize for ModularAuthentication {}
impl api::PaymentSync for ModularAuthentication {}
impl api::PaymentCapture for ModularAuthentication {}
impl api::PaymentVoid for ModularAuthentication {}
impl api::Refund for ModularAuthentication {}
impl api::ConnectorAccessToken for ModularAuthentication {}
impl api::RefundExecute for ModularAuthentication {}
impl api::RefundSync for ModularAuthentication {}
impl api::PaymentToken for ModularAuthentication {}
// impl api::UasPreAuthentication for ModularAuthentication {}
// impl api::UasPostAuthentication for ModularAuthentication {}
// impl api::UasAuthenticationConfirmation for ModularAuthentication {}
// impl api::UasAuthentication for ModularAuthentication {}
// impl api::UasProcessWebhook for ModularAuthentication {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for ModularAuthentication
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for ModularAuthentication
{
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for ModularAuthentication
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for ModularAuthentication
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for ModularAuthentication
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for ModularAuthentication {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
    for ModularAuthentication
{
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>
    for ModularAuthentication
{
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for ModularAuthentication {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for ModularAuthentication {}

// ExternalAuthentication sub-traits (ConnectorAuthentication and ConnectorPreAuthentication
// are already implemented above via the actual flow impls; we just need the marker impls)
// impl api::authentication::ConnectorAuthentication for ModularAuthentication {}
// impl api::authentication::ConnectorAuthenticationCreate for ModularAuthentication {}
// impl api::authentication::ConnectorPreAuthentication for ModularAuthentication {}

impl
    ConnectorIntegration<
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        UasAuthenticationResponseData,
    > for ModularAuthentication
{
}
// impl api::authentication::ConnectorPreAuthenticationVersionCall for ModularAuthentication {}
// impl api::authentication::ConnectorPostAuthentication for ModularAuthentication {}
// impl api::authentication::ExternalAuthentication for ModularAuthentication {}

// FraudCheck (empty trait when frm feature is disabled; also stub sub-traits when enabled)
impl api::FraudCheck for ModularAuthentication {}

impl
    ConnectorIntegration<
        AuthenticationConfirmation,
        UasConfirmationRequestData,
        UasAuthenticationResponseData,
    > for ModularAuthentication
{
}

impl
    ConnectorIntegration<
        ProcessIncomingWebhook,
        UasWebhookRequestData,
        UasAuthenticationResponseData,
    > for ModularAuthentication
{
}

// UnifiedAuthenticationService marker traits
// impl api::ModularAuthAuthenticationCreate for ModularAuthentication {}
impl api::UasPreAuthentication for ModularAuthentication {}
impl api::UasPostAuthentication for ModularAuthentication {}
impl api::UasAuthenticationConfirmation for ModularAuthentication {}
impl api::UasAuthentication for ModularAuthentication {}
impl api::UasProcessWebhook for ModularAuthentication {}
// impl api::UnifiedAuthenticationService for ModularAuthentication {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for ModularAuthentication {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for ModularAuthentication
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for ModularAuthentication {
    fn id(&self) -> &'static str {
        "modular_authentication"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.modular_authentication.base_url.as_str()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = modular_authentication::ModularAuthenticationAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: modular_authentication::ModularAuthenticationErrorResponse = res
            .response
            .parse_struct("ModularAuthentication ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.error_message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for ModularAuthentication {}

// Add AuthenticationCreate
impl
    ConnectorIntegration<
        AuthenticationCreate,
        AuthenticationCreateRequestData,
        UasAuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &RouterData<
            AuthenticationCreate,
            AuthenticationCreateRequestData,
            UasAuthenticationResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<
            AuthenticationCreate,
            AuthenticationCreateRequestData,
            UasAuthenticationResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}authentication", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<
            AuthenticationCreate,
            AuthenticationCreateRequestData,
            UasAuthenticationResponseData,
        >,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = modular_authentication::construct_authentication_create_request(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &RouterData<
            AuthenticationCreate,
            AuthenticationCreateRequestData,
            UasAuthenticationResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&AuthenticationCreateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(AuthenticationCreateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(AuthenticationCreateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<
            AuthenticationCreate,
            AuthenticationCreateRequestData,
            UasAuthenticationResponseData,
        >,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<
            AuthenticationCreate,
            AuthenticationCreateRequestData,
            UasAuthenticationResponseData,
        >,
        errors::ConnectorError,
    > {
        let response: api_models::authentication::AuthenticationResponse = res
            .response
            .parse_struct("AuthenticationResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// Add PreAuthentication
impl
    ConnectorIntegration<
        PreAuthenticate,
        UasPreAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &UasPreAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &UasPreAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_id = req
            .authentication_id
            .as_ref()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "authentication_id",
            })?
            .get_string_repr();
        Ok(format!(
            "{}authentication/{auth_id}/eligibility",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &UasPreAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = modular_authentication::construct_pre_auth_request(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &UasPreAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&UasPreAuthenticationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(UasPreAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(UasPreAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &UasPreAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<UasPreAuthenticationRouterData, errors::ConnectorError> {
        let response: api_models::authentication::AuthenticationEligibilityResponse = res
            .response
            .parse_struct("ModularAuthentication AuthenticationEligibilityResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// Add Authentication
impl ConnectorIntegration<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>
    for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &UasAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &UasAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let authentication_id = req
            .authentication_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
        Ok(format!(
            "{}authentication/{}/authenticate",
            self.base_url(connectors),
            authentication_id.get_string_repr()
        ))
    }

    fn get_request_body(
        &self,
        req: &UasAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = modular_authentication::construct_authentication_request(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &UasAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&UasAuthenticationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(UasAuthenticationType::get_headers(self, req, connectors)?)
                .set_body(UasAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &UasAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<UasAuthenticationRouterData, errors::ConnectorError> {
        let response: api_models::authentication::AuthenticationAuthenticateResponse = res
            .response
            .parse_struct("ModularAuthentication AuthenticationAuthenticateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// Add PostAuthentication
impl
    ConnectorIntegration<
        PostAuthenticate,
        UasPostAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &UasPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &UasPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let authentication_id = req
            .request
            .connector_authentication_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
        Ok(format!(
            "{}authentication/{}/{}/sync",
            self.base_url(connectors),
            req.merchant_id.get_string_repr(),
            authentication_id
        ))
    }

    fn get_request_body(
        &self,
        req: &UasPostAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = modular_authentication::construct_post_auth_request(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &UasPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&UasPostAuthenticationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(UasPostAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(UasPostAuthenticationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &UasPostAuthenticationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<UasPostAuthenticationRouterData, errors::ConnectorError> {
        let response: api_models::authentication::AuthenticationSyncResponse = res
            .response
            .parse_struct("ModularAuthentication AuthenticationSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorSpecifications for ModularAuthentication {}
