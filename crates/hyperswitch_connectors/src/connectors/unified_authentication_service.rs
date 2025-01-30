pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        Authenticate, AuthenticationConfirmation, PostAuthenticate, PreAuthenticate,
    },
    router_request_types::{
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        UasAuthenticationConfirmationRouterData, UasPostAuthenticationRouterData,
        UasPreAuthenticationRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::NO_ERROR_MESSAGE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use transformers as unified_authentication_service;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct UnifiedAuthenticationService {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl UnifiedAuthenticationService {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for UnifiedAuthenticationService {}
impl api::PaymentSession for UnifiedAuthenticationService {}
impl api::ConnectorAccessToken for UnifiedAuthenticationService {}
impl api::MandateSetup for UnifiedAuthenticationService {}
impl api::PaymentAuthorize for UnifiedAuthenticationService {}
impl api::PaymentSync for UnifiedAuthenticationService {}
impl api::PaymentCapture for UnifiedAuthenticationService {}
impl api::PaymentVoid for UnifiedAuthenticationService {}
impl api::Refund for UnifiedAuthenticationService {}
impl api::RefundExecute for UnifiedAuthenticationService {}
impl api::RefundSync for UnifiedAuthenticationService {}
impl api::PaymentToken for UnifiedAuthenticationService {}
impl api::UnifiedAuthenticationService for UnifiedAuthenticationService {}
impl api::UasPreAuthentication for UnifiedAuthenticationService {}
impl api::UasPostAuthentication for UnifiedAuthenticationService {}
impl api::UasAuthenticationConfirmation for UnifiedAuthenticationService {}
impl api::UasAuthentication for UnifiedAuthenticationService {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response>
    for UnifiedAuthenticationService
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::SOURCE.to_string(),
                self.get_content_type().to_string().into(),
            ),
        ];
        Ok(header)
    }
}

impl ConnectorCommon for UnifiedAuthenticationService {
    fn id(&self) -> &'static str {
        "unified_authentication_service"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.unified_authentication_service.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: unified_authentication_service::UnifiedAuthenticationServiceErrorResponse =
            res.response
                .parse_struct("UnifiedAuthenticationServiceErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.clone(),
            message: NO_ERROR_MESSAGE.to_owned(),
            reason: Some(response.error),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for UnifiedAuthenticationService {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
}

impl
    ConnectorIntegration<
        AuthenticationConfirmation,
        UasConfirmationRequestData,
        UasAuthenticationResponseData,
    > for UnifiedAuthenticationService
{
    fn get_headers(
        &self,
        req: &UasAuthenticationConfirmationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &UasAuthenticationConfirmationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}confirmation", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &UasAuthenticationConfirmationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.transaction_amount,
            req.request.transaction_currency,
        )?;

        let connector_router_data =
            unified_authentication_service::UnifiedAuthenticationServiceRouterData::from((
                amount, req,
            ));
        let connector_req =
            unified_authentication_service::UnifiedAuthenticationServiceAuthenticateConfirmationRequest::try_from(
                &connector_router_data,
            )?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &UasAuthenticationConfirmationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::UasAuthenticationConfirmationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::UasAuthenticationConfirmationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::UasAuthenticationConfirmationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &UasAuthenticationConfirmationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<UasAuthenticationConfirmationRouterData, errors::ConnectorError> {
        let response: unified_authentication_service::UnifiedAuthenticationServiceAuthenticateConfirmationResponse =
            res.response
                .parse_struct("UnifiedAuthenticationService UnifiedAuthenticationServiceAuthenticateConfirmationResponse")
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

impl
    ConnectorIntegration<
        PreAuthenticate,
        UasPreAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for UnifiedAuthenticationService
{
    fn get_headers(
        &self,
        req: &UasPreAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &UasPreAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pre_authentication_processing",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &UasPreAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let transaction_details = req.request.transaction_details.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "transaction_details",
            },
        )?;
        let amount = utils::convert_amount(
            self.amount_converter,
            transaction_details
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                })?,
            transaction_details
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?,
        )?;

        let connector_router_data =
            unified_authentication_service::UnifiedAuthenticationServiceRouterData::from((
                amount, req,
            ));
        let connector_req =
            unified_authentication_service::UnifiedAuthenticationServicePreAuthenticateRequest::try_from(
                &connector_router_data,
            )?;
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
                .url(&types::UasPreAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::UasPreAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::UasPreAuthenticationType::get_request_body(
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
        let response: unified_authentication_service::UnifiedAuthenticationServicePreAuthenticateResponse =
            res.response
                .parse_struct("UnifiedAuthenticationService UnifiedAuthenticationServicePreAuthenticateResponse")
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

impl
    ConnectorIntegration<
        PostAuthenticate,
        UasPostAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for UnifiedAuthenticationService
{
    fn get_headers(
        &self,
        req: &UasPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &UasPostAuthenticationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}post_authentication_sync",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &UasPostAuthenticationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            unified_authentication_service::UnifiedAuthenticationServicePostAuthenticateRequest::try_from(
                req,
            )?;
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
                .url(&types::UasPostAuthenticationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::UasPostAuthenticationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::UasPostAuthenticationType::get_request_body(
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
        let response: unified_authentication_service::UnifiedAuthenticationServicePostAuthenticateResponse =
            res.response
                .parse_struct("UnifiedAuthenticationService UnifiedAuthenticationServicePostAuthenticateResponse")
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

impl ConnectorIntegration<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData>
    for UnifiedAuthenticationService
{
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData>
    for UnifiedAuthenticationService
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for UnifiedAuthenticationService {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorSpecifications for UnifiedAuthenticationService {}
