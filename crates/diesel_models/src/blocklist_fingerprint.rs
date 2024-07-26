use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::blocklist_fingerprint;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = blocklist_fingerprint)]
pub struct BlocklistFingerprintNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub encrypted_fingerprint: String,
    pub created_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Queryable, Identifiable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = blocklist_fingerprint, check_for_backend(diesel::pg::Pg))]
pub struct BlocklistFingerprint {
    #[serde(skip_serializing)]
    pub id: i32,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub encrypted_fingerprint: String,
    pub created_at: time::PrimitiveDateTime,
}
