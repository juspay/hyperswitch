use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    enums,
    query::generics,
    schema::dashboard_metadata::dsl,
    user::dashboard_metadata::{
        DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate,
        DashboardMetadataUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl DashboardMetadataNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<DashboardMetadata> {
        generics::generic_insert(conn, self).await
    }
}

impl DashboardMetadata {
    pub async fn update(
        conn: &PgPooledConn,
        user_id: Option<String>,
        merchant_id: String,
        org_id: String,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: DashboardMetadataUpdate,
    ) -> StorageResult<Self> {
        let predicate = dsl::merchant_id
            .eq(merchant_id.to_owned())
            .and(dsl::org_id.eq(org_id.to_owned()))
            .and(dsl::data_key.eq(data_key.to_owned()));

        if let Some(uid) = user_id {
            generics::generic_update_with_unique_predicate_get_result::<
                <Self as HasTable>::Table,
                _,
                _,
                _,
            >(
                conn,
                predicate.and(dsl::user_id.eq(uid)),
                DashboardMetadataUpdateInternal::from(dashboard_metadata_update),
            )
            .await
        } else {
            generics::generic_update_with_unique_predicate_get_result::<
                <Self as HasTable>::Table,
                _,
                _,
                _,
            >(
                conn,
                predicate.and(dsl::user_id.is_null()),
                DashboardMetadataUpdateInternal::from(dashboard_metadata_update),
            )
            .await
        }
    }

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

    pub async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }

    pub async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
        data_key: enums::DashboardMetadata,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id))
                .and(dsl::data_key.eq(data_key)),
        )
        .await
    }
}
