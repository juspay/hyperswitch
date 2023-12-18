use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::pm_fingerprint;

#[derive(Default, Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = pm_fingerprint)]
pub struct PmFingerprintNew {
    pub fingerprint_id: String,
    pub kms_hash: String,
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Queryable, Identifiable, Deserialize, Serialize)]
#[diesel(table_name = pm_fingerprint)]
pub struct PmFingerprint {
    #[serde(skip_serializing)]
    pub id: i32,
    pub fingerprint_id: String,
    pub kms_hash: String,
}
