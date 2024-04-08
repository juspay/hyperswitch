use std::collections::HashMap;

use diesel_models::enums;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RetryMapping {
    pub start_after: i32,
    pub frequencies: Vec<(i32, i32)>, // (frequency, count)
}

#[derive(Serialize, Deserialize)]
pub struct ConnectorPTMapping {
    pub default_mapping: RetryMapping,
    pub custom_merchant_mapping: HashMap<String, RetryMapping>,
    pub max_retries_count: i32,
}

impl Default for ConnectorPTMapping {
    fn default() -> Self {
        Self {
            custom_merchant_mapping: HashMap::new(),
            default_mapping: RetryMapping {
                start_after: 60,
                frequencies: vec![(300, 5)],
            },
            max_retries_count: 5,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PaymentMethodsPTMapping {
    pub default_mapping: RetryMapping,
    pub custom_pm_mapping: HashMap<enums::PaymentMethod, RetryMapping>,
    pub max_retries_count: i32,
}

impl Default for PaymentMethodsPTMapping {
    fn default() -> Self {
        Self {
            custom_pm_mapping: HashMap::new(),
            default_mapping: RetryMapping {
                start_after: 900,
                frequencies: vec![(300, 5)],
            },
            max_retries_count: 5,
        }
    }
}

/// Configuration for outgoing webhook retries.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutgoingWebhookRetryProcessTrackerMapping {
    /// Default (fallback) retry configuration used when no merchant-specific retry configuration
    /// exists.
    pub default_mapping: RetryMapping,

    /// Merchant-specific retry configuration.
    pub custom_merchant_mapping: HashMap<String, RetryMapping>,
}

impl Default for OutgoingWebhookRetryProcessTrackerMapping {
    fn default() -> Self {
        Self {
            default_mapping: RetryMapping {
                // 1st attempt happens after 1 minute
                start_after: 60,

                frequencies: vec![
                    // 2nd and 3rd attempts happen at intervals of 5 minutes each
                    (60 * 5, 2),
                    // 4th, 5th, 6th, 7th and 8th attempts happen at intervals of 10 minutes each
                    (60 * 10, 5),
                    // 9th, 10th, 11th, 12th and 13th attempts happen at intervals of 1 hour each
                    (60 * 60, 5),
                    // 14th, 15th and 16th attempts happen at intervals of 6 hours each
                    (60 * 60 * 6, 3),
                ],
            },
            custom_merchant_mapping: HashMap::new(),
        }
    }
}
