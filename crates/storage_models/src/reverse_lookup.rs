use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

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
    pub pk_id: String,
    pub lookup_id: String,
    pub result_id: String,
    pub source: String,
    pub sk_id: String,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = reverse_lookup)]
pub struct ReverseLookupUpdateInternal {
    pub pk_id: Option<String>,
    pub lookup_id: Option<String>,
    pub result_id: Option<String>,
    pub source: Option<String>,
    pub sk_id: Option<String>,
}

#[derive(Debug)]
pub enum ReverseLookupUpdate {
    UpdateLookupResult {
        lookup_id: String,
        result_id: String,
    },
}

impl From<ReverseLookupUpdate> for ReverseLookupUpdateInternal {
    fn from(item: ReverseLookupUpdate) -> Self {
        match item {
            ReverseLookupUpdate::UpdateLookupResult {
                lookup_id,
                result_id,
            } => Self {
                lookup_id: Some(lookup_id),
                result_id: Some(result_id),
                ..Default::default()
            },
        }
    }
}
