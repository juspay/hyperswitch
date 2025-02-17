use std::fmt::Debug;

use common_utils::events::ApiEventMetric;
use utoipa::ToSchema;

use crate::enums as storage_enums;

#[derive(serde::Deserialize, ToSchema)]
pub struct CardsInfoRequestParams {
    #[schema(example = "pay_OSERgeV9qAy7tlK7aKpc_secret_TuDUoh11Msxh12sXn3Yp")]
    pub client_secret: Option<String>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CardsInfoRequest {
    pub client_secret: Option<String>,
    pub card_iin: String,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct CardInfoResponse {
    #[schema(example = "374431")]
    pub card_iin: String,
    #[schema(example = "AMEX")]
    pub card_issuer: Option<String>,
    #[schema(example = "AMEX")]
    pub card_network: Option<String>,
    #[schema(example = "CREDIT")]
    pub card_type: Option<String>,
    #[schema(example = "CLASSIC")]
    pub card_sub_type: Option<String>,
    #[schema(example = "INDIA")]
    pub card_issuing_country: Option<String>,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct CardInfoMigrateResponseRecord {
    pub card_iin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardInfoCreateRequest {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<storage_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
}

impl ApiEventMetric for CardInfoCreateRequest {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardInfoUpdateRequest {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<storage_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
    pub line_number: Option<i64>,
}

impl ApiEventMetric for CardInfoUpdateRequest {}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema, Clone)]
pub struct CardInfoRecord {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<storage_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
    pub line_number: Option<i64>,
}

impl ApiEventMetric for CardInfoRecord {}

#[derive(Debug, Default, serde::Serialize)]
pub enum CardInfoMigrationStatus {
    Success,
    #[default]
    Failed,
}
#[derive(Debug, Default, serde::Serialize)]
pub struct CardInfoMigrationResponse {
    pub line_number: Option<i64>,
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_error: Option<String>,
    pub migration_status: CardInfoMigrationStatus,
}
impl ApiEventMetric for CardInfoMigrationResponse {}

type CardInfoMigrationResponseType = (
    Result<CardInfoMigrateResponseRecord, String>,
    CardInfoUpdateRequest,
);

impl From<CardInfoMigrationResponseType> for CardInfoMigrationResponse {
    fn from((response, record): CardInfoMigrationResponseType) -> Self {
        match response {
            Ok(res) => Self {
                card_iin: record.card_iin,
                line_number: record.line_number,
                card_issuer: res.card_issuer,
                card_network: res.card_network,
                card_type: res.card_type,
                card_sub_type: res.card_sub_type,
                card_issuing_country: res.card_issuing_country,
                migration_status: CardInfoMigrationStatus::Success,
                migration_error: None,
            },
            Err(e) => Self {
                card_iin: record.card_iin,
                migration_status: CardInfoMigrationStatus::Failed,
                migration_error: Some(e),
                line_number: record.line_number,
                ..Self::default()
            },
        }
    }
}
