use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    #[schema(value_type = Option<Vec<Connector>>)]
    pub connectors: Option<Vec<common_enums::connector_enums::Connector>>,
}

#[derive(Debug, Clone, ToSchema, Serialize)]
pub struct CardSpecificFeatures {
    /// Indicates whether three_ds card payments are supported
    #[schema(value_type = FeatureStatus)]
    pub three_ds: common_enums::FeatureStatus,
    /// Indicates whether non three_ds card payments are supported
    #[schema(value_type = FeatureStatus)]
    pub no_three_ds: common_enums::FeatureStatus,
    /// List of supported card networks
    #[schema(value_type = Vec<CardNetwork>)]
    pub supported_card_networks: Vec<common_enums::CardNetwork>,
}

#[derive(Debug, Clone, ToSchema, Serialize)]
#[serde(untagged)]
pub enum PaymentMethodSpecificFeatures {
    /// Card specific features
    Card(CardSpecificFeatures),
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethod {
    /// The payment method supported by the connector
    #[schema(value_type = PaymentMethod)]
    pub payment_method: common_enums::PaymentMethod,
    /// The payment method type supported by the connector
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_type: common_enums::PaymentMethodType,
    /// The display name of the payment method type
    pub payment_method_type_display_name: String,
    /// Indicates whether the payment method supports mandates via the connector
    #[schema(value_type = FeatureStatus)]
    pub mandates: common_enums::FeatureStatus,
    /// Indicates whether the payment method supports refunds via the connector
    #[schema(value_type = FeatureStatus)]
    pub refunds: common_enums::FeatureStatus,
    /// List of supported capture methods supported by the payment method type
    #[schema(value_type = Vec<CaptureMethod>)]
    pub supported_capture_methods: Vec<common_enums::CaptureMethod>,
    /// Information on the Payment method specific payment features
    #[serde(flatten)]
    pub payment_method_specific_features: Option<PaymentMethodSpecificFeatures>,
    /// List of countries supported by the payment method type via the connector
    #[schema(value_type = Option<HashSet<CountryAlpha3>>)]
    pub supported_countries: Option<HashSet<common_enums::CountryAlpha3>>,
    /// List of currencies supported by the payment method type via the connector
    #[schema(value_type = Option<HashSet<Currency>>)]
    pub supported_currencies: Option<HashSet<common_enums::Currency>>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct ConnectorFeatureMatrixResponse {
    /// The name of the connector
    pub name: String,
    /// The display name of the connector
    pub display_name: String,
    /// The description of the connector
    pub description: String,
    /// The base url of the connector
    pub base_url: Option<String>,
    /// The category of the connector
    #[schema(value_type = HyperswitchConnectorCategory, example = "payment_gateway")]
    pub category: common_enums::HyperswitchConnectorCategory,
    /// The integration status of the connector
    #[schema(value_type = ConnectorIntegrationStatus, example = "live")]
    pub integration_status: common_enums::ConnectorIntegrationStatus,
    /// The list of payment methods supported by the connector
    pub supported_payment_methods: Option<Vec<SupportedPaymentMethod>>,
    /// The list of webhook flows supported by the connector
    #[schema(value_type = Option<Vec<EventClass>>)]
    pub supported_webhook_flows: Option<Vec<common_enums::EventClass>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureMatrixListResponse {
    /// The number of connectors included in the response
    pub connector_count: usize,
    // The list of payments response objects
    pub connectors: Vec<ConnectorFeatureMatrixResponse>,
}

impl common_utils::events::ApiEventMetric for FeatureMatrixListResponse {}
impl common_utils::events::ApiEventMetric for FeatureMatrixRequest {}
