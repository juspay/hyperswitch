use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::blocklist_lookup;

#[derive(Default, Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = blocklist_lookup)]
pub struct BlocklistLookupNew {
    pub merchant_id: String,
    pub fingerprint: String,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = blocklist_lookup)]
pub struct BlocklistLookup {
    #[serde(skip)]
    pub id: i32,
    pub merchant_id: String,
    pub fingerprint: String,
}
