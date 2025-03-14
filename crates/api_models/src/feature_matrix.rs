use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums::{
    CaptureMethod, CardNetwork, Connector, CountryAlpha2, Currency, EventClass, FeatureStatus,
    PaymentConnectorCategory, PaymentMethod, PaymentMethodType,
};

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    pub connectors: Option<Vec<Connector>>,
}

#[derive(Debug, Clone, ToSchema, Serialize)]
pub struct CardSpecificFeatures {
    /// Indicates whether three_ds card payments are supported.
    pub three_ds: FeatureStatus,
    /// Indicates whether non three_ds card payments are supported.
    pub no_three_ds: FeatureStatus,
    /// List of supported card networks
    pub supported_card_networks: Vec<CardNetwork>,
}

#[derive(Debug, Clone, ToSchema, Serialize)]
#[serde(untagged)]
pub enum PaymentMethodSpecificFeatures {
    /// Card specific features
    Card(CardSpecificFeatures),
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethod {
    pub payment_method: PaymentMethod,
    pub payment_method_type: PaymentMethodType,
    pub mandates: FeatureStatus,
    pub refunds: FeatureStatus,
    pub supported_capture_methods: Vec<CaptureMethod>,
    #[serde(flatten)]
    pub payment_method_specific_features: Option<PaymentMethodSpecificFeatures>,
    pub supported_countries: Option<HashSet<CountryAlpha2>>,
    pub supported_currencies: Option<HashSet<Currency>>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct ConnectorFeatureMatrixResponse {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub category: Option<PaymentConnectorCategory>,
    pub supported_payment_methods: Vec<SupportedPaymentMethod>,
    pub supported_webhook_flows: Option<Vec<EventClass>>,
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
