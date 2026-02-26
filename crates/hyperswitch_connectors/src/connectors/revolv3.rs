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
pub struct Revolv3 {}

impl Revolv3 {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Revolv3 {}
impl api::PaymentSession for Revolv3 {}
impl api::ConnectorAccessToken for Revolv3 {}
impl api::MandateSetup for Revolv3 {}
impl api::PaymentAuthorize for Revolv3 {}
impl api::PaymentSync for Revolv3 {}
impl api::PaymentCapture for Revolv3 {}
impl api::PaymentVoid for Revolv3 {}
impl api::Refund for Revolv3 {}
impl api::RefundExecute for Revolv3 {}
impl api::RefundSync for Revolv3 {}
impl api::PaymentToken for Revolv3 {}

impl api::ConnectorCommon for Revolv3 {
    fn id(&self) -> &'static str {
        "revolv3"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.revolv3.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    fn build_error_response(
        &self,
        _res: Response,
        _event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Revolv3".to_string()).into())
    }
}

impl api::ConnectorValidation for Revolv3 {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Revolv3 {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Revolv3 {}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Revolv3
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Revolv3 {}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Revolv3 {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Revolv3 {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Revolv3 {}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Revolv3 {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Revolv3 {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Revolv3
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Revolv3 {
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
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err((errors::ConnectorError::WebhooksNotImplemented).into())
    }
}

static REVOLV3_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let default_capture_methods = vec![
        common_enums::CaptureMethod::Automatic,
        common_enums::CaptureMethod::Manual,
        common_enums::CaptureMethod::SequentialAutomatic,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::Visa,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::UnionPay,
    ];

    let mut revolv3_supported_payment_methods = SupportedPaymentMethods::new();

    revolv3_supported_payment_methods.add(
        common_enums::PaymentMethod::Card,
        common_enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::NotSupported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    },
                ),
            ),
        },
    );

    revolv3_supported_payment_methods
});

static REVOLV3_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Revolv3",
    description: "Revolv3 is a payment platform that enables secure card processing and alternative payment solutions for merchants through a unified API.",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Beta,
};

static REVOLV3_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl api::ConnectorSpecifications for Revolv3 {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&REVOLV3_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*REVOLV3_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&REVOLV3_SUPPORTED_WEBHOOK_FLOWS)
    }
}
