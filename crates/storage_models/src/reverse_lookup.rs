use diesel::{Identifiable, Insertable, Queryable};

use crate::schema::reverse_lookup;

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Identifiable, Queryable, Eq, PartialEq,
)]
#[diesel(table_name = reverse_lookup)]
#[diesel(primary_key(sk_id))]
pub struct ReverseLookup {
    pub sk_id: i32,
    pub pk_id: String,
    pub lookup_id: String,
    pub result_id: String,
    pub source: String,
}

#[derive(
    Clone, Debug, Insertable, router_derive::DebugAsDisplay, Eq, PartialEq, serde::Serialize,
)]
#[diesel(table_name = reverse_lookup)]
pub struct ReverseLookupNew {
    pub pk_id: String,
    pub lookup_id: String,
    pub result_id: String,
    pub source: String,
}

impl ReverseLookupNew {
    pub fn new(pk_id: String, lookup_id: String, result_id: String, source: String) -> Self {
        Self {
            pk_id,
            lookup_id,
            result_id,
            source,
        }
    }
}
