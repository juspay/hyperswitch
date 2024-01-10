use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::pm_blocklist;

#[derive(Default, Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = pm_blocklist)]
pub struct PmBlocklistNew {
    pub merchant_id: String,
    pub fingerprint: String,
    pub fingerprint_type: String,
    pub metadata: Option<String>,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = pm_blocklist)]
pub struct PmBlocklist {
    #[serde(skip)]
    pub id: i32,
    pub merchant_id: String,
    pub fingerprint: String,
    pub fingerprint_type: String,
    pub metadata: Option<String>,
}
