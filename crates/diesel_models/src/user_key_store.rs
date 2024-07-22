use common_utils::encryption::Encryption;
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::user_key_store;

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Identifiable, Queryable, Selectable,
)]
#[diesel(table_name = user_key_store, primary_key(user_id), check_for_backend(diesel::pg::Pg))]
pub struct UserKeyStore {
    pub user_id: String,
    pub key: Encryption,
    pub created_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Insertable)]
#[diesel(table_name = user_key_store)]
pub struct UserKeyStoreNew {
    pub user_id: String,
    pub key: Encryption,
    pub created_at: PrimitiveDateTime,
}
