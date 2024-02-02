use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, debug_query, ExpressionMethods, TextExpressionMethods};
use error_stack::{IntoReport, ResultExt};
use router_env::logger;

use crate::{
    errors,
    schema::{
        payment_attempt::dsl as payment_attempt_dsl, payment_intent::dsl as payment_intent_dsl,
        refund::dsl as refund_dsl,
    },
    user::sample_data::PaymentAttemptBatchNew,
    PaymentAttempt, PaymentIntent, PaymentIntentNew, PgPooledConn, Refund, RefundNew,
    StorageResult,
};

/// Inserts a batch of new payment intents into the database and returns a vector of the inserted payment intents.
pub async fn insert_payment_intents(
    conn: &PgPooledConn,
    batch: Vec<PaymentIntentNew>,
) -> StorageResult<Vec<PaymentIntent>> {
    let query = diesel::insert_into(<PaymentIntent>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting payment intents")
}
/// Asynchronously inserts a batch of payment attempts into the database using the provided connection.
/// 
/// # Arguments
/// 
/// * `conn` - A reference to a pooled database connection
/// * `batch` - A vector of PaymentAttemptBatchNew structs containing the payment attempts to be inserted
/// 
/// # Returns
/// 
/// A Result containing a vector of PaymentAttempt structs if the insertion is successful, or a StorageError if an error occurs
pub async fn insert_payment_attempts(
    conn: &PgPooledConn,
    batch: Vec<PaymentAttemptBatchNew>,
) -> StorageResult<Vec<PaymentAttempt>> {
    let query = diesel::insert_into(<PaymentAttempt>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting payment attempts")
}

/// Asynchronously inserts a batch of refunds into the database using the provided connection.
/// 
/// # Arguments
/// 
/// * `conn` - A reference to a pooled PostgreSQL connection.
/// * `batch` - A vector of `RefundNew` instances to be inserted into the database.
/// 
/// # Returns
/// 
/// A `StorageResult` containing a vector of `Refund` instances if successful, otherwise an error.
pub async fn insert_refunds(
    conn: &PgPooledConn,
    batch: Vec<RefundNew>,
) -> StorageResult<Vec<Refund>> {
    let query = diesel::insert_into(<Refund>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting refunds")
}

/// Deletes payment intents for a specific merchant from the database.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `merchant_id` - The ID of the merchant whose payment intents are to be deleted
///
/// # Returns
///
/// A `StorageResult` containing a vector of `PaymentIntent` if successful, an error otherwise
///
pub async fn delete_payment_intents(
    conn: &PgPooledConn,
    merchant_id: &str,
) -> StorageResult<Vec<PaymentIntent>> {
    let query = diesel::delete(<PaymentIntent>::table())
        .filter(payment_intent_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(payment_intent_dsl::payment_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting payment intents")
        .and_then(|result| match result.len() {
            n if n > 0 => {
                logger::debug!("{n} records deleted");
                Ok(result)
            }
            0 => Err(error_stack::report!(errors::DatabaseError::NotFound)
                .attach_printable("No records deleted")),
            _ => Ok(result),
        })
}
/// Deletes payment attempts for a specific merchant from the database based on the provided merchant_id.
pub async fn delete_payment_attempts(
    conn: &PgPooledConn,
    merchant_id: &str,
) -> StorageResult<Vec<PaymentAttempt>> {
    let query = diesel::delete(<PaymentAttempt>::table())
        .filter(payment_attempt_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(payment_attempt_dsl::payment_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting payment attempts")
        .and_then(|result| match result.len() {
            n if n > 0 => {
                logger::debug!("{n} records deleted");
                Ok(result)
            }
            0 => Err(error_stack::report!(errors::DatabaseError::NotFound)
                .attach_printable("No records deleted")),
            _ => Ok(result),
        })
}

/// Deletes refunds from the database that match the given merchant ID and have a payment ID starting with "test_".
/// Returns a vector of the deleted refund records.
pub async fn delete_refunds(conn: &PgPooledConn, merchant_id: &str) -> StorageResult<Vec<Refund>> {
    let query = diesel::delete(<Refund>::table())
        .filter(refund_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(refund_dsl::payment_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .into_report()
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting refunds")
        .and_then(|result| match result.len() {
            n if n > 0 => {
                logger::debug!("{n} records deleted");
                Ok(result)
            }
            0 => Err(error_stack::report!(errors::DatabaseError::NotFound)
                .attach_printable("No records deleted")),
            _ => Ok(result),
        })
}
