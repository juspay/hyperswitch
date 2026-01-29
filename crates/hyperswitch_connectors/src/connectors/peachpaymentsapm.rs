pub mod transformers;

use std::sync::LazyLock;

use common_enums::{enums, CaptureMethod};
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
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
use masking::ExposeInterface;
use transformers as peachpaymentsapm;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Peachpaymentsapm {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Peachpaymentsapm {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Peachpaymentsapm {}
impl api::PaymentSession for Peachpaymentsapm {}
impl api::ConnectorAccessToken for Peachpaymentsapm {}
impl api::MandateSetup for Peachpaymentsapm {}
impl api::PaymentAuthorize for Peachpaymentsapm {}
impl api::PaymentSync for Peachpaymentsapm {}
impl api::PaymentCapture for Peachpaymentsapm {}
impl api::PaymentVoid for Peachpaymentsapm {}
impl api::Refund for Peachpaymentsapm {}
impl api::RefundExecute for Peachpaymentsapm {}
impl api::RefundSync for Peachpaymentsapm {}
impl api::PaymentToken for Peachpaymentsapm {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Peachpaymentsapm
{
    // Not Implemented
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Peachpaymentsapm
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // PeachPayments APM uses form-encoded body for auth, not headers
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )])
    }
}

impl ConnectorCommon for Peachpaymentsapm {
    fn id(&self) -> &'static str {
        "peachpaymentsapm"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.peachpaymentsapm.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // Auth is included in the request body, not headers
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: peachpaymentsapm::PeachpaymentsapmErrorResponse = res
            .response
            .parse_struct("PeachpaymentsapmErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.result.code.clone(),
            message: response.result.description.clone(),
            reason: Some(response.result.description),
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

impl ConnectorValidation for Peachpaymentsapm {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Peachpaymentsapm {
    // Not implemented - sessions not supported
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Peachpaymentsapm
{
    // Not implemented - uses inline auth in request body
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Peachpaymentsapm
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented(
            "Setup Mandate flow for Peachpaymentsapm".to_string(),
        )
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Peachpaymentsapm
{
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
        Ok(format!("{}/payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = peachpaymentsapm::PeachpaymentsapmRouterData::from((amount, req));
        let connector_req =
            peachpaymentsapm::PeachpaymentsapmPaymentsRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: peachpaymentsapm::PeachpaymentsapmPaymentsResponse = res
            .response
            .parse_struct("Peachpaymentsapm PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Peachpaymentsapm {
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
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        let auth = peachpaymentsapm::PeachpaymentsapmAuthType::try_from(&req.connector_auth_type)?;

        // Payments API v2: GET /payments/{uniqueId}?authentication.entityId=xxx&...
        Ok(format!(
            "{}/payments/{}?authentication.entityId={}&authentication.userId={}&authentication.password={}",
            self.base_url(connectors),
            connector_payment_id,
            auth.entity_id.expose(),
            auth.username.expose(),
            auth.password.expose()
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
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: peachpaymentsapm::PeachpaymentsapmSyncResponse = res
            .response
            .parse_struct("Peachpaymentsapm PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Peachpaymentsapm {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        // APM payments are typically auto-captured
        Err(errors::ConnectorError::NotSupported {
            message: "Manual capture".to_string(),
            connector: "Peachpaymentsapm",
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Peachpaymentsapm {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Payment cancellation".to_string(),
            connector: "Peachpaymentsapm",
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Peachpaymentsapm {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Payments API v2: POST /payments/{uniqueId} for refunds
        Ok(format!(
            "{}/payments/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = peachpaymentsapm::PeachpaymentsapmRefundRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::RefundExecuteType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: peachpaymentsapm::PeachpaymentsapmRefundResponse = res
            .response
            .parse_struct("Peachpaymentsapm RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Peachpaymentsapm {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_refund_id = req
            .request
            .connector_refund_id
            .as_ref()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;
        let auth = peachpaymentsapm::PeachpaymentsapmAuthType::try_from(&req.connector_auth_type)?;

        // Payments API v2: GET /payments/{uniqueId}?authentication.entityId=xxx&...
        Ok(format!(
            "{}/payments/{}?authentication.entityId={}&authentication.userId={}&authentication.password={}",
            self.base_url(connectors),
            connector_refund_id,
            auth.entity_id.expose(),
            auth.username.expose(),
            auth.password.expose()
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: peachpaymentsapm::PeachpaymentsapmSyncResponse = res
            .response
            .parse_struct("Peachpaymentsapm RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        // Map sync response to refund status
        let refund_status = match response.result.code.as_str() {
            c if c.starts_with("000.000") || c.starts_with("000.100") => {
                enums::RefundStatus::Success
            }
            c if c.starts_with("000.200") => enums::RefundStatus::Pending,
            _ => enums::RefundStatus::Failure,
        };

        Ok(RefundSyncRouterData {
            response: Ok(RefundsResponseData {
                connector_refund_id: response.id,
                refund_status,
            }),
            ..data.clone()
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
impl webhooks::IncomingWebhook for Peachpaymentsapm {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_body: peachpaymentsapm::PeachpaymentsapmWebhookBody = request
            .body
            .parse_struct("PeachpaymentsapmWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(webhook_body.id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook_body: peachpaymentsapm::PeachpaymentsapmWebhookBody = request
            .body
            .parse_struct("PeachpaymentsapmWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let status = peachpaymentsapm::map_result_code_to_status(&webhook_body.result.code);

        match status {
            enums::AttemptStatus::Charged => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
            }
            enums::AttemptStatus::Failure | enums::AttemptStatus::AuthenticationFailed => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
            }
            enums::AttemptStatus::Pending | enums::AttemptStatus::AuthenticationPending => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing)
            }
            _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_body: peachpaymentsapm::PeachpaymentsapmWebhookBody = request
            .body
            .parse_struct("PeachpaymentsapmWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(webhook_body))
    }
}

// Supported payment methods configuration
static PEACHPAYMENTSAPM_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![CaptureMethod::Automatic];

        let mut supported = SupportedPaymentMethods::new();

        // Bank Transfer - Local (PayShap, Capitec Pay, Peach EFT)
        supported.add(
            enums::PaymentMethod::BankTransfer,
            enums::PaymentMethodType::LocalBankTransfer,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        // Real Time Payment (PayShap)
        supported.add(
            enums::PaymentMethod::RealTimePayment,
            enums::PaymentMethodType::Fps,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        supported
    });

static PEACHPAYMENTSAPM_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Peach Payments APM",
    description: "PeachPayments Alternative Payment Methods for African markets including EFT, Mobile Money, and BNPL.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static PEACHPAYMENTSAPM_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] =
    [enums::EventClass::Payments];

impl ConnectorSpecifications for Peachpaymentsapm {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PEACHPAYMENTSAPM_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PEACHPAYMENTSAPM_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&PEACHPAYMENTSAPM_SUPPORTED_WEBHOOK_FLOWS)
    }
}
