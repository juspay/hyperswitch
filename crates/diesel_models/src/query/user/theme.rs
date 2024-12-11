use common_utils::types::theme::ThemeLineage;
use diesel::{
    associations::HasTable,
    pg::Pg,
    sql_types::{Bool, Nullable},
    BoolExpressionMethods, ExpressionMethods,
};

use crate::{
    query::generics,
    schema::themes::dsl,
    user::theme::{Theme, ThemeNew},
    PgPooledConn, StorageResult,
};

impl ThemeNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Theme> {
        generics::generic_insert(conn, self).await
    }
}

impl Theme {
    fn lineage_filter(
        lineage: ThemeLineage,
    ) -> Box<
        dyn diesel::BoxableExpression<<Self as HasTable>::Table, Pg, SqlType = Nullable<Bool>>
            + 'static,
    > {
        match lineage {
            // TODO: Add back Tenant variant when we introduce Tenant Variant in EntityType
            // ThemeLineage::Tenant { tenant_id } => Box::new(
            //     dsl::tenant_id
            //         .eq(tenant_id)
            //         .and(dsl::org_id.is_null())
            //         .and(dsl::merchant_id.is_null())
            //         .and(dsl::profile_id.is_null())
            //         .nullable(),
            // ),
            ThemeLineage::Organization { tenant_id, org_id } => Box::new(
                dsl::tenant_id
                    .eq(tenant_id)
                    .and(dsl::org_id.eq(org_id))
                    .and(dsl::merchant_id.is_null())
                    .and(dsl::profile_id.is_null()),
            ),
            ThemeLineage::Merchant {
                tenant_id,
                org_id,
                merchant_id,
            } => Box::new(
                dsl::tenant_id
                    .eq(tenant_id)
                    .and(dsl::org_id.eq(org_id))
                    .and(dsl::merchant_id.eq(merchant_id))
                    .and(dsl::profile_id.is_null()),
            ),
            ThemeLineage::Profile {
                tenant_id,
                org_id,
                merchant_id,
                profile_id,
            } => Box::new(
                dsl::tenant_id
                    .eq(tenant_id)
                    .and(dsl::org_id.eq(org_id))
                    .and(dsl::merchant_id.eq(merchant_id))
                    .and(dsl::profile_id.eq(profile_id)),
            ),
        }
    }

    pub async fn find_by_theme_id(conn: &PgPooledConn, theme_id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::theme_id.eq(theme_id),
        )
        .await
    }

    pub async fn find_by_lineage(
        conn: &PgPooledConn,
        lineage: ThemeLineage,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            Self::lineage_filter(lineage),
        )
        .await
    }

    pub async fn delete_by_theme_id_and_lineage(
        conn: &PgPooledConn,
        theme_id: String,
        lineage: ThemeLineage,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::theme_id
                .eq(theme_id)
                .and(Self::lineage_filter(lineage)),
        )
        .await
    }
}
