use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::id_type;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, result::Error as DieselError,
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::{report, ResultExt};

use crate::{
    enums::{UserRoleVersion, UserStatus},
    errors,
    query::generics,
    schema::user_roles::dsl,
    user_role::*,
    PgPooledConn, StorageResult,
};

impl UserRoleNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<UserRole> {
        generics::generic_insert(conn, self).await
    }
}

impl UserRole {
    pub async fn find_by_user_id(
        conn: &PgPooledConn,
        user_id: String,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id.eq(user_id).and(dsl::version.eq(version)),
        )
        .await
    }

    pub async fn find_by_user_id_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: id_type::MerchantId,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id))
                .and(dsl::version.eq(version)),
        )
        .await
    }

    pub async fn list_by_user_id(
        conn: &PgPooledConn,
        user_id: String,
        version: UserRoleVersion,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::user_id.eq(user_id).and(dsl::version.eq(version)),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }

    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: id_type::MerchantId,
        version: UserRoleVersion,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id)
                .and(dsl::version.eq(version)),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }

    pub async fn find_by_user_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: Option<id_type::ProfileId>,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        // Checking in user roles, for a user in token hierarchy, only one of the relation will be true, either org level, merchant level or profile level
        // (org_id = ? && merchant_id = null && profile_id = null)  || (org_id = ? && merchant_id = ? && profile_id = null) || (org_id = ? && merchant_id = ? && profile_id = ?)
        let check_lineage = dsl::org_id
            .eq(org_id.clone())
            .and(dsl::merchant_id.is_null().and(dsl::profile_id.is_null()))
            .or(dsl::org_id.eq(org_id.clone()).and(
                dsl::merchant_id
                    .eq(merchant_id.clone())
                    .and(dsl::profile_id.is_null()),
            ))
            .or(dsl::org_id.eq(org_id).and(
                dsl::merchant_id
                    .eq(merchant_id)
                    //TODO: In case of None, profile_id = NULL its unexpected behaviour, after V1 profile id will not be option
                    .and(dsl::profile_id.eq(profile_id)),
            ));

        let predicate = dsl::user_id
            .eq(user_id)
            .and(check_lineage)
            .and(dsl::version.eq(version));

        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, predicate).await
    }

    pub async fn update_by_user_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        org_id: id_type::OrganizationId,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
        update: UserRoleUpdate,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        // Checking in user roles, for a user in token hierarchy, only one of the relation will be true, either org level, merchant level or profile level
        // (org_id = ? && merchant_id = null && profile_id = null)  || (org_id = ? && merchant_id = ? && profile_id = null) || (org_id = ? && merchant_id = ? && profile_id = ?)
        let check_lineage = dsl::org_id
            .eq(org_id.clone())
            .and(dsl::merchant_id.is_null().and(dsl::profile_id.is_null()))
            .or(dsl::org_id.eq(org_id.clone()).and(
                dsl::merchant_id
                    .eq(merchant_id.clone())
                    .and(dsl::profile_id.is_null()),
            ))
            .or(dsl::org_id.eq(org_id).and(
                dsl::merchant_id
                    .eq(merchant_id)
                    //TODO: In case of None, profile_id = NULL its unexpected behaviour, after V1 profile id will not be option
                    .and(dsl::profile_id.eq(profile_id)),
            ));

        let predicate = dsl::user_id
            .eq(user_id)
            .and(check_lineage)
            .and(dsl::version.eq(version));

        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            UserRoleUpdateInternal,
            _,
            _,
        >(conn, predicate, update.into())
        .await
    }

    pub async fn delete_by_user_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: Option<id_type::ProfileId>,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        // Checking in user roles, for a user in token hierarchy, only one of the relation will be true, either org level, merchant level or profile level
        // (org_id = ? && merchant_id = null && profile_id = null)  || (org_id = ? && merchant_id = ? && profile_id = null) || (org_id = ? && merchant_id = ? && profile_id = ?)
        let check_lineage = dsl::org_id
            .eq(org_id.clone())
            .and(dsl::merchant_id.is_null().and(dsl::profile_id.is_null()))
            .or(dsl::org_id.eq(org_id.clone()).and(
                dsl::merchant_id
                    .eq(merchant_id.clone())
                    .and(dsl::profile_id.is_null()),
            ))
            .or(dsl::org_id.eq(org_id).and(
                dsl::merchant_id
                    .eq(merchant_id)
                    //TODO: In case of None, profile_id = NULL its unexpected behaviour, after V1 profile id will not be option
                    .and(dsl::profile_id.eq(profile_id)),
            ));

        let predicate = dsl::user_id
            .eq(user_id)
            .and(check_lineage)
            .and(dsl::version.eq(version));

        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(conn, predicate)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn generic_user_roles_list_for_user(
        conn: &PgPooledConn,
        user_id: String,
        org_id: Option<id_type::OrganizationId>,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
        entity_id: Option<String>,
        status: Option<UserStatus>,
        version: Option<UserRoleVersion>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::user_id.eq(user_id))
            .into_boxed();

        if let Some(org_id) = org_id {
            query = query.filter(dsl::org_id.eq(org_id));
        }

        if let Some(merchant_id) = merchant_id {
            query = query.filter(dsl::merchant_id.eq(merchant_id));
        }

        if let Some(profile_id) = profile_id {
            query = query.filter(dsl::profile_id.eq(profile_id));
        }

        if let Some(entity_id) = entity_id {
            query = query.filter(dsl::entity_id.eq(entity_id));
        }

        if let Some(version) = version {
            query = query.filter(dsl::version.eq(version));
        }

        if let Some(status) = status {
            query = query.filter(dsl::status.eq(status));
        }

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        router_env::logger::debug!(query = %debug_query::<Pg,_>(&query).to_string());

        match generics::db_metrics::track_database_call::<Self, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                DieselError::NotFound => {
                    Err(report!(err)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
            },
        }
    }

    pub async fn generic_user_roles_list_for_org_and_extra(
        conn: &PgPooledConn,
        user_id: Option<String>,
        org_id: id_type::OrganizationId,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
        version: Option<UserRoleVersion>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::org_id.eq(org_id))
            .into_boxed();

        if let Some(user_id) = user_id {
            query = query.filter(dsl::user_id.eq(user_id));
        }

        if let Some(merchant_id) = merchant_id {
            query = query.filter(dsl::merchant_id.eq(merchant_id));
        }

        if let Some(profile_id) = profile_id {
            query = query.filter(dsl::profile_id.eq(profile_id));
        }

        if let Some(version) = version {
            query = query.filter(dsl::version.eq(version));
        }

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        router_env::logger::debug!(query = %debug_query::<Pg,_>(&query).to_string());

        match generics::db_metrics::track_database_call::<Self, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                DieselError::NotFound => {
                    Err(report!(err)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
            },
        }
    }
}
