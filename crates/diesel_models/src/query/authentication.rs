use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    authentication::{
        Authentication, AuthenticationNew, AuthenticationUpdate, AuthenticationUpdateInternal,
    },
    errors,
    schema::authentication::dsl,
    PgPooledConn, StorageResult,
};

impl AuthenticationNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Authentication> {
        generics::generic_insert(conn, self).await
    }
}

impl Authentication {
    pub async fn update_by_merchant_id_authentication_id(
        conn: &PgPooledConn,
        merchant_id: String,
        authentication_id: String,
        authorization_update: AuthenticationUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::authentication_id.eq(authentication_id.to_owned())),
            AuthenticationUpdateInternal::from(authorization_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => Err(error.attach_printable(
                    "Authentication with the given Authentication ID does not exist",
                )),
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                        conn,
                        dsl::merchant_id
                            .eq(merchant_id.to_owned())
                            .and(dsl::authentication_id.eq(authentication_id.to_owned())),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn find_by_merchant_id_authentication_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        authentication_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::authentication_id.eq(authentication_id.to_owned())),
        )
        .await
    }

    pub async fn find_authentication_by_merchant_id_connector_authentication_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_authentication_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_authentication_id.eq(connector_authentication_id.to_owned())),
        )
        .await
    }
}
