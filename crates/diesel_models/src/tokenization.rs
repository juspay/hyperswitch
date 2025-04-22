
use time::PrimitiveDateTime;
use serde::{Deserialize, Serialize};
use crate::{
    schema_v2::tokenization,
    PgPooledConn, StorageResult,
    query::generics,
};
use common_utils::{
    consts::MAX_LOCKER_ID_LENGTH,
    id_type::{GlobalTokenId, MerchantId},
    tokenization as tokenization_utils,
};
use diesel::{
    deserialize::FromSqlRow, 
    expression::AsExpression, 
    sql_types::{Jsonb, Text},
    pg::Pg,
    serialize::{ToSql, Output},
    AsChangeset, Identifiable, Insertable, Queryable, Selectable
};

use common_enums;
#[derive(Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = tokenization)]
pub struct Tokenization {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub locker_id: String,
    pub flag: common_enums::enums::TokenizationFlag,
    pub version: common_enums::enums::ApiVersion,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = tokenization)]
pub struct TokenizationNew {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub version: common_enums::enums::ApiVersion,
    pub flag: common_enums::enums::TokenizationFlag,
}

impl Tokenization {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_insert(conn, self).await
    }
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = tokenization)]
pub struct TokenizationUpdate {
    pub updated_at: Option<PrimitiveDateTime>,
    pub version: Option<common_enums::enums::ApiVersion>,
    pub flag: Option<common_enums::enums::TokenizationFlag>,
}

// Add this to your TokenizationFlag enum definition
// #[derive(
//     Clone,
//     Copy,
//     Debug,
//     Eq,
//     PartialEq,
//     serde::Deserialize,
//     serde::Serialize,
//     AsExpression
// )]

// #[router_derive::diesel_enum(storage_type = "db_enum")]
// pub enum TokenizationFlag {
//     Enabled,
//     Disabled,
// }


// impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::pg::Pg> for TokenizationFlag {
//     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
//         match *self {
//             TokenizationFlag::Enabled => out.write_all(b"enabled")?,
//             TokenizationFlag::Disabled => out.write_all(b"disabled")?,
//         }
//         Ok(diesel::serialize::IsNull::No)
//     }
// }


// impl diesel::expression::QueryFragment for TokenizationFlag {
//     fn walk_ast<'b>(&'b self, mut out: diesel::pg::PgAstPass<'_, 'b>) -> diesel::QueryResult<()> {
//         let s = match *self {
//             TokenizationFlag::Enabled => "enabled",
//             TokenizationFlag::Disabled => "disabled",
//         };
//         out.push_bind_param::<diesel::sql_types::Text, &str>(&s)
//     }
// }