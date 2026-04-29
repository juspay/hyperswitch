use common_utils::id_type;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::{report, ResultExt};

use crate::{
    enums, errors,
    query::generics,
    schema::dashboard_metadata::dsl,
    user::dashboard_metadata::{
        DashboardMetadata, DashboardMetadataNew, DashboardMetadataUpdate,
        DashboardMetadataUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

macro_rules! dashboard_metadata_filter {
    ($user_id:expr, $profile_id:expr, $merchant_id:expr, $org_id:expr, $data_key:expr, $predicate:ident, $action:block) => {
        match ($user_id, $profile_id) {
            (Some(uid), Some(pid)) => {
                let $predicate = dsl::user_id
                    .eq(uid.to_owned())
                    .and(dsl::profile_id.eq(pid.to_owned()))
                    .and(dsl::merchant_id.eq($merchant_id.to_owned()))
                    .and(dsl::org_id.eq($org_id.to_owned()))
                    .and(dsl::data_key.eq($data_key));
                $action
            }
            (Some(uid), None) => {
                let $predicate = dsl::user_id
                    .eq(uid.to_owned())
                    .and(dsl::profile_id.is_null())
                    .and(dsl::merchant_id.eq($merchant_id.to_owned()))
                    .and(dsl::org_id.eq($org_id.to_owned()))
                    .and(dsl::data_key.eq($data_key));
                $action
            }
            (None, Some(pid)) => {
                let $predicate = dsl::user_id
                    .is_null()
                    .and(dsl::profile_id.eq(pid.to_owned()))
                    .and(dsl::merchant_id.eq($merchant_id.to_owned()))
                    .and(dsl::org_id.eq($org_id.to_owned()))
                    .and(dsl::data_key.eq($data_key));
                $action
            }
            (None, None) => {
                let $predicate = dsl::user_id
                    .is_null()
                    .and(dsl::profile_id.is_null())
                    .and(dsl::merchant_id.eq($merchant_id.to_owned()))
                    .and(dsl::org_id.eq($org_id.to_owned()))
                    .and(dsl::data_key.eq($data_key));
                $action
            }
        }
    };
}

impl DashboardMetadataNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<DashboardMetadata> {
        generics::generic_insert(conn, self).await
    }
}

impl DashboardMetadata {
    pub async fn update(
        conn: &PgPooledConn,
        user_id: Option<String>,
        merchant_id: id_type::MerchantId,
        org_id: id_type::OrganizationId,
        profile_id: Option<String>,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: DashboardMetadataUpdate,
    ) -> StorageResult<Self> {
        let changeset = DashboardMetadataUpdateInternal::from(dashboard_metadata_update);

        dashboard_metadata_filter!(
            user_id.as_ref(),
            profile_id.as_ref(),
            merchant_id,
            org_id,
            data_key,
            predicate,
            {
                generics::generic_update_with_unique_predicate_get_result::<
                    <Self as HasTable>::Table,
                    _,
                    _,
                    _,
                >(conn, predicate, changeset)
                .await
                .map_err(|e| match e.current_context() {
                    errors::DatabaseError::NotFound => report!(errors::DatabaseError::NotFound),
                    _ => e,
                })
                .attach_printable("Error while updating dashboard metadata")
            }
        )
    }

    pub async fn find_user_scoped_dashboard_metadata(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: id_type::MerchantId,
        org_id: id_type::OrganizationId,
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
        merchant_id: id_type::MerchantId,
        org_id: id_type::OrganizationId,
        data_types: Vec<enums::DashboardMetadata>,
    ) -> StorageResult<Vec<Self>> {
        let predicate = dsl::merchant_id
            .eq(merchant_id)
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

    pub async fn find_dashboard_metadata_by_user_merchant_org_profile_key(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: id_type::MerchantId,
        org_id: id_type::OrganizationId,
        profile_id: Option<String>,
        data_key: enums::DashboardMetadata,
    ) -> StorageResult<Option<Self>> {
        dashboard_metadata_filter!(
            Some(&user_id),
            profile_id.as_ref(),
            merchant_id,
            org_id,
            data_key,
            predicate,
            {
                generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
                    conn, predicate,
                )
                .await
            }
        )
    }

    pub async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: id_type::MerchantId,
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
        merchant_id: id_type::MerchantId,
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

    pub async fn delete_all_by_user_id(
        conn: &PgPooledConn,
        user_id: String,
    ) -> StorageResult<bool> {
        match generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::user_id.eq(user_id),
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => Ok(true),
                _ => Err(error),
            },
        }
    }
}
