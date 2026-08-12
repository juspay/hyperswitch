use std::collections::BTreeMap;

use common_utils::events::{ApiEventMetric, ApiEventsType};

/// Context entry returned by Superposition list/create endpoints.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextResponse {
    /// Unique identifier of the context.
    pub id: String,
    /// Dimension values that define the context match condition.
    pub value: serde_json::Value,
    /// Override values applied when this context matches.
    pub r#override: serde_json::Value,
    /// Identifier of the override record.
    pub override_id: String,
    /// Priority weight used during context resolution.
    pub weight: String,
    /// Human-readable description of the context.
    pub description: String,
    /// Reason recorded for the most recent change.
    pub change_reason: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// User who created the context.
    pub created_by: String,
    /// Last modification timestamp in RFC3339 format.
    pub last_modified_at: String,
    /// User who last modified the context.
    pub last_modified_by: String,
}

impl ApiEventMetric for ContextResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

/// Default configuration entry returned by Superposition.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DefaultConfigResponse {
    /// Configuration key.
    pub key: String,
    /// Default value for the configuration key.
    pub value: serde_json::Value,
    /// JSON schema describing the value.
    pub schema: serde_json::Value,
    /// Human-readable description.
    pub description: String,
    /// Reason recorded for the most recent change.
    pub change_reason: String,
    /// Optional name of the validation function.
    pub value_validation_function_name: Option<String>,
    /// Optional name of the compute function.
    pub value_compute_function_name: Option<String>,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// User who created the config.
    pub created_by: String,
    /// Last modification timestamp in RFC3339 format.
    pub last_modified_at: String,
    /// User who last modified the config.
    pub last_modified_by: String,
}

impl ApiEventMetric for DefaultConfigResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

/// Dimension definition returned by Superposition.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DimensionResponse {
    /// Dimension name.
    pub dimension: String,
    /// Evaluation order for this dimension.
    pub position: i32,
    /// JSON schema describing the dimension value.
    pub schema: serde_json::Value,
    /// Optional name of the validation function.
    pub value_validation_function_name: Option<String>,
    /// Human-readable description.
    pub description: String,
    /// Reason recorded for the most recent change.
    pub change_reason: String,
    /// Last modification timestamp in RFC3339 format.
    pub last_modified_at: String,
    /// User who last modified the dimension.
    pub last_modified_by: String,
    /// Creation timestamp in RFC3339 format.
    pub created_at: String,
    /// User who created the dimension.
    pub created_by: String,
    /// Mapping of dependent dimensions.
    pub dependency_graph: serde_json::Value,
    /// Dimension type (regular, local cohort, remote cohort).
    pub dimension_type: serde_json::Value,
    /// Optional name of the compute function.
    pub value_compute_function_name: Option<String>,
    /// Whether the dimension is mandatory.
    pub mandatory: bool,
}

impl ApiEventMetric for DimensionResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

/// Audit log entry returned by Superposition.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditLogResponse {
    /// Unique identifier of the audit log entry.
    pub id: String,
    /// Name of the table the action was performed on.
    pub table_name: String,
    /// User who performed the action.
    pub user_name: String,
    /// Timestamp of the action in RFC3339 format.
    pub timestamp: String,
    /// Action performed (e.g. CREATE, UPDATE, DELETE).
    pub action: String,
    /// Snapshot of the record prior to the change.
    pub original_data: Option<serde_json::Value>,
    /// Snapshot of the record after the change.
    pub new_data: Option<serde_json::Value>,
    /// Query string associated with the action.
    pub query: String,
}

impl ApiEventMetric for AuditLogResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PaginatedListResponse<T> {
    pub total_pages: i32,
    pub total_items: i32,
    pub data: Vec<T>,
}

impl<T> ApiEventMetric for PaginatedListResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

/// A single resolved configuration entry returned by Superposition's detailed-resolve
/// endpoint: the resolved value alongside its default-config metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedConfigEntry {
    /// Human-readable description of the configuration key.
    pub description: String,
    /// JSON schema describing the value.
    pub schema: serde_json::Value,
    /// The resolved value for this key.
    pub value: serde_json::Value,
}

/// Detailed resolved configuration returned by the Superposition resolve endpoint.
///
/// Maps each configuration key to its resolved value and metadata, mirroring the body of
/// Superposition's `/config/resolve/detailed`. The upstream `version` / `last_modified` /
/// `audit_id` are returned as HTTP headers, not in the body.
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct ResolveConfigResponse(pub BTreeMap<String, ResolvedConfigEntry>);

impl ApiEventMetric for ResolveConfigResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

/// A single matching context in a resolved-config explanation timeline.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolveExplanationEntry {
    /// Identifier of the context that matched.
    pub context_id: String,
    /// Dimension conditions that made this context match.
    pub condition: serde_json::Value,
    /// Identifier of the override applied by this context.
    pub override_id: String,
    /// Value before this context's override was applied.
    pub value_before: serde_json::Value,
    /// Value after this context's override was applied.
    pub value_after: serde_json::Value,
}

/// Explanation of how matching contexts affect a single resolved config key.
#[derive(Debug, serde::Serialize)]
pub struct ResolveConfigExplanationResponse {
    /// The configuration key being explained.
    pub key: String,
    /// Ordered list of contexts that contributed to the resolved value.
    pub timeline: Vec<ResolveExplanationEntry>,
}

impl ApiEventMetric for ResolveConfigExplanationResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}
