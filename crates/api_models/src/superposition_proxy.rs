use std::collections::HashMap;

use common_utils::events::{ApiEventMetric, ApiEventsType};
use serde_json::Map;
use superposition_types::api::context::PutRequest as ContextPutRequest;

#[derive(Debug, serde::Serialize)]
pub struct PaginatedListResponse {
    pub total_pages: i32,
    pub total_items: i32,
    pub data: Vec<serde_json::Value>,
}

impl ApiEventMetric for PaginatedListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListContextsRequest {
    pub org_id: String,
    pub workspace_id: String,
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
}

impl ApiEventMetric for ListContextsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListDefaultConfigsRequest {
    pub org_id: String,
    pub workspace_id: String,
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
    pub name: Option<Vec<String>>,
}

impl ApiEventMetric for ListDefaultConfigsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ListDimensionsRequest {
    pub org_id: String,
    pub workspace_id: String,
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
    pub org_id: String,
    pub workspace_id: String,
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
    pub org_id: String,
    pub workspace_id: String,
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
    pub org_id: String,
    pub workspace_id: String,
}

impl ApiEventMetric for ProxyResolveConfigRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}
