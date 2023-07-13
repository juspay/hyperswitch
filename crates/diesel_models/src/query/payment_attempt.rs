use std::collections::HashSet;

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, QueryDsl, Table};
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    enums::{self, IntentStatus},
    errors::{self, DatabaseError},
    payment_attempt::{
        PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentAttemptUpdateInternal,
        PaymentListFilters,
    },
    payment_intent::PaymentIntent,
    schema::payment_attempt::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentAttemptNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentAttempt> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentAttempt {
    #[instrument(skip(conn))]
    pub async fn update_with_attempt_id(
        self,
        conn: &PgPooledConn,
        payment_attempt: PaymentAttemptUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::attempt_id
                .eq(self.attempt_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            PaymentAttemptUpdateInternal::from(payment_attempt),
        )
        .await
        {
            Err(error) => match error.current_context() {
                DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
    pub async fn find_optional_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_connector_transaction_id_payment_id_merchant_id(
        conn: &PgPooledConn,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::connector_transaction_id
                .eq(connector_transaction_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned()))
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    pub async fn find_last_successful_attempt_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        // perform ordering on the application level instead of database level
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            Self,
        >(
            conn,
            dsl::payment_id
                .eq(payment_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::status.eq(enums::AttemptStatus::Charged)),
            None,
            None,
            None,
        )
        .await?
        .into_iter()
        .fold(
            Err(DatabaseError::NotFound).into_report(),
            |acc, cur| match acc {
                Ok(value) if value.modified_at > cur.modified_at => Ok(value),
                _ => Ok(cur),
            },
        )
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_connector_txn_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_txn_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_transaction_id.eq(connector_txn_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_attempt_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        attempt_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::attempt_id.eq(attempt_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_merchant_id_preprocessing_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        preprocessing_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::preprocessing_step_id.eq(preprocessing_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id_attempt_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id.eq(payment_id.to_owned()).and(
                dsl::merchant_id
                    .eq(merchant_id.to_owned())
                    .and(dsl::attempt_id.eq(attempt_id.to_owned())),
            ),
        )
        .await
    }
    pub async fn get_filters_for_payments(
        conn: &PgPooledConn,
        pi: &[PaymentIntent],
        merchant_id: &str,
    ) -> StorageResult<PaymentListFilters> {
        let active_attempts: Vec<String> = pi
            .iter()
            .map(|payment_intent| payment_intent.clone().active_attempt_id)
            .collect();

        let filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .filter(dsl::attempt_id.eq_any(active_attempts));

        let intent_status: Vec<IntentStatus> = pi
            .iter()
            .map(|payment_intent| payment_intent.status)
            .collect::<HashSet<IntentStatus>>()
            .into_iter()
            .collect();

        let filter_connector = filter
            .clone()
            .select(dsl::connector)
            .distinct()
            .get_results_async::<Option<String>>(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by connector")?
            .into_iter()
            .flatten()
            .collect::<Vec<String>>();

        let filter_currency = filter
            .clone()
            .select(dsl::currency)
            .distinct()
            .get_results_async::<Option<enums::Currency>>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)
            .attach_printable("Error filtering records by currency")?
            .into_iter()
            .flatten()
            .collect::<Vec<enums::Currency>>();

        let filter_payment_method = filter
            .clone()
            .select(dsl::payment_method)
            .distinct()
            .get_results_async::<Option<enums::PaymentMethod>>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)
            .attach_printable("Error filtering records by payment method")?
            .into_iter()
            .flatten()
            .collect::<Vec<enums::PaymentMethod>>();

        let filters = PaymentListFilters {
            connector: filter_connector,
            currency: filter_currency,
            status: intent_status,
            payment_method: filter_payment_method,
        };

        Ok(filters)
    }
}
