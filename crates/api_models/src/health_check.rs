#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterHealthCheckResponse {
    pub database: bool,
    pub redis: bool,
    pub locker: bool,
    #[cfg(feature = "olap")]
    pub analytics: bool,
}

impl common_utils::events::ApiEventMetric for RouterHealthCheckResponse {}
