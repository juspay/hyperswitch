use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    capture::{Capture, CaptureNew, CaptureUpdate, CaptureUpdateInternal},
    errors,
    schema::captures::dsl,
    PgPooledConn, StorageResult,
};

impl CaptureNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Capture> {
        generics::generic_insert(conn, self).await
    }
}

impl Capture {
    pub async fn find_by_capture_id(conn: &PgPooledConn, capture_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::capture_id.eq(capture_id.to_owned()),
        )
        .await
    }

    pub async fn update_with_capture_id(
        self,
        conn: &PgPooledConn,
        capture: CaptureUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::capture_id.eq(self.capture_id.to_owned()),
            CaptureUpdateInternal::from(capture),
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

    pub async fn find_all_by_merchant_id_payment_id_authorized_attempt_id(
        merchant_id: &str,
        payment_id: &str,
        authorized_attempt_id: &str,
        conn: &PgPooledConn,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::authorized_attempt_id
                .eq(authorized_attempt_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
