use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::blocklist;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = blocklist)]
pub struct BlocklistNew {
    pub merchant_id: String,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = blocklist)]
pub struct Blocklist {
    #[serde(skip)]
    pub id: i32,
    pub merchant_id: String,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
}
