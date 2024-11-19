pub mod transformers;

use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Mask};

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
    request::{Method, Request, RequestBuilder, RequestContent},
};

use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData,
        PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorValidation},
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils,
};

use transformers as uas;

#[derive(Clone)]
pub struct Uas {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync)
}

impl Uas {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector
        }
    }
}

impl api::Payment for Uas {}
impl api::PaymentSession for Uas {}
impl api::ConnectorAccessToken for Uas {}
impl api::MandateSetup for Uas {}
impl api::PaymentAuthorize for Uas {}
impl api::PaymentSync for Uas {}
impl api::PaymentCapture for Uas {}
impl api::PaymentVoid for Uas {}
impl api::Refund for Uas {}
impl api::RefundExecute for Uas {}
impl api::RefundSync for Uas {}
impl api::PaymentToken for Uas {}

impl
    ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Uas
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Uas
where
    Self: ConnectorIntegration<Flow, Request, Response>,{
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

impl ConnectorCommon for Uas {
    fn id(&self) -> &'static str {
        "uas"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.uas.base_url.as_ref()
    }

    fn get_auth_header(&self, auth_type:&ConnectorAuthType)-> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        let auth =  uas::UasAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key.expose().into_masked())])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: uas::UasErrorResponse = res
            .response
            .parse_struct("UasErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Uas
{
    //TODO: implement functions when support enabled
}

impl
    ConnectorIntegration<
        Session,
        PaymentsSessionData,
        PaymentsResponseData,
    > for Uas
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Uas
{
}

impl
    ConnectorIntegration<
        SetupMandate,
        SetupMandateRequestData,
        PaymentsResponseData,
    > for Uas
{
}

impl
    ConnectorIntegration<
        Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for Uas {}

impl
    ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
    for Uas
{}

impl
    ConnectorIntegration<
        Capture,
        PaymentsCaptureData,
        PaymentsResponseData,
    > for Uas
{}

impl
    ConnectorIntegration<
        Void,
        PaymentsCancelData,
        PaymentsResponseData,
    > for Uas
{}

impl
    ConnectorIntegration<
        Execute,
        RefundsData,
        RefundsResponseData,
    > for Uas 
{}

impl
    ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Uas 
{}

impl api::ConnectorPreAuthentication for Uas {}
impl api::ConnectorPreAuthenticationVersionCall for Uas {}
impl api::ExternalAuthentication for Uas {}
impl api::ConnectorAuthentication for Uas {}
impl api::ConnectorPostAuthentication for Uas {}

impl ConnectorIntegration<
    api::PreAuthentication,
    types::authentication::PreAuthNRequestData,
    types::authentication::AuthenticationResponseData,
> for Uas {
    fn get_headers(
        &self,
        _req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(std::vec![])
    }

    fn get_content_type(&self) -> &'static str {
        mime::APPLICATION_JSON.essence_str()
    }

    fn get_http_method(&self) -> Method {
        Method::Post
    }

    fn get_url(
        &self,
        _req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(String::new())
    }

    fn get_request_body(
        &self,
        _req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::Json(Box::new(serde_json::json!(r#"{}"#))))
    }

    fn get_request_form_data(
        &self,
        _req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
    ) -> CustomResult<Option<reqwest::multipart::Form>, errors::ConnectorError> {
        Ok(None)
    }

    fn build_request(
        &self,
        req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        hyperswitch_interfaces::metrics::UNIMPLEMENTED_FLOW.add(
            &hyperswitch_interfaces::metrics::CONTEXT,
            1,
            &router_env::metrics::add_attributes([("connector", req.connector.clone())]),
        );
        Ok(None)
    }

    fn handle_response(
        &self,
        data: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        _res: types::Response,
    ) -> CustomResult<RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>, errors::ConnectorError>
    where
        api::PreAuthentication: Clone,
        types::authentication::PreAuthNRequestData: Clone,
        types::authentication::AuthenticationResponseData: Clone,
    {
        event_builder.map(|e| e.set_error(serde_json::json!({"error": "Not Implemented"})));
        Ok(data.clone())
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
        Ok(ErrorResponse::get_not_implemented())
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
        let error_message = match res.status_code {
            500 => "internal_server_error",
            501 => "not_implemented",
            502 => "bad_gateway",
            503 => "service_unavailable",
            504 => "gateway_timeout",
            505 => "http_version_not_supported",
            506 => "variant_also_negotiates",
            507 => "insufficient_storage",
            508 => "loop_detected",
            510 => "not_extended",
            511 => "network_authentication_required",
            _ => "unknown_error",
        };
        Ok(ErrorResponse {
            code: res.status_code.to_string(),
            message: error_message.to_string(),
            reason: String::from_utf8(res.response.to_vec()).ok(),
            status_code: res.status_code,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }

    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<api::CaptureSyncMethod, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("multiple capture sync".into()).into())
    }

    fn get_certificate(
        &self,
        _req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }

    fn get_certificate_key(
        &self,
        _req: &RouterData<api::PreAuthentication, types::authentication::PreAuthNRequestData, types::authentication::AuthenticationResponseData>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        Ok(None)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Uas {
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
