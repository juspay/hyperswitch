use diesel::{AsChangeset, Identifiable, Queryable, Selectable, Insertable};
use time::PrimitiveDateTime;
use common_utils::events::ApiEventMetric;

use crate::{enums as storage_enums, schema::cards_info};

#[derive(
    Clone, Debug, Queryable, Identifiable, Selectable, serde::Deserialize, serde::Serialize,Insertable
)]
#[diesel(table_name = cards_info, primary_key(card_iin), check_for_backend(diesel::pg::Pg))]
pub struct CardInfo {
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

impl ApiEventMetric for CardInfo {}

#[derive(Clone, Debug, PartialEq, Eq, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize,)]
#[diesel(table_name = cards_info)]
pub struct UpdateCardInfo {
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
impl ApiEventMetric for UpdateCardInfo {}
