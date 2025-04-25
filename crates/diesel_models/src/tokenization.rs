use common_enums;
use common_utils::{
    consts::MAX_LOCKER_ID_LENGTH,
    id_type::{GlobalTokenId, MerchantId},
    tokenization as tokenization_utils,
};
use diesel::{
    associations::HasTable,
    deserialize::FromSqlRow,
    expression::AsExpression,
    pg::Pg,
    serialize::{Output, ToSql},
    sql_types::{Jsonb, Text},
    AsChangeset, Identifiable, Insertable, Queryable, Selectable,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{query::generics, schema_v2::tokenization, PgPooledConn, StorageResult};
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

impl Tokenization {
    pub async fn find_by_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::GlobalTokenId,
    ) -> StorageResult<Self> {
        use diesel::ExpressionMethods;
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            tokenization::dsl::id.eq(id.to_owned()),
        )
        .await
    }
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
