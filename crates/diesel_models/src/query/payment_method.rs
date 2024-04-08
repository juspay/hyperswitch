use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods,
    QueryDsl, Table,
};
use error_stack::ResultExt;

use super::generics;
use crate::{
    enums as storage_enums, errors,
    payment_method::{self, PaymentMethod, PaymentMethodNew},
    schema::payment_methods::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentMethodNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentMethod> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentMethod {
    pub async fn delete_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: String,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self>(
            conn,
            dsl::payment_method_id.eq(payment_method_id),
        )
        .await
    }

    pub async fn delete_by_merchant_id_payment_method_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_method_id.eq(payment_method_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_locker_id(conn: &PgPooledConn, locker_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::locker_id.eq(locker_id.to_owned()),
        )
        .await
    }

    pub async fn find_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_method_id.eq(payment_method_id.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            limit,
            None,
            Some(dsl::last_used_at.desc()),
        )
        .await
    }

    pub async fn get_count_by_customer_id_merchant_id_status(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
        status: common_enums::PaymentMethodStatus,
    ) -> StorageResult<i64> {
        let filter = <Self as HasTable>::table()
            .count()
            .filter(
                dsl::customer_id
                    .eq(customer_id.to_owned())
                    .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                    .and(dsl::status.eq(status.to_owned())),
            )
            .into_boxed();

        router_env::logger::debug!(query = %debug_query::<Pg, _>(&filter).to_string());

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_result_async::<i64>(conn),
            generics::db_metrics::DatabaseOperation::Count,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to get a count of payment methods")
    }

    pub async fn find_by_customer_id_merchant_id_status(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
        status: storage_enums::PaymentMethodStatus,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::status.eq(status)),
            limit,
            None,
            Some(dsl::last_used_at.desc()),
        )
        .await
    }

    pub async fn update_with_payment_method_id(
        self,
        conn: &PgPooledConn,
        payment_method: payment_method::PaymentMethodUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::payment_method_id.eq(self.payment_method_id.to_owned()),
            payment_method::PaymentMethodUpdateInternal::from(payment_method),
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

}
