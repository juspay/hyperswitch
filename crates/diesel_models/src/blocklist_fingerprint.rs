use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::blocklist_fingerprint;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = blocklist_fingerprint)]
pub struct BlocklistFingerprintNew {
    pub merchant_id: String,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub encrypted_fingerprint: String,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, Queryable, Identifiable, Deserialize, Serialize)]
#[diesel(table_name = blocklist_fingerprint)]
pub struct BlocklistFingerprint {
    #[serde(skip_serializing)]
    pub id: i32,
    pub merchant_id: String,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub encrypted_fingerprint: String,
    pub created_at: time::PrimitiveDateTime,
}
