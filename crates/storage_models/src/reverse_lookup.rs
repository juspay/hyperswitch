use diesel::{Identifiable, Insertable, Queryable};

use crate::schema::reverse_lookup;

///
/// This reverse lookup table basically looks up id's and get result_id that you want. This is
/// useful for KV where you can't lookup without key
/// ## Field
/// * lookup_id: Primary key. The key id.
/// * pk_id: the value id. i.e the id you want to access KV table.
/// * sk_id: the `field` in KV database. Which is used to differentiate between two same keys
/// * source: the source of insertion for reference
///
#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Identifiable, Queryable, Eq, PartialEq,
)]
#[diesel(table_name = reverse_lookup)]
#[diesel(primary_key(lookup_id))]
pub struct ReverseLookup {
    pub lookup_id: String,
    pub pk_id: String,
    pub sk_id: String,
    pub source: String,
}

#[derive(
    Clone, Debug, Insertable, router_derive::DebugAsDisplay, Eq, PartialEq, serde::Serialize,
)]
#[diesel(table_name = reverse_lookup)]
pub struct ReverseLookupNew {
    pub lookup_id: String,
    pub pk_id: String,
    pub sk_id: String,
    pub source: String,
}
