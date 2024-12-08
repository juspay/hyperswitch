use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{enums, webhooks::WebhookFlow};

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    pub connectors: Option<Vec<enums::Connector>>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize)]
pub struct SupportedPaymentMethod {
    pub payment_method: enums::PaymentMethodType,
    pub supports_mandate: bool,
    pub supports_refund: bool,
    pub supported_capture_methods: Vec<enums::CaptureMethod>,
    pub supported_countries: Option<HashSet<enums::CountryAlpha2>>,
    pub supported_currencies: Option<HashSet<enums::Currency>>,
}

#[cfg(feature = "v1")]
#[derive(Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethodTypes {
    pub payment_method_type: enums::PaymentMethod,
    pub payment_methods: Vec<SupportedPaymentMethod>,
}

#[cfg(feature = "v1")]
#[derive(Debug, ToSchema, Serialize)]
pub struct ConnectorFeatureMatrixResponse {
    pub connector: String,
    pub description: Option<String>,
    pub connector_type: Option<enums::PaymentsConnectorType>,
    pub payment_method_types: Vec<SupportedPaymentMethodTypes>,
    pub supported_webhook_flows: Option<Vec<WebhookFlow>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureMatrixListResponse {
    /// The number of connectors included in the response
    pub size: usize,
    // The list of payments response objects
    pub data: Vec<ConnectorFeatureMatrixResponse>,
}

impl common_utils::events::ApiEventMetric for FeatureMatrixListResponse {}
impl common_utils::events::ApiEventMetric for FeatureMatrixRequest {}
