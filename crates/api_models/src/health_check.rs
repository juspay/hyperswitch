#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterHealthCheckResponse {
    pub database: String,
    pub redis: String,
    pub locker: LockerHealthResponse,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockerHealthResponse {
    pub status: String,
    pub key_custodian_status: KeyCustodianStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum KeyCustodianStatus {
    Unavailable,
    Locked,
    Unlocked,
}
