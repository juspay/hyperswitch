use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    enums, org_authentication_method::*, query::generics, schema::org_authentication_methods::dsl,
    PgPooledConn, StorageResult,
};

impl OrgAuthenticationMethodNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<OrgAuthenticationMethod> {
        generics::generic_insert(conn, self).await
    }
}

impl OrgAuthenticationMethod {
    pub async fn get_org_authentication_methods_details(
        conn: &PgPooledConn,
        org_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::org_id.eq(org_id.to_owned()),
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }

    pub async fn update_org_authentication_method(
        conn: &PgPooledConn,
        org_id: &str,
        auth_method: enums::AuthMethod,
        org_authentication_method_update: OrgAuthenticationMethodUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::org_id
                .eq(org_id.to_owned())
                .and(dsl::auth_method.eq(auth_method)),
            OrgAuthenticationMethodUpdateInternal::from(org_authentication_method_update),
        )
        .await
    }
}
