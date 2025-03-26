use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    #[schema(value_type = Option<Connector>, example = "stripe")]
    pub connectors: Option<Vec<common_enums::connector_enums::Connector>>,
}

#[derive(Debug, Clone, ToSchema, Serialize)]
pub struct CardSpecificFeatures {
    /// Indicates whether three_ds card payments are supported.
    #[schema(value_type = Option<FeatureStatus>, example = "supported")]
    pub three_ds: common_enums::FeatureStatus,
    /// Indicates whether non three_ds card payments are supported.
    #[schema(value_type = Option<FeatureStatus>, example = "supported")]
    pub no_three_ds: common_enums::FeatureStatus,
    /// List of supported card networks
    #[schema(value_type = Option<CardNetwork>, example = "VISA")]
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
    #[schema(value_type = Option<PaymentMethod>, example = "card")]
    pub payment_method: common_enums::PaymentMethod,
    #[schema(value_type = Option<PaymentMethodType>, example = "apple_pay")]
    pub payment_method_type: common_enums::PaymentMethodType,
    pub payment_method_type_display_name: String,
    #[schema(value_type = Option<FeatureStatus>, example = "supported")]
    pub mandates: common_enums::FeatureStatus,
    #[schema(value_type = Option<FeatureStatus>, example = "supported")]
    pub refunds: common_enums::FeatureStatus,
    #[schema(value_type = Option<CaptureMethod>, example = "automatic")]
    pub supported_capture_methods: Vec<common_enums::CaptureMethod>,
    #[serde(flatten)]
    pub payment_method_specific_features: Option<PaymentMethodSpecificFeatures>,
    #[schema(value_type = Option<CountryAlpha3>, example = "USA")]
    pub supported_countries: Option<HashSet<common_enums::CountryAlpha3>>,
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub supported_currencies: Option<HashSet<common_enums::Currency>>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct ConnectorFeatureMatrixResponse {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[schema(value_type = Option<PaymentConnectorCategory>, example = "payment_gateway")]
    pub category: Option<common_enums::PaymentConnectorCategory>,
    pub supported_payment_methods: Vec<SupportedPaymentMethod>,
    #[schema(value_type = Option<Currency>, example = "payments")]
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