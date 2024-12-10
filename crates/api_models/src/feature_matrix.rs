use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums::{
    CaptureMethod, Connector, CountryAlpha2, Currency, EventClass, PaymentMethod,
    PaymentMethodType, PaymentsConnectorType,
};

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    pub connectors: Option<Vec<Connector>>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethod {
    pub payment_method: PaymentMethodType,
    pub supports_mandate: bool,
    pub supports_refund: bool,
    pub supported_capture_methods: Vec<CaptureMethod>,
    pub supported_countries: Option<HashSet<CountryAlpha2>>,
    pub supported_currencies: Option<HashSet<Currency>>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethodTypes {
    pub payment_method_type: PaymentMethod,
    pub payment_methods: Vec<SupportedPaymentMethod>,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct ConnectorFeatureMatrixResponse {
    pub connector: String,
    pub description: Option<String>,
    pub connector_type: Option<PaymentsConnectorType>,
    pub payment_method_types: Vec<SupportedPaymentMethodTypes>,
    pub supported_webhook_flows: Option<Vec<EventClass>>,
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
