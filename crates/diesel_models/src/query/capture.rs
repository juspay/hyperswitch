use common_utils::types::ConnectorTransactionIdTrait;
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
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
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

impl ConnectorTransactionIdTrait for Capture {
    fn get_optional_connector_transaction_id(&self) -> Option<&String> {
        match self
            .connector_capture_id
            .as_ref()
            .map(|capture_id| capture_id.get_txn_id(self.processor_capture_data.as_ref()))
            .transpose()
        {
            Ok(capture_id) => capture_id,

            // In case hashed data is missing from DB, use the hashed ID as connector transaction ID
            Err(_) => self
                .connector_capture_id
                .as_ref()
                .map(|txn_id| txn_id.get_id()),
        }
    }
}
