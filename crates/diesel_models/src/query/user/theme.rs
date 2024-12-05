use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::EntityType;
use common_utils::{id_type, types::theme::ThemeLineage};
use diesel::{
    associations::HasTable,
    debug_query,
    pg::Pg,
    result::Error as DieselError,
    sql_types::{Bool, Nullable},
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::{report, ResultExt};
use router_env::logger;
use strum::IntoEnumIterator;

use crate::{
    errors::DatabaseError,
    query::generics::{
        self,
        db_metrics::{track_database_call, DatabaseOperation},
    },
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

    pub async fn find_most_specific_theme_in_lineage(
        conn: &PgPooledConn,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        min_entity: EntityType,
    ) -> StorageResult<Self> {
        let query = <Self as HasTable>::table().into_boxed();

        let query = EntityType::iter().fold(query, |mut query, entity_type| {
            query = query.or_filter(Self::lineage_filter(ThemeLineage::new(
                entity_type,
                tenant_id.clone(),
                org_id.clone(),
                merchant_id.clone(),
                profile_id.clone(),
            )));
            query
        });

        logger::debug!(query = %debug_query::<Pg,_>(&query).to_string());

        let data: Vec<Self> = match track_database_call::<Self, _, _>(
            query.get_results_async(conn),
            DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                DieselError::NotFound => Err(report!(err)).change_context(DatabaseError::NotFound),
                _ => Err(report!(err)).change_context(DatabaseError::Others),
            },
        }?;

        data.into_iter()
            .filter(|theme| theme.entity_type >= min_entity)
            .min_by_key(|theme| theme.entity_type)
            .ok_or(report!(DatabaseError::NotFound))
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
