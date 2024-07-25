use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::blocklist;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = blocklist)]
pub struct BlocklistNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = blocklist, primary_key(merchant_id, fingerprint_id), check_for_backend(diesel::pg::Pg))]
pub struct Blocklist {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
}
