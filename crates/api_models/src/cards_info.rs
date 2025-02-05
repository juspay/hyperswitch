use std::fmt::Debug;

use common_utils::events::ApiEventMetric;
use time::PrimitiveDateTime;
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
    pub date_created: PrimitiveDateTime,
    pub last_updated: Option<PrimitiveDateTime>,
    pub last_updated_provider: Option<String>,
}

impl ApiEventMetric for CardInfoCreateRequest {}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
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
    pub last_updated: Option<PrimitiveDateTime>,
    pub last_updated_provider: Option<String>,
}

impl ApiEventMetric for CardInfoUpdateRequest {}
