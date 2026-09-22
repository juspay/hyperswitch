use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::id_type;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::ResultExt;

use super::generics;
use crate::{
    card_issuer::{CardIssuer, CardIssuerListItem, NewCardIssuer, UpdateCardIssuer},
    errors,
    schema::card_issuers::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl CardIssuer {
    pub async fn list_all(
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<Vec<CardIssuerListItem>> {
        dsl::card_issuers
            .select((dsl::id, dsl::issuer_name))
            .order(dsl::issuer_name.asc())
            .get_results_async::<CardIssuerListItem>(conn.raw_connection())
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error while listing card issuers")
    }

    pub async fn find_by_ids(
        conn: &DatabaseConnectionWithContext<'_>,
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
        conn: &DatabaseConnectionWithContext<'_>,
        id: id_type::CardIssuerId,
        data: UpdateCardIssuer,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(conn, id, data).await
    }

    pub async fn delete_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: id_type::CardIssuerId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::id.eq(id)).await
    }
}

impl NewCardIssuer {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<CardIssuer> {
        generics::generic_insert(conn, self).await
    }
}
