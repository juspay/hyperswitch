use std::collections::HashMap;

use common_utils::events::ApiEventMetric;
use serde_json::{Map, Value};
use superposition_types::Config;

use crate::enums as api_enums;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuperPositionConfigResponse {
    pub raw_configs: Option<Config>,
    pub resolved_configs: Option<Map<String, Value>>,
    pub context_used: Map<String, Value>,
    pub dynamic_fields: Option<DynamicFields>,
}

impl ApiEventMetric for SuperPositionConfigResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DynamicFields {
    pub payment_methods: Vec<PaymentMethodGroup>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodGroup {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_types: Vec<PaymentMethodTypeWithFields>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodTypeWithFields {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub required_fields: HashMap<String, crate::payment_methods::RequiredFieldInfo>,
}
