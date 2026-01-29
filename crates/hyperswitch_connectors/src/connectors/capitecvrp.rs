pub mod transformers;

use std::sync::LazyLock;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        mandate_revoke::MandateRevoke,
        payments::{Authorize, Capture, PaymentMethodToken, PSync, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        webhooks::VerifyWebhookSource,
    },
    router_request_types::{
        AccessTokenRequestData, MandateRevokeRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        MandateRevokeResponseData, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt, VerifyWebhookSourceResponseData,
    },
    types::{
        MandateRevokeRouterData, PaymentsAuthorizeRouterData, PaymentsSyncRouterData,
        RefreshTokenRouterData, RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData,
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
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{Mask, PeekInterface};
use transformers as capitecvrp;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils,
};

#[derive(Clone)]
pub struct Capitecvrp {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Capitecvrp {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Capitecvrp {}
impl api::PaymentSession for Capitecvrp {}
impl api::ConnectorAccessToken for Capitecvrp {}
impl api::MandateSetup for Capitecvrp {}
impl api::PaymentAuthorize for Capitecvrp {}
impl api::PaymentSync for Capitecvrp {}
impl api::PaymentCapture for Capitecvrp {}
impl api::PaymentVoid for Capitecvrp {}
impl api::Refund for Capitecvrp {}
impl api::RefundExecute for Capitecvrp {}
impl api::RefundSync for Capitecvrp {}
impl api::ConnectorMandateRevoke for Capitecvrp {}
impl api::ConnectorVerifyWebhookSource for Capitecvrp {}
impl api::PaymentToken for Capitecvrp {}

// Session - not implemented for VRP flow
impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Capitecvrp {}

// PaymentMethodToken - not implemented for VRP flow
impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Capitecvrp
{
}

// Capture - not implemented (VRP payments are captured automatically)
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Capitecvrp {}

// Azure AD token URL for OAuth2
const AZURE_AD_TOKEN_URL: &str =
    "https://login.microsoftonline.com/a428b46f-c29b-4a6c-85f1-8e05c10b6671/oauth2/token";

impl ConnectorCommon for Capitecvrp {
    fn id(&self) -> &'static str {
        "capitecvrp"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.capitecvrp.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: capitecvrp::CapitecvrpErrorResponse = res
            .response
            .parse_struct("CapitecvrpErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code.clone(),
            message: response.message.clone(),
            reason: Some(response.message),
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Capitecvrp {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple
            | enums::CaptureMethod::Scheduled
            | enums::CaptureMethod::SequentialAutomatic => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("capitecvrp"),
                ))?
            }
        }
    }
}

