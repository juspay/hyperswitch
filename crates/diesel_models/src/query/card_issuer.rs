use common_utils::id_type;
use diesel::{associations::HasTable, ExpressionMethods, PgTextExpressionMethods};

use super::generics;
use crate::{
    card_issuer::{CardIssuer, NewCardIssuer, UpdateCardIssuer},
    schema::card_issuers::dsl,
    PgPooledConn, StorageResult,
};

impl CardIssuer {
    pub async fn list_filtered(
        conn: &PgPooledConn,
        query: Option<String>,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let pattern = query.map_or("%".to_string(), |q| format!("{}%", q));
        generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
            conn,
            dsl::issuer_name.ilike(pattern),
            limit,
            None,
            Some(dsl::issuer_name.asc()),
        )
        .await
    }

    pub async fn find_by_ids(
        conn: &PgPooledConn,
        ids: Vec<id_type::CardIssuerId>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as diesel::Table>::PrimaryKey,
            Self,
        >(conn, dsl::id.eq_any(ids), None, None, None)
        .await
    }

    pub async fn update(
        conn: &PgPooledConn,
        id: id_type::CardIssuerId,
        data: UpdateCardIssuer,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(conn, id, data).await
    }
}

impl NewCardIssuer {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<CardIssuer> {
        generics::generic_insert(conn, self).await
    }
}
