use diesel::{associations::HasTable, ExpressionMethods, PgTextExpressionMethods};
use error_stack;

use super::generics;
use crate::{
    card_issuer::{CardIssuer, NewCardIssuer, UpdateCardIssuer},
    errors,
    schema::card_issuers::dsl,
    PgPooledConn, StorageResult,
};

impl CardIssuer {
    pub async fn list_filtered(
        conn: &PgPooledConn,
        query: Option<String>,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let pattern = query.map_or("%".to_string(), |q| format!("%{}%", q));
        generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
            conn,
            dsl::issuer_name.ilike(pattern),
            limit,
            None,
            Some(dsl::issuer_name.asc()),
        )
        .await
    }

    pub async fn find_by_ids(conn: &PgPooledConn, ids: Vec<String>) -> StorageResult<Vec<Self>> {
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
        id: String,
        data: UpdateCardIssuer,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, Self>(
            conn,
            dsl::id.eq(id),
            data,
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            error_stack::report!(errors::DatabaseError::NotFound)
                .attach_printable("Card issuer not found for the given id")
        })
    }
}

impl NewCardIssuer {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<CardIssuer> {
        generics::generic_insert(conn, self).await
    }
}
