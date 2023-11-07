use diesel::{Identifiable, Queryable};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::cards_info};

#[derive(Clone, Debug, Queryable, Identifiable, serde::Deserialize, serde::Serialize)]
#[diesel(table_name = cards_info, primary_key(card_iin))]
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
