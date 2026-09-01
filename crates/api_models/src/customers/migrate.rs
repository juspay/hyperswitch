use common_utils::{events::ApiEventMetric, id_type};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomerGlobalIdMigrationStatus {
    UpdatedNullId,
    UpdatedNonGlobalId,
    AlreadyGlobalId,
    SkippedNonV1,
    NotFound,
    InvalidCsvRow,
    UpdateFailed,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerGlobalIdMigrationRowResult {
    pub row_number: usize,
    #[schema(value_type = Option<String>)]
    pub merchant_id: Option<id_type::MerchantId>,
    #[schema(value_type = Option<String>)]
    pub customer_id: Option<id_type::CustomerId>,
    pub status: CustomerGlobalIdMigrationStatus,
    pub old_id: Option<String>,
    pub new_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerGlobalIdMigrationResponse {
    pub total_rows: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub results: Vec<CustomerGlobalIdMigrationRowResult>,
}

impl ApiEventMetric for CustomerGlobalIdMigrationResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
