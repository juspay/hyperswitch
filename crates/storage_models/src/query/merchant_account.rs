use diesel::{associations::HasTable, ExpressionMethods};
use router_env::tracing::{self, instrument};

use super::generics::{self, ExecuteQuery};
use crate::{
    errors,
    merchant_account::{
        MerchantAccount, MerchantAccountNew, MerchantAccountUpdate, MerchantAccountUpdateInternal,
    },
    schema::merchant_account::dsl,
    CustomResult, PgPooledConn,
};

impl MerchantAccountNew {
    #[instrument(skip(conn))]
    pub async fn insert(
        self,
        conn: &PgPooledConn,
    ) -> CustomResult<MerchantAccount, errors::DatabaseError> {
        generics::generic_insert::<_, _, MerchantAccount, _>(conn, self, ExecuteQuery::new()).await
    }
}

impl MerchantAccount {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<Self, errors::DatabaseError> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, Self, _>(
            conn,
            self.id,
            MerchantAccountUpdateInternal::from(merchant_account),
            ExecuteQuery::new(),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn delete_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::DatabaseError> {
        generics::generic_delete::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            ExecuteQuery::<Self>::new(),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_api_key(
        conn: &PgPooledConn,
        api_key: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::api_key.eq(api_key.to_owned()),
        )
        .await
    }

    #[instrument(skip_all)]
    pub async fn find_by_publishable_key(
        conn: &PgPooledConn,
        publishable_key: &str,
    ) -> CustomResult<Self, errors::DatabaseError> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::publishable_key.eq(publishable_key.to_owned()),
        )
        .await
    }
}
