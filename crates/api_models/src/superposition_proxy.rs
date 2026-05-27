use std::collections::HashMap;

use common_utils::events::{ApiEventMetric, ApiEventsType};
use hyperswitch_masking::Secret;
use serde_json::Map;
use superposition_types::api::context::PutRequest as ContextPutRequest;

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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListContextsRequest {
    pub org_id: Secret<String>,
    pub workspace_id: Secret<String>,
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
    pub prefix: Option<Vec<String>>,
    pub sort_on: Option<String>,
    pub sort_by: Option<String>,
    pub created_by: Option<Vec<String>>,
    pub last_modified_by: Option<Vec<String>>,
    pub plaintext: Option<String>,
    pub dimension_params: HashMap<String, String>,
    pub dimension_match_strategy: Option<String>,
}

impl ApiEventMetric for ListContextsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListDefaultConfigsRequest {
    pub org_id: Secret<String>,
    pub workspace_id: Secret<String>,
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
    pub name: Option<String>,
}

impl ApiEventMetric for ListDefaultConfigsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListDimensionsRequest {
    pub org_id: Secret<String>,
    pub workspace_id: Secret<String>,
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
}

impl ApiEventMetric for ListDimensionsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListAuditLogsRequest {
    pub org_id: Secret<String>,
    pub workspace_id: Secret<String>,
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub table: Option<Vec<String>>,
    pub action: Option<Vec<String>>,
    pub username: Option<String>,
    pub sort_by: Option<String>,
    pub dimension_params: HashMap<String, String>,
}

impl ApiEventMetric for ListAuditLogsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProxyCreateContextRequest {
    pub body: ContextPutRequest,
    pub org_id: Secret<String>,
    pub workspace_id: Secret<String>,
}

impl std::fmt::Debug for ProxyCreateContextRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyCreateContextRequest")
            .field("org_id", &self.org_id)
            .field("workspace_id", &self.workspace_id)
            .finish()
    }
}

impl ApiEventMetric for ProxyCreateContextRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ResolveConfigBody {
    pub context: Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
pub struct ProxyResolveConfigRequest {
    pub body: ResolveConfigBody,
    pub org_id: Secret<String>,
    pub workspace_id: Secret<String>,
}

impl ApiEventMetric for ProxyResolveConfigRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}
