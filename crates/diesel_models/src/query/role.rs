use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::EntityType;
use common_utils::id_type;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, result::Error as DieselError,
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::{report, ResultExt};
use strum::IntoEnumIterator;

use crate::{
    enums::RoleScope, errors, query::generics, role::*, schema::roles::dsl, PgPooledConn,
    StorageResult,
};

impl RoleNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Role> {
        generics::generic_insert(conn, self).await
    }
}

impl Role {
    fn get_entity_list(
        current_entity: EntityType,
        is_lineage_data_required: bool,
    ) -> Vec<EntityType> {
        is_lineage_data_required
            .then(|| {
                EntityType::iter()
                    .filter(|variant| *variant <= current_entity)
                    .collect()
            })
            .unwrap_or(vec![current_entity])
    }

    pub async fn find_by_role_id(conn: &PgPooledConn, role_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id.eq(role_id.to_owned()),
        )
        .await
    }

    pub async fn find_by_role_id_in_lineage(
        conn: &PgPooledConn,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        profile_id: &id_type::ProfileId,
        tenant_id: &id_type::TenantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id
                .eq(role_id.to_owned())
                .and(dsl::tenant_id.eq(tenant_id.to_owned()))
                .and(dsl::org_id.eq(org_id.to_owned()))
                .and(
                    dsl::scope
                        .eq(RoleScope::Organization)
                        .or(dsl::merchant_id
                            .eq(merchant_id.to_owned())
                            .and(dsl::scope.eq(RoleScope::Merchant)))
                        .or(dsl::profile_id
                            .eq(profile_id.to_owned())
                            .and(dsl::scope.eq(RoleScope::Profile))),
                ),
        )
        .await
    }

    pub async fn find_by_role_id_org_id_tenant_id(
        conn: &PgPooledConn,
        role_id: &str,
        org_id: &id_type::OrganizationId,
        tenant_id: &id_type::TenantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id
                .eq(role_id.to_owned())
                .and(dsl::tenant_id.eq(tenant_id.to_owned()))
                .and(dsl::org_id.eq(org_id.to_owned())),
        )
        .await
    }

    pub async fn update_by_role_id(
        conn: &PgPooledConn,
        role_id: &str,
        role_update: RoleUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::role_id.eq(role_id.to_owned()),
            RoleUpdateInternal::from(role_update),
        )
        .await
    }

    pub async fn delete_by_role_id(conn: &PgPooledConn, role_id: &str) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id.eq(role_id.to_owned()),
        )
        .await
    }

    //TODO: Remove once generic_list_roles_by_entity_type is stable
    pub async fn generic_roles_list_for_org(
        conn: &PgPooledConn,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
        merchant_id: Option<id_type::MerchantId>,
        entity_type: Option<EntityType>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::tenant_id.eq(tenant_id).and(dsl::org_id.eq(org_id)))
            .into_boxed();

        if let Some(merchant_id) = merchant_id {
            query = query.filter(
                (dsl::merchant_id
                    .eq(merchant_id)
                    .and(dsl::scope.eq(RoleScope::Merchant)))
                .or(dsl::scope.eq(RoleScope::Organization)),
            );
        }

        if let Some(entity_type) = entity_type {
            query = query.filter(dsl::entity_type.eq(entity_type))
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

    pub async fn generic_list_roles_by_entity_type(
        conn: &PgPooledConn,
        payload: ListRolesByEntityPayload,
        is_lineage_data_required: bool,
        tenant_id: id_type::TenantId,
        org_id: id_type::OrganizationId,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .into_boxed()
            .filter(dsl::tenant_id.eq(tenant_id))
            .filter(dsl::org_id.eq(org_id));

        match payload {
            ListRolesByEntityPayload::Organization => {
                let entity_in_vec =
                    Self::get_entity_list(EntityType::Organization, is_lineage_data_required);
                query = query.filter(dsl::entity_type.eq_any(entity_in_vec))
            }

            ListRolesByEntityPayload::Merchant(merchant_id) => {
                let entity_in_vec =
                    Self::get_entity_list(EntityType::Merchant, is_lineage_data_required);
                query = query
                    .filter(
                        dsl::scope
                            .eq(RoleScope::Organization)
                            .or(dsl::merchant_id.eq(merchant_id)),
                    )
                    .filter(dsl::entity_type.eq_any(entity_in_vec))
            }

            ListRolesByEntityPayload::Profile(merchant_id, profile_id) => {
                let entity_in_vec =
                    Self::get_entity_list(EntityType::Profile, is_lineage_data_required);
                query = query
                    .filter(
                        dsl::scope
                            .eq(RoleScope::Organization)
                            .or(dsl::scope
                                .eq(RoleScope::Merchant)
                                .and(dsl::merchant_id.eq(merchant_id.clone())))
                            .or(dsl::profile_id.eq(profile_id)),
                    )
                    .filter(dsl::entity_type.eq_any(entity_in_vec))
            }
        };

        router_env::logger::debug!(query = %debug_query::<Pg,_>(&query).to_string());

        match generics::db_metrics::track_database_call::<Self, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => Err(report!(err)).change_context(errors::DatabaseError::Others),
        }
    }
}
