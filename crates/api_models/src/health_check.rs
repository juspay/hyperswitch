#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterHealthCheckResponse {
    pub database: bool,
    pub redis: bool,
    pub locker: bool,
    #[cfg(feature = "olap")]
    pub analytics: bool,
    pub outgoing_request: bool,
}

impl common_utils::events::ApiEventMetric for RouterHealthCheckResponse {}

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

impl common_utils::events::ApiEventMetric for SchedulerHealthCheckResponse {}
