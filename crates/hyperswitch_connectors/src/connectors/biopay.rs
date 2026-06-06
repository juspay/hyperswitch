pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};

use error_stack::{report, ResultExt};

use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::{PaymentsAuthenticateData, PaymentsPostAuthenticateData},
    router_response_types::{ConnectorInfo, PaymentsResponseData, SupportedPaymentMethods},
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
    webhooks,
};

use hyperswitch_masking::ExposeInterface;
use std::sync::LazyLock;

use common_enums::enums;

use crate::{constants::headers, types::ResponseRouterData};

use transformers as biopay;

#[derive(Clone)]
pub struct Biopay;

impl Biopay {
    pub fn new() -> &'static Self {
        &Self
    }
}

impl api::Payment for Biopay {}
impl api::PaymentAuthorize for Biopay {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Biopay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = biopay::BiopayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                "X-BioPay-Platform-Secret".to_string(),
                auth.api_key.expose().into_masked(),
            ),
        ])
    }
}

impl ConnectorCommon for Biopay {
    fn id(&self) -> &'static str {
        "biopay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.biopay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = biopay::BiopayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(vec![(
            "X-BioPay-Platform-Secret".to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: biopay::BiopayErrorResponse = res
            .response
            .parse_struct("BiopayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
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

impl ConnectorValidation for Biopay {}

impl
    ConnectorIntegration<
        api::Authenticate,
        PaymentsAuthenticateData,
        PaymentsResponseData,
    > for Biopay
{
    fn get_headers(
        &self,
        req: &RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
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
        _req: &RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/api/create_session.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = biopay::BiopayAuthenticateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&<Self as ConnectorIntegration<
                    api::Authenticate,
                    PaymentsAuthenticateData,
                    PaymentsResponseData,
                >>::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(<Self as ConnectorIntegration<
                    api::Authenticate,
                    PaymentsAuthenticateData,
                    PaymentsResponseData,
                >>::get_headers(self, req, connectors)?)
                .set_body(<Self as ConnectorIntegration<
                    api::Authenticate,
                    PaymentsAuthenticateData,
                    PaymentsResponseData,
                >>::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: biopay::BiopayAuthenticateResponse = res
            .response
            .parse_struct("BiopayAuthenticateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));

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
        api::PostAuthenticate,
        PaymentsPostAuthenticateData,
        PaymentsResponseData,
    > for Biopay
{
    fn get_headers(
        &self,
        req: &RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
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
        _req: &RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/api/session_status.php", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = biopay::BiopayPostAuthenticateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&<Self as ConnectorIntegration<
                    api::PostAuthenticate,
                    PaymentsPostAuthenticateData,
                    PaymentsResponseData,
                >>::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(<Self as ConnectorIntegration<
                    api::PostAuthenticate,
                    PaymentsPostAuthenticateData,
                    PaymentsResponseData,
                >>::get_headers(self, req, connectors)?)
                .set_body(<Self as ConnectorIntegration<
                    api::PostAuthenticate,
                    PaymentsPostAuthenticateData,
                    PaymentsResponseData,
                >>::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: biopay::BiopayPostAuthenticateResponse = res
            .response
            .parse_struct("BiopayPostAuthenticateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));

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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Biopay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookResourceData>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static BIOPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static BIOPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "BioPay",
    description: "Passkey powered biometric checkout authentication layer.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static BIOPAY_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Biopay {
    fn is_authentication_flow_required(&self, _current_flow: api::CurrentFlowInfo) -> bool {
        true
    }

    fn is_post_authentication_flow_required(&self, _current_flow: api::CurrentFlowInfo) -> bool {
        true
    }

    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&BIOPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*BIOPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&BIOPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
