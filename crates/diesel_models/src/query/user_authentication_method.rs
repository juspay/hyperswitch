use diesel::{associations::HasTable, ExpressionMethods};

use crate::{
    query::generics, schema::user_authentication_methods::dsl, user_authentication_method::*,
    PgPooledConn, StorageResult,
};

impl UserAuthenticationMethodNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<UserAuthenticationMethod> {
        generics::generic_insert(conn, self).await
    }
}

impl UserAuthenticationMethod {
    pub async fn list_user_authentication_methods_for_auth_id(
        conn: &PgPooledConn,
        auth_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::auth_id.eq(auth_id.to_owned()),
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }

    pub async fn list_user_authentication_methods_for_owner_id(
        conn: &PgPooledConn,
        owner_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::owner_id.eq(owner_id.to_owned()),
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }

    pub async fn update_user_authentication_method(
        conn: &PgPooledConn,
        id: &str,
        user_authentication_method_update: UserAuthenticationMethodUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::id.eq(id.to_owned()),
            OrgAuthenticationMethodUpdateInternal::from(user_authentication_method_update),
        )
        .await
    }
}
