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
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        ExternalVaultInsertFlow, ExternalVaultRetrieveFlow,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData, VaultRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData, VaultResponseData},
    types::{RefreshTokenRouterData, VaultRouterData},
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
use masking::{Mask, PeekInterface};
use transformers as vgs;

use crate::{constants::headers, types::ResponseRouterData};

#[derive(Clone)]
pub struct Vgs;

impl api::Payment for Vgs {}
impl api::PaymentSession for Vgs {}
impl api::ConnectorAccessToken for Vgs {}
impl api::MandateSetup for Vgs {}
impl api::PaymentAuthorize for Vgs {}
impl api::PaymentSync for Vgs {}
impl api::PaymentCapture for Vgs {}
impl api::PaymentVoid for Vgs {}
impl api::Refund for Vgs {}
impl api::RefundExecute for Vgs {}
impl api::RefundSync for Vgs {}
impl api::PaymentToken for Vgs {}
impl api::ExternalVaultInsert for Vgs {}
impl api::ExternalVault for Vgs {}
impl api::ExternalVaultRetrieve for Vgs {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Vgs
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Vgs
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", access_token.token.peek()).into_masked(),
        )])
    }
}

impl ConnectorCommon for Vgs {
    fn id(&self) -> &'static str {
        "vgs"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.vgs.base_url.as_ref()
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
        let response: vgs::VgsErrorResponse = res
            .response
            .parse_struct("VgsErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error = response
            .errors
            .first()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: error.code.clone(),
            message: error.code.clone(),
            reason: error.detail.clone(),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Vgs {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Vgs {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Vgs {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Vgs {
    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_base_url = connectors
            .vgs
            .secondary_base_url
            .as_ref()
            .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
        Ok(format!(
            "{}auth/realms/vgs/protocol/openid-connect/token",
            auth_base_url
        ))
    }
    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }
    fn get_headers(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefreshTokenType::get_content_type(self)
                .to_string()
                .into(),
        )])
    }
    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = vgs::VgsAuthUpdateRequest::try_from(req)?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefreshTokenRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let req = Some(
            RequestBuilder::new()
                .method(Method::Post)
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .set_body(types::RefreshTokenType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        );

        Ok(req)
    }

    fn handle_response(
        &self,
        data: &RefreshTokenRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefreshTokenRouterData, errors::ConnectorError> {
        let response: vgs::VgsAuthUpdateResponse = res
            .response
            .parse_struct("Vgs VgsAuthUpdateResponse")
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
        let response: vgs::VgsAccessTokenErrorResponse = res
            .response
            .parse_struct("Vgs AccessTokenErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.clone(),
            message: response.error.clone(),
            reason: Some(response.error_description),
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData> for Vgs {
    fn get_url(
        &self,
        req: &VaultRouterData<ExternalVaultInsertFlow>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth = vgs::VgsAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let vault_specific_url = base_url.replace("{{vault_id}}", auth.vault_id.peek());

        Ok(format!("{}aliases", vault_specific_url))
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
        let connector_req = vgs::VgsInsertRequest::try_from(req)?;
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
        let response: vgs::VgsInsertResponse = res
            .response
            .parse_struct("VgsInsertResponse")
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

impl ConnectorIntegration<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData> for Vgs {
    fn get_url(
        &self,
        req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = self.base_url(connectors);
        let auth = vgs::VgsAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let vault_specific_url = base_url.replace("{{vault_id}}", auth.vault_id.peek());

        let alias = req.request.connector_vault_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_vault_id",
            },
        )?;

        Ok(format!("{}aliases/{alias}", vault_specific_url))
    }

    fn get_headers(
        &self,
        req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn build_request(
        &self,
        req: &VaultRouterData<ExternalVaultRetrieveFlow>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::ExternalVaultRetrieveType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ExternalVaultRetrieveType::get_headers(
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
        let response: vgs::VgsRetrieveResponse =
            res.response
                .parse_struct("VgsRetrieveResponse")
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
impl webhooks::IncomingWebhook for Vgs {
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

impl ConnectorSpecifications for Vgs {}
