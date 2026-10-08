pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, RouterData},
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
        PaymentsSessionRouterData, PaymentsSyncRouterData, RefreshTokenRouterData,
        RefundExecuteRouterData, RefundSyncRouterData, SetupMandateRouterData,
        TokenizationRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors, webhooks,
};
use transformers as betterpayment;

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

#[derive(Clone)]
pub struct Betterpayment {}

impl Betterpayment {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Betterpayment {}
impl api::PaymentSession for Betterpayment {}
impl api::ConnectorAccessToken for Betterpayment {}
impl api::MandateSetup for Betterpayment {}
impl api::PaymentAuthorize for Betterpayment {}
impl api::PaymentSync for Betterpayment {}
impl api::PaymentCapture for Betterpayment {}
impl api::PaymentVoid for Betterpayment {}
impl api::Refund for Betterpayment {}
impl api::RefundExecute for Betterpayment {}
impl api::RefundSync for Betterpayment {}
impl api::PaymentToken for Betterpayment {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Betterpayment
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(vec![(
            crate::constants::headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )])
    }
}

impl ConnectorCommon for Betterpayment {
    fn id(&self) -> &'static str {
        "betterpayment"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Major
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.betterpayment.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        // api_key is sent in the request body; no auth header needed
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: hyperswitch_domain_models::router_data::Response,
        event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
    ) -> CustomResult<hyperswitch_domain_models::router_data::ErrorResponse, errors::ConnectorError>
    {
        let response: serde_json::Value = serde_json::from_slice(&res.response)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let message = response
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string();

        crate::utils::build_error_response(message, None, res.status_code, event_builder)
    }
}

impl ConnectorValidation for Betterpayment {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Betterpayment
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Betterpayment {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Betterpayment
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Betterpayment
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
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
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/rest/payment", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<crate::types::RequestContent, errors::ConnectorError> {
        let amount = crate::utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_req = betterpayment::BetterpaymentPaymentsRequest::try_from(
            &betterpayment::BetterpaymentRouterData::from((amount, req)),
        )?;
        Ok(crate::types::RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            crate::utils::RequestBuilder::new()
                .method(common_utils::request::Method::Post)
                .url(&crate::types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(crate::types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(crate::types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        _event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
        res: hyperswitch_domain_models::router_data::Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: betterpayment::BetterpaymentPaymentsResponse = res
            .response
            .parse_struct("BetterpaymentPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: hyperswitch_domain_models::router_data::Response,
        event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
    ) -> CustomResult<hyperswitch_domain_models::router_data::ErrorResponse, errors::ConnectorError>
    {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Betterpayment {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transaction_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .map_err(|_| errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(format!(
            "{}/rest/payment/{}",
            self.base_url(connectors),
            transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            crate::utils::RequestBuilder::new()
                .method(common_utils::request::Method::Get)
                .url(&crate::types::PaymentsSyncType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(crate::types::PaymentsSyncType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        _event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
        res: hyperswitch_domain_models::router_data::Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: betterpayment::BetterpaymentPaymentsResponse = res
            .response
            .parse_struct("BetterpaymentPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: hyperswitch_domain_models::router_data::Response,
        event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
    ) -> CustomResult<hyperswitch_domain_models::router_data::ErrorResponse, errors::ConnectorError>
    {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Betterpayment {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Betterpayment {
    fn get_headers(
        &self,
        req: &RefundExecuteRouterData,
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
        _req: &RefundExecuteRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/rest/refund", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<crate::types::RequestContent, errors::ConnectorError> {
        let amount = crate::utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        let connector_req = betterpayment::BetterpaymentRefundRequest::try_from(
            &betterpayment::BetterpaymentRouterData::from((amount, req)),
        )?;
        Ok(crate::types::RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundExecuteRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            crate::utils::RequestBuilder::new()
                .method(common_utils::request::Method::Post)
                .url(&crate::types::RefundExecuteType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(crate::types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(crate::types::RefundExecuteType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundExecuteRouterData,
        _event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
        res: hyperswitch_domain_models::router_data::Response,
    ) -> CustomResult<RefundExecuteRouterData, errors::ConnectorError> {
        let response: betterpayment::BetterpaymentRefundResponse = res
            .response
            .parse_struct("BetterpaymentRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        RefundExecuteRouterData::try_from(RefundsResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: hyperswitch_domain_models::router_data::Response,
        event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
    ) -> CustomResult<hyperswitch_domain_models::router_data::ErrorResponse, errors::ConnectorError>
    {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Betterpayment {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
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
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}/rest/payment/{}",
            self.base_url(connectors),
            refund_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            crate::utils::RequestBuilder::new()
                .method(common_utils::request::Method::Get)
                .url(&crate::types::RefundSyncType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(crate::types::RefundSyncType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        _event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
        res: hyperswitch_domain_models::router_data::Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: betterpayment::BetterpaymentRefundResponse = res
            .response
            .parse_struct("BetterpaymentRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        RefundSyncRouterData::try_from(RefundsResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: hyperswitch_domain_models::router_data::Response,
        event_builder: Option<&mut hyperswitch_domain_models::event_builder::InstrumentationEventBuilder>,
    ) -> CustomResult<hyperswitch_domain_models::router_data::ErrorResponse, errors::ConnectorError>
    {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Betterpayment {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let payload: betterpayment::BetterpaymentWebhookPayload =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(payload.order_id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let payload: betterpayment::BetterpaymentWebhookPayload =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(match payload.status {
            betterpayment::BetterpaymentPaymentStatus::Completed
            | betterpayment::BetterpaymentPaymentStatus::Success => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            }
            betterpayment::BetterpaymentPaymentStatus::Failed
            | betterpayment::BetterpaymentPaymentStatus::Error => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
            betterpayment::BetterpaymentPaymentStatus::Pending => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            }
            betterpayment::BetterpaymentPaymentStatus::Cancelled => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled
            }
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let payload: betterpayment::BetterpaymentWebhookPayload =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(Box::new(payload))
    }

    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Sha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let payload: betterpayment::BetterpaymentWebhookPayload =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        let hash_str = payload
            .hash
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;

        hex::decode(hash_str).change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let payload: betterpayment::BetterpaymentWebhookPayload =
            serde_json::from_slice(request.body)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let status_str = serde_json::to_string(&payload.status)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let status_str = status_str.trim_matches('"');

        let outgoing_key = std::str::from_utf8(&connector_webhook_secrets.secret)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        // SHA256(transaction_id + order_id + status + outgoing_key)
        let message = format!(
            "{}{}{}{}",
            payload.transaction_id, payload.order_id, status_str, outgoing_key
        );

        Ok(message.into_bytes())
    }
}

static BETTERPAYMENT_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let mut supported = SupportedPaymentMethods::new();
        supported.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::Wero,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: vec![enums::CaptureMethod::Automatic],
                specific_features: None,
            },
        );
        supported
    });

static BETTERPAYMENT_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Better Payment",
    description: "Better Payment Germany GmbH (Deutsche Bank Group) — Wero wallet payment method",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static BETTERPAYMENT_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] =
    [enums::EventClass::Payments];

impl ConnectorSpecifications for Betterpayment {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&BETTERPAYMENT_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*BETTERPAYMENT_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&BETTERPAYMENT_SUPPORTED_WEBHOOK_FLOWS)
    }
}
