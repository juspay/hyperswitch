use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::EntityType;
use common_utils::id_type;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, result::Error as DieselError,
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::{report, ResultExt};

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
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id
                .eq(role_id.to_owned())
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

    pub async fn find_by_role_id_and_org_id(
        conn: &PgPooledConn,
        role_id: &str,
        org_id: &id_type::OrganizationId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id
                .eq(role_id.to_owned())
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

    pub async fn find_role_by_role_name_in_lineage(
        conn: &PgPooledConn,
        role_name: &str,
        entity_type: ListRolesByEntityPayload,
    ) -> StorageResult<Self> {
        let mut query = <Self as HasTable>::table().into_boxed();

        if let Some(org_id) = entity_type.get_organization_id() {
            query = query.filter(
                dsl::org_id
                    .eq(org_id)
                    .and(dsl::scope.eq(RoleScope::Organization))
                    .and(dsl::entity_type.eq_any(vec![
                        EntityType::Profile,
                        EntityType::Merchant,
                        EntityType::Organization,
                    ])),
            );
        };

        if let Some(merchant_id) = entity_type.get_merchant_id() {
            query = query.or_filter(
                dsl::merchant_id
                    .eq(merchant_id)
                    .and(dsl::scope.eq(RoleScope::Merchant))
                    .and(dsl::entity_type.eq_any(vec![EntityType::Profile, EntityType::Merchant])),
            );
        };

        if let Some(profile_id) = entity_type.get_profile_id() {
            query = query.or_filter(
                dsl::profile_id
                    .eq(profile_id)
                    .and(dsl::scope.eq(RoleScope::Profile))
                    .and(dsl::entity_type.eq(EntityType::Profile)),
            );
        };

        query = query.filter(dsl::role_name.eq(role_name.to_owned()));

        router_env::logger::debug!(query = %debug_query::<Pg,_>(&query).to_string());

        match generics::db_metrics::track_database_call::<Self, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Err(err) => match err {
                DieselError::NotFound => {
                    Err(report!(err)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
            },
            Ok(mut role_info) => role_info
                .pop()
                .ok_or(error_stack::report!(errors::DatabaseError::NotFound)),
        }
    }

    //TODO: Remove once generic_list_roles_by_entity_type is stable
    pub async fn generic_roles_list_for_org(
        conn: &PgPooledConn,
        org_id: id_type::OrganizationId,
        merchant_id: Option<id_type::MerchantId>,
        entity_type: Option<EntityType>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table()
            .filter(dsl::org_id.eq(org_id))
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
        entity_type: ListRolesByEntityPayload,
        is_lineage_data_required: bool,
        limit: Option<u32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = <Self as HasTable>::table().into_boxed();

        match entity_type {
            ListRolesByEntityPayload::Organization(org_id) => {
                let entity_in_vec = if is_lineage_data_required {
                    vec![
                        EntityType::Organization,
                        EntityType::Merchant,
                        EntityType::Profile,
                    ]
                } else {
                    vec![EntityType::Organization]
                };
                query = query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(
                        dsl::scope
                            .eq(RoleScope::Organization)
                            .or(dsl::scope.eq(RoleScope::Merchant))
                            .or(dsl::scope.eq(RoleScope::Profile)),
                    )
                    .filter(dsl::entity_type.eq_any(entity_in_vec))
            }

            ListRolesByEntityPayload::Merchant(org_id, merchant_id) => {
                let entity_in_vec = if is_lineage_data_required {
                    vec![EntityType::Merchant, EntityType::Profile]
                } else {
                    vec![EntityType::Merchant]
                };
                query = query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(
                        dsl::scope
                            .eq(RoleScope::Organization)
                            .or(dsl::scope
                                .eq(RoleScope::Merchant)
                                .and(dsl::merchant_id.eq(merchant_id.clone())))
                            .or(dsl::scope
                                .eq(RoleScope::Profile)
                                .and(dsl::merchant_id.eq(merchant_id))),
                    )
                    .filter(dsl::entity_type.eq_any(entity_in_vec))
            }

            ListRolesByEntityPayload::Profile(org_id, merchant_id, profile_id) => {
                let entity_in_vec = vec![EntityType::Profile];
                query = query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(
                        dsl::scope
                            .eq(RoleScope::Organization)
                            .or(dsl::scope
                                .eq(RoleScope::Merchant)
                                .and(dsl::merchant_id.eq(merchant_id.clone())))
                            .or(dsl::scope
                                .eq(RoleScope::Profile)
                                .and(dsl::merchant_id.eq(merchant_id))
                                .and(dsl::profile_id.eq(profile_id))),
                    )
                    .filter(dsl::entity_type.eq_any(entity_in_vec))
            }
        };

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
