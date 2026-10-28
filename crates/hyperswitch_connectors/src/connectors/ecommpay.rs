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
pub struct Ecommpay {}

impl Ecommpay {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Ecommpay {}
impl api::PaymentSession for Ecommpay {}
impl api::ConnectorAccessToken for Ecommpay {}
impl api::MandateSetup for Ecommpay {}
impl api::PaymentAuthorize for Ecommpay {}
impl api::PaymentSync for Ecommpay {}
impl api::PaymentCapture for Ecommpay {}
impl api::PaymentVoid for Ecommpay {}
impl api::Refund for Ecommpay {}
impl api::RefundExecute for Ecommpay {}
impl api::RefundSync for Ecommpay {}
impl api::PaymentToken for Ecommpay {}

impl api::ConnectorCommon for Ecommpay {
    fn id(&self) -> &'static str {
        "ecommpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.ecommpay.base_url.as_ref()
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
        Err(errors::ConnectorError::NotImplemented("Ecommpay".to_string()).into())
    }
}

impl api::ConnectorValidation for Ecommpay {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Ecommpay {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Ecommpay {}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Ecommpay
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Ecommpay
{
}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Ecommpay {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Ecommpay {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Ecommpay {}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Ecommpay {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Ecommpay {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Ecommpay
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Ecommpay {
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

static ECOMMPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let default_capture_methods = vec![
            common_enums::CaptureMethod::Automatic,
            common_enums::CaptureMethod::Manual,
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

        let mut ecommpay_supported_payment_methods = SupportedPaymentMethods::new();

        ecommpay_supported_payment_methods.add(
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

        ecommpay_supported_payment_methods.add(
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

        ecommpay_supported_payment_methods.add(
            common_enums::PaymentMethod::Wallet,
            common_enums::PaymentMethodType::ApplePay,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: default_capture_methods.clone(),
                specific_features: None,
            },
        );

        ecommpay_supported_payment_methods.add(
            common_enums::PaymentMethod::Wallet,
            common_enums::PaymentMethodType::GooglePay,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: default_capture_methods.clone(),
                specific_features: None,
            },
        );

        ecommpay_supported_payment_methods
    });

static ECOMMPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Ecommpay",
    description: "Ecommpay is a European payment service provider offering card, wallet and alternative payment processing through its Gate API.",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Beta,
};

static ECOMMPAY_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl api::ConnectorSpecifications for Ecommpay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ECOMMPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ECOMMPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&ECOMMPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
