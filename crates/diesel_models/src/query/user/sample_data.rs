use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, debug_query, ExpressionMethods, TextExpressionMethods};
use error_stack::ResultExt;
use router_env::logger;

#[cfg(feature = "v1")]
use crate::schema::{
    payment_attempt::dsl as payment_attempt_dsl, payment_intent::dsl as payment_intent_dsl,
    refund::dsl as refund_dsl,
};
#[cfg(feature = "v2")]
use crate::schema_v2::{
    payment_attempt::dsl as payment_attempt_dsl, payment_intent::dsl as payment_intent_dsl,
    refund::dsl as refund_dsl,
};
use crate::{
    errors, schema::dispute::dsl as dispute_dsl, Dispute, DisputeNew, PaymentAttempt,
    PaymentIntent, PgPooledConn, Refund, RefundNew, StorageResult,
};
#[cfg(feature = "v1")]
use crate::{user, PaymentIntentNew};

#[cfg(feature = "v1")]
pub async fn insert_payment_intents(
    conn: &PgPooledConn,
    batch: Vec<PaymentIntentNew>,
) -> StorageResult<Vec<PaymentIntent>> {
    let query = diesel::insert_into(<PaymentIntent>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting payment intents")
}

#[cfg(feature = "v1")]
pub async fn insert_payment_attempts(
    conn: &PgPooledConn,
    batch: Vec<user::sample_data::PaymentAttemptBatchNew>,
) -> StorageResult<Vec<PaymentAttempt>> {
    let query = diesel::insert_into(<PaymentAttempt>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting payment attempts")
}

pub async fn insert_refunds(
    conn: &PgPooledConn,
    batch: Vec<RefundNew>,
) -> StorageResult<Vec<Refund>> {
    let query = diesel::insert_into(<Refund>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting refunds")
}

pub async fn insert_disputes(
    conn: &PgPooledConn,
    batch: Vec<DisputeNew>,
) -> StorageResult<Vec<Dispute>> {
    let query = diesel::insert_into(<Dispute>::table()).values(batch);

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while inserting disputes")
}

#[cfg(feature = "v1")]
pub async fn delete_payment_intents(
    conn: &PgPooledConn,
    merchant_id: &common_utils::id_type::MerchantId,
) -> StorageResult<Vec<PaymentIntent>> {
    let query = diesel::delete(<PaymentIntent>::table())
        .filter(payment_intent_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(payment_intent_dsl::payment_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
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

#[cfg(feature = "v2")]
pub async fn delete_payment_intents(
    conn: &PgPooledConn,
    merchant_id: &common_utils::id_type::MerchantId,
) -> StorageResult<Vec<PaymentIntent>> {
    let query = diesel::delete(<PaymentIntent>::table())
        .filter(payment_intent_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(payment_intent_dsl::merchant_reference_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
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
pub async fn delete_payment_attempts(
    conn: &PgPooledConn,
    merchant_id: &common_utils::id_type::MerchantId,
) -> StorageResult<Vec<PaymentAttempt>> {
    let query = diesel::delete(<PaymentAttempt>::table())
        .filter(payment_attempt_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(payment_attempt_dsl::payment_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
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

pub async fn delete_refunds(
    conn: &PgPooledConn,
    merchant_id: &common_utils::id_type::MerchantId,
) -> StorageResult<Vec<Refund>> {
    let query = diesel::delete(<Refund>::table())
        .filter(refund_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(refund_dsl::payment_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
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

pub async fn delete_disputes(
    conn: &PgPooledConn,
    merchant_id: &common_utils::id_type::MerchantId,
) -> StorageResult<Vec<Dispute>> {
    let query = diesel::delete(<Dispute>::table())
        .filter(dispute_dsl::merchant_id.eq(merchant_id.to_owned()))
        .filter(dispute_dsl::dispute_id.like("test_%"));

    logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

    query
        .get_results_async(conn)
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Error while deleting disputes")
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
