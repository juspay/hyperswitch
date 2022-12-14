use diesel::{Identifiable, Insertable, Queryable};

use crate::schema::reverse_lookup;

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Identifiable, Queryable, Eq, PartialEq,
)]
#[diesel(table_name = reverse_lookup)]
#[diesel(primary_key(lookup_id))]
pub struct ReverseLookup {
    pub lookup_id: String,
    pub sk_id: String,
    pub pk_id: String,
    pub result_id: String,
    pub source: String,
}

#[derive(
    Clone, Debug, Insertable, router_derive::DebugAsDisplay, Eq, PartialEq, serde::Serialize,
)]
#[diesel(table_name = reverse_lookup)]
pub struct ReverseLookupNew {
    pub lookup_id: String,
    pub result_id: String,
    pub source: String,
    pub pk_id: String,
    pub sk_id: String,
}