fn get_correlation_id_header(
) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    let correlation_id = uuid::Uuid::new_v4().to_string();
    Ok(vec![(
        "x-capitec-correlation-id".to_string(),
        correlation_id.into(),
    )])
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Capitecvrp {
    fn get_url(
        &self,
        _req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(AZURE_AD_TOKEN_URL.to_string())
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
            "application/x-www-form-urlencoded".to_string().into(),
        )])
    }

    fn get_request_body(
        &self,
        req: &RefreshTokenRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let auth = capitecvrp::CapitecvrpAuthType::try_from(&req.connector_auth_type)?;
        let connector_req = capitecvrp::CapitecvrpAccessTokenRequest {
            grant_type: "password".to_string(),
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret,
            username: auth.username,
            password: auth.password,
            resource: auth.client_id,
            scope: "openid".to_string(),
        };
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
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .url(&self.get_url(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
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
        let response: capitecvrp::CapitecvrpAccessTokenResponse = res
            .response
            .parse_struct("CapitecvrpAccessTokenResponse")
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

// Setup Mandate (Consent Creation)
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Capitecvrp
{
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Default to once-off consent - routing logic can choose recurring
        Ok(format!("{}/consent/vrp/onceoff", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = capitecvrp::CapitecvrpOnceOffConsentRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: capitecvrp::CapitecvrpConsentResponse = res
            .response
            .parse_struct("CapitecvrpConsentResponse")
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

// Payment Sync (Consent Status)
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Capitecvrp {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let consent_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(format!(
            "{}/consent/status/{}",
            self.base_url(connectors),
            consent_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: capitecvrp::CapitecvrpConsentStatusResponse = res
            .response
            .parse_struct("CapitecvrpConsentStatusResponse")
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

// Payment Authorize (Payment Action)
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Capitecvrp {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Default to once-off payment action
        Ok(format!(
            "{}/consent/vrp/onceoff/action_payment",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = capitecvrp::CapitecvrpOnceOffPaymentRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: capitecvrp::CapitecvrpPaymentResponse = res
            .response
            .parse_struct("CapitecvrpPaymentResponse")
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

// Payment Void (not supported)
impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Capitecvrp {}

// Mandate Revoke (Consent Revocation)
impl ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>
    for Capitecvrp
{
    fn get_headers(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let consent_id = req.request.connector_mandate_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_mandate_id",
            },
        )?;

        Ok(format!(
            "{}/consent/revoke/{}",
            self.base_url(connectors),
            consent_id
        ))
    }

    fn build_request(
        &self,
        req: &MandateRevokeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Delete)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &MandateRevokeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<MandateRevokeRouterData, errors::ConnectorError> {
        // 204 No Content response
        event_builder.map(|i| i.set_response_body(&serde_json::json!({"status": "revoked"})));

        RouterData::try_from(ResponseRouterData {
            response: (),
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

// Refund Execute (not supported)
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Capitecvrp {
    fn build_request(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented(
            "Refunds are not supported for Capitec VRP".to_string(),
        )
        .into())
    }
}

// Refund Sync (not supported)
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Capitecvrp {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented(
            "Refund sync is not supported for Capitec VRP".to_string(),
        )
        .into())
    }
}

// Webhook verification
impl ConnectorIntegration<VerifyWebhookSource, VerifyWebhookSourceRequestData, VerifyWebhookSourceResponseData>
    for Capitecvrp
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Capitecvrp
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
        ];

        // Add correlation ID header
        let correlation_headers = get_correlation_id_header()?;
        headers.extend(correlation_headers);

        // Add bearer token if available
        if let Some(access_token) = &req.access_token {
            headers.push((
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into_masked(),
            ));
        }

        Ok(headers)
    }
}

// Incoming Webhook handling
impl IncomingWebhook for Capitecvrp {
    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        // Capitec sends GET request with consentId as query param
        let consent_id = request
            .query_params
            .split('&')
            .find_map(|param| {
                let mut parts = param.split('=');
                match (parts.next(), parts.next()) {
                    (Some("consentId"), Some(value)) => Some(value.to_string()),
                    _ => None,
                }
            })
            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(consent_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        // Capitec webhook doesn't include status - triggers a sync
        Ok(IncomingWebhookEvent::PaymentIntentProcessing)
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let consent_id = request
            .query_params
            .split('&')
            .find_map(|param| {
                let mut parts = param.split('=');
                match (parts.next(), parts.next()) {
                    (Some("consentId"), Some(value)) => Some(value.to_string()),
                    _ => None,
                }
            })
            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(Box::new(serde_json::json!({
            "consent_id": consent_id
        })))
    }
}

// Define supported payment methods for Capitec VRP
static CAPITECVRP_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut capitecvrp_supported_payment_methods = SupportedPaymentMethods::new();

        // Add OpenBanking -> OpenBankingCapitec payment method type
        capitecvrp_supported_payment_methods.add(
            enums::PaymentMethod::OpenBanking,
            enums::PaymentMethodType::OpenBankingCapitec,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::NotSupported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        capitecvrp_supported_payment_methods
    });

impl ConnectorSpecifications for Capitecvrp {
    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*CAPITECVRP_SUPPORTED_PAYMENT_METHODS)
    }
}
