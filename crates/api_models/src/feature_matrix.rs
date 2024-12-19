use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::enums;

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    pub connectors: Option<Vec<enums::Connector>>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethod {
    pub payment_method: enums::PaymentMethod,
    pub payment_method_type: enums::PaymentMethodType,
    pub mandates: enums::FeatureStatus,
    pub refunds: enums::FeatureStatus,
    pub supported_capture_methods: Vec<enums::CaptureMethod>,
    pub supported_countries: Option<HashSet<enums::CountryAlpha2>>,
    pub supported_currencies: Option<HashSet<enums::Currency>>,
}


#[derive(Debug, ToSchema, Serialize)]
pub struct ConnectorFeatureMatrixResponse {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<enums::PaymentConnectorCategory>,
    pub supported_payment_methods: Vec<SupportedPaymentMethod>,
    pub supported_webhook_flows: Option<Vec<enums::EventClass>>,
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
