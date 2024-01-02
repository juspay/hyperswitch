#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterHealthCheckResponse {
    pub database: String,
    pub redis: String,
    pub locker: String,
}
