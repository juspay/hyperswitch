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
#[diesel(table_name = blocklist_lookup, check_for_backend(diesel::pg::Pg))]
pub struct BlocklistLookup {
    #[serde(skip)]
    pub id: i32,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub fingerprint: String,
}
