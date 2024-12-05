use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums;

#[derive(Default, Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FeatureMatrixRequest {
    // List of connectors for which the feature matrix is requested
    pub connectors: Option<Vec<enums::Connector>>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Serialize)]
pub struct SupportedPaymentMethod {
    pub payment_method: enums::PaymentMethodType,
    pub supports_mandate: bool,
    pub supports_refund: bool,
    pub supported_capture_methods: Vec<enums::CaptureMethod>,
    pub supported_countries: Option<HashSet<enums::CountryAlpha2>>,
    pub supported_currencies: Option<HashSet<enums::Currency>>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct SupportedPaymentMethodTypes {
    pub payment_method_type: enums::PaymentMethod,
    pub payment_methods: Vec<SupportedPaymentMethod>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, ToSchema, Serialize)]
pub struct FeatureMatrixResponse {
    pub connector: enums::Connector,
    pub payment_method_types: Vec<SupportedPaymentMethodTypes>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct FeatureMatrixListResponse {
    /// The number of connectors included in the list
    pub size: usize,
    // The list of payments response objects
    pub data: Vec<FeatureMatrixResponse>,
}

impl common_utils::events::ApiEventMetric for FeatureMatrixListResponse {}
impl common_utils::events::ApiEventMetric for FeatureMatrixRequest {}
