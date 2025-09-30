pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        ExternalVaultInsertFlow, ExternalVaultRetrieveFlow,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData, VaultRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods,
        VaultResponseData,
    },
    types::VaultRouterData,
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask};
use transformers as tokenex;

use crate::{constants::headers, types::ResponseRouterData};

#[derive(Clone)]
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
impl api::ExternalVaultInsert for Tokenex {}
impl api::ExternalVault for Tokenex {}
impl api::ExternalVaultRetrieve for Tokenex {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Tokenex
{
    // Not Implemented (R)
}

pub mod auth_headers {
    pub const TOKENEX_ID: &str = "tx-tokenex-id";
    pub const TOKENEX_API_KEY: &str = "tx-apikey";
    pub const TOKENEX_SCHEME: &str = "tx-token-scheme";
    pub const TOKENEX_SCHEME_VALUE: &str = "PCI";
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Tokenex
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = tokenex::TokenexAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                auth_headers::TOKENEX_ID.to_string(),
                auth.tokenex_id.expose().into_masked(),
            ),
            (
                auth_headers::TOKENEX_API_KEY.to_string(),
                auth.api_key.expose().into_masked(),
            ),
            (
                auth_headers::TOKENEX_SCHEME.to_string(),
                auth_headers::TOKENEX_SCHEME_VALUE.to_string().into(),
            ),
        ];
        Ok(header)
    }
}

impl ConnectorCommon for Tokenex {
    fn id(&self) -> &'static str {
        "tokenex"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.tokenex.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = tokenex::TokenexAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: tokenex::TokenexErrorResponse = res
            .response
            .parse_struct("TokenexErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let (code, message) = response.error.split_once(':').unwrap_or(("", ""));

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: code.to_string(),
            message: message.to_string(),
            reason: Some(response.message),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Tokenex {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Tokenex {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Tokenex {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Tokenex {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Tokenex {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Tokenex {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Tokenex {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Tokenex {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Tokenex {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Tokenex {}

impl ConnectorIntegration<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>
    for Tokenex
{
    fn get_url(
        &self,
        _req: &VaultRouterData<ExternalVaultInsertFlow>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v2/Pci/Tokenize", self.base_url(connectors)))
    }

    fn get_headers(
        &self,
        req: &VaultRouterData<ExternalVaultInsertFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &VaultRouterData<ExternalVaultInsertFlow>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = tokenex::TokenexInsertRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &VaultRouterData<ExternalVaultInsertFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ExternalVaultInsertType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ExternalVaultInsertType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ExternalVaultInsertType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &VaultRouterData<ExternalVaultInsertFlow>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<VaultRouterData<ExternalVaultInsertFlow>, errors::ConnectorError> {
        let response: tokenex::TokenexInsertResponse = res
            .response
            .parse_struct("TokenexInsertResponse")
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

impl ConnectorIntegration<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>
    for Tokenex
{
    fn get_url(
        &self,
        _req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v2/Pci/DetokenizeWithCvv",
            self.base_url(connectors)
        ))
    }

    fn get_headers(
        &self,
        req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = tokenex::TokenexRetrieveRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ExternalVaultRetrieveType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ExternalVaultRetrieveType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ExternalVaultRetrieveType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &VaultRouterData<ExternalVaultRetrieveFlow>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<VaultRouterData<ExternalVaultRetrieveFlow>, errors::ConnectorError> {
        let response: tokenex::TokenexRetrieveResponse = res
            .response
            .parse_struct("TokenexRetrieveResponse")
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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Tokenex {
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

static TOKENEX_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static TOKENEX_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Tokenex",
    description: "Tokenex connector",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static TOKENEX_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Tokenex {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&TOKENEX_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*TOKENEX_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&TOKENEX_SUPPORTED_WEBHOOK_FLOWS)
    }
}
