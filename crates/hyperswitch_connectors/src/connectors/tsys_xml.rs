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
pub struct TsysXml {}

impl TsysXml {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for TsysXml {}
impl api::PaymentSession for TsysXml {}
impl api::ConnectorAccessToken for TsysXml {}
impl api::MandateSetup for TsysXml {}
impl api::PaymentAuthorize for TsysXml {}
impl api::PaymentSync for TsysXml {}
impl api::PaymentCapture for TsysXml {}
impl api::PaymentVoid for TsysXml {}
impl api::Refund for TsysXml {}
impl api::RefundExecute for TsysXml {}
impl api::RefundSync for TsysXml {}
impl api::PaymentToken for TsysXml {}

impl api::ConnectorCommon for TsysXml {
    fn id(&self) -> &'static str {
        "tsys_xml"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "text/xml"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.tsys_xml.base_url.as_ref()
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
        Err(errors::ConnectorError::NotImplemented("TsysXml".to_string()).into())
    }
}

impl api::ConnectorValidation for TsysXml {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for TsysXml {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for TsysXml {}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for TsysXml
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for TsysXml {}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for TsysXml {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for TsysXml {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for TsysXml {}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for TsysXml {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for TsysXml {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for TsysXml
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for TsysXml {
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

static TSYS_XML_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
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

    let mut tsys_xml_supported_payment_methods = SupportedPaymentMethods::new();

    tsys_xml_supported_payment_methods.add(
        common_enums::PaymentMethod::Card,
        common_enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    },
                ),
            ),
        },
    );

    tsys_xml_supported_payment_methods.add(
        common_enums::PaymentMethod::Card,
        common_enums::PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: common_enums::FeatureStatus::Supported,
            refunds: common_enums::FeatureStatus::Supported,
            supported_capture_methods: default_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::Supported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    },
                ),
            ),
        },
    );

    tsys_xml_supported_payment_methods
});

static TSYS_XML_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "TsysXml",
    description: "TSYS XML (TransIT) is a TSYS gateway integration using the TransIT XML API for card processing.",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Beta,
};

static TSYS_XML_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl api::ConnectorSpecifications for TsysXml {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&TSYS_XML_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*TSYS_XML_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&TSYS_XML_SUPPORTED_WEBHOOK_FLOWS)
    }
}
