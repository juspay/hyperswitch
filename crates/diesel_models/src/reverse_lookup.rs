use diesel::{Identifiable, Insertable, Queryable};

use crate::schema::reverse_lookup;

///
/// This reverse lookup table basically looks up id's and get result_id that you want. This is
/// useful for KV where you can't lookup without key
#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Identifiable, Queryable, Eq, PartialEq,
)]
#[diesel(table_name = reverse_lookup)]
#[diesel(primary_key(lookup_id))]
pub struct ReverseLookup {
    /// Primary key. The key id.
    pub lookup_id: String,
    /// the `field` in KV database. Which is used to differentiate between two same keys
    pub sk_id: String,
    /// the value id. i.e the id you want to access KV table.
    pub pk_id: String,
    /// the source of insertion for reference
    pub source: String,
    pub updated_by: String,
}

#[derive(
    Clone,
    Debug,
    Insertable,
    router_derive::DebugAsDisplay,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
#[diesel(table_name = reverse_lookup)]
pub struct ReverseLookupNew {
    pub lookup_id: String,
    pub pk_id: String,
    pub sk_id: String,
    pub source: String,
    pub updated_by: String,
}

impl From<ReverseLookupNew> for ReverseLookup {
    fn from(new: ReverseLookupNew) -> Self {
        Self {
            lookup_id: new.lookup_id,
            sk_id: new.sk_id,
            pk_id: new.pk_id,
            source: new.source,
            updated_by: new.updated_by,
        }
    }
}
