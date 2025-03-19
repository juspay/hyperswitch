pub mod gpayments_types;
pub mod transformers;

use common_utils::{
    request::RequestContent,
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use transformers as gpayments;

use crate::{
    configs::settings,
    connector::{gpayments::gpayments_types::GpaymentsConnectorMetaData, utils::to_connector_meta},
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers, services,
    services::{request, ConnectorIntegration, ConnectorSpecifications, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::BytesExt,
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

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Gpayments
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Gpayments
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.gpayments.base_url.as_ref()
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
        let response: gpayments_types::TDS2ApiError = res
            .response
            .parse_struct("gpayments_types TDS2ApiError")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.error_description,
            reason: response.error_detail,
            attempt_status: None,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorValidation for Gpayments {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Gpayments
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Gpayments
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Gpayments
{
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Gpayments
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Gpayments {
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

impl api::ExternalAuthentication for Gpayments {}
impl api::ConnectorAuthentication for Gpayments {}
impl api::ConnectorPreAuthentication for Gpayments {}
impl api::ConnectorPreAuthenticationVersionCall for Gpayments {}
impl api::ConnectorPostAuthentication for Gpayments {}

fn build_endpoint(
    base_url: &str,
    connector_metadata: &Option<common_utils::pii::SecretSerdeValue>,
) -> CustomResult<String, errors::ConnectorError> {
    let metadata = gpayments::GpaymentsMetaData::try_from(connector_metadata)?;
    let endpoint_prefix = metadata.endpoint_prefix;
    Ok(base_url.replace("{{merchant_endpoint_prefix}}", &endpoint_prefix))
}

impl
    ConnectorIntegration<
        api::Authentication,
        types::authentication::ConnectorAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Gpayments
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
        req: &types::authentication::ConnectorAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
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
        req: &types::authentication::ConnectorAuthenticationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = gpayments::GpaymentsRouterData::from((MinorUnit::zero(), req));
        let req_obj =
            gpayments_types::GpaymentsAuthenticationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }
    fn build_request(
        &self,
        req: &types::authentication::ConnectorAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
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
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
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
        let response: gpayments_types::GpaymentsAuthenticationSuccessResponse = res
            .response
            .parse_struct("gpayments GpaymentsAuthenticationResponse")
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
        api::PostAuthentication,
        types::authentication::ConnectorPostAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for Gpayments
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
        req: &types::authentication::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!(
            "{}/api/v2/auth/brw/result?threeDSServerTransID={}",
            base_url, req.request.threeds_server_transaction_id,
        ))
    }

    fn build_request(
        &self,
        req: &types::authentication::ConnectorPostAuthenticationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
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
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
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
        let response: gpayments_types::GpaymentsPostAuthenticationResponse = res
            .response
            .parse_struct("gpayments PaymentsSyncResponse")
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

impl
    ConnectorIntegration<
        api::PreAuthentication,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for Gpayments
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
        Ok(format!("{}/api/v2/auth/brw/init?mode=custom", base_url,))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = gpayments::GpaymentsRouterData::from((MinorUnit::zero(), req));
        let req_obj =
            gpayments_types::GpaymentsPreAuthenticationRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::authentication::PreAuthNRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
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
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::PreAuthNRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::authentication::PreAuthNRouterData, errors::ConnectorError> {
        let response: gpayments_types::GpaymentsPreAuthenticationResponse = res
            .response
            .parse_struct("gpayments GpaymentsPreAuthenticationResponse")
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
        api::PreAuthenticationVersionCall,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for Gpayments
{
    fn get_headers(
        &self,
        req: &types::authentication::PreAuthNVersionCallRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::authentication::PreAuthNVersionCallRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = build_endpoint(self.base_url(connectors), &req.connector_meta_data)?;
        Ok(format!("{}/api/v2/auth/enrol", base_url,))
    }

    fn get_request_body(
        &self,
        req: &types::authentication::PreAuthNVersionCallRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = gpayments::GpaymentsRouterData::from((MinorUnit::zero(), req));
        let req_obj =
            gpayments_types::GpaymentsPreAuthVersionCallRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &types::authentication::PreAuthNVersionCallRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let gpayments_auth_type = gpayments::GpaymentsAuthType::try_from(&req.connector_auth_type)?;
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(
                    &types::authentication::ConnectorPreAuthenticationVersionCallType::get_url(
                        self, req, connectors,
                    )?,
                )
                .attach_default_headers()
                .headers(
                    types::authentication::ConnectorPreAuthenticationVersionCallType::get_headers(
                        self, req, connectors,
                    )?,
                )
                .set_body(
                    types::authentication::ConnectorPreAuthenticationVersionCallType::get_request_body(
                        self, req, connectors,
                    )?,
                )
                .add_certificate(Some(gpayments_auth_type.certificate))
                .add_certificate_key(Some(gpayments_auth_type.private_key))
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::authentication::PreAuthNVersionCallRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::authentication::PreAuthNVersionCallRouterData, errors::ConnectorError>
    {
        let response: gpayments_types::GpaymentsPreAuthVersionCallResponse = res
            .response
            .parse_struct("gpayments GpaymentsPreAuthVersionCallResponse")
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

impl ConnectorSpecifications for Gpayments {}
