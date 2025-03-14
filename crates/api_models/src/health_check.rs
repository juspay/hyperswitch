use std::collections::hash_map::HashMap;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterHealthCheckResponse {
    pub database: bool,
    pub redis: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<bool>,
    #[cfg(feature = "olap")]
    pub analytics: bool,
    #[cfg(feature = "olap")]
    pub opensearch: bool,
    pub outgoing_request: bool,
    #[cfg(feature = "dynamic_routing")]
    pub grpc_health_check: HealthCheckMap,
}

impl common_utils::events::ApiEventMetric for RouterHealthCheckResponse {}

/// gRPC based services eligible for Health check
#[derive(Debug, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckServices {
    /// Dynamic routing service
    DynamicRoutingService,
}

pub type HealthCheckMap = HashMap<HealthCheckServices, bool>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchedulerHealthCheckResponse {
    pub database: bool,
    pub redis: bool,
    pub outgoing_request: bool,
}

pub enum HealthState {
    Running,
    Error,
    NotApplicable,
}

impl From<HealthState> for bool {
    fn from(value: HealthState) -> Self {
        match value {
            HealthState::Running => true,
            HealthState::Error | HealthState::NotApplicable => false,
        }
    }
}
impl From<HealthState> for Option<bool> {
    fn from(value: HealthState) -> Self {
        match value {
            HealthState::Running => Some(true),
            HealthState::Error => Some(false),
            HealthState::NotApplicable => None,
        }
    }
}

impl common_utils::events::ApiEventMetric for SchedulerHealthCheckResponse {}
