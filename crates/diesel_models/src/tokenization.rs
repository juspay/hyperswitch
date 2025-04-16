use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::tokenization};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Insertable, Queryable, Identifiable)]
#[diesel(table_name = tokenization)]
#[diesel(primary_key(id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tokenization {
    pub id: i32,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub flag: storage_enums::TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = tokenization)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenizationNew {
    pub locker_id: String,
    pub flag: storage_enums::TokenizationFlag,
} 