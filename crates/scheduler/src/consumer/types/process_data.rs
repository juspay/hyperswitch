use std::collections::HashMap;

use diesel_models::enums;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct RetryMapping {
    pub start_after: i32,
    pub frequency: Vec<i32>,
    pub count: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorPTMapping {
    pub default_mapping: RetryMapping,
    pub custom_merchant_mapping: HashMap<String, RetryMapping>,
    pub max_retries_count: i32,
}

impl Default for ConnectorPTMapping {
        /// Creates a new instance of the struct with default values for its fields.
    fn default() -> Self {
        Self {
            custom_merchant_mapping: HashMap::new(),
            default_mapping: RetryMapping {
                start_after: 60,
                frequency: vec![300],
                count: vec![5],
            },
            max_retries_count: 5,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodsPTMapping {
    pub default_mapping: RetryMapping,
    pub custom_pm_mapping: HashMap<enums::PaymentMethod, RetryMapping>,
    pub max_retries_count: i32,
}

impl Default for PaymentMethodsPTMapping {
        /// Returns a new instance of Self with default values for custom_pm_mapping, default_mapping, and max_retries_count.
    fn default() -> Self {
        Self {
            custom_pm_mapping: HashMap::new(),
            default_mapping: RetryMapping {
                start_after: 900,
                frequency: vec![300],
                count: vec![5],
            },
            max_retries_count: 5,
        }
    }
}
