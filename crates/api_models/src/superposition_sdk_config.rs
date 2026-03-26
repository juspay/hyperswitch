use common_utils::events::ApiEventMetric;
use serde_json::{Map, Value};
use superposition_types::Config;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuperPositionConfigResponse {
    pub raw_configs: Option<Config>,
    pub resolved_configs: Option<Map<String, Value>>,
    pub context_used: Map<String, Value>,
}

impl ApiEventMetric for SuperPositionConfigResponse {}
