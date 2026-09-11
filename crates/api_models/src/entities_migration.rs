#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct EntitiesMigrationRequest {
    pub merchant_ids: Vec<common_utils::id_type::MerchantId>,
}

impl common_utils::events::ApiEventMetric for EntitiesMigrationRequest {}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityMigrationStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntityMigrationResult {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub status: EntityMigrationStatus,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<time::PrimitiveDateTime>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntitiesMigrationResponse {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<EntityMigrationResult>,
}
