use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::blocklist_lookup;

#[derive(Default, Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = blocklist_lookup)]
pub struct BlocklistLookupNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint: String,
}

#[derive(
    Default,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Identifiable,
    Queryable,
    Selectable,
    Deserialize,
    Serialize,
)]
#[diesel(table_name = blocklist_lookup, primary_key(merchant_id, fingerprint), check_for_backend(diesel::pg::Pg))]
pub struct BlocklistLookup {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint: String,
}
