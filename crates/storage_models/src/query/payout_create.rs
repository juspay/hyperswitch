use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use error_stack::report;
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    payout_create::{PayoutCreate, PayoutCreateNew, PayoutCreateUpdate, PayoutCreateUpdateInternal},
    schema::payout_create::dsl,
    PgPooledConn, StorageResult,
};

impl PayoutCreateNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PayoutCreate> {
        generics::generic_insert(conn, self).await
    }
}

impl PayoutCreate {
    pub async fn find_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payout_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_merchant_id_customer_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        customer_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::customer_id.eq(customer_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn update_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payout_id: &str,
        payout: PayoutCreateUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
            PayoutCreateUpdateInternal::from(payout),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound).attach_printable("Error while updating payout")
        })
    }

    #[instrument(skip(conn))]
    pub async fn delete_by_payout_id(
        conn: &PgPooledConn,
        payout_id: String,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self>(
            conn,
            dsl::payout_id.eq(payout_id),
        )
        .await
    }
}
