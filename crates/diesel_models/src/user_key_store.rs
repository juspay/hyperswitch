use diesel::{Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, schema::user_key_store};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Identifiable, Queryable)]
#[diesel(table_name = user_key_store)]
#[diesel(primary_key(user_id))]
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
