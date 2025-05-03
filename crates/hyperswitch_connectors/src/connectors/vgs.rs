pub mod transformers;

use base64::Engine;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payments::VaultData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        VaultInsertFlow,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData, VaultRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData, VaultResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData, VaultRouterData,
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
use transformers as vgs;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Vgs {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Vgs {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

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
impl api::VaultInsert for Vgs {}
impl api::Vault for Vgs {}

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
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Vgs {
    fn id(&self) -> &'static str {
        "vgs"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        todo!()
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.vgs.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = vgs::VgsAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_value = auth
            .username
            .zip(auth.password)
            .map(|(project_id, secret_key)| {
                format!(
                    "Basic {}",
                    common_utils::consts::BASE64_ENGINE
                        .encode(format!("{}:{}", project_id, secret_key))
                )
            });
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth_value.into_masked(),
        )])
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

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.errors[0].code.clone(),
            message: response.errors[0].code.clone(),
            reason: response.errors[0].detail.clone(),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Vgs {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Vgs {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Vgs {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Vgs {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Vgs {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Vgs {}

impl ConnectorIntegration<VaultInsertFlow, VaultRequestData, VaultResponseData> for Vgs {
    fn get_url(
        &self,
        _req: &VaultRouterData<VaultInsertFlow>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}aliases", self.base_url(connectors)))
    }

    fn get_headers(
        &self,
        req: &VaultRouterData<VaultInsertFlow>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = vgs::VgsAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth_value = auth
            .username
            .zip(auth.password)
            .map(|(project_id, secret_key)| {
                format!(
                    "Basic {}",
                    common_utils::consts::BASE64_ENGINE
                        .encode(format!("{}:{}", project_id, secret_key))
                )
            });
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth_value.into_masked(),
        )])
    }

    fn get_request_body(
        &self,
        req: &VaultRouterData<VaultInsertFlow>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        // let connector_router_data = vgs::VgsRouterData::from((amount, req));
        let connector_req = vgs::VgsPaymentsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &VaultRouterData<VaultInsertFlow>,
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
        data: &VaultRouterData<VaultInsertFlow>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<VaultRouterData<VaultInsertFlow>, errors::ConnectorError> {
        let response: vgs::VgsPaymentsResponse = res
            .response
            .parse_struct("Vgs PaymentsAuthorizeResponse")
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
