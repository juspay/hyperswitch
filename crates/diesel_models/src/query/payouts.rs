use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;

use super::generics;
use crate::{
    errors,
    payouts::{Payouts, PayoutsNew, PayoutsUpdate, PayoutsUpdateInternal},
    schema::payouts::dsl,
    PgPooledConn, StorageResult,
};

impl PayoutsNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Payouts> {
        generics::generic_insert(conn, self).await
    }
}
impl Payouts {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payout_update: PayoutsUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::payout_id
                .eq(self.payout_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            PayoutsUpdateInternal::from(payout_update),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            Ok(mut payouts) => payouts
                .pop()
                .ok_or(error_stack::report!(errors::DatabaseError::NotFound)),
        }
    }

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

    pub async fn update_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payout_id: &str,
        payout: PayoutsUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
            PayoutsUpdateInternal::from(payout),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound).attach_printable("Error while updating payout")
        })
    }
}
