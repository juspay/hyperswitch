//! PostgreSQL rows for alert dictionary state.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_dictionary;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_dictionary)]
pub struct DictionaryEntryNew {
    pub name: String,
    pub key_: String,
    pub product: String,
    pub values_: String,
    pub metadata: String,
    pub updated_by: String,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = alert_dictionary,
    primary_key(name, key_),
    check_for_backend(diesel::pg::Pg)
)]
pub struct DictionaryEntry {
    pub name: String,
    pub key_: String,
    pub product: String,
    pub values_: String,
    pub metadata: String,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
}
