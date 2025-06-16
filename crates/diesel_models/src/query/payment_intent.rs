use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
#[cfg(feature = "v1")]
use crate::schema::payment_intent::dsl;
#[cfg(feature = "v2")]
use crate::schema_v2::payment_intent::dsl;
use crate::{
    errors,
    payment_intent::{self, PaymentIntent, PaymentIntentNew},
    PgPooledConn, StorageResult,
};

impl PaymentIntentNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentIntent> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentIntent {
    #[cfg(feature = "v2")]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_intent_update: payment_intent::PaymentIntentUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id.to_owned(),
            payment_intent_update,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            Ok(payment_intent) => Ok(payment_intent),
        }
    }

    #[cfg(feature = "v2")]
    pub async fn find_by_global_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::GlobalPaymentId,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id.to_owned()).await
    }

    #[cfg(feature = "v1")]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_intent: payment_intent::PaymentIntentUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::payment_id
                .eq(self.payment_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            payment_intent::PaymentIntentUpdateInternal::from(payment_intent),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            Ok(mut payment_intents) => payment_intents
                .pop()
                .ok_or(error_stack::report!(errors::DatabaseError::NotFound)),
        }
    }

    #[cfg(feature = "v2")]
    pub async fn find_by_merchant_reference_id_merchant_id(
        conn: &PgPooledConn,
        merchant_reference_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_reference_id.eq(merchant_reference_id.to_owned())),
        )
        .await
    }

    // This query should be removed in the future because direct queries to the intent table without an intent ID are not allowed.
    // In an active-active setup, a lookup table should be implemented, and the merchant reference ID will serve as the idempotency key.
    #[cfg(feature = "v2")]
    pub async fn find_by_merchant_reference_id_profile_id(
        conn: &PgPooledConn,
        merchant_reference_id: &common_utils::id_type::PaymentReferenceId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::profile_id
                .eq(profile_id.to_owned())
                .and(dsl::merchant_reference_id.eq(merchant_reference_id.to_owned())),
        )
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[cfg(feature = "v2")]
    pub async fn find_optional_by_merchant_reference_id_merchant_id(
        conn: &PgPooledConn,
        merchant_reference_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_reference_id.eq(merchant_reference_id.to_owned())),
        )
        .await
    }

    #[cfg(feature = "v1")]
    pub async fn find_optional_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }
}
