use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::{IntoReport, ResultExt};
use router_env::tracing::{self, instrument};

use crate::{
    dashboard_metadata::*,
    enums,
    errors::{self},
    query::generics,
    schema::dashboard_metadata::dsl,
    PgPooledConn, StorageResult,
};

impl DashboardMetadataNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<DashboardMetadata> {
        generics::generic_insert(conn, self).await
    }
    pub async fn upsert(self, conn: &PgPooledConn) -> StorageResult<DashboardMetadata> {
        let conflict_target = (dsl::merchant_id, dsl::org_id, dsl::data_key);
        let query = diesel::insert_into(<DashboardMetadata>::table())
            .values(self.clone())
            .on_conflict(conflict_target)
            .do_update()
            .set(self);
        router_env::logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg,_>(&query).to_string());
        query
            .get_result_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error while updating metadata")
    }
}

impl DashboardMetadata {
    pub async fn find_user_scoped_dashboard_metadata(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
        org_id: String,
        data_types: Vec<enums::DashboardMetadata>,
    ) -> StorageResult<Vec<Self>> {
        let predicate = dsl::user_id
            .eq(user_id)
            .and(dsl::merchant_id.eq(merchant_id))
            .and(dsl::org_id.eq(org_id))
            .and(dsl::data_key.eq_any(data_types));

        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            predicate,
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }

    pub async fn find_merchant_scoped_dashboard_metadata(
        conn: &PgPooledConn,
        merchant_id: String,
        org_id: String,
        data_types: Vec<enums::DashboardMetadata>,
    ) -> StorageResult<Vec<Self>> {
        let predicate = dsl::user_id
            .is_null()
            .and(dsl::merchant_id.eq(merchant_id))
            .and(dsl::org_id.eq(org_id))
            .and(dsl::data_key.eq_any(data_types));

        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            predicate,
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }
}
