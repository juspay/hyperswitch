pub mod transformers;

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
        payments::{
            Authorize, Capture, CreateConnectorCustomer, PSync, PaymentMethodToken, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        vault::ExternalVaultCreateFlow,
    },
    router_request_types::{
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData, VaultRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData, VaultResponseData},
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, PaymentsCancelRouterData,
        PaymentsCaptureRouterData, PaymentsSessionRouterData, PaymentsSyncRouterData,
        RefreshTokenRouterData, RefundsRouterData, SetupMandateRouterData, TokenizationRouterData,
        VaultRouterData,
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
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask};
use transformers as hyperswitch_vault;

use crate::{constants::headers, types::ResponseRouterData};

#[derive(Clone)]
pub struct HyperswitchVault;

impl api::Payment for HyperswitchVault {}
impl api::PaymentSession for HyperswitchVault {}
impl api::ConnectorAccessToken for HyperswitchVault {}
impl api::MandateSetup for HyperswitchVault {}
impl api::PaymentAuthorize for HyperswitchVault {}
impl api::PaymentSync for HyperswitchVault {}
impl api::PaymentCapture for HyperswitchVault {}
impl api::PaymentVoid for HyperswitchVault {}
impl api::Refund for HyperswitchVault {}
impl api::RefundExecute for HyperswitchVault {}
impl api::RefundSync for HyperswitchVault {}
impl api::PaymentToken for HyperswitchVault {}
impl api::ExternalVaultCreate for HyperswitchVault {}
impl api::ConnectorCustomer for HyperswitchVault {}

impl ConnectorIntegration<ExternalVaultCreateFlow, VaultRequestData, VaultResponseData>
    for HyperswitchVault
{
    fn get_headers(
        &self,
        req: &VaultRouterData<ExternalVaultCreateFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &VaultRouterData<ExternalVaultCreateFlow>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v2/payment-method-sessions",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &VaultRouterData<ExternalVaultCreateFlow>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = hyperswitch_vault::HyperswitchVaultCreateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &VaultRouterData<ExternalVaultCreateFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ExternalVaultCreateType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ExternalVaultCreateType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ExternalVaultCreateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &VaultRouterData<ExternalVaultCreateFlow>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<VaultRouterData<ExternalVaultCreateFlow>, errors::ConnectorError> {
        let response: hyperswitch_vault::HyperswitchVaultCreateResponse = res
            .response
            .parse_struct("HyperswitchVault HyperswitchVaultCreateResponse")
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for HyperswitchVault
{
    fn build_request(
        &self,
        _req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentMethodTokenization".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for HyperswitchVault
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = hyperswitch_vault::HyperswitchVaultAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let api_key = auth.api_key.expose();

        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("api-key={api_key}").into_masked(),
            ),
            (
                headers::X_PROFILE_ID.to_string(),
                auth.profile_id.expose().into_masked(),
            ),
        ];
        Ok(header)
    }
}

impl ConnectorCommon for HyperswitchVault {
    fn id(&self) -> &'static str {
        "hyperswitch_vault"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.hyperswitch_vault.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: hyperswitch_vault::HyperswitchVaultErrorResponse = res
            .response
            .parse_struct("HyperswitchVaultErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.code,
            message: response.error.error_type,
            reason: response.error.message,
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
    for HyperswitchVault
{
    fn get_headers(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v2/customers", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req =
            hyperswitch_vault::HyperswitchVaultCustomerCreateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ConnectorCustomerType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorCustomerType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ConnectorCustomerType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorCustomerRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<ConnectorCustomerRouterData, errors::ConnectorError> {
        let response: hyperswitch_vault::HyperswitchVaultCustomerCreateResponse = res
            .response
            .parse_struct("HyperswitchVault HyperswitchVaultCustomerCreateResponse")
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

impl ConnectorValidation for HyperswitchVault {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for HyperswitchVault {
    fn build_request(
        &self,
        _req: &PaymentsSessionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentsSession".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for HyperswitchVault
{
    fn build_request(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "AccessTokenAuthorize".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for HyperswitchVault
{
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "SetupMandate".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for HyperswitchVault
{
    fn build_request(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentsAuthorize".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for HyperswitchVault {
    fn build_request(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentsSync".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for HyperswitchVault {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentsCapture".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for HyperswitchVault {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "PaymentsCapture".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for HyperswitchVault {
    fn build_request(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refunds".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for HyperswitchVault {
    fn build_request(
        &self,
        _req: &RefundsRouterData<RSync>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "RefundsSync".to_string(),
            connector: self.id().to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for HyperswitchVault {
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

impl ConnectorSpecifications for HyperswitchVault {
    fn should_call_connector_customer(
        &self,
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> bool {
        true
    }
}
