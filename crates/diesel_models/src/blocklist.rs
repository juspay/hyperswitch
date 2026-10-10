use diesel::{Insertable, Queryable, Selectable};
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
    pub processor_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub created_by: Option<String>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub transaction_type: common_enums::TransactionType,
}

#[derive(Clone, Debug, Eq, PartialEq, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = blocklist, check_for_backend(diesel::pg::Pg))]
pub struct Blocklist {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint_id: String,
    pub data_kind: common_enums::BlocklistDataKind,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub processor_merchant_id: Option<common_utils::id_type::MerchantId>,
    pub created_by: Option<String>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub transaction_type: common_enums::TransactionType,
}

// There is no physical primary key: profile and flow uniqueness is enforced by indexes.
// Keep the table association without claiming a merchant/fingerprint-only row identity.
impl diesel::associations::HasTable for Blocklist {
    type Table = blocklist::table;

    fn table() -> Self::Table {
        blocklist::table
    }
}
