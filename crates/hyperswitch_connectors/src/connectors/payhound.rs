pub mod transformers;
use std::sync::LazyLock;

use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse},
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
};
use hyperswitch_interfaces::{
    api, configs::Connectors, errors, events::connector_api_logs::ConnectorEvent, types::Response,
    webhooks,
};

#[derive(Clone)]
pub struct Payhound {}

impl Payhound {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Payhound {}
impl api::PaymentSession for Payhound {}
impl api::ConnectorAccessToken for Payhound {}
impl api::MandateSetup for Payhound {}
impl api::PaymentAuthorize for Payhound {}
impl api::PaymentSync for Payhound {}
impl api::PaymentCapture for Payhound {}
impl api::PaymentVoid for Payhound {}
impl api::Refund for Payhound {}
impl api::RefundExecute for Payhound {}
impl api::RefundSync for Payhound {}
impl api::PaymentToken for Payhound {}

impl api::ConnectorCommon for Payhound {
    fn id(&self) -> &'static str {
        "payhound"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.payhound.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(Vec::new())
    }

    fn build_error_response(
        &self,
        _res: Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Payhound".to_string()).into())
    }
}

impl api::ConnectorValidation for Payhound {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Payhound {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Payhound {}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Payhound
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Payhound
{
}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Payhound {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Payhound {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Payhound {}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Payhound {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Payhound {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Payhound
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Payhound {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }
}

static PAYHOUND_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            common_enums::CaptureMethod::Automatic,
            common_enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut payhound_supported_payment_methods = SupportedPaymentMethods::new();

        payhound_supported_payment_methods.add(
            common_enums::PaymentMethod::Crypto,
            common_enums::PaymentMethodType::CryptoCurrency,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::NotSupported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        payhound_supported_payment_methods
    });

static PAYHOUND_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Payhound",
    description: "Payhound is a hosted cryptocurrency payment gateway that lets merchants accept crypto payments through a redirect based invoice checkout.",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Beta,
};

static PAYHOUND_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 1] =
    [common_enums::EventClass::Payments];

impl api::ConnectorSpecifications for Payhound {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PAYHOUND_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PAYHOUND_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&PAYHOUND_SUPPORTED_WEBHOOK_FLOWS)
    }
}
