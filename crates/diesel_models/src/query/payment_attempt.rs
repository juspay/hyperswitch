use std::collections::HashSet;

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, pg::Pg, BoolExpressionMethods, ExpressionMethods,
    QueryDsl, Table,
};
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    enums::{self, IntentStatus},
    errors::{self, DatabaseError},
    payment_attempt::{
        PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentAttemptUpdateInternal,
    },
    query::generics::db_metrics,
    schema::payment_attempt::dsl,
    PaymentIntent, PgPooledConn, StorageResult,
};

impl PaymentAttemptNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a PaymentAttempt into the database using the provided database connection.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled Postgres connection
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the inserted `PaymentAttempt` if successful, or an error if the insertion fails.
    ///
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentAttempt> {
        generics::generic_insert(conn, self.populate_derived_fields()).await
    }
}

impl PaymentAttempt {
    #[instrument(skip(conn))]
        /// Asynchronously updates the database record with the given payment attempt ID and returns the updated record.
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
            PaymentAttemptUpdateInternal::from(payment_attempt).populate_derived_fields(&self),
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
        /// Asynchronously finds an optional record by payment_id and merchant_id in the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection.
    /// * `payment_id` - A string reference representing the payment ID.
    /// * `merchant_id` - A string reference representing the merchant ID.
    /// 
    /// # Returns
    /// 
    /// An `Option` containing the found record if it exists, or `None` if no record is found.
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
        /// Asynchronously finds a record in the database by the given connector_transaction_id, payment_id, and merchant_id.
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

        /// Finds the last successful attempt by payment ID and merchant ID. Performs ordering on the application level instead of the database level. 
    pub async fn find_last_successful_attempt_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
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

        /// Finds the last successful or partially captured attempt by payment ID and merchant ID.
    pub async fn find_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
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
                .and(
                    dsl::status
                        .eq(enums::AttemptStatus::Charged)
                        .or(dsl::status.eq(enums::AttemptStatus::PartialCharged)),
                ),
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
        /// Asynchronously finds a record in the database by the given merchant ID and connector transaction ID.
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
        /// Asynchronously finds a record in the database by the given merchant ID and attempt ID.
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
        /// Asynchronously finds a record in the database by the given merchant_id and preprocessing_id.
    /// Returns a Result containing the found record or an error.
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
        /// Asynchronously finds a record in the database with the given payment_id, merchant_id, and attempt_id.
    /// Returns a result containing the found record or an error.
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

    #[instrument(skip(conn))]
        /// Asynchronously finds records by merchant_id and payment_id in the database.
    pub async fn find_by_merchant_id_payment_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_id: &str,
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
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

        /// Retrieves filters for payments based on the given payment intents and merchant ID.
    pub async fn get_filters_for_payments(
        conn: &PgPooledConn,
        pi: &[PaymentIntent],
        merchant_id: &str,
    ) -> StorageResult<(
        Vec<String>,
        Vec<enums::Currency>,
        Vec<IntentStatus>,
        Vec<enums::PaymentMethod>,
        Vec<enums::PaymentMethodType>,
        Vec<enums::AuthenticationType>,
    )> {
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

        let filter_payment_method_type = filter
            .clone()
            .select(dsl::payment_method_type)
            .distinct()
            .get_results_async::<Option<enums::PaymentMethodType>>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)
            .attach_printable("Error filtering records by payment method type")?
            .into_iter()
            .flatten()
            .collect::<Vec<enums::PaymentMethodType>>();

        let filter_authentication_type = filter
            .clone()
            .select(dsl::authentication_type)
            .distinct()
            .get_results_async::<Option<enums::AuthenticationType>>(conn)
            .await
            .into_report()
            .change_context(DatabaseError::Others)
            .attach_printable("Error filtering records by authentication type")?
            .into_iter()
            .flatten()
            .collect::<Vec<enums::AuthenticationType>>();

        Ok((
            filter_connector,
            filter_currency,
            intent_status,
            filter_payment_method,
            filter_payment_method_type,
            filter_authentication_type,
        ))
    }
        /// Asynchronously retrieves the total count of attempts based on the provided filters and parameters.
    /// 
    /// # Arguments
    /// * `conn` - A reference to a pooled database connection
    /// * `merchant_id` - The ID of the merchant for which the attempts are being counted
    /// * `active_attempt_ids` - A slice of active attempt IDs
    /// * `connector` - An optional vector of connector strings
    /// * `payment_method` - An optional vector of payment methods
    /// * `payment_method_type` - An optional vector of payment method types
    /// * `authentication_type` - An optional vector of authentication types
    /// 
    /// # Returns
    /// A `StorageResult` containing the total count of attempts as an `i64`, or a database error if the operation fails
    pub async fn get_total_count_of_attempts(
        conn: &PgPooledConn,
        merchant_id: &str,
        active_attempt_ids: &[String],
        connector: Option<Vec<String>>,
        payment_method: Option<Vec<enums::PaymentMethod>>,
        payment_method_type: Option<Vec<enums::PaymentMethodType>>,
        authentication_type: Option<Vec<enums::AuthenticationType>>,
    ) -> StorageResult<i64> {
        let mut filter = <Self as HasTable>::table()
            .count()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .filter(dsl::attempt_id.eq_any(active_attempt_ids.to_owned()))
            .into_boxed();

        if let Some(connector) = connector.clone() {
            filter = filter.filter(dsl::connector.eq_any(connector));
        }

        if let Some(payment_method) = payment_method.clone() {
            filter = filter.filter(dsl::payment_method.eq_any(payment_method));
        }
        if let Some(payment_method_type) = payment_method_type.clone() {
            filter = filter.filter(dsl::payment_method_type.eq_any(payment_method_type));
        }
        if let Some(authentication_type) = authentication_type.clone() {
            filter = filter.filter(dsl::authentication_type.eq_any(authentication_type));
        }
        router_env::logger::debug!(query = %debug_query::<Pg, _>(&filter).to_string());

        db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            filter.get_result_async::<i64>(conn),
            db_metrics::DatabaseOperation::Filter,
        )
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error filtering count of payments")
    }
}
