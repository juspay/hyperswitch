use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::id_type;
use diesel::{
    associations::HasTable,
    debug_query,
    pg::Pg,
    result::Error as DieselError,
    sql_types::{Bool, Nullable},
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
    fn check_user_in_lineage(
        tenant_id: id_type::TenantId,
        org_id: Option<id_type::OrganizationId>,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
    ) -> Box<
        dyn diesel::BoxableExpression<<Self as HasTable>::Table, Pg, SqlType = Nullable<Bool>>
            + 'static,
    > {
        // Checking in user roles, for a user in token hierarchy, only one of the relations will be true:
        // either tenant level, org level, merchant level, or profile level
        // Tenant-level: (tenant_id = ? && org_id = null && merchant_id = null && profile_id = null)
        // Org-level: (org_id = ? && merchant_id = null && profile_id = null)
        // Merchant-level: (org_id = ? && merchant_id = ? && profile_id = null)
        // Profile-level: (org_id = ? && merchant_id = ? && profile_id = ?)
        Box::new(
            // Tenant-level condition
            dsl::tenant_id
                .eq(tenant_id.clone())
                .and(dsl::org_id.is_null())
                .and(dsl::merchant_id.is_null())
                .and(dsl::profile_id.is_null())
                .or(
                    // Org-level condition
                    dsl::tenant_id
                        .eq(tenant_id.clone())
                        .and(dsl::org_id.eq(org_id.clone()))
                        .and(dsl::merchant_id.is_null())
                        .and(dsl::profile_id.is_null()),
                )
                .or(
                    // Merchant-level condition
                    dsl::tenant_id
                        .eq(tenant_id.clone())
                        .and(dsl::org_id.eq(org_id.clone()))
                        .and(dsl::merchant_id.eq(merchant_id.clone()))
                        .and(dsl::profile_id.is_null()),
                )
                .or(
                    // Profile-level condition
                    dsl::tenant_id
                        .eq(tenant_id)
                        .and(dsl::org_id.eq(org_id))
                        .and(dsl::merchant_id.eq(merchant_id))
                        .and(dsl::profile_id.eq(profile_id)),
                ),
        )
    }

    pub async fn find_by_user_id_tenant_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        let check_lineage = Self::check_user_in_lineage(
            tenant_id,
            Some(org_id),
            Some(merchant_id),
            Some(profile_id),
        );

        let predicate = dsl::user_id
            .eq(user_id)
            .and(check_lineage)
            .and(dsl::version.eq(version));

        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, predicate).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_by_user_id_tenant_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
        update: UserRoleUpdate,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        let check_lineage = dsl::tenant_id
            .eq(tenant_id.clone())
            .and(dsl::org_id.is_null())
            .and(dsl::merchant_id.is_null())
            .and(dsl::profile_id.is_null())
            .or(
                // Org-level condition
                dsl::tenant_id
                    .eq(tenant_id.clone())
                    .and(dsl::org_id.eq(org_id.clone()))
                    .and(dsl::merchant_id.is_null())
                    .and(dsl::profile_id.is_null()),
            )
            .or(
                // Merchant-level condition
                dsl::tenant_id
                    .eq(tenant_id.clone())
                    .and(dsl::org_id.eq(org_id.clone()))
                    .and(dsl::merchant_id.eq(merchant_id.clone()))
                    .and(dsl::profile_id.is_null()),
            )
            .or(
                // Profile-level condition
                dsl::tenant_id
                    .eq(tenant_id)
                    .and(dsl::org_id.eq(org_id))
                    .and(dsl::merchant_id.eq(merchant_id))
                    .and(dsl::profile_id.eq(profile_id)),
            );

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

    pub async fn delete_by_user_id_tenant_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        let check_lineage = Self::check_user_in_lineage(
            tenant_id,
            Some(org_id),
            Some(merchant_id),
            Some(profile_id),
        );

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
        tenant_id: id_type::TenantId,
        org_id: Option<id_type::OrganizationId>,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
        entity_id: Option<String>,
        status: Option<UserStatus>,
        version: Option<UserRoleVersion>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::user_id.eq(user_id).and(dsl::tenant_id.eq(tenant_id)))
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

    #[allow(clippy::too_many_arguments)]
    pub async fn generic_user_roles_list_for_org_and_extra(
        conn: &PgPooledConn,
        user_id: Option<String>,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: Option<id_type::MerchantId>,
        profile_id: Option<id_type::ProfileId>,
        version: Option<UserRoleVersion>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::org_id.eq(org_id).and(dsl::tenant_id.eq(tenant_id)))
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

    pub async fn list_user_roles_by_user_id_across_tenants(
        conn: &PgPooledConn,
        user_id: String,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::user_id.eq(user_id))
            .into_boxed();
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
