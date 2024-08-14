use common_utils::id_type;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    enums::UserRoleVersion, query::generics, schema::user_roles::dsl, user_role::*, PgPooledConn,
    StorageResult,
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

    pub async fn update_by_user_id_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: id_type::MerchantId,
        update: UserRoleUpdate,
        version: UserRoleVersion,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id))
                .and(dsl::version.eq(version)),
            UserRoleUpdateInternal::from(update),
        )
        .await
    }

    pub async fn update_by_user_id_org_id(
        conn: &PgPooledConn,
        user_id: String,
        org_id: id_type::OrganizationId,
        update: UserRoleUpdate,
        version: UserRoleVersion,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::org_id.eq(org_id))
                .and(dsl::version.eq(version)),
            UserRoleUpdateInternal::from(update),
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
        profile_id: Option<String>,
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

    pub async fn delete_by_user_id_org_id_merchant_id_profile_id(
        conn: &PgPooledConn,
        user_id: String,
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_id: Option<String>,
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
}
