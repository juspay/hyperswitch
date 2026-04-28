pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::authentication::{
        Authentication, AuthenticationCreate, PostAuthentication, PreAuthentication,
        PreAuthenticationVersionCall,
    },
    router_request_types::authentication::{
        ConnectorAuthenticationCreateRequestData, ConnectorAuthenticationRequestData,
        ConnectorPostAuthenticationRequestData, PreAuthNRequestData,
    },
    router_response_types::AuthenticationResponseData,
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
};
use hyperswitch_masking::Mask;

use crate::{constants::headers, types, types::ResponseRouterData};
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

impl api::authentication::ConnectorAuthentication for ModularAuthentication {}
impl api::authentication::ConnectorAuthenticationCreate for ModularAuthentication {}
impl api::authentication::ConnectorPreAuthentication for ModularAuthentication {}
impl api::authentication::ConnectorPreAuthenticationVersionCall for ModularAuthentication {}
impl api::authentication::ConnectorPostAuthentication for ModularAuthentication {}

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
        ConnectorAuthenticationCreateRequestData,
        AuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &RouterData<
            AuthenticationCreate,
            ConnectorAuthenticationCreateRequestData,
            AuthenticationResponseData,
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
            ConnectorAuthenticationCreateRequestData,
            AuthenticationResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}authentication", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<
            AuthenticationCreate,
            ConnectorAuthenticationCreateRequestData,
            AuthenticationResponseData,
        >,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = modular_authentication::ModularAuthenticationCreateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &RouterData<
            AuthenticationCreate,
            ConnectorAuthenticationCreateRequestData,
            AuthenticationResponseData,
        >,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::AuthenticationCreateType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::AuthenticationCreateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::AuthenticationCreateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<
            AuthenticationCreate,
            ConnectorAuthenticationCreateRequestData,
            AuthenticationResponseData,
        >,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<
            AuthenticationCreate,
            ConnectorAuthenticationCreateRequestData,
            AuthenticationResponseData,
        >,
        errors::ConnectorError,
    > {
        let response: modular_authentication::ModularAuthenticationCreateResponse = res
            .response
            .parse_struct("ModularAuthenticationCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(types::ResponseRouterData {
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
impl ConnectorIntegration<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
    for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &types::PreAuthNRouterData,
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
        req: &types::PreAuthNRouterData,
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
        req: &types::PreAuthNRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            modular_authentication::ModularAuthenticationPreAuthRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PreAuthNRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        data: &types::PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PreAuthNRouterData, errors::ConnectorError> {
        let response: modular_authentication::ModularAuthenticationPreAuthResponse = res
            .response
            .parse_struct("ModularAuthentication ModularAuthenticationPreAuthResponse")
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

// Add PreAuthenticationVersionCall
impl
    ConnectorIntegration<
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &types::PreAuthNVersionCallRouterData,
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
        req: &types::PreAuthNVersionCallRouterData,
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
            "{}authentication/{auth_id}/eligibility-check",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PreAuthNVersionCallRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            modular_authentication::ModularAuthenticationPreAuthVersionCallRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PreAuthNVersionCallRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ConnectorPreAuthenticationVersionCallType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(
                    types::ConnectorPreAuthenticationVersionCallType::get_headers(
                        self, req, connectors,
                    )?,
                )
                .set_body(
                    types::ConnectorPreAuthenticationVersionCallType::get_request_body(
                        self, req, connectors,
                    )?,
                )
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PreAuthNVersionCallRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PreAuthNVersionCallRouterData, errors::ConnectorError> {
        let response: modular_authentication::ModularAuthenticationPreAuthVersionCallResponse = res
            .response
            .parse_struct("ModularAuthentication ModularAuthenticationPreAuthVersionCallResponse")
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
impl
    ConnectorIntegration<
        Authentication,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
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
        req: &types::ConnectorAuthenticationRouterData,
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
        req: &types::ConnectorAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            modular_authentication::ModularAuthenticationAuthenticationRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::ConnectorAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::ConnectorAuthenticationRouterData, errors::ConnectorError> {
        let response: modular_authentication::ModularAuthenticationAuthenticationResponse = res
            .response
            .parse_struct("ModularAuthentication ModularAuthenticationAuthenticationResponse")
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
        PostAuthentication,
        ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    > for ModularAuthentication
{
    fn get_headers(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
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
        req: &types::ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let authentication_id = req
            .authentication_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorAuthenticationID)?;
        Ok(format!(
            "{}authentication/{}/{}/sync",
            self.base_url(connectors),
            req.merchant_id.get_string_repr(),
            authentication_id.get_string_repr()
        ))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            modular_authentication::ModularAuthenticationPostAuthRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::ConnectorPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::ConnectorPostAuthenticationRouterData, errors::ConnectorError> {
        let response: modular_authentication::ModularAuthenticationPostAuthResponse = res
            .response
            .parse_struct("ModularAuthentication ModularAuthenticationPostAuthResponse")
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
