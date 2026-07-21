use std::sync::LazyLock;

use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, PostCaptureVoidSync, PreAuthorizeVoid,
            Session, SetupMandate, UpdatePostConfirm, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCancelPostCaptureSyncData, PaymentsCaptureData,
        PaymentsPreAuthorizeCancelData, PaymentsSessionData, PaymentsSyncData,
        PaymentsUpdatePostConfirmData, RefundsData, SetupMandateRequestData,
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
use hyperswitch_masking::Secret;

#[derive(Clone)]
pub struct TsysTransit {}

impl TsysTransit {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

pub struct TsysTransitAuthType {
    pub device_id: Secret<String>,
    pub transaction_key: Secret<String>,
    pub developer_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TsysTransitAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                device_id: api_key.to_owned(),
                transaction_key: key1.to_owned(),
                developer_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

/// Optional merchant metadata for tsys_transit. The full structure is consumed
/// connector-side (UCS); Hyperswitch validates its shape at the edge so a
/// malformed `connector_metadata` is rejected up front. Absent metadata is
/// allowed — tsys_transit does not require it.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TsysTransitMetadataObject {
    #[serde(default)]
    pub payment_channel: Option<String>,
    #[serde(default)]
    pub commercial_card: Option<serde_json::Value>,
    #[serde(default)]
    pub tsys_transit: Option<serde_json::Value>,
    pub merchant_street_address: Option<Secret<String>>,
    pub customer_service_number: Option<Secret<String>>,
    pub merchant_url: Option<url::Url>,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for TsysTransitMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
        use error_stack::ResultExt;
        use hyperswitch_masking::PeekInterface;
        match meta_data {
            Some(value) => serde_json::from_value::<Self>(value.peek().clone()).change_context(
                errors::ConnectorError::InvalidConnectorConfig { config: "metadata" },
            ),
            None => Ok(Self::default()),
        }
    }
}

impl api::Payment for TsysTransit {}
impl api::PaymentSession for TsysTransit {}
impl api::ConnectorAccessToken for TsysTransit {}
impl api::MandateSetup for TsysTransit {}
impl api::PaymentAuthorize for TsysTransit {}
impl api::PaymentSync for TsysTransit {}
impl api::PaymentCapture for TsysTransit {}
impl api::PaymentVoid for TsysTransit {}
impl api::PaymentUpdate for TsysTransit {}
impl api::PaymentPreAuthorizeVoid for TsysTransit {}
impl api::PaymentPostCaptureVoidSync for TsysTransit {}
impl api::Refund for TsysTransit {}
impl api::RefundExecute for TsysTransit {}
impl api::RefundSync for TsysTransit {}
impl api::PaymentToken for TsysTransit {}

impl api::ConnectorCommon for TsysTransit {
    fn id(&self) -> &'static str {
        "tsys_transit"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "text/xml"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.tsys_transit.base_url.as_ref()
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
        Err(errors::ConnectorError::NotImplemented("TsysTransit".to_string()).into())
    }
}

impl api::ConnectorValidation for TsysTransit {}
impl api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for TsysTransit {}
impl api::ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for TsysTransit
{
}
impl api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for TsysTransit
{
}
impl api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for TsysTransit
{
}
impl api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for TsysTransit {}
impl api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for TsysTransit {}
impl api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for TsysTransit {}
impl
    api::ConnectorIntegration<
        UpdatePostConfirm,
        PaymentsUpdatePostConfirmData,
        PaymentsResponseData,
    > for TsysTransit
{
}
impl
    api::ConnectorIntegration<
        PreAuthorizeVoid,
        PaymentsPreAuthorizeCancelData,
        PaymentsResponseData,
    > for TsysTransit
{
}
impl
    api::ConnectorIntegration<
        PostCaptureVoidSync,
        PaymentsCancelPostCaptureSyncData,
        PaymentsResponseData,
    > for TsysTransit
{
}
impl api::ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for TsysTransit {}
impl api::ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for TsysTransit {}
impl
    api::ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for TsysTransit
{
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for TsysTransit {
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

static TSYS_TRANSIT_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
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

        let mut tsys_transit_supported_payment_methods = SupportedPaymentMethods::new();

        tsys_transit_supported_payment_methods.add(
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

        tsys_transit_supported_payment_methods.add(
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

        tsys_transit_supported_payment_methods
    });

static TSYS_TRANSIT_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "TsysTransit",
    description: "TSYS XML (TransIT) is a TSYS gateway integration using the TransIT XML API for card processing.",
    connector_type: common_enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: common_enums::ConnectorIntegrationStatus::Beta,
};

static TSYS_TRANSIT_SUPPORTED_WEBHOOK_FLOWS: [common_enums::EventClass; 0] = [];

impl api::ConnectorSpecifications for TsysTransit {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&TSYS_TRANSIT_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*TSYS_TRANSIT_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        Some(&TSYS_TRANSIT_SUPPORTED_WEBHOOK_FLOWS)
    }
}
